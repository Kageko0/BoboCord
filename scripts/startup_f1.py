"""Measure process launch -> renderer ready, including CDP probe overhead.
Usage: python scripts/startup_f1.py target/debug/bobocord-desktop.exe target/f1-idle
Only accepts data directories inside this workspace's target/.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from desktop_smoke import DevTools, HELPERS
from f1_smoke import close_window


def main():
    binary = Path(sys.argv[1]).resolve()
    data = Path(sys.argv[2]).resolve()
    root = Path(__file__).resolve().parents[1]
    assert data.is_relative_to(root / "target"), "Use an isolated target/ data directory"
    environment = os.environ.copy()
    port = int(environment.get("BOBOCORD_TEST_DEBUG_PORT", "9237"))
    environment["BOBOCORD_DATA_DIR"] = str(data)
    environment["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = f"--remote-debugging-port={port}"
    started = time.perf_counter()
    with open(root / "target/startup-stderr.log", "w", encoding="utf-8") as log:
        process = subprocess.Popen([str(binary)], env=environment, stdout=subprocess.DEVNULL, stderr=log, creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            tools = DevTools(port)
            tools.evaluate(f"(async()=>{{{HELPERS}}})()")
            elapsed = (time.perf_counter() - started) * 1000
            print(json.dumps({"binary": str(binary), "data": str(data), "startup_to_ready_ms_including_probe": round(elapsed, 2), "rows": tools.evaluate("document.querySelectorAll('.message-row').length")}))
        finally:
            if process.poll() is None:
                close_window(process.pid)
                process.wait(timeout=10)


if __name__ == "__main__":
    main()
