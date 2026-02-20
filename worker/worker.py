import os
import socket
import traceback
import time
import signal
import threading
import requests
from concurrent.futures import ThreadPoolExecutor
from redis_client import RedisClient
from zap_engine import ZapEngine


WORKER_ID = socket.gethostname()
ZAP_PROXY = os.getenv("ZAP_PROXY", "http://127.0.0.1:8090")
MAX_WORKERS = int(os.getenv("MAX_WORKERS", "5"))
ZAP_THREADS_PER_HOST = int(os.getenv("ZAP_THREADS_PER_HOST", "10"))
ZAP_CONCURRENT_HOSTS = int(os.getenv("ZAP_CONCURRENT_HOSTS", "1"))


class ZapWorker:
    def __init__(self):
        self.redis = RedisClient()
        self.zap = ZapEngine(
            proxy=ZAP_PROXY, 
            threads_per_host=ZAP_THREADS_PER_HOST,
            concurrent_hosts=ZAP_CONCURRENT_HOSTS
        )
        self.executor = ThreadPoolExecutor(max_workers=MAX_WORKERS)
        self.running = True
        self.active_jobs = {}
        self._setup_signals()

    def _setup_signals(self):
        signal.signal(signal.SIGINT, self.stop)
        signal.signal(signal.SIGTERM, self.stop)

    def stop(self, signum=None, frame=None):
        if not self.running:
            return
            
        print(f"\n[!] Shutdown signal received. Draining {len(self.active_jobs)} jobs...")
        self.running = False
        
        # Shutdown executor and wait for jobs to finish
        self.executor.shutdown(wait=True)
        
        # Cleanup ZAP engine
        self.zap.shutdown()
        
        print("[+] All jobs finished and ZAP cleaned up. Exiting.")

    def process_job(self, job):
        job_id = job["job_id"]
        config_name = job.get("config_name", "Unknown")
        targets = job.get("targets", [])
        options = job.get("options", {})
        max_duration = options.get("max_duration", 1800)

        self.active_jobs[job_id] = job
        self.redis.set_inflight(job_id, WORKER_ID)
        self.redis.update_status(
            job_id, 
            state="running", 
            started_at=time.time(),
            worker_id=WORKER_ID,
            config_name=config_name
        )

        try:
            for idx, target in enumerate(targets, 1):
                if not self.running:
                    break
                    
                self.redis.update_status(
                    job_id,
                    progress=f"{idx}/{len(targets)}",
                    current_target=target,
                )

                context_name = self.zap.scan_target_full(target, max_duration)

                os.makedirs("reports", exist_ok=True)
                report_file = os.path.join("reports", f"{job_id}_{idx}.html")
                
                self.zap.export_report(report_file, context_name=context_name)
                
                # Fetch alerts and summarize
                alerts = self.zap.zap.core.alerts(baseurl=target)
                summary = {"high": 0, "medium": 0, "low": 0, "informational": 0}
                for alert in alerts:
                    risk = alert.get("risk", "").lower()
                    if risk in summary:
                        summary[risk] += 1
                
                # Report result to backend
                try:
                    backend_url = os.getenv("BACKEND_URL", "http://localhost:3000")
                    res_payload = {
                        "config_name": config_name,
                        "url": target,
                        "total_vulnerabilities": len(alerts),
                        "high_sev": summary["high"],
                        "medium_sev": summary["medium"],
                        "low_sev": summary["low"],
                        "info_sev": summary["informational"],
                        "report_path": os.path.abspath(report_file)
                    }
                    requests.post(f"{backend_url}/api/jobs/{job_id}/results", json=res_payload)
                except Exception as e:
                    print(f"[-] Failed to report result to backend: {e}")

                self.zap.cleanup_context(context_name)
                self.redis.append_result(job_id, report_file)

            state = "completed" if self.running else "aborted"
            self.redis.update_status(job_id, state=state, finished_at=time.time())

            # Successful completion cleanup
            if self.running:
                self.redis.clear_inflight(job_id)
                self.redis.r.hdel("scan:job_details", job_id)

        except Exception as e:
            error_msg = str(e)
            print(f"[-] Job {job_id} failed: {error_msg}")
            self.redis.handle_job_failure(job_id, error_msg)
        finally:
            self.active_jobs.pop(job_id, None)


    def run(self):
        print(f"[+] Worker started: {WORKER_ID} (Concurrency: {MAX_WORKERS})")
        
        # Start heartbeat thread
        def heartbeat():
            while self.running:
                self.redis.set_heartbeat(WORKER_ID)
                time.sleep(30)
        
        threading.Thread(target=heartbeat, daemon=True).start()

        while self.running:
            try:
                job = self.redis.get_job(timeout=5)
                if not job:
                    continue

                print(f"[*] Picking up job: {job['job_id']}")
                self.executor.submit(self.process_job, job)
            except Exception as e:
                if self.running:
                    print(f"[-] Error fetching job: {e}")
                time.sleep(2)


if __name__ == "__main__":
    worker = ZapWorker()
    worker.run()
