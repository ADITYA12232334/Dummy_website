from zap_engine import ZapEngine
import os
import sys

# Ensure we use the correct proxy
ZAP_PROXY = os.getenv("ZAP_PROXY", "http://127.0.0.1:8090")

print(f"[*] Initializing ZapEngine with proxy: {ZAP_PROXY}")
try:
    # Initialize with specific settings
    engine = ZapEngine(
        proxy=ZAP_PROXY,
        threads_per_host=7,  # Unusual number to verify it's set
        concurrent_hosts=2
    )

    # Verify settings via API
    threads = int(engine.zap.ascan.option_thread_per_host)
    hosts = int(engine.zap.ascan.option_host_per_scan)

    print(f"[+] Current ZAP Settings:")
    print(f"    Threads per host: {threads}")
    print(f"    Concurrent hosts: {hosts}")

    if threads == 7 and hosts == 2:
        print("[SUCCESS] Settings applied correctly!")
    else:
        print("[FAILURE] Settings did not match expected values.")
        sys.exit(1)

except Exception as e:
    print(f"[-] Error during verification: {e}")
    sys.exit(1)
