"""Gestión de configuración y arranque automático con Windows."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import json
import os
from pathlib import Path
import sys
import winreg


def _is_frozen() -> bool:
    return bool(getattr(sys, "frozen", False))


def _resolve_bundle_dir() -> Path:
    if _is_frozen() and hasattr(sys, "_MEIPASS"):
        return Path(getattr(sys, "_MEIPASS"))
    return Path(__file__).resolve().parent


def _resolve_app_dir() -> Path:
    if _is_frozen():
        return Path(sys.executable).resolve().parent
    return Path(__file__).resolve().parent


BUNDLE_DIR = _resolve_bundle_dir()
APP_DIR = _resolve_app_dir()
ASSETS_DIR = APP_DIR / "assets" if (APP_DIR / "assets").exists() else BUNDLE_DIR / "assets"
DATA_DIR = Path(os.environ.get("APPDATA") or Path.home()) / "AntigravityRichPresence"
CONFIG_PATH = DATA_DIR / "config.json"
LOG_PATH = DATA_DIR / "antigravity_rpc.log"
LEGACY_CONFIG_PATH = APP_DIR / "config.json"
STARTUP_REG_PATH = r"Software\Microsoft\Windows\CurrentVersion\Run"
STARTUP_VALUE_NAME = "AntigravityDiscordRPC"


@dataclass
class AppConfig:
    client_id: str = "1571349694016266300"
    large_image: str = "antigravity"
    large_text: str = "Google Antigravity"
    poll_interval_sec: int = 3
    show_conversation_title: bool = True
    reset_timer_on_project_change: bool = True
    fallback_to_last_project: bool = False


def load_config() -> AppConfig:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    if not CONFIG_PATH.exists():
        # Si existe un config.json junto al ejecutable/código, lo toma como semilla inicial
        seed_path = LEGACY_CONFIG_PATH if LEGACY_CONFIG_PATH.exists() else None
        if seed_path is not None:
            try:
                raw = json.loads(seed_path.read_text(encoding="utf-8"))
                defaults = asdict(AppConfig())
                merged = {k: raw.get(k, v) for k, v in defaults.items()}
                cfg = AppConfig(**merged)
                save_config(cfg)
                return cfg
            except Exception:
                pass
        cfg = AppConfig()
        save_config(cfg)
        return cfg
    try:
        raw = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
        defaults = asdict(AppConfig())
        merged = {k: raw.get(k, v) for k, v in defaults.items()}
        merged["client_id"] = str(merged["client_id"]).strip()
        merged["poll_interval_sec"] = max(2, min(30, int(merged["poll_interval_sec"])))
        return AppConfig(**merged)
    except Exception:
        return AppConfig()


def save_config(cfg: AppConfig) -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    CONFIG_PATH.write_text(
        json.dumps(asdict(cfg), indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def _get_pythonw_executable() -> str:
    exe = Path(sys.executable)
    pythonw = exe.with_name("pythonw.exe")
    if pythonw.exists():
        return str(pythonw)
    return str(exe)


def get_launch_command() -> str:
    if _is_frozen():
        return f'"{Path(sys.executable).resolve()}"'
    pythonw = _get_pythonw_executable()
    main_py = APP_DIR / "main.py"
    return f'"{pythonw}" "{main_py}"'


def is_startup_enabled() -> bool:
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, STARTUP_REG_PATH, 0, winreg.KEY_READ) as key:
            winreg.QueryValueEx(key, STARTUP_VALUE_NAME)
            return True
    except FileNotFoundError:
        return False
    except OSError:
        return False


def set_startup_enabled(enabled: bool) -> None:
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, STARTUP_REG_PATH, 0, winreg.KEY_SET_VALUE) as key:
        if enabled:
            winreg.SetValueEx(key, STARTUP_VALUE_NAME, 0, winreg.REG_SZ, get_launch_command())
        else:
            try:
                winreg.DeleteValue(key, STARTUP_VALUE_NAME)
            except FileNotFoundError:
                pass
