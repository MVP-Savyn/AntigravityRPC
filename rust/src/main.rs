#![windows_subsystem = "windows"]

mod config;
mod detector;
mod discord_ipc;
mod setup_ui;
mod tray;
mod winutil;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;

use crate::config::{config_path, load_config, log_path, AppConfig};
use crate::detector::{detect_antigravity_state, AntigravityState, Surface};
use crate::discord_ipc::DiscordIpc;
use crate::setup_ui::open_config_dialog;
use crate::tray::{
    close_running_tray_instance, run_tray, SharedTrayState, TrayCallbacks, TrayHandle,
};
use crate::winutil::{data_dir, to_wide};

fn log_line(msg: &str) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path()) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

fn acquire_single_instance_mutex() -> bool {
    let name = to_wide(r"Local\AntigravityDiscordRPCMutex");
    unsafe {
        let handle = CreateMutexW(ptr::null(), 0, name.as_ptr());
        if handle.is_null() {
            return true;
        }
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

fn format_agent_status(state: &AntigravityState, en: bool) -> Option<String> {
    let n = state.interactions;
    if state.surface == Surface::App && state.is_busy {
        if n > 0 {
            Some(if en {
                format!("Working ({n})")
            } else {
                format!("Trabajando ({n})")
            })
        } else {
            Some(if en {
                "Working...".to_string()
            } else {
                "Trabajando...".to_string()
            })
        }
    } else if n > 0 {
        Some(if en {
            if n == 1 {
                "1 interaction".to_string()
            } else {
                format!("{n} interactions")
            }
        } else if n == 1 {
            "1 interacción".to_string()
        } else {
            format!("{n} interacciones")
        })
    } else {
        None
    }
}

fn format_project_stats(days: u32, active_secs: u64, en: bool) -> Option<String> {
    if days == 0 && active_secs == 0 {
        return None;
    }
    let time_str = if active_secs >= 36_000 {
        format!("{}h", (active_secs as f64 / 3_600.0).round() as u64)
    } else if active_secs >= 3_600 {
        let hours = active_secs as f64 / 3_600.0;
        let tenths = (hours * 10.0).round() as u64;
        if tenths % 10 == 0 {
            format!("{}h", tenths / 10)
        } else {
            format!("{:.1}h", tenths as f64 / 10.0)
        }
    } else {
        let mins = (active_secs / 60).max(1);
        format!("{mins}m")
    };
    let label = if en { "Invested:" } else { "Invertido:" };
    if days > 0 {
        Some(format!("{label} {days}d | {time_str}"))
    } else {
        Some(format!("{label} {time_str}"))
    }
}

pub fn build_presence_fields(
    state: &AntigravityState,
    cfg: &AppConfig,
) -> (String, Option<String>, String, Option<(String, String)>) {
    let en = cfg.is_english();
    let surface_label = if state.surface == Surface::Ide {
        "Antigravity IDE"
    } else {
        "Google Antigravity"
    };

    let branch_suffix = if cfg.show_git_branch {
        state
            .git_branch
            .as_deref()
            .filter(|b| !b.is_empty())
            .map(|b| format!(" ({b})"))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let agent_status = if cfg.show_agent_status {
        format_agent_status(state, en)
    } else {
        None
    };

    let editing_prefix = if en { "Editing" } else { "Editando" };
    let default_chat = if en {
        "Active chat"
    } else {
        "Conversación activa"
    };

    let (details, second) = if let Some(proj) = &state.project {
        let d = format!("{proj}{branch_suffix}");
        let mut parts: Vec<String> = Vec::new();

        if cfg.show_project_stats {
            if let Some(stats) =
                format_project_stats(state.project_days, state.project_active_secs, en)
            {
                parts.push(stats);
            }
        }

        if cfg.show_conversation_title {
            let activity = if state.surface == Surface::Ide {
                state
                    .active_file
                    .as_ref()
                    .map(|f| format!("{editing_prefix} {f}"))
                    .or_else(|| state.detail_line.clone())
            } else {
                state.detail_line.clone()
            };
            if let Some(a) = activity {
                parts.push(a);
            }
        }

        if let Some(st) = agent_status {
            parts.push(st);
        }

        let s = if parts.is_empty() {
            None
        } else {
            Some(parts.join(" | "))
        };
        (d, s)
    } else {
        let d = state
            .detail_line
            .clone()
            .or_else(|| {
                state
                    .active_file
                    .as_ref()
                    .map(|f| format!("{editing_prefix} {f}"))
            })
            .unwrap_or_else(|| default_chat.to_string());
        (d, agent_status)
    };

    let large_text = if cfg.large_text.trim().is_empty() {
        surface_label.to_string()
    } else {
        cfg.large_text.clone()
    };

    let button = if cfg.show_repo_button {
        let btn_label = if en {
            "View repository"
        } else {
            "Ver repositorio"
        };
        state
            .repo_url
            .as_ref()
            .map(|url| (btn_label.to_string(), url.clone()))
    } else {
        None
    };

    (details, second, large_text, button)
}

struct AppController {
    stop_flag: AtomicBool,
    reconnect_flag: AtomicBool,
    status_text: Mutex<String>,
    tray_slot: Mutex<Option<(TrayHandle, SharedTrayState)>>,
}

impl AppController {
    fn new() -> Self {
        Self {
            stop_flag: AtomicBool::new(false),
            reconnect_flag: AtomicBool::new(false),
            status_text: Mutex::new("Iniciando...".to_string()),
            tray_slot: Mutex::new(None),
        }
    }

    fn set_status(&self, text: &str) {
        if let Ok(mut g) = self.status_text.lock() {
            *g = text.to_string();
        }
        if let Ok(g) = self.tray_slot.lock() {
            if let Some((handle, state)) = g.as_ref() {
                handle.update_tooltip(text, state);
            }
        }
    }
}

impl TrayCallbacks for AppController {
    fn status_text(&self) -> String {
        self.status_text
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "Antigravity Rich Presence".to_string())
    }

    fn on_configure(&self) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = Command::new(exe).arg("--setup").spawn();
        }
    }

    fn on_reconnect(&self) {
        self.reconnect_flag.store(true, Ordering::Relaxed);
    }

    fn on_exit(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

#[derive(PartialEq, Eq)]
struct PayloadKey {
    details: String,
    second: Option<String>,
    start_ts: i64,
    large_image: String,
    large_text: String,
    button: Option<(String, String)>,
}

fn launch_antigravity_app(prefer_ide: bool) {
    let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let programs = std::path::PathBuf::from(local_appdata).join("Programs");
    let app_exe = programs.join("antigravity").join("Antigravity.exe");
    let ide_exe = programs.join("Antigravity IDE").join("Antigravity IDE.exe");

    let target = if prefer_ide {
        if ide_exe.exists() {
            Some(ide_exe)
        } else if app_exe.exists() {
            Some(app_exe)
        } else {
            None
        }
    } else if app_exe.exists() {
        Some(app_exe)
    } else if ide_exe.exists() {
        Some(ide_exe)
    } else {
        None
    };

    if let Some(exe) = target {
        let _ = Command::new(exe).spawn();
    }
}

fn run_sync_loop(ctrl: Arc<AppController>, auto_exit_on_close: bool) {
    let mut cfg = load_config();
    let cfg_file = config_path();
    let mut last_mtime = fs::metadata(&cfg_file).and_then(|m| m.modified()).ok();
    let mut ipc = DiscordIpc::new(&cfg.client_id);

    let mut last_payload: Option<PayloadKey> = None;
    let mut last_proj_key: Option<String> = None;
    let mut start_ts: Option<i64> = None;
    let mut saw_antigravity = false;
    let started_at = Instant::now();

    while !ctrl.stop_flag.load(Ordering::Relaxed) {
        if let Ok(meta) = fs::metadata(&cfg_file) {
            if let Ok(mtime) = meta.modified() {
                if Some(mtime) != last_mtime {
                    last_mtime = Some(mtime);
                    cfg = load_config();
                    ipc.set_client_id(&cfg.client_id);
                    last_payload = None;
                }
            }
        }

        if ctrl.reconnect_flag.swap(false, Ordering::Relaxed) {
            ipc.close();
            last_payload = None;
        }

        let en = cfg.is_english();
        if cfg.client_id.is_empty() {
            ctrl.set_status(if en {
                "Missing Client ID"
            } else {
                "Falta Client ID"
            });
            std::thread::sleep(Duration::from_secs(2));
            continue;
        }

        let state = detect_antigravity_state(cfg.fallback_to_last_project);
        if !state.running {
            if auto_exit_on_close {
                if saw_antigravity {
                    std::thread::sleep(Duration::from_secs(2));
                    if !detect_antigravity_state(cfg.fallback_to_last_project).running {
                        ctrl.stop_flag.store(true, Ordering::Relaxed);
                        break;
                    }
                } else if started_at.elapsed() > Duration::from_secs(45) {
                    ctrl.stop_flag.store(true, Ordering::Relaxed);
                    break;
                }
            }
            if last_payload.is_some() {
                ipc.clear_activity();
                last_payload = None;
                last_proj_key = None;
                start_ts = None;
            }
            ctrl.set_status(if en {
                "Antigravity closed"
            } else {
                "Antigravity cerrado"
            });
            std::thread::sleep(Duration::from_secs(cfg.poll_interval_sec));
            continue;
        }
        saw_antigravity = true;

        let surf = if state.surface == Surface::Ide { "ide" } else { "app" };
        let proj_key = format!("{surf}:{}", state.project.as_deref().unwrap_or(""));
        if start_ts.is_none()
            || (cfg.reset_timer_on_project_change && last_proj_key.as_deref() != Some(&proj_key))
        {
            start_ts = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
            );
        }
        last_proj_key = Some(proj_key);

        let ts = start_ts.unwrap_or(0);
        let (details, second, large_text, button) = build_presence_fields(&state, &cfg);
        let key = PayloadKey {
            details: details.clone(),
            second: second.clone(),
            start_ts: ts,
            large_image: cfg.large_image.clone(),
            large_text: large_text.clone(),
            button: button.clone(),
        };

        if !ipc.is_connected() || last_payload.as_ref() != Some(&key) {
            let btn_ref = button.as_ref().map(|(l, u)| (l.as_str(), u.as_str()));
            match ipc.set_activity(
                &details,
                second.as_deref(),
                ts,
                &cfg.large_image,
                &large_text,
                btn_ref,
            ) {
                Ok(()) => {
                    let fallback_sum = if en { "No project" } else { "Sin proyecto" };
                    let summary = state.project.as_deref().unwrap_or(fallback_sum);
                    ctrl.set_status(&format!(
                        "{} ({summary})",
                        if en { "Connected" } else { "Conectado" }
                    ));
                    let sec_log = second.as_deref().unwrap_or("-");
                    log_line(&format!("Actividad actualizada: {details} | {sec_log}"));
                    last_payload = Some(key);
                }
                Err(err) => {
                    ctrl.set_status(if en {
                        "Waiting for Discord..."
                    } else {
                        "Esperando a Discord..."
                    });
                    log_line(&format!("Discord IPC error: {err}"));
                    ipc.close();
                    last_payload = None;
                }
            }
        }

        for _ in 0..(cfg.poll_interval_sec * 4) {
            if ctrl.stop_flag.load(Ordering::Relaxed) || ctrl.reconnect_flag.load(Ordering::Relaxed)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    ipc.clear_activity();
    ipc.close();
    if auto_exit_on_close {
        close_running_tray_instance();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h" || a == "--version") {
        return;
    }

    if args.iter().any(|a| a == "--setup") {
        open_config_dialog();
        return;
    }

    let launch_ide = args.iter().any(|a| a == "--launch-ide");
    let launch_app = args.iter().any(|a| a == "--launch") || launch_ide;
    if launch_app {
        launch_antigravity_app(launch_ide);
    }

    let cfg = load_config();
    if cfg.client_id.is_empty() {
        let saved = open_config_dialog();
        if saved.map(|c| c.client_id.is_empty()).unwrap_or(true) {
            return;
        }
    }

    if !acquire_single_instance_mutex() {
        return;
    }

    let ctrl = Arc::new(AppController::new());
    let worker_ctrl = Arc::clone(&ctrl);
    let worker = std::thread::spawn(move || run_sync_loop(worker_ctrl, launch_app));

    let tray_ctrl = Arc::clone(&ctrl);
    run_tray(ctrl.clone(), move |handle, state| {
        if let Ok(mut guard) = tray_ctrl.tray_slot.lock() {
            *guard = Some((handle, state));
        }
    });

    ctrl.stop_flag.store(true, Ordering::Relaxed);
    let _ = worker.join();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_and_presence_formatting() {
        let st_default = detect_antigravity_state(false);
        assert!(st_default.running, "Antigravity should be detected as running");
        assert_eq!(st_default.surface, Surface::App);

        let st_fallback = detect_antigravity_state(true);
        assert!(st_fallback.running);

        let cfg = AppConfig::default();
        assert!(!cfg.show_conversation_title);
        assert_eq!(cfg.language, "es");
        let (details, second, large_text, _) = build_presence_fields(&st_fallback, &cfg);
        assert!(!details.is_empty());
        assert!(!details.contains("En Antigravity"));
        if let Some(s) = &second {
            assert!(!s.contains("En Antigravity"));
        }
        assert_eq!(large_text, "Google Antigravity");

        let ide_st = AntigravityState {
            running: true,
            surface: Surface::Ide,
            project: Some("SAVIN-HOLLOWDRIVE".to_string()),
            detail_line: None,
            active_file: Some("App.tsx".to_string()),
            git_branch: Some("main".to_string()),
            repo_url: Some("https://github.com/MVP-Savyn/SAVIN-HOLLOWDRIVE".to_string()),
            interactions: 413,
            is_busy: false,
            project_days: 30,
            project_active_secs: 74 * 3_600,
        };
        let (d_ide, s_ide, _, btn_ide) = build_presence_fields(&ide_st, &cfg);
        assert_eq!(d_ide, "SAVIN-HOLLOWDRIVE (main)");
        assert_eq!(
            s_ide.as_deref(),
            Some("Invertido: 30d | 74h | 413 interacciones")
        );
        assert_eq!(
            btn_ide,
            Some((
                "Ver repositorio".to_string(),
                "https://github.com/MVP-Savyn/SAVIN-HOLLOWDRIVE".to_string()
            ))
        );
        assert_eq!(cfg.client_id, config::DEFAULT_CLIENT_ID);

        let mut cfg_en = cfg.clone();
        cfg_en.language = "en".to_string();
        let (d_en, s_en, _, btn_en) = build_presence_fields(&ide_st, &cfg_en);
        assert_eq!(d_en, "SAVIN-HOLLOWDRIVE (main)");
        assert_eq!(
            s_en.as_deref(),
            Some("Invested: 30d | 74h | 413 interactions")
        );
        assert_eq!(
            btn_en,
            Some((
                "View repository".to_string(),
                "https://github.com/MVP-Savyn/SAVIN-HOLLOWDRIVE".to_string()
            ))
        );

        let no_proj_st = AntigravityState {
            running: true,
            surface: Surface::App,
            project: None,
            detail_line: Some("Mi chat libre".to_string()),
            active_file: None,
            git_branch: None,
            repo_url: None,
            interactions: 4,
            is_busy: false,
            project_days: 0,
            project_active_secs: 0,
        };
        let (d_np, s_np, _, _) = build_presence_fields(&no_proj_st, &cfg);
        assert_eq!(d_np, "Mi chat libre");
        assert_eq!(s_np.as_deref(), Some("4 interacciones"));
    }

    #[test]
    fn test_live_discord_ipc_handshake() {
        let mut ipc = DiscordIpc::new(config::DEFAULT_CLIENT_ID);
        ipc.connect().expect("Discord IPC handshake failed");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        ipc.set_activity(
            "SAVIN-HOLLOWDRIVE (main)",
            Some("Verificando Antigravity Rich Presence"),
            now,
            config::DEFAULT_LARGE_IMAGE,
            "Google Antigravity",
            Some((
                "Ver repositorio",
                "https://github.com/MVP-Savyn/SAVIN-HOLLOWDRIVE",
            )),
        )
        .expect("Discord SET_ACTIVITY failed");
        ipc.clear_activity();
        ipc.close();
    }
}
