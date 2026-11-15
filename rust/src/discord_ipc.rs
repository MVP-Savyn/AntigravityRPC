use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_nonce() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("agy-{nanos}-{seq}")
}

fn clamp_str(input: &str, max_chars: usize) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let count = trimmed.chars().count();
    let mut out = if count > max_chars {
        let cut: String = trimmed.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", cut.trim_end())
    } else {
        trimmed.to_string()
    };
    if out.chars().count() < 2 {
        out.push(' ');
    }
    Some(out)
}

pub struct DiscordIpc {
    client_id: String,
    pipe: Option<File>,
}

impl DiscordIpc {
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into().trim().to_string(),
            pipe: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.pipe.is_some()
    }

    pub fn set_client_id(&mut self, client_id: impl Into<String>) {
        let next = client_id.into().trim().to_string();
        if next != self.client_id {
            self.client_id = next;
            self.close();
        }
    }

    pub fn close(&mut self) {
        self.pipe = None;
    }

    pub fn connect(&mut self) -> Result<(), String> {
        self.close();
        if self.client_id.is_empty() {
            return Err("Discord Client ID no configurado.".to_string());
        }

        let mut last_err: Option<String> = None;
        for idx in 0..10 {
            let path = format!(r"\\.\pipe\discord-ipc-{idx}");
            match OpenOptions::new().read(true).write(true).open(&path) {
                Ok(file) => {
                    self.pipe = Some(file);
                    let handshake = json!({
                        "v": 1,
                        "client_id": self.client_id,
                    });
                    if let Err(err) = self.send_frame(OP_HANDSHAKE, &handshake) {
                        last_err = Some(err);
                        self.close();
                        continue;
                    }
                    match self.recv_frame() {
                        Ok((op, data)) => {
                            if op == OP_CLOSE {
                                let msg = data
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Handshake rechazado por Discord");
                                last_err = Some(msg.to_string());
                                self.close();
                                continue;
                            }
                            if data.get("evt").and_then(|v| v.as_str()) == Some("ERROR") {
                                let msg = data
                                    .pointer("/data/message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Error en handshake");
                                last_err = Some(msg.to_string());
                                self.close();
                                continue;
                            }
                            return Ok(());
                        }
                        Err(err) => {
                            last_err = Some(err);
                            self.close();
                        }
                    }
                }
                Err(err) => {
                    last_err = Some(err.to_string());
                }
            }
        }

        Err(last_err.unwrap_or_else(|| "Discord no está abierto o no expone ningún pipe IPC.".to_string()))
    }

    fn send_frame(&mut self, op: u32, payload: &Value) -> Result<(), String> {
        let pipe = self.pipe.as_mut().ok_or_else(|| "Pipe IPC no abierto".to_string())?;
        let body = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
        let mut buf = Vec::with_capacity(8 + body.len());
        buf.extend_from_slice(&op.to_le_bytes());
        buf.extend_from_slice(&(body.len() as u32).to_le_bytes());
        buf.extend_from_slice(&body);
        if let Err(err) = pipe.write_all(&buf).and_then(|_| pipe.flush()) {
            self.close();
            return Err(format!("Error enviando a Discord: {err}"));
        }
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<(u32, Value), String> {
        let pipe = self.pipe.as_mut().ok_or_else(|| "Pipe IPC no abierto".to_string())?;
        let mut header = [0u8; 8];
        if let Err(err) = pipe.read_exact(&mut header) {
            self.close();
            return Err(format!("Error leyendo cabecera de Discord: {err}"));
        }
        let op = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        if len > 1_048_576 {
            self.close();
            return Err("Frame IPC demasiado grande".to_string());
        }
        let mut body = vec![0u8; len];
        if len > 0 {
            if let Err(err) = pipe.read_exact(&mut body) {
                self.close();
                return Err(format!("Error leyendo payload de Discord: {err}"));
            }
        }
        let val = serde_json::from_slice(&body).unwrap_or(Value::Null);
        Ok((op, val))
    }

    pub fn set_activity(
        &mut self,
        details: &str,
        state: Option<&str>,
        start_timestamp: i64,
        large_image: &str,
        large_text: &str,
        button: Option<(&str, &str)>,
    ) -> Result<(), String> {
        if !self.is_connected() {
            self.connect()?;
        }

        let mut activity = serde_json::Map::new();
        if let Some(d) = clamp_str(details, 128) {
            activity.insert("details".to_string(), Value::String(d));
        }
        if let Some(st) = state {
            if let Some(s) = clamp_str(st, 128) {
                activity.insert("state".to_string(), Value::String(s));
            }
        }
        activity.insert("timestamps".to_string(), json!({ "start": start_timestamp }));

        let mut assets = serde_json::Map::new();
        if !large_image.trim().is_empty() {
            assets.insert("large_image".to_string(), Value::String(large_image.trim().to_string()));
        }
        if let Some(lt) = clamp_str(large_text, 128) {
            assets.insert("large_text".to_string(), Value::String(lt));
        }
        if !assets.is_empty() {
            activity.insert("assets".to_string(), Value::Object(assets));
        }

        if let Some((label, url)) = button {
            if let Some(lbl) = clamp_str(label, 32) {
                if url.starts_with("https://") || url.starts_with("http://") {
                    activity.insert(
                        "buttons".to_string(),
                        json!([{ "label": lbl, "url": url }]),
                    );
                }
            }
        }

        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": Value::Object(activity),
            },
            "nonce": next_nonce(),
        });

        self.send_frame(OP_FRAME, &payload)?;
        let (op, resp) = self.recv_frame()?;
        if op == OP_CLOSE {
            self.close();
            return Err("Discord cerró el canal IPC".to_string());
        }
        if resp.get("evt").and_then(|v| v.as_str()) == Some("ERROR") {
            let msg = resp
                .pointer("/data/message")
                .and_then(|v| v.as_str())
                .unwrap_or("Error actualizando actividad");
            return Err(msg.to_string());
        }
        Ok(())
    }

    pub fn clear_activity(&mut self) {
        if !self.is_connected() {
            return;
        }
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id() },
            "nonce": next_nonce(),
        });
        if self.send_frame(OP_FRAME, &payload).is_ok() {
            let _ = self.recv_frame();
        }
    }
}
