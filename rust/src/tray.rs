use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, LoadIconW, LoadImageW,
    PostMessageW, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow,
    TrackPopupMenu, TranslateMessage, HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE,
    LR_LOADFROMFILE, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_RIGHTBUTTON, WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WM_USER, WNDCLASSEXW,
};

use crate::config::load_config;
use crate::winutil::{assets_dir, to_wide};

const TRAY_WND_CLASS: &str = "AntigravityRichPresenceTrayRS";
const WM_TRAYICON: u32 = WM_USER + 20;
const WM_UPDATE_TIP: u32 = WM_USER + 21;
const WM_REMOTE_EXIT: u32 = WM_USER + 22;

static WM_TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

const ID_STATUS: usize = 1000;
const ID_CONFIG: usize = 1001;
const ID_RECONNECT: usize = 1003;
const ID_EXIT: usize = 1004;

pub trait TrayCallbacks: Send + Sync {
    fn status_text(&self) -> String;
    fn on_configure(&self);
    fn on_reconnect(&self);
    fn on_exit(&self);
}

pub struct TrayState {
    callbacks: Arc<dyn TrayCallbacks>,
    hicon_raw: usize,
    tooltip: Mutex<String>,
}

pub struct TrayHandle {
    hwnd_raw: usize,
}

impl TrayHandle {
    pub fn update_tooltip(&self, text: &str, state: &Arc<TrayState>) {
        if let Ok(mut guard) = state.tooltip.lock() {
            *guard = format!("Antigravity RPC - {text}");
        }
        unsafe {
            PostMessageW(self.hwnd_raw as HWND, WM_UPDATE_TIP, 0, 0);
        }
    }
}

pub fn load_app_icon() -> HICON {
    unsafe {
        let hinst = GetModuleHandleW(ptr::null());
        let res_icon = LoadIconW(hinst, 1usize as *const u16);
        if !res_icon.is_null() {
            return res_icon;
        }
        let ico_path = assets_dir().join("antigravity-icon.ico");
        if ico_path.exists() {
            let wide = to_wide(&ico_path.to_string_lossy());
            let loaded = LoadImageW(
                ptr::null_mut(),
                wide.as_ptr(),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            );
            if !loaded.is_null() {
                return loaded as HICON;
            }
        }
        LoadIconW(ptr::null_mut(), IDI_APPLICATION)
    }
}

fn copy_wide_fixed<const N: usize>(dest: &mut [u16; N], text: &str) {
    dest.fill(0);
    for (i, ch) in text.encode_utf16().take(N - 1).enumerate() {
        dest[i] = ch;
    }
}

unsafe fn add_tray_icon(hwnd: HWND, hicon: HICON, tip: &str) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = hicon;
    copy_wide_fixed(&mut nid.szTip, tip);
    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

unsafe fn show_context_menu(hwnd: HWND, state: &TrayState) {
    let en = load_config().is_english();
    let menu = CreatePopupMenu();
    let status = state.callbacks.status_text();
    let status_w = to_wide(&status);
    let cfg_w = to_wide(if en { "Settings..." } else { "Configurar..." });
    let recon_w = to_wide(if en {
        "Reconnect Discord"
    } else {
        "Reconectar Discord"
    });
    let exit_w = to_wide(if en { "Close RPC" } else { "Cerrar RPC" });

    AppendMenuW(menu, MF_STRING | MF_GRAYED, ID_STATUS, status_w.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, ID_CONFIG, cfg_w.as_ptr());
    AppendMenuW(menu, MF_STRING, ID_RECONNECT, recon_w.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, ID_EXIT, exit_w.as_ptr());

    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
        pt.x,
        pt.y,
        0,
        hwnd,
        ptr::null(),
    );
    PostMessageW(hwnd, 0, 0, 0);
    DestroyMenu(menu);
}

pub fn is_tray_instance_running() -> bool {
    let cls = to_wide(TRAY_WND_CLASS);
    unsafe { !FindWindowW(cls.as_ptr(), ptr::null()).is_null() }
}

pub fn close_running_tray_instance() -> bool {
    let cls = to_wide(TRAY_WND_CLASS);
    unsafe {
        let hwnd = FindWindowW(cls.as_ptr(), ptr::null());
        if hwnd.is_null() {
            return false;
        }
        PostMessageW(hwnd, WM_REMOTE_EXIT, 0, 0) != 0
    }
}

static TRAY_STATE: Mutex<Option<Arc<TrayState>>> = Mutex::new(None);

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_opt = TRAY_STATE.lock().ok().and_then(|g| g.clone());

    let taskbar_msg = WM_TASKBAR_CREATED.load(Ordering::Relaxed);
    if taskbar_msg != 0 && msg == taskbar_msg {
        if let Some(state) = state_opt {
            let tip = state
                .tooltip
                .lock()
                .map(|g| g.clone())
                .unwrap_or_else(|_| "Antigravity Rich Presence".to_string());
            add_tray_icon(hwnd, state.hicon_raw as HICON, &tip);
        }
        return 0;
    }

    match msg {
        WM_TRAYICON => {
            if let Some(state) = state_opt {
                let evt = lparam as u32;
                if evt == WM_RBUTTONUP {
                    show_context_menu(hwnd, &state);
                } else if evt == WM_LBUTTONUP {
                    state.callbacks.on_configure();
                }
            }
            0
        }
        WM_UPDATE_TIP => {
            if let Some(state) = state_opt {
                let tip = state
                    .tooltip
                    .lock()
                    .map(|g| g.clone())
                    .unwrap_or_else(|_| "Antigravity Rich Presence".to_string());
                let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                nid.hWnd = hwnd;
                nid.uID = 1;
                nid.uFlags = NIF_TIP;
                copy_wide_fixed(&mut nid.szTip, &tip);
                Shell_NotifyIconW(NIM_MODIFY, &nid);
            }
            0
        }
        WM_COMMAND => {
            if let Some(state) = state_opt {
                let cmd = wparam & 0xFFFF;
                if cmd == ID_CONFIG {
                    state.callbacks.on_configure();
                } else if cmd == ID_RECONNECT {
                    state.callbacks.on_reconnect();
                } else if cmd == ID_EXIT {
                    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                    nid.hWnd = hwnd;
                    nid.uID = 1;
                    Shell_NotifyIconW(NIM_DELETE, &nid);
                    state.callbacks.on_exit();
                    DestroyWindow(hwnd);
                }
            }
            0
        }
        WM_REMOTE_EXIT => {
            if let Some(state) = state_opt {
                let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                nid.hWnd = hwnd;
                nid.uID = 1;
                Shell_NotifyIconW(NIM_DELETE, &nid);
                state.callbacks.on_exit();
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn run_tray(
    callbacks: Arc<dyn TrayCallbacks>,
    on_ready: impl FnOnce(TrayHandle, Arc<TrayState>),
) {
    unsafe {
        let hinst = GetModuleHandleW(ptr::null());
        let class_name = to_wide(TRAY_WND_CLASS);
        let title = to_wide("Antigravity Rich Presence");

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(tray_wnd_proc);
        wc.hInstance = hinst;
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassExW(&wc);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            hinst,
            ptr::null(),
        );

        let taskbar_created_w = to_wide("TaskbarCreated");
        WM_TASKBAR_CREATED.store(
            RegisterWindowMessageW(taskbar_created_w.as_ptr()),
            Ordering::Relaxed,
        );

        let hicon = load_app_icon();
        let state = Arc::new(TrayState {
            callbacks,
            hicon_raw: hicon as usize,
            tooltip: Mutex::new("Antigravity Rich Presence".to_string()),
        });
        if let Ok(mut guard) = TRAY_STATE.lock() {
            *guard = Some(Arc::clone(&state));
        }

        add_tray_icon(hwnd, state.hicon_raw as HICON, "Antigravity Rich Presence");

        on_ready(TrayHandle { hwnd_raw: hwnd as usize }, Arc::clone(&state));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if let Ok(mut guard) = TRAY_STATE.lock() {
            *guard = None;
        }
    }
}

pub type SharedTrayState = Arc<TrayState>;
