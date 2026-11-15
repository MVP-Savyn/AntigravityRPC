use std::fs;
use std::path::PathBuf;
use std::ptr;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ,
};

use crate::winutil::{data_dir, exe_dir, to_wide};

const STARTUP_REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_VALUE_NAME: &str = "AntigravityDiscordRPC";
pub const DEFAULT_CLIENT_ID: &str = "1571349694016266300";
pub const DEFAULT_LARGE_IMAGE: &str = "antigravity";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub client_id: String,
    pub large_image: String,
    pub large_text: String,
    pub language: String,
    pub poll_interval_sec: u64,
    pub show_conversation_title: bool,
    pub show_project_stats: bool,
    pub show_git_branch: bool,
    pub show_agent_status: bool,
    pub show_repo_button: bool,
    pub reset_timer_on_project_change: bool,
    pub fallback_to_last_project: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            client_id: DEFAULT_CLIENT_ID.to_string(),
            large_image: DEFAULT_LARGE_IMAGE.to_string(),
            large_text: "Google Antigravity".to_string(),
            language: "es".to_string(),
            poll_interval_sec: 3,
            show_conversation_title: false,
            show_project_stats: true,
            show_git_branch: true,
            show_agent_status: true,
            show_repo_button: true,
            reset_timer_on_project_change: true,
            fallback_to_last_project: false,
        }
    }
}

impl AppConfig {
    pub fn is_english(&self) -> bool {
        self.language.trim().eq_ignore_ascii_case("en")
    }
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn log_path() -> PathBuf {
    data_dir().join("antigravity_rpc.log")
}

pub fn load_config() -> AppConfig {
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);
    let path = config_path();

    if !path.exists() {
        let legacy = exe_dir().join("config.json");
        if legacy.exists() {
            if let Ok(raw) = fs::read_to_string(&legacy) {
                if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&raw) {
                    normalize(&mut cfg);
                    let _ = save_config(&cfg);
                    return cfg;
                }
            }
        }
        let cfg = AppConfig::default();
        let _ = save_config(&cfg);
        return cfg;
    }

    match fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<AppConfig>(&raw).ok())
    {
        Some(mut cfg) => {
            normalize(&mut cfg);
            cfg
        }
        None => AppConfig::default(),
    }
}

fn normalize(cfg: &mut AppConfig) {
    let trimmed_id = cfg.client_id.trim();
    cfg.client_id = if trimmed_id.is_empty() {
        DEFAULT_CLIENT_ID.to_string()
    } else {
        trimmed_id.to_string()
    };
    if cfg.large_image.trim().is_empty() {
        cfg.large_image = DEFAULT_LARGE_IMAGE.to_string();
    } else {
        cfg.large_image = cfg.large_image.trim().to_string();
    }
    if cfg.large_text.trim().is_empty() {
        cfg.large_text = "Google Antigravity".to_string();
    }
    cfg.language = if cfg.language.trim().eq_ignore_ascii_case("en") {
        "en".to_string()
    } else {
        "es".to_string()
    };
    cfg.poll_interval_sec = cfg.poll_interval_sec.clamp(2, 30);
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Error creando carpeta de datos: {e}"))?;
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("Error serializando configuración: {e}"))?;
    fs::write(config_path(), format!("{json}\n"))
        .map_err(|e| format!("Error escribiendo config.json: {e}"))
}

pub fn is_startup_enabled() -> bool {
    let subkey = to_wide(STARTUP_REG_PATH);
    let value = to_wide(STARTUP_VALUE_NAME);
    let mut hkey: HKEY = ptr::null_mut();
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != ERROR_SUCCESS
        {
            return false;
        }
        let res = RegQueryValueExW(
            hkey,
            value.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        RegCloseKey(hkey);
        res == ERROR_SUCCESS
    }
}

pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    let subkey = to_wide(STARTUP_REG_PATH);
    let value = to_wide(STARTUP_VALUE_NAME);
    let mut hkey: HKEY = ptr::null_mut();
    unsafe {
        let open_res = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if open_res != ERROR_SUCCESS {
            return Err(format!("No se pudo abrir la clave de inicio ({open_res})"));
        }

        let op_res = if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| format!("No se pudo obtener la ruta del ejecutable: {e}"))?;
            let cmd = format!("\"{}\"", exe.display());
            let cmd_w = to_wide(&cmd);
            let byte_len = (cmd_w.len() * std::mem::size_of::<u16>()) as u32;
            RegSetValueExW(
                hkey,
                value.as_ptr(),
                0,
                REG_SZ,
                cmd_w.as_ptr() as *const u8,
                byte_len,
            )
        } else {
            RegDeleteValueW(hkey, value.as_ptr())
        };

        RegCloseKey(hkey);
        if enabled && op_res != ERROR_SUCCESS {
            return Err(format!("No se pudo guardar el arranque automático ({op_res})"));
        }
    }
    Ok(())
}
