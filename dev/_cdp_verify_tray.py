# -*- coding: utf-8 -*-
"""WebView2 CDP 无头验证：托盘后台运行 + 关闭询问弹窗。

沙箱桌面看不到 GUI，用远程调试端口检查 DOM 与 Tauri 命令：
  1. 关闭窗口 → 弹出「最小化到托盘 / 直接退出」询问
  2. 点「最小化到托盘」→ 窗口隐藏、进程保持
  3. 后端 quit_app → 进程退出
"""
import base64
import json
import os
import pathlib
import socket
import struct
import subprocess
import sys
import time
import urllib.request

EXE = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "src-tauri", "target", "release", "CloudSatchel.exe",
)
PORT = 9333


class Cdp:
    def __init__(self, url):
        self.sock = socket.create_connection(("127.0.0.1", PORT), timeout=10)
        path = url.split(str(PORT))[1]
        key = base64.b64encode(os.urandom(16)).decode()
        req = (
            f"GET {path} HTTP/1.1\r\n"
            f"Host: 127.0.0.1:{PORT}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(req.encode())
        self.buf = b""
        while b"\r\n\r\n" not in self.buf:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("socket closed during handshake")
            self.buf += chunk
        head, _, rest = self.buf.partition(b"\r\n\r\n")
        assert b"101" in head, head[:100]
        self.buf = rest
        self.msg_id = 0

    def _recv_exact(self, n):
        data = b""
        while len(data) < n:
            chunk = self.sock.recv(n - len(data))
            if not chunk:
                raise ConnectionError("socket closed")
            data += chunk
        return data

    def _send_frame(self, payload):
        mask = os.urandom(4)
        header = bytearray([0x81])
        n = len(payload)
        if n < 126:
            header.append(0x80 | n)
        elif n < 65536:
            header.append(0x80 | 126)
            header += struct.pack(">H", n)
        else:
            header.append(0x80 | 127)
            header += struct.pack(">Q", n)
        masked = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
        self.sock.sendall(bytes(header) + mask + masked)

    def _recv_frame(self):
        while True:
            h = self._recv_exact(2)
            fin_op = h[0]
            opcode = fin_op & 0x0F
            mask_bit = h[1] >> 7
            n = h[1] & 0x7F
            if n == 126:
                n = struct.unpack(">H", self._recv_exact(2))[0]
            elif n == 127:
                n = struct.unpack(">Q", self._recv_exact(8))[0]
            mask = self._recv_exact(4) if mask_bit else b""
            payload = self._recv_exact(n)
            if mask:
                payload = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
            if opcode == 1:
                return payload
            if opcode == 8:
                raise ConnectionError("websocket closed")

    def eval(self, expr, await_promise=True):
        self.msg_id += 1
        msg = {
            "id": self.msg_id,
            "method": "Runtime.evaluate",
            "params": {
                "expression": expr,
                "returnByValue": True,
                "awaitPromise": await_promise,
            },
        }
        self._send_frame(json.dumps(msg).encode())
        while True:
            data = json.loads(self._recv_frame().decode())
            if data.get("id") == self.msg_id:
                if "exceptionDetails" in data.get("result", {}):
                    raise RuntimeError(json.dumps(data["result"]["exceptionDetails"], ensure_ascii=False))
                return data["result"].get("result", {}).get("value")


def find_page():
    for _ in range(60):
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/list", timeout=2) as r:
                pages = json.load(r)
            for p in pages:
                if p.get("type") == "page" and p.get("webSocketDebuggerUrl"):
                    return p["webSocketDebuggerUrl"]
        except Exception:
            pass
        time.sleep(0.5)
    return None


def check(name, cond, extra=""):
    tag = "PASS" if cond else "FAIL"
    print(f"[{tag}] {name} {extra}", flush=True)
    return cond


def main():
    if not os.path.exists(EXE):
        print(f"exe 不存在: {EXE}")
        return 1

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
        ok &= check("get_state 正常", isinstance(state, dict) and state.get("theme") == "system", str(state))
        ok &= check("get_state 含 autostart", isinstance(state, dict) and "autostart" in state)
        ok &= check("标题栏使用新图标", cdp.eval("document.querySelector('.brand-img') !== null"))

        # 开机自启动：开启 → 启动文件夹出现快捷方式；关闭 → 快捷方式删除
        startup_dir = (
            pathlib.Path(os.environ["APPDATA"])
            / "Microsoft" / "Windows" / "Start Menu" / "Programs" / "Startup"
        )
        lnk = startup_dir / "云笈.lnk"
        if lnk.exists():
            lnk.unlink()
        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('set_autostart', { enabled: true })")
        ok &= check("开启自启动", isinstance(st, dict) and st.get("autostart") is True, str(st))
        ok &= check("启动文件夹出现快捷方式", lnk.exists(), str(lnk))
        st = cdp.eval("window.__TAURI_INTERNALS__.invoke('set_autostart', { enabled: false })")
        ok &= check("关闭自启动", isinstance(st, dict) and st.get("autostart") is False, str(st))
        ok &= check("快捷方式已删除", not lnk.exists())

        # 关闭窗口 → 前端应弹出询问
        cdp.eval("window.__TAURI_INTERNALS__.invoke('close_window')")
        time.sleep(1.0)
        dialog = cdp.eval("document.querySelector('.dialog-card') !== null")
        ok &= check("关闭时弹出询问", dialog)
        if dialog:
            title = cdp.eval("document.querySelector('.dialog-title')?.textContent")
            ok &= check("询问标题正确", title == "关闭云笈？", str(title))

            # 点「最小化到托盘」→ 窗口隐藏、进程存活
            cdp.eval("document.querySelectorAll('.dialog-btn.primary')[0].click()")
            time.sleep(0.8)
            ok &= check("最小化到托盘后进程存活", proc.poll() is None)

        # 直接退出（后端恢复后结束进程）
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
    print("\n=== CDP VERIFY RESULT:", "PASS" if ok else "FAIL", "===")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
