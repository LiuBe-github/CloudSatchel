# -*- coding: utf-8 -*-
"""主机性能监控功能的 WebView2 CDP 无头冒烟。"""
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from _cdp_verify_tray import Cdp, check, find_page  # noqa: E402

EXE = os.path.join(HERE, "..", "src-tauri", "target", "release", "CloudSatchel.exe")
PORT = 9333


def cleanup_existing():
    subprocess.run(["taskkill", "/IM", "CloudSatchel.exe", "/F"], capture_output=True, check=False)
    time.sleep(0.5)


def main():
    if not os.path.exists(EXE):
        print("exe 不存在:", EXE)
        return 1
    cleanup_existing()
    env = dict(os.environ)
    env["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = (
        f"--remote-debugging-port={PORT} --remote-allow-origins=*"
    )
    proc = subprocess.Popen([EXE], env=env)
    ok = True
    cdp = None
    try:
        url = find_page()
        ok &= check("CDP 页面可达", url is not None)
        if not url:
            return 1
        cdp = Cdp(url)
        time.sleep(0.6)

        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('get_state')")
        ok &= check("get_state 含 performanceMonitor", isinstance(st, dict) and "performanceMonitor" in st, str(st))

        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('set_performance_monitor', { enabled: true })")
        ok &= check("set_performance_monitor 开启", isinstance(st, dict) and st.get("performanceMonitor") is True, str(st))

        time.sleep(2.2)
        snap = cdp.eval("window.__TAURI_INTERNALS__.invoke('get_perf_snapshot')")
        ok &= check(
            "get_perf_snapshot 返回采样",
            isinstance(snap, dict)
            and isinstance(snap.get("cpu"), dict)
            and isinstance(snap.get("memory"), dict)
            and isinstance(snap.get("network"), dict),
            str(snap),
        )
        if isinstance(snap, dict):
            cpu = snap.get("cpu") or {}
            ok &= check("CPU 使用率数值存在", isinstance(cpu.get("usage"), (int, float)), str(cpu))
            ok &= check("逻辑处理器数大于 0", isinstance(cpu.get("logicalProcessorCount"), int) and cpu.get("logicalProcessorCount") > 0, str(cpu))

        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('set_performance_monitor', { enabled: false })")
        ok &= check("set_performance_monitor 关闭", isinstance(st, dict) and st.get("performanceMonitor") is False, str(st))
        snap = cdp.eval("window.__TAURI_INTERNALS__.invoke('get_perf_snapshot')")
        ok &= check("关闭后快照清空", snap is None, str(snap))

        cdp.eval("window.__TAURI_INTERNALS__.invoke('quit_app')")
        for _ in range(30):
            if proc.poll() is not None:
                break
            time.sleep(0.3)
        ok &= check("quit_app 后进程退出", proc.poll() is not None)
    finally:
        if proc.poll() is None:
            proc.terminate()
        if cdp:
            try:
                cdp.sock.close()
            except Exception:
                pass
    print("\n=== PERF CDP SMOKE RESULT:", "PASS" if ok else "FAIL", "===")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
