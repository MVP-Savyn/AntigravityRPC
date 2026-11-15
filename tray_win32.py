"""Icono de bandeja del sistema (System Tray) nativo de Windows usando Win32 ctypes."""

from __future__ import annotations

import ctypes
from ctypes import wintypes
from pathlib import Path
from typing import Callable


user32 = ctypes.windll.user32
shell32 = ctypes.windll.shell32
kernel32 = ctypes.windll.kernel32

LRESULT = ctypes.c_ssize_t
WNDPROC = ctypes.WINFUNCTYPE(LRESULT, wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)

WM_DESTROY = 0x0002
WM_COMMAND = 0x0111
WM_USER = 0x0400
WM_TRAYICON = WM_USER + 20
WM_LBUTTONDBLCLK = 0x0203
WM_RBUTTONUP = 0x0205

NIM_ADD = 0x00000000
NIM_MODIFY = 0x00000001
NIM_DELETE = 0x00000002

NIF_MESSAGE = 0x00000001
NIF_ICON = 0x00000002
NIF_TIP = 0x00000004

IMAGE_ICON = 1
LR_LOADFROMFILE = 0x00000010
LR_DEFAULTSIZE = 0x00000040
IDI_APPLICATION = 32512

MF_STRING = 0x00000000
MF_SEPARATOR = 0x00000800
MF_GRAYED = 0x00000001
TPM_LEFTALIGN = 0x0000
TPM_BOTTOMALIGN = 0x0020
TPM_RIGHTBUTTON = 0x0002

# Declaraciones explícitas de tipos para compatibilidad x64
user32.DefWindowProcW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
user32.DefWindowProcW.restype = LRESULT
user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
user32.PostMessageW.restype = wintypes.BOOL
user32.LoadImageW.argtypes = [
    wintypes.HINSTANCE,
    wintypes.LPCWSTR,
    wintypes.UINT,
    ctypes.c_int,
    ctypes.c_int,
    wintypes.UINT,
]
user32.LoadImageW.restype = wintypes.HANDLE
user32.LoadIconW.argtypes = [wintypes.HINSTANCE, wintypes.LPCWSTR]
user32.LoadIconW.restype = wintypes.HICON
user32.CreatePopupMenu.restype = wintypes.HMENU
user32.AppendMenuW.argtypes = [wintypes.HMENU, wintypes.UINT, ctypes.c_size_t, wintypes.LPCWSTR]
user32.TrackPopupMenu.argtypes = [
    wintypes.HMENU,
    wintypes.UINT,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    wintypes.HWND,
    ctypes.c_void_p,
]
user32.DestroyMenu.argtypes = [wintypes.HMENU]
user32.SetForegroundWindow.argtypes = [wintypes.HWND]


class WNDCLASSEXW(ctypes.Structure):
    _fields_ = [
        ("cbSize", wintypes.UINT),
        ("style", wintypes.UINT),
        ("lpfnWndProc", WNDPROC),
        ("cbClsExtra", ctypes.c_int),
        ("cbWndExtra", ctypes.c_int),
        ("hInstance", wintypes.HINSTANCE),
        ("hIcon", wintypes.HICON),
        ("hCursor", wintypes.HANDLE),
        ("hbrBackground", wintypes.HBRUSH),
        ("lpszMenuName", wintypes.LPCWSTR),
        ("lpszClassName", wintypes.LPCWSTR),
        ("hIconSm", wintypes.HICON),
    ]


class NOTIFYICONDATAW(ctypes.Structure):
    _fields_ = [
        ("cbSize", wintypes.DWORD),
        ("hWnd", wintypes.HWND),
        ("uID", wintypes.UINT),
        ("uFlags", wintypes.UINT),
        ("uCallbackMessage", wintypes.UINT),
        ("hIcon", wintypes.HICON),
        ("szTip", wintypes.WCHAR * 128),
        ("dwState", wintypes.DWORD),
        ("dwStateMask", wintypes.DWORD),
        ("szInfo", wintypes.WCHAR * 256),
        ("uVersion", wintypes.UINT),
        ("szInfoTitle", wintypes.WCHAR * 64),
        ("dwInfoFlags", wintypes.DWORD),
        ("guidItem", ctypes.c_byte * 16),
        ("hBalloonIcon", wintypes.HICON),
    ]


class Win32TrayApp:
    """Gestiona un icono en el área de notificación de Windows."""

    ID_STATUS = 1000
    ID_CONFIG = 1001
    ID_ASSETS = 1002
    ID_RECONNECT = 1003
    ID_EXIT = 1004

    def __init__(
        self,
        icon_path: Path,
        *,
        get_status_text: Callable[[], str],
        on_configure: Callable[[], None],
        on_open_assets: Callable[[], None],
        on_reconnect: Callable[[], None],
        on_exit: Callable[[], None],
    ) -> None:
        self.icon_path = icon_path
        self.get_status_text = get_status_text
        self.on_configure = on_configure
        self.on_open_assets = on_open_assets
        self.on_reconnect = on_reconnect
        self.on_exit = on_exit
        self._hwnd: wintypes.HWND | None = None
        self._hicon: wintypes.HICON | None = None
        self._tooltip = "Antigravity Rich Presence"
        self._wndproc_ref = WNDPROC(self._wnd_proc)

    def _load_icon(self) -> wintypes.HICON:
        if self.icon_path.exists():
            handle = user32.LoadImageW(
                None,
                str(self.icon_path),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
            if handle:
                return wintypes.HICON(handle)
        return user32.LoadIconW(None, ctypes.cast(IDI_APPLICATION, wintypes.LPCWSTR))

    def update_tooltip(self, text: str) -> None:
        self._tooltip = (text or "Antigravity Rich Presence")[:127]
        if self._hwnd and self._hicon:
            nid = NOTIFYICONDATAW()
            nid.cbSize = ctypes.sizeof(NOTIFYICONDATAW)
            nid.hWnd = self._hwnd
            nid.uID = 1
            nid.uFlags = NIF_TIP
            nid.szTip = self._tooltip
            shell32.Shell_NotifyIconW(NIM_MODIFY, ctypes.byref(nid))

    def _show_menu(self, hwnd: wintypes.HWND) -> None:
        menu = user32.CreatePopupMenu()
        status_label = self.get_status_text()[:120]
        user32.AppendMenuW(menu, MF_STRING | MF_GRAYED, self.ID_STATUS, status_label)
        user32.AppendMenuW(menu, MF_SEPARATOR, 0, None)
        user32.AppendMenuW(menu, MF_STRING, self.ID_CONFIG, "Configurar...")
        user32.AppendMenuW(menu, MF_STRING, self.ID_ASSETS, "Abrir carpeta del icono (assets)")
        user32.AppendMenuW(menu, MF_STRING, self.ID_RECONNECT, "Reconectar Discord")
        user32.AppendMenuW(menu, MF_SEPARATOR, 0, None)
        user32.AppendMenuW(menu, MF_STRING, self.ID_EXIT, "Salir")

        pt = wintypes.POINT()
        user32.GetCursorPos(ctypes.byref(pt))
        user32.SetForegroundWindow(hwnd)
        user32.TrackPopupMenu(menu, TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None)
        user32.PostMessageW(hwnd, 0, 0, 0)
        user32.DestroyMenu(menu)

    def _wnd_proc(self, hwnd: wintypes.HWND, msg: int, wparam: int, lparam: int) -> int:
        if msg == WM_TRAYICON:
            if lparam == WM_RBUTTONUP:
                self._show_menu(hwnd)
            elif lparam == WM_LBUTTONDBLCLK:
                self.on_configure()
            return 0

        if msg == WM_COMMAND:
            cmd_id = wparam & 0xFFFF
            if cmd_id == self.ID_CONFIG:
                self.on_configure()
            elif cmd_id == self.ID_ASSETS:
                self.on_open_assets()
            elif cmd_id == self.ID_RECONNECT:
                self.on_reconnect()
            elif cmd_id == self.ID_EXIT:
                self.stop()
                self.on_exit()
            return 0

        if msg == WM_DESTROY:
            user32.PostQuitMessage(0)
            return 0

        return user32.DefWindowProcW(hwnd, msg, wparam, lparam)

    def stop(self) -> None:
        if self._hwnd:
            nid = NOTIFYICONDATAW()
            nid.cbSize = ctypes.sizeof(NOTIFYICONDATAW)
            nid.hWnd = self._hwnd
            nid.uID = 1
            shell32.Shell_NotifyIconW(NIM_DELETE, ctypes.byref(nid))
            user32.DestroyWindow(self._hwnd)
            self._hwnd = None

    def run(self) -> None:
        hinst = kernel32.GetModuleHandleW(None)
        class_name = "AntigravityRichPresenceTray"

        wc = WNDCLASSEXW()
        wc.cbSize = ctypes.sizeof(WNDCLASSEXW)
        wc.lpfnWndProc = self._wndproc_ref
        wc.hInstance = hinst
        wc.lpszClassName = class_name
        user32.RegisterClassExW(ctypes.byref(wc))

        hwnd = user32.CreateWindowExW(
            0,
            class_name,
            "Antigravity Rich Presence",
            0,
            0,
            0,
            0,
            0,
            None,
            None,
            hinst,
            None,
        )
        self._hwnd = hwnd
        self._hicon = self._load_icon()

        nid = NOTIFYICONDATAW()
        nid.cbSize = ctypes.sizeof(NOTIFYICONDATAW)
        nid.hWnd = hwnd
        nid.uID = 1
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP
        nid.uCallbackMessage = WM_TRAYICON
        nid.hIcon = self._hicon
        nid.szTip = self._tooltip
        shell32.Shell_NotifyIconW(NIM_ADD, ctypes.byref(nid))

        msg = wintypes.MSG()
        while user32.GetMessageW(ctypes.byref(msg), None, 0, 0) > 0:
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
