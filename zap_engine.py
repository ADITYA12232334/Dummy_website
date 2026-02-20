import time
from zapv2 import ZAPv2


class ZapEngine:
    def __init__(self, proxy, api_key=None, threads_per_host=None, concurrent_hosts=None):
        self.zap = ZAPv2(
            apikey=api_key,
            proxies={"http": proxy, "https": proxy},
        )
        if threads_per_host or concurrent_hosts:
            self.configure_scanner(threads_per_host, concurrent_hosts)

    def configure_scanner(self, threads_per_host=None, concurrent_hosts=None):
        """Configure ZAP scanner concurrency settings."""
        try:
            if threads_per_host is not None:
                print(f"[*] ZapEngine: Setting threads per host to {threads_per_host}")
                self.zap.ascan.set_option_thread_per_host(int(threads_per_host))
            
            if concurrent_hosts is not None:
                print(f"[*] ZapEngine: Setting concurrent hosts to {concurrent_hosts}")
                self.zap.ascan.set_option_host_per_scan(int(concurrent_hosts))
        except Exception as e:
            print(f"[-] ZapEngine: Failed to configure scanner: {e}")

    def _wait_spider(self, scan_id, timeout):
        start = time.time()
        while True:
            progress = int(self.zap.spider.status(scan_id))
            if progress >= 100:
                return
            if time.time() - start > timeout:
                raise TimeoutError("Spider timeout")
            time.sleep(2)

    def _wait_active(self, scan_id, timeout):
        start = time.time()
        while True:
            progress = int(self.zap.ascan.status(scan_id))
            if progress >= 100:
                return
            if time.time() - start > timeout:
                raise TimeoutError("Active scan timeout")
            time.sleep(5)

    def wait_passive(self, timeout):
        start = time.time()
        while int(self.zap.pscan.records_to_scan) > 0:
            if time.time() - start > timeout:
                raise TimeoutError("Passive scan timeout")
            time.sleep(2)

    def scan_target_full(self, target, max_duration):
        # Create a unique context for this target to isolate it
        context_name = f"context_{int(time.time()*1000)}"
        context_id = self.zap.context.new_context(context_name)
        
        try:
            # Include the target in the context
            self.zap.context.include_in_context(context_name, f"{target}.*")
            
            self.zap.urlopen(target)
            time.sleep(2)

            spider_id = self.zap.spider.scan(target, contextname=context_name)
            self._wait_spider(spider_id, max_duration)

            self.wait_passive(max_duration)

            ascan_id = self.zap.ascan.scan(target, contextid=context_id)
            self._wait_active(ascan_id, max_duration)
            
            return context_name
        except Exception:
            self.cleanup_context(context_name)
            raise

    def export_report(self, filename, context_name=None):
        # Using context-aware report generation if possible, else standard report
        # Note: zap.core.htmlreport() generates for everything. 
        # For true production, one might use zap.reports.generate
        report = self.zap.core.htmlreport()
        with open(filename, "w") as f:
            f.write(report)

    def cleanup_context(self, context_name):
        try:
            self.zap.context.remove_context(context_name)
        except:
            pass

    def shutdown(self):
        """Stop all active scans and cleanup."""
        print("[*] ZapEngine: Stopping all active scans...")
        try:
            # Stop all active scans
            for scan in self.zap.ascan.scans:
                self.zap.ascan.stop(scan['id'])
            
            # Stop all spider scans
            for scan in self.zap.spider.scans:
                self.zap.spider.stop(scan['id'])
                
            # Cleanup all contexts
            for context in self.zap.context.context_list:
                self.zap.context.remove_context(context)
                
            print("[+] ZapEngine: Shutdown complete.")
        except Exception as e:
            print(f"[-] ZapEngine: Error during shutdown: {e}")
