# -*- coding: utf-8 -*-
"""开发辅助：全屏截图（ctypes GDI，零依赖）。用法: python _screenshot.py out.png"""
import ctypes
import struct
import sys
import zlib
from ctypes import wintypes

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32

SRCCOPY = 0x00CC0020


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", wintypes.DWORD), ("biWidth", wintypes.LONG), ("biHeight", wintypes.LONG),
        ("biPlanes", wintypes.WORD), ("biBitCount", wintypes.WORD), ("biCompression", wintypes.DWORD),
        ("biSizeImage", wintypes.DWORD), ("biXPelsPerMeter", wintypes.LONG),
        ("biYPelsPerMeter", wintypes.LONG), ("biClrUsed", wintypes.DWORD),
        ("biClrImportant", wintypes.DWORD),
    ]


def _write_png(path, w, h, bgra):
    """bgra: bytes 行序 top-down。"""
    def chunk(tag, data):
        return (struct.pack(">I", len(data)) + tag + data
                + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF))

    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            i = (y * w + x) * 4
            raw += bytes((bgra[i + 2], bgra[i + 1], bgra[i], bgra[i + 3]))  # BGRA->RGBA
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 6)))
        f.write(chunk(b"IEND", b""))


def shot(path):
    w = user32.GetSystemMetrics(0)
    h = user32.GetSystemMetrics(1)
    hdc = user32.GetDC(None)
    mem = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mem, bmp)
    gdi32.BitBlt(mem, 0, 0, w, h, hdc, 0, 0, SRCCOPY)

    bih = BITMAPINFOHEADER()
    bih.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bih.biWidth = w
    bih.biHeight = -h  # top-down
    bih.biPlanes = 1
    bih.biBitCount = 32
    bih.biCompression = 0

    buf = ctypes.create_string_buffer(w * h * 4)
    gdi32.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bih), 0)
    _write_png(path, w, h, bytes(buf))

    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mem)
    user32.ReleaseDC(None, hdc)
    print("saved", path, f"{w}x{h}")


if __name__ == "__main__":
    shot(sys.argv[1] if len(sys.argv) > 1 else "shot.bmp")
