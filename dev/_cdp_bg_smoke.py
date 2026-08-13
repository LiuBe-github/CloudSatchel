# -*- coding: utf-8 -*-
"""背景图片功能的 WebView2 CDP 无头冒烟（不含原生文件对话框，那需要真人点选）。"""
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
        ok &= check("get_state 含背景字段", isinstance(st, dict) and "backgroundImagePath" in st, str(st))

        # 设置一个背景并立即读回，验证 set_background + read_background_image 链路
        st = cdp.eval(
            "window.__TAURI_INTERNALS__.invoke('set_background', { settings: "
            "{ imagePath: 'C:/nonexistent-bg.png', fit: 'cover', dim: 0.3, blur: 4, scale: 1.2, positionX: 30, positionY: 60 } })"
        )
        ok &= check("set_background 生效", isinstance(st, dict) and st.get("backgroundDim") == 0.3, str(st))

        data = cdp.eval(
            "window.__TAURI_INTERNALS__.invoke('read_background_image', { path: 'C:/nonexistent-bg.png' })"
        )
        ok &= check("read_background_image 文件不存在返回 null", data is None, str(data))

        # 生成一张真实的小 PNG 并验证 data URL 返回
        png = (
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        )
        import base64
        test_dir = os.path.join(os.environ.get("LOCALAPPDATA", os.environ.get("TEMP", ".")), "CloudSatchel", "backgrounds")
        os.makedirs(test_dir, exist_ok=True)
        test_path = os.path.join(test_dir, "bg-smoke.png")
        with open(test_path, "wb") as f:
            f.write(base64.b64decode(png))
        data = cdp.eval(
            f"window.__TAURI_INTERNALS__.invoke('read_background_image', {{ path: {test_path!r} }})"
        )
        ok &= check("read_background_image 返回 data URL", isinstance(data, str) and data.startswith("data:image/png;base64,"), str(data)[:80])

        # 设置面板里应出现「背景图片」区块
        has_label = cdp.eval(
            "Array.from(document.querySelectorAll('*')).some(el => el.textContent === '背景图片')"
        )
        ok &= check("设置面板含背景图片区块", has_label is True, str(has_label))

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
    print("\n=== BG CDP SMOKE RESULT:", "PASS" if ok else "FAIL", "===")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
