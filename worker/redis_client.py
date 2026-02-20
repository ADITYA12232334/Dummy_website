import redis
import json
import os
import time

REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379/0")
MAX_RETRIES = int(os.getenv("MAX_RETRIES", "3"))

class RedisClient:
    def __init__(self):
        self.r = redis.from_url(REDIS_URL, decode_responses=True)

    def enqueue_job(self, job):
        job_id = job.get("job_id")
        if job_id:
            if "retry_count" not in job:
                job["retry_count"] = 0
            self.r.hset("scan:job_details", job_id, json.dumps(job))
        self.r.lpush("scan:jobs", json.dumps(job))

    def get_job(self, timeout=5):
        result = self.r.brpop("scan:jobs", timeout=timeout)
        if not result:
            return None
        _, raw = result
        return json.loads(raw)

    def set_inflight(self, job_id, worker_id):
        self.r.hset("scan:inflight", job_id, worker_id)

    def clear_inflight(self, job_id):
        self.r.hdel("scan:inflight", job_id)

    def update_status(self, job_id, **fields):
        key = f"scan:status:{job_id}"
        fields["last_updated"] = time.time()
        self.r.hset(key, mapping=fields)

    def append_result(self, job_id, value):
        key = f"scan:results:{job_id}"
        self.r.rpush(key, value)
        
    def set_heartbeat(self, worker_id):
        self.r.setex(f"worker:heartbeat:{worker_id}", 60, time.time())

    def check_and_recover_jobs(self):
        """Check for dead workers and re-enqueue their in-flight jobs."""
        inflight_jobs = self.r.hgetall("scan:inflight")
        recovered_count = 0

        for job_id, worker_id in inflight_jobs.items():
            # Check if worker is alive
            if not self.r.exists(f"worker:heartbeat:{worker_id}"):
                print(f"[!] Worker {worker_id} is dead. Recovering job {job_id}...")
                
                # Fetch job details
                raw_job = self.r.hget("scan:job_details", job_id)
                if raw_job:
                    job = json.loads(raw_job)
                    job["retry_count"] = job.get("retry_count", 0) + 1
                    
                    if job["retry_count"] > MAX_RETRIES:
                        self.move_to_dlq(job_id, f"Max retries exceeded (dead worker {worker_id})")
                    else:
                        # Re-enqueue
                        self.r.hset("scan:job_details", job_id, json.dumps(job))
                        self.r.lpush("scan:jobs", json.dumps(job))
                        # Remove from inflight
                        self.r.hdel("scan:inflight", job_id)
                        recovered_count += 1
                else:
                    print(f"[-] Missing details for job {job_id}, cannot recover.")
        
        return recovered_count

    def move_to_dlq(self, job_id, reason):
        """Move a job to the Dead Letter Queue."""
        print(f"[!] Moving job {job_id} to DLQ. Reason: {reason}")
        
        raw_job = self.r.hget("scan:job_details", job_id)
        if raw_job:
            job = json.loads(raw_job)
            job["dlq_reason"] = reason
            job["failed_at"] = time.time()
            
            # Push to DLQ list
            self.r.lpush("scan:dlq", json.dumps(job))
            
            # Update status
            self.update_status(job_id, state="dead_letter", dlq_reason=reason)
            
            # Cleanup
            self.r.hdel("scan:inflight", job_id)
            self.r.hdel("scan:job_details", job_id)
        else:
            print(f"[-] Cannot move job {job_id} to DLQ: details missing.")

    def handle_job_failure(self, job_id, error_message):
        """Handle a job failure from a worker, including retries and DLQ."""
        raw_job = self.r.hget("scan:job_details", job_id)
        if not raw_job:
            print(f"[-] Cannot handle failure for job {job_id}: details missing.")
            return

        job = json.loads(raw_job)
        job["retry_count"] = job.get("retry_count", 0) + 1
        job["last_error"] = error_message
        
        if job["retry_count"] > MAX_RETRIES:
            self.move_to_dlq(job_id, f"Max retries exceeded. Last error: {error_message}")
        else:
            print(f"[*] Job {job_id} failed. Retrying ({job['retry_count']}/{MAX_RETRIES})...")
            # Update details and re-enqueue
            self.r.hset("scan:job_details", job_id, json.dumps(job))
            self.r.lpush("scan:jobs", json.dumps(job))
            # Status update
            self.update_status(job_id, state="retry_queued", error=error_message)
            # Cleanup inflight
            self.r.hdel("scan:inflight", job_id)
