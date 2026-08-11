# -*- coding: utf-8 -*-
"""WebView2 CDP 无头冒烟：验证图标双击修复版可启动、钩子可重复启停、退出正常。

沙箱桌面看不到 GUI，也无法真实双击桌面图标（无 Progman），
因此只验证运行态：set_enabled 启停 WH_MOUSE_LL 钩子线程不崩溃、进程正常退出。
注意：本脚本用与 _cdp_verify_tray.py 相同的 9333 端口（已在本机验证可用），
且启动前会先清理残留 AsYouWishToolBox 实例（单实例应用，残留会导致新实例直接退出）。
"""
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from _cdp_verify_tray import Cdp, check, find_page  # noqa: E402

EXE = os.path.join(HERE, "..", "src-tauri", "target", "release", "AsYouWishToolBox.exe")
PORT = 9333  # 与 _cdp_verify_tray 一致（导入的 find_page/Cdp 内部也用 9333）


def cleanup_existing():
    """结束残留实例，避免单实例互斥量让新实例立即退出。"""
    subprocess.run(
        ["taskkill", "/IM", "AsYouWishToolBox.exe", "/F"],
        capture_output=True,
        check=False,
    )
    time.sleep(0.5)


def main():
    if not os.path.exists(EXE):
        print(f"exe 不存在: {EXE}")
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
        time.sleep(0.5)

        ok &= check("Tauri 桥接存在", cdp.eval("window.__TAURI_INTERNALS__ !== undefined"))
        state = cdp.eval("window.__TAURI_INTERNALS__.invoke('get_state')")
        ok &= check("get_state 正常", isinstance(state, dict) and "enabled" in state, str(state))

        # 启用图标功能（启动 WH_MOUSE_LL 钩子线程）
        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('set_enabled', { enabled: true })")
        time.sleep(0.6)
        ok &= check("启用后进程存活", proc.poll() is None)
        ok &= check("启用状态返回", isinstance(st, dict) and st.get("enabled") is True, str(st))

        # 停用（停止钩子并恢复图标）
        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('set_enabled', { enabled: false })")
        time.sleep(0.6)
        ok &= check("停用后进程存活", proc.poll() is None)
        ok &= check("停用状态返回", isinstance(st, dict) and st.get("enabled") is False, str(st))

        # 再次启用，确保钩子可重复启停
        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('set_enabled', { enabled: true })")
        time.sleep(0.6)
        ok &= check("再次启用进程存活", proc.poll() is None)

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
    print("\n=== CDP SMOKE RESULT:", "PASS" if ok else "FAIL", "===")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
