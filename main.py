"""Punto de entrada principal para Antigravity Discord Rich Presence."""

from __future__ import annotations

import argparse
import ctypes
import logging
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

from config_store import APP_DIR, ASSETS_DIR, CONFIG_PATH, DATA_DIR, LOG_PATH, AppConfig, load_config
from detector import AntigravityState, detect_antigravity_state
from discord_ipc import DiscordIPC, DiscordIPCError
from setup_ui import open_config_dialog
from tray_win32 import Win32TrayApp


DATA_DIR.mkdir(parents=True, exist_ok=True)
logging.basicConfig(
    filename=str(LOG_PATH),
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    encoding="utf-8",
)
logger = logging.getLogger("antigravity_rpc")


def _acquire_single_instance_mutex() -> bool:
    kernel32 = ctypes.windll.kernel32
    handle = kernel32.CreateMutexW(None, False, "Local\\AntigravityDiscordRPCMutex")
    if not handle:
        return True
    last_err = kernel32.GetLastError()
    ERROR_ALREADY_EXISTS = 183
    return last_err != ERROR_ALREADY_EXISTS


def build_presence_fields(state: AntigravityState, cfg: AppConfig) -> tuple[str, str, str]:
    """Devuelve (details, state_text, large_text) para Discord."""
    surface_label = "Antigravity IDE" if state.surface == "ide" else "Google Antigravity"
    if state.project:
        details = f"Proyecto: {state.project}"
        if cfg.show_conversation_title:
            if state.surface == "ide":
                second = f"Editando {state.active_file}" if state.active_file else "En Antigravity IDE"
            else:
                second = state.detail_line or "Trabajando con agente"
        else:
            second = "En Antigravity IDE" if state.surface == "ide" else "En Antigravity"
    else:
        details = "En Antigravity IDE" if state.surface == "ide" else "En Antigravity"
        if cfg.show_conversation_title and state.detail_line:
            second = state.detail_line
        else:
            second = "Sin proyecto activo"
    return details, second, cfg.large_text or surface_label


class PresenceRunner:
    """Bucle en segundo plano que sincroniza Antigravity -> Discord RPC."""

    def __init__(self, *, cli_mode: bool = False) -> None:
        self.cli_mode = cli_mode
        self.stop_event = threading.Event()
        self.reconnect_event = threading.Event()
        self.tray: Win32TrayApp | None = None
        self._status_lock = threading.Lock()
        self._status_text = "Iniciando..."
        self._ipc: DiscordIPC | None = None

    def get_status_text(self) -> str:
        with self._status_lock:
            return self._status_text

    def _set_status(self, text: str) -> None:
        with self._status_lock:
            self._status_text = text
        if self.tray is not None:
            self.tray.update_tooltip(f"Antigravity RPC - {text}")
        if self.cli_mode:
            print(f"[Antigravity RPC] {text}", flush=True)

    def request_reconnect(self) -> None:
        self.reconnect_event.set()

    def stop(self) -> None:
        self.stop_event.set()
        if self._ipc is not None:
            try:
                self._ipc.clear_activity()
            except Exception:
                pass
            self._ipc.close()

    def run_loop(self) -> None:
        cfg = load_config()
        last_cfg_mtime = CONFIG_PATH.stat().st_mtime if CONFIG_PATH.exists() else 0.0
        self._ipc = DiscordIPC(cfg.client_id)

        last_payload_key: tuple[object, ...] | None = None
        last_project_key: str | None = None
        start_ts: int | None = None

        while not self.stop_event.is_set():
            try:
                if CONFIG_PATH.exists():
                    mtime = CONFIG_PATH.stat().st_mtime
                    if mtime != last_cfg_mtime:
                        last_cfg_mtime = mtime
                        new_cfg = load_config()
                        if new_cfg.client_id != cfg.client_id:
                            self._ipc.close()
                            self._ipc = DiscordIPC(new_cfg.client_id)
                        cfg = new_cfg
                        last_payload_key = None

                if self.reconnect_event.is_set():
                    self.reconnect_event.clear()
                    self._ipc.close()
                    last_payload_key = None

                if not cfg.client_id:
                    self._set_status("Falta Client ID (clic derecho -> Configurar)")
                    self.stop_event.wait(2.0)
                    continue

                state = detect_antigravity_state(fallback_to_last_project=cfg.fallback_to_last_project)

                if not state.running:
                    if last_payload_key is not None:
                        try:
                            self._ipc.clear_activity()
                        except Exception:
                            self._ipc.close()
                        last_payload_key = None
                        last_project_key = None
                        start_ts = None
                    self._set_status("Antigravity cerrado")
                    self.stop_event.wait(cfg.poll_interval_sec)
                    continue

                proj_key = f"{state.surface}:{state.project or ''}"
                if start_ts is None or (cfg.reset_timer_on_project_change and proj_key != last_project_key):
                    start_ts = int(time.time())
                last_project_key = proj_key

                details, second, large_text = build_presence_fields(state, cfg)
                payload_key = (details, second, start_ts, cfg.large_image, large_text)

                if not self._ipc.connected or payload_key != last_payload_key:
                    self._ipc.set_activity(
                        details=details,
                        state=second,
                        start_timestamp=start_ts,
                        large_image=cfg.large_image,
                        large_text=large_text,
                    )
                    last_payload_key = payload_key
                    status_summary = state.project or "Sin proyecto"
                    self._set_status(f"Conectado ({status_summary})")
                    logger.info("Actividad actualizada: %s | %s", details, second)

            except DiscordIPCError as exc:
                self._set_status("Esperando a Discord...")
                logger.debug("DiscordIPCError: %s", exc)
                if self._ipc is not None:
                    self._ipc.close()
                last_payload_key = None
            except Exception as exc:
                logger.exception("Error inesperado en el bucle de presencia: %s", exc)
                self._set_status("Error interno (ver log)")

            self.stop_event.wait(cfg.poll_interval_sec)


def _launch_setup_subprocess() -> None:
    creationflags = 0x08000000 if os.name == "nt" else 0
    if getattr(sys, "frozen", False):
        cmd = [sys.executable, "--setup"]
    else:
        cmd = [sys.executable, str(APP_DIR / "setup_ui.py")]
    subprocess.Popen(cmd, creationflags=creationflags)


def main() -> int:
    parser = argparse.ArgumentParser(description="Discord Rich Presence para Google Antigravity")
    parser.add_argument("--cli", action="store_true", help="Ejecuta en consola sin icono de bandeja")
    parser.add_argument("--setup", action="store_true", help="Abre la ventana de configuración")
    args = parser.parse_args()

    if args.setup:
        open_config_dialog()
        return 0

    cfg = load_config()
    if not cfg.client_id and not args.cli:
        saved = open_config_dialog()
        if saved is None or not saved.client_id:
            return 0

    if not args.cli and not _acquire_single_instance_mutex():
        return 0

    runner = PresenceRunner(cli_mode=args.cli)

    if args.cli:
        try:
            runner.run_loop()
        except KeyboardInterrupt:
            runner.stop()
        return 0

    worker = threading.Thread(target=runner.run_loop, daemon=True)
    worker.start()

    tray = Win32TrayApp(
        ASSETS_DIR / "antigravity-icon.ico",
        get_status_text=runner.get_status_text,
        on_configure=_launch_setup_subprocess,
        on_open_assets=lambda: os.startfile(str(ASSETS_DIR)),
        on_reconnect=runner.request_reconnect,
        on_exit=runner.stop,
    )
    runner.tray = tray
    try:
        tray.run()
    finally:
        runner.stop()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
