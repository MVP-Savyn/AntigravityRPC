"""Pure-Python Discord Rich Presence IPC client for Windows."""

from __future__ import annotations

import json
import os
import struct
import uuid
from typing import Any


OP_HANDSHAKE = 0
OP_FRAME = 1
OP_CLOSE = 2
OP_PING = 3
OP_PONG = 4


class DiscordIPCError(Exception):
    """Raised when Discord IPC communication fails."""


def _clamp_str(value: str | None, min_len: int = 2, max_len: int = 128) -> str | None:
    if not value:
        return None
    text = value.strip()
    if not text:
        return None
    if len(text) < min_len:
        text = text.ljust(min_len)
    if len(text) > max_len:
        text = text[: max_len - 3].rstrip() + "..."
    return text


class DiscordIPC:
    """Minimal synchronous Discord Rich Presence client over Windows named pipes."""

    def __init__(self, client_id: str) -> None:
        self.client_id = str(client_id).strip()
        self._pipe: Any = None
        self._connected = False

    @property
    def connected(self) -> bool:
        return self._connected and self._pipe is not None

    def connect(self) -> None:
        self.close()
        if not self.client_id:
            raise DiscordIPCError("Discord Client ID no configurado.")

        last_err: Exception | None = None
        for idx in range(10):
            pipe_path = rf"\\.\pipe\discord-ipc-{idx}"
            if not os.path.exists(pipe_path):
                continue
            try:
                self._pipe = open(pipe_path, "r+b", buffering=0)
                self._send(OP_HANDSHAKE, {"v": 1, "client_id": self.client_id})
                op, data = self._recv()
                if op == OP_CLOSE:
                    msg = data.get("message", "Handshake rechazado por Discord")
                    raise DiscordIPCError(f"Discord cerró la conexión: {msg}")
                if data.get("evt") == "ERROR":
                    msg = data.get("data", {}).get("message", "Error en handshake")
                    raise DiscordIPCError(msg)
                self._connected = True
                return
            except Exception as exc:
                last_err = exc
                self.close()

        if last_err is not None:
            raise DiscordIPCError(f"No se pudo conectar con Discord: {last_err}") from last_err
        raise DiscordIPCError("Discord no está abierto o no expone ningún pipe IPC.")

    def _send(self, op: int, payload: dict[str, Any]) -> None:
        if self._pipe is None:
            raise DiscordIPCError("Pipe IPC no abierto.")
        encoded = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        header = struct.pack("<II", op, len(encoded))
        try:
            self._pipe.write(header + encoded)
            self._pipe.flush()
        except OSError as exc:
            self.close()
            raise DiscordIPCError(f"Error enviando datos a Discord: {exc}") from exc

    def _recv(self) -> tuple[int, dict[str, Any]]:
        if self._pipe is None:
            raise DiscordIPCError("Pipe IPC no abierto.")
        try:
            header = self._read_exact(8)
            op, length = struct.unpack("<II", header)
            raw = self._read_exact(length) if length > 0 else b"{}"
            return op, json.loads(raw.decode("utf-8", errors="replace"))
        except OSError as exc:
            self.close()
            raise DiscordIPCError(f"Error leyendo respuesta de Discord: {exc}") from exc

    def _read_exact(self, size: int) -> bytes:
        chunks: list[bytes] = []
        remaining = size
        while remaining > 0:
            chunk = self._pipe.read(remaining)
            if not chunk:
                raise OSError("Conexión IPC cerrada por Discord.")
            chunks.append(chunk)
            remaining -= len(chunk)
        return b"".join(chunks)

    def set_activity(
        self,
        *,
        details: str | None = None,
        state: str | None = None,
        start_timestamp: int | None = None,
        large_image: str | None = None,
        large_text: str | None = None,
        small_image: str | None = None,
        small_text: str | None = None,
    ) -> None:
        if not self.connected:
            self.connect()

        activity: dict[str, Any] = {}
        clamped_details = _clamp_str(details)
        clamped_state = _clamp_str(state)
        if clamped_details:
            activity["details"] = clamped_details
        if clamped_state:
            activity["state"] = clamped_state
        if start_timestamp is not None:
            activity["timestamps"] = {"start": int(start_timestamp)}

        assets: dict[str, str] = {}
        if large_image:
            assets["large_image"] = large_image.strip()
        if large_text:
            clamped_lt = _clamp_str(large_text)
            if clamped_lt:
                assets["large_text"] = clamped_lt
        if small_image:
            assets["small_image"] = small_image.strip()
        if small_text:
            clamped_st = _clamp_str(small_text)
            if clamped_st:
                assets["small_text"] = clamped_st
        if assets:
            activity["assets"] = assets

        payload = {
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": os.getpid(),
                "activity": activity,
            },
            "nonce": str(uuid.uuid4()),
        }
        self._send(OP_FRAME, payload)
        op, resp = self._recv()
        if op == OP_CLOSE:
            raise DiscordIPCError("Discord cerró el canal IPC.")
        if resp.get("evt") == "ERROR":
            msg = resp.get("data", {}).get("message", "Error al actualizar actividad")
            raise DiscordIPCError(msg)

    def clear_activity(self) -> None:
        if not self.connected:
            return
        payload = {
            "cmd": "SET_ACTIVITY",
            "args": {"pid": os.getpid()},
            "nonce": str(uuid.uuid4()),
        }
        self._send(OP_FRAME, payload)
        self._recv()

    def close(self) -> None:
        if self._pipe is not None:
            try:
                self._pipe.close()
            except Exception:
                pass
        self._pipe = None
        self._connected = False
