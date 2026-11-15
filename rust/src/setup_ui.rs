use std::ptr;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, COLOR_BTNFACE, FW_BOLD, FW_NORMAL, HBRUSH, HFONT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW, GetMessageW,
    GetSystemMetrics, MessageBoxW, PostQuitMessage, RegisterClassExW, SendMessageW,
    SetForegroundWindow, SetWindowTextW, ShowWindow, TranslateMessage, BM_GETCHECK, BM_SETCHECK,
    BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CBS_DROPDOWNLIST, CBN_SELCHANGE,
    CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, MB_ICONERROR, MB_ICONWARNING, MB_OK, MSG,
    SM_CXSCREEN, SM_CYSCREEN, SW_SHOWNORMAL, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_SETFONT, WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_DISABLED, WS_SYSMENU, WS_TABSTOP,
    WS_VISIBLE, WS_VSCROLL,
};

use crate::config::{
    is_startup_enabled, load_config, save_config, set_startup_enabled, AppConfig,
};
use crate::tray::{close_running_tray_instance, is_tray_instance_running, load_app_icon};
use crate::winutil::to_wide;

const IDC_COMBO_LANG: usize = 204;
const IDC_CHK_TITLE: usize = 205;
const IDC_CHK_PROJECT_STATS: usize = 206;
const IDC_CHK_GIT_BRANCH: usize = 207;
const IDC_CHK_AGENT_STATUS: usize = 208;
const IDC_CHK_REPO_BTN: usize = 209;
const IDC_CHK_TIMER: usize = 210;
const IDC_CHK_FALLBACK: usize = 211;
const IDC_CHK_STARTUP: usize = 212;
const IDC_BTN_SAVE: usize = 213;
const IDC_BTN_CANCEL: usize = 214;
const IDC_BTN_CLOSE_RPC: usize = 215;

struct DialogState {
    lbl_subtitle: usize,
    combo_lang: usize,
    chk_title: usize,
    chk_project_stats: usize,
    chk_git_branch: usize,
    chk_agent_status: usize,
    chk_repo_btn: usize,
    chk_timer: usize,
    chk_fallback: usize,
    chk_startup: usize,
    btn_close_rpc: usize,
    btn_cancel: usize,
    btn_save: usize,
    rpc_running: bool,
    font_normal: usize,
    font_bold: usize,
    saved_cfg: Option<AppConfig>,
}

static DIALOG_STATE: Mutex<Option<DialogState>> = Mutex::new(None);

unsafe fn set_ctrl_text(hwnd: HWND, text: &str) {
    let w = to_wide(text);
    SetWindowTextW(hwnd, w.as_ptr());
}

unsafe fn apply_ui_language(dialog_hwnd: HWND, st: &DialogState, en: bool) {
    set_ctrl_text(
        dialog_hwnd,
        if en {
            "Antigravity Rich Presence - Settings"
        } else {
            "Antigravity Rich Presence - Configuración"
        },
    );
    set_ctrl_text(
        st.lbl_subtitle as HWND,
        if en {
            "Choose what information to display on Discord and how RPC behaves:"
        } else {
            "Ajusta qué información mostrar en Discord y cómo se comporta el RPC:"
        },
    );
    set_ctrl_text(
        st.chk_title as HWND,
        if en {
            "Show active file or conversation title"
        } else {
            "Mostrar archivo activo o título de conversación"
        },
    );
    set_ctrl_text(
        st.chk_project_stats as HWND,
        if en {
            "Show active days and total project hours"
        } else {
            "Mostrar días activos y horas totales del proyecto"
        },
    );
    set_ctrl_text(
        st.chk_git_branch as HWND,
        if en {
            "Show project Git branch"
        } else {
            "Mostrar rama de Git del proyecto"
        },
    );
    set_ctrl_text(
        st.chk_agent_status as HWND,
        if en {
            "Show agent status and interactions"
        } else {
            "Mostrar estado del agente e interacciones"
        },
    );
    set_ctrl_text(
        st.chk_repo_btn as HWND,
        if en {
            "Show \"View repository\" button (if HTTPS remote exists)"
        } else {
            "Mostrar botón \"Ver repositorio\" (si hay remoto HTTPS)"
        },
    );
    set_ctrl_text(
        st.chk_timer as HWND,
        if en {
            "Reset elapsed timer when switching projects"
        } else {
            "Reiniciar contador de tiempo al cambiar de proyecto"
        },
    );
    set_ctrl_text(
        st.chk_fallback as HWND,
        if en {
            "In free chats, show the last opened project"
        } else {
            "En chats libres, mostrar el último proyecto abierto"
        },
    );
    set_ctrl_text(
        st.chk_startup as HWND,
        if en {
            "Start automatically with Windows"
        } else {
            "Iniciar automáticamente con Windows"
        },
    );
    set_ctrl_text(
        st.btn_close_rpc as HWND,
        match (st.rpc_running, en) {
            (true, true) => "Close RPC",
            (true, false) => "Cerrar RPC",
            (false, true) => "RPC idle",
            (false, false) => "RPC inactivo",
        },
    );
    set_ctrl_text(st.btn_cancel as HWND, if en { "Cancel" } else { "Cancelar" });
    set_ctrl_text(st.btn_save as HWND, if en { "Save" } else { "Guardar" });
}

unsafe fn is_checked(hwnd: HWND) -> bool {
    SendMessageW(hwnd, BM_GETCHECK, 0, 0) == 1
}

unsafe fn set_checked(hwnd: HWND, checked: bool) {
    SendMessageW(hwnd, BM_SETCHECK, if checked { 1 } else { 0 }, 0);
}

unsafe fn set_font(hwnd: HWND, font: HFONT) {
    if !font.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1);
    }
}

unsafe fn create_ctrl(
    parent: HWND,
    hinst: *mut core::ffi::c_void,
    class_name: &str,
    text: &str,
    ex_style: u32,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: usize,
    font: HFONT,
) -> HWND {
    let cls = to_wide(class_name);
    let txt = to_wide(text);
    let ctrl = CreateWindowExW(
        ex_style,
        cls.as_ptr(),
        txt.as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        x,
        y,
        w,
        h,
        parent,
        id as _,
        hinst,
        ptr::null(),
    );
    set_font(ctrl, font);
    ctrl
}

unsafe fn show_msg(parent: HWND, title: &str, text: &str, flags: u32) {
    let t = to_wide(title);
    let m = to_wide(text);
    MessageBoxW(parent, m.as_ptr(), t.as_ptr(), flags);
}

unsafe extern "system" fn setup_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let hinst = GetModuleHandleW(ptr::null());
            let face = to_wide("Segoe UI");
            let font_normal = CreateFontW(
                -15,
                0,
                0,
                0,
                FW_NORMAL as i32,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                face.as_ptr(),
            );
            let font_bold = CreateFontW(
                -17,
                0,
                0,
                0,
                FW_BOLD as i32,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                face.as_ptr(),
            );

            let cfg = load_config();
            let startup = is_startup_enabled();
            let rpc_running = is_tray_instance_running();
            let en = cfg.is_english();

            create_ctrl(
                hwnd,
                hinst,
                "STATIC",
                "Antigravity Discord Rich Presence",
                0,
                0,
                20,
                16,
                460,
                24,
                0,
                font_bold,
            );

            let lbl_subtitle = create_ctrl(
                hwnd,
                hinst,
                "STATIC",
                "",
                0,
                0,
                20,
                44,
                460,
                20,
                0,
                font_normal,
            );

            create_ctrl(
                hwnd,
                hinst,
                "STATIC",
                "Idioma / Language:",
                0,
                0,
                20,
                74,
                140,
                22,
                0,
                font_normal,
            );

            let combo_lang = create_ctrl(
                hwnd,
                hinst,
                "COMBOBOX",
                "",
                0,
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
                165,
                70,
                150,
                100,
                IDC_COMBO_LANG,
                font_normal,
            );
            let es_w = to_wide("Español");
            let en_w = to_wide("English");
            SendMessageW(combo_lang, CB_ADDSTRING, 0, es_w.as_ptr() as LPARAM);
            SendMessageW(combo_lang, CB_ADDSTRING, 0, en_w.as_ptr() as LPARAM);
            SendMessageW(combo_lang, CB_SETCURSEL, if en { 1 } else { 0 }, 0);

            let chk_title = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                108,
                460,
                22,
                IDC_CHK_TITLE,
                font_normal,
            );
            set_checked(chk_title, cfg.show_conversation_title);

            let chk_project_stats = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                134,
                460,
                22,
                IDC_CHK_PROJECT_STATS,
                font_normal,
            );
            set_checked(chk_project_stats, cfg.show_project_stats);

            let chk_git_branch = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                160,
                460,
                22,
                IDC_CHK_GIT_BRANCH,
                font_normal,
            );
            set_checked(chk_git_branch, cfg.show_git_branch);

            let chk_agent_status = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                186,
                460,
                22,
                IDC_CHK_AGENT_STATUS,
                font_normal,
            );
            set_checked(chk_agent_status, cfg.show_agent_status);

            let chk_repo_btn = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                212,
                460,
                22,
                IDC_CHK_REPO_BTN,
                font_normal,
            );
            set_checked(chk_repo_btn, cfg.show_repo_button);

            let chk_timer = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                238,
                460,
                22,
                IDC_CHK_TIMER,
                font_normal,
            );
            set_checked(chk_timer, cfg.reset_timer_on_project_change);

            let chk_fallback = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                264,
                460,
                22,
                IDC_CHK_FALLBACK,
                font_normal,
            );
            set_checked(chk_fallback, cfg.fallback_to_last_project);

            let chk_startup = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
                20,
                290,
                460,
                22,
                IDC_CHK_STARTUP,
                font_normal,
            );
            set_checked(chk_startup, startup);

            let close_rpc_style = BS_PUSHBUTTON as u32
                | WS_TABSTOP
                | if rpc_running { 0 } else { WS_DISABLED };
            let btn_close_rpc = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                close_rpc_style,
                20,
                330,
                114,
                32,
                IDC_BTN_CLOSE_RPC,
                font_normal,
            );

            let btn_cancel = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_PUSHBUTTON as u32 | WS_TABSTOP,
                274,
                330,
                96,
                32,
                IDC_BTN_CANCEL,
                font_normal,
            );

            let btn_save = create_ctrl(
                hwnd,
                hinst,
                "BUTTON",
                "",
                0,
                BS_DEFPUSHBUTTON as u32 | WS_TABSTOP,
                380,
                330,
                100,
                32,
                IDC_BTN_SAVE,
                font_normal,
            );

            let st = DialogState {
                lbl_subtitle: lbl_subtitle as usize,
                combo_lang: combo_lang as usize,
                chk_title: chk_title as usize,
                chk_project_stats: chk_project_stats as usize,
                chk_git_branch: chk_git_branch as usize,
                chk_agent_status: chk_agent_status as usize,
                chk_repo_btn: chk_repo_btn as usize,
                chk_timer: chk_timer as usize,
                chk_fallback: chk_fallback as usize,
                chk_startup: chk_startup as usize,
                btn_close_rpc: btn_close_rpc as usize,
                btn_cancel: btn_cancel as usize,
                btn_save: btn_save as usize,
                rpc_running,
                font_normal: font_normal as usize,
                font_bold: font_bold as usize,
                saved_cfg: None,
            };
            apply_ui_language(hwnd, &st, en);

            if let Ok(mut guard) = DIALOG_STATE.lock() {
                *guard = Some(st);
            }
            0
        }
        WM_COMMAND => {
            let cmd = wparam & 0xFFFF;
            let notif = ((wparam >> 16) & 0xFFFF) as u32;
            if cmd == IDC_COMBO_LANG && notif == CBN_SELCHANGE {
                if let Ok(guard) = DIALOG_STATE.lock() {
                    if let Some(st) = guard.as_ref() {
                        let sel = SendMessageW(st.combo_lang as HWND, CB_GETCURSEL, 0, 0);
                        apply_ui_language(hwnd, st, sel == 1);
                    }
                }
            } else if cmd == IDC_BTN_CLOSE_RPC {
                close_running_tray_instance();
                DestroyWindow(hwnd);
            } else if cmd == IDC_BTN_CANCEL {
                DestroyWindow(hwnd);
            } else if cmd == IDC_BTN_SAVE {
                let mut valid_save = false;
                let mut startup_err: Option<String> = None;
                let mut save_err: Option<String> = None;
                let mut is_en = false;

                if let Ok(mut guard) = DIALOG_STATE.lock() {
                    if let Some(st) = guard.as_mut() {
                        let sel = SendMessageW(st.combo_lang as HWND, CB_GETCURSEL, 0, 0);
                        is_en = sel == 1;

                        let mut cfg = load_config();
                        cfg.language = if is_en { "en" } else { "es" }.to_string();
                        cfg.show_conversation_title = is_checked(st.chk_title as HWND);
                        cfg.show_project_stats = is_checked(st.chk_project_stats as HWND);
                        cfg.show_git_branch = is_checked(st.chk_git_branch as HWND);
                        cfg.show_agent_status = is_checked(st.chk_agent_status as HWND);
                        cfg.show_repo_button = is_checked(st.chk_repo_btn as HWND);
                        cfg.reset_timer_on_project_change = is_checked(st.chk_timer as HWND);
                        cfg.fallback_to_last_project = is_checked(st.chk_fallback as HWND);

                        if let Err(err) = save_config(&cfg) {
                            save_err = Some(err);
                        } else {
                            let startup_on = is_checked(st.chk_startup as HWND);
                            if let Err(err) = set_startup_enabled(startup_on) {
                                startup_err = Some(err);
                            }
                            st.saved_cfg = Some(cfg);
                            valid_save = true;
                        }
                    }
                }

                if let Some(err) = save_err {
                    show_msg(
                        hwnd,
                        if is_en { "Save error" } else { "Error al guardar" },
                        &err,
                        MB_OK | MB_ICONERROR,
                    );
                } else {
                    if let Some(err) = startup_err {
                        show_msg(
                            hwnd,
                            if is_en {
                                "Windows Startup"
                            } else {
                                "Inicio con Windows"
                            },
                            &format!(
                                "{}\n{err}",
                                if is_en {
                                    "Settings saved, but updating Windows startup failed:"
                                } else {
                                    "Configuración guardada, pero falló el registro de inicio automático:"
                                }
                            ),
                            MB_OK | MB_ICONWARNING,
                        );
                    }
                    if valid_save {
                        DestroyWindow(hwnd);
                    }
                }
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            if let Ok(mut guard) = DIALOG_STATE.lock() {
                if let Some(st) = guard.as_mut() {
                    if st.font_normal != 0 {
                        DeleteObject(st.font_normal as _);
                    }
                    if st.font_bold != 0 {
                        DeleteObject(st.font_bold as _);
                    }
                }
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn open_config_dialog() -> Option<AppConfig> {
    unsafe {
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&icc);

        let hinst = GetModuleHandleW(ptr::null());
        let class_name = to_wide("AntigravityRPCSetupDialog");
        let existing = FindWindowW(class_name.as_ptr(), ptr::null());
        if !existing.is_null() {
            ShowWindow(existing, SW_SHOWNORMAL);
            SetForegroundWindow(existing);
            return None;
        }
        let title = to_wide("Antigravity Rich Presence");
        let hicon = load_app_icon();

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(setup_wnd_proc);
        wc.hInstance = hinst;
        wc.hIcon = hicon;
        wc.hIconSm = hicon;
        wc.hbrBackground = (COLOR_BTNFACE + 1) as isize as HBRUSH;
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassExW(&wc);

        let win_w = 516;
        let win_h = 414;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let x = ((sx - win_w) / 2).max(0);
        let y = ((sy - win_h) / 2).max(0);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            x,
            y,
            win_w,
            win_h,
            ptr::null_mut(),
            ptr::null_mut(),
            hinst,
            ptr::null(),
        );

        ShowWindow(hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        DIALOG_STATE
            .lock()
            .ok()
            .and_then(|mut g| g.take())
            .and_then(|st| st.saved_cfg)
    }
}
