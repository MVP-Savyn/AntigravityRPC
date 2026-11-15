"""Detecta si Antigravity (2.0 o IDE) está abierto y en qué proyecto trabaja el usuario."""

from __future__ import annotations

import ctypes
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import sqlite3
from urllib.parse import unquote, urlparse

try:
    import psutil
except ImportError:  # pragma: no cover
    psutil = None


@dataclass(frozen=True)
class AntigravityState:
    running: bool
    surface: str  # "app" | "ide" | "none"
    project: str | None = None
    detail_line: str | None = None
    active_file: str | None = None
    is_busy: bool = False


def _decode_workspace_uri(uri: str) -> str | None:
    if not uri:
        return None
    parsed = urlparse(uri)
    raw_path = unquote(parsed.path or uri)
    # Windows paths like /E:/vscode/SAVIN-HOLLOWDRIVE -> E:/vscode/SAVIN-HOLLOWDRIVE
    if re.match(r"^/[A-Za-z]:", raw_path):
        raw_path = raw_path[1:]
    raw_path = raw_path.rstrip("/\\")
    if not raw_path:
        return None
    name = os.path.basename(raw_path)
    return name or raw_path


def _parse_workspace_uris(raw_uris: str) -> str | None:
    if not raw_uris:
        return None
    raw_uris = raw_uris.strip()
    if not raw_uris or raw_uris == "[]":
        return None
    try:
        items = json.loads(raw_uris)
        if isinstance(items, list):
            for item in items:
                if isinstance(item, str):
                    name = _decode_workspace_uri(item)
                    if name:
                        return name
    except json.JSONDecodeError:
        return _decode_workspace_uri(raw_uris)
    return None


def _get_running_antigravity_pids() -> tuple[set[int], set[int]]:
    """Devuelve (app_pids, ide_pids) para Antigravity 2.0 y Antigravity IDE."""
    app_pids: set[int] = set()
    ide_pids: set[int] = set()

    if psutil is not None:
        for proc in psutil.process_iter(["pid", "name", "exe"]):
            try:
                name = (proc.info.get("name") or "").lower()
                exe = (proc.info.get("exe") or "").lower()
                pid = int(proc.info["pid"])
                if name == "antigravity ide.exe" or "antigravity ide" in exe:
                    ide_pids.add(pid)
                elif name == "antigravity.exe":
                    app_pids.add(pid)
            except (psutil.NoSuchProcess, psutil.AccessDenied, KeyError, TypeError):
                continue
        return app_pids, ide_pids

    # Fallback sin psutil usando tasklist
    import subprocess

    for image_name, target in [("Antigravity IDE.exe", ide_pids), ("Antigravity.exe", app_pids)]:
        try:
            out = subprocess.check_output(
                ["tasklist", "/FI", f"IMAGENAME eq {image_name}", "/FO", "CSV", "/NH"],
                text=True,
                creationflags=0x08000000,
            )
            for line in out.strip().splitlines():
                parts = [p.strip('"') for p in line.split('","')]
                if len(parts) >= 2 and parts[1].isdigit():
                    target.add(int(parts[1]))
        except Exception:
            pass
    return app_pids, ide_pids


def _get_foreground_pid() -> int | None:
    try:
        user32 = ctypes.windll.user32
        hwnd = user32.GetForegroundWindow()
        if not hwnd:
            return None
        pid = ctypes.c_ulong()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        return int(pid.value)
    except Exception:
        return None


def _get_window_titles_for_pids(pids: set[int]) -> list[str]:
    if not pids:
        return []
    titles: list[str] = []
    try:
        user32 = ctypes.windll.user32
        enum_proc = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_void_p)

        def callback(hwnd: int, _lparam: int) -> bool:
            if not user32.IsWindowVisible(hwnd):
                return True
            pid = ctypes.c_ulong()
            user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
            if pid.value not in pids:
                return True
            length = user32.GetWindowTextLengthW(hwnd)
            if length > 0:
                buf = ctypes.create_unicode_buffer(length + 1)
                user32.GetWindowTextW(hwnd, buf, length + 1)
                text = buf.value.strip()
                if text:
                    titles.append(text)
            return True

        user32.EnumWindows(enum_proc(callback), 0)
    except Exception:
        pass
    return titles


def _detect_ide_project(ide_pids: set[int]) -> tuple[str | None, str | None]:
    """Extrae (project, active_file) desde las ventanas de Antigravity IDE o storage.json."""
    titles = _get_window_titles_for_pids(ide_pids)
    ignored_tabs = {"welcome", "get started", "antigravity", "settings", "keyboard shortcuts"}

    for title in titles:
        clean = title.lstrip("●• ").strip()
        parts = [p.strip() for p in re.split(r"\s+[-\u2014]\s+", clean) if p.strip()]
        if not parts:
            continue
        if parts[-1].lower().startswith("antigravity"):
            parts = parts[:-1]
        if not parts:
            continue
        if len(parts) >= 2:
            project = parts[-1]
            candidate_file = parts[0]
            if candidate_file.lower() in ignored_tabs:
                candidate_file = None
            return project, candidate_file
        if len(parts) == 1 and parts[0].lower() not in ignored_tabs:
            return parts[0], None

    # Fallback: storage.json de VS Code / Antigravity IDE
    storage_path = Path(os.path.expandvars(r"%APPDATA%\Antigravity\User\globalStorage\storage.json"))
    if storage_path.exists():
        try:
            data = json.loads(storage_path.read_text(encoding="utf-8"))
            win_state = data.get("windowsState", {})
            for win in win_state.get("openedWindows", []):
                folder = win.get("folder") or win.get("workspaceIdentifier", {}).get("configURIPath")
                name = _decode_workspace_uri(folder or "")
                if name:
                    return name, None
            last_win = win_state.get("lastActiveWindow", {})
            folder = last_win.get("folder")
            name = _decode_workspace_uri(folder or "")
            if name:
                return name, None
        except Exception:
            pass
    return None, None


def _detect_app_project(fallback_to_last_project: bool = False) -> tuple[str | None, str | None, bool]:
    """Lee conversation_summaries.db en modo solo-lectura para Antigravity 2.0."""
    db_path = Path.home() / ".gemini" / "antigravity" / "conversation_summaries.db"
    if not db_path.exists():
        return None, None, False

    conn: sqlite3.Connection | None = None
    try:
        conn = sqlite3.connect(f"file:{db_path.as_posix()}?mode=ro", uri=True, timeout=1.0)
        cur = conn.cursor()
        cur.execute(
            """
            SELECT conversation_id, title, workspace_uris, project_id, not_fully_idle
            FROM conversation_summaries
            ORDER BY last_modified_time DESC
            LIMIT 10
            """
        )
        rows = cur.fetchall()
        if not rows:
            return None, None, False

        conv_id, title, workspace_uris, project_id, not_fully_idle = rows[0]
        project = _parse_workspace_uris(workspace_uris)

        if not project and conv_id:
            cur.execute(
                """
                SELECT project, workspace
                FROM conversations_fts
                WHERE conversation_id = ?
                LIMIT 1
                """,
                (conv_id,),
            )
            fts_row = cur.fetchone()
            if fts_row:
                proj_fts, ws_fts = fts_row
                project = (proj_fts or "").strip() or unquote((ws_fts or "").strip()) or None

        if not project and fallback_to_last_project:
            for _, _, ws_uris, _, _ in rows[1:]:
                project = _parse_workspace_uris(ws_uris)
                if project:
                    break

        clean_title = (title or "").strip() or None
        return project, clean_title, bool(not_fully_idle)
    except Exception:
        return None, None, False
    finally:
        if conn is not None:
            try:
                conn.close()
            except Exception:
                pass


def detect_antigravity_state(fallback_to_last_project: bool = False) -> AntigravityState:
    """Detecta el estado actual de Antigravity (IDE o Desktop App 2.0)."""
    app_pids, ide_pids = _get_running_antigravity_pids()
    if not app_pids and not ide_pids:
        return AntigravityState(running=False, surface="none")

    fg_pid = _get_foreground_pid()
    prefer_ide = False
    if ide_pids and not app_pids:
        prefer_ide = True
    elif ide_pids and fg_pid in ide_pids:
        prefer_ide = True
    elif app_pids and fg_pid in app_pids:
        prefer_ide = False
    elif ide_pids:
        # Si ambos están abiertos y ninguno está en primer plano, revisa si el IDE tiene título con proyecto
        ide_project, _ = _detect_ide_project(ide_pids)
        if ide_project:
            prefer_ide = True

    if prefer_ide:
        project, active_file = _detect_ide_project(ide_pids)
        return AntigravityState(
            running=True,
            surface="ide",
            project=project,
            active_file=active_file,
        )

    project, title, is_busy = _detect_app_project(fallback_to_last_project=fallback_to_last_project)
    return AntigravityState(
        running=True,
        surface="app",
        project=project,
        detail_line=title,
        is_busy=is_busy,
    )
