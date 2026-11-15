use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, INVALID_HANDLE_VALUE, LPARAM};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible,
};

use crate::winutil::{from_wide_slice, home_dir, percent_decode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    None,
    App,
    Ide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntigravityState {
    pub running: bool,
    pub surface: Surface,
    pub project: Option<String>,
    pub detail_line: Option<String>,
    pub active_file: Option<String>,
    pub git_branch: Option<String>,
    pub repo_url: Option<String>,
    pub interactions: u64,
    pub is_busy: bool,
    pub project_days: u32,
    pub project_active_secs: u64,
}

static SIDEBAR_TRACKER: Mutex<SidebarProjectTracker> = Mutex::new(SidebarProjectTracker {
    last_selected_project: None,
    last_annotation_id: None,
    prefer_sidebar_project: false,
});

struct SidebarProjectTracker {
    last_selected_project: Option<String>,
    last_annotation_id: Option<String>,
    prefer_sidebar_project: bool,
}

fn decode_workspace_path(uri: &str) -> Option<PathBuf> {
    let trimmed = uri.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_scheme = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    let mut decoded = percent_decode(without_scheme);
    let bytes = decoded.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        decoded.remove(0);
    }
    let clean = decoded.trim_end_matches(['/', '\\']);
    if clean.is_empty() {
        return None;
    }
    Some(PathBuf::from(clean))
}

fn folder_name_from_path(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn normalize_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

fn parse_workspace_uris(raw: &str) -> (Option<String>, Option<PathBuf>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return (None, None);
    }
    if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(trimmed) {
        for item in items {
            if let Some(s) = item.as_str() {
                if let Some(path) = decode_workspace_path(s) {
                    let name = folder_name_from_path(&path);
                    return (name, Some(path));
                }
            }
        }
        return (None, None);
    }
    let path = decode_workspace_path(trimmed);
    let name = path.as_ref().and_then(|p| folder_name_from_path(p));
    (name, path)
}

fn resolve_git_dir(project_dir: &Path) -> Option<PathBuf> {
    let dot_git = project_dir.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    if dot_git.is_file() {
        if let Ok(txt) = fs::read_to_string(&dot_git) {
            if let Some(rest) = txt.trim().strip_prefix("gitdir:") {
                let target = PathBuf::from(rest.trim());
                return Some(if target.is_absolute() {
                    target
                } else {
                    project_dir.join(target)
                });
            }
        }
    }
    None
}

fn normalize_git_remote_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.starts_with("https://") || url.starts_with("http://") {
        let clean = url.trim_end_matches(".git").trim_end_matches('/');
        return Some(clean.to_string());
    }
    if let Some(rest) = url.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            let clean_path = path.trim_end_matches(".git").trim_end_matches('/');
            return Some(format!("https://{host}/{clean_path}"));
        }
    }
    None
}

fn read_git_info(project_dir: &Path) -> (Option<String>, Option<String>) {
    let Some(git_dir) = resolve_git_dir(project_dir) else {
        return (None, None);
    };

    let branch = fs::read_to_string(git_dir.join("HEAD")).ok().and_then(|head| {
        let t = head.trim();
        if let Some(ref_path) = t.strip_prefix("ref: refs/heads/") {
            Some(ref_path.trim().to_string())
        } else if let Some(ref_path) = t.strip_prefix("ref: ") {
            ref_path.rsplit('/').next().map(|s| s.to_string())
        } else if t.len() >= 7 {
            Some(t[..7].to_string())
        } else {
            None
        }
    });

    let repo_url = fs::read_to_string(git_dir.join("config"))
        .ok()
        .and_then(|cfg| {
            let mut in_origin = false;
            for line in cfg.lines() {
                let t = line.trim();
                if t.starts_with('[') {
                    in_origin = t.eq_ignore_ascii_case("[remote \"origin\"]");
                    continue;
                }
                if in_origin {
                    if let Some(val) = t.strip_prefix("url") {
                        if let Some((_, rhs)) = val.split_once('=') {
                            if let Some(norm) = normalize_git_remote_url(rhs) {
                                return Some(norm);
                            }
                        }
                    }
                }
            }
            None
        });

    (branch, repo_url)
}

fn read_project_json(project_id: &str) -> (Option<String>, Option<PathBuf>) {
    let trimmed = project_id.trim();
    if trimmed.is_empty() || trimmed == "outside-of-project" {
        return (None, None);
    }
    let Some(home) = home_dir() else {
        return (None, None);
    };
    let proj_file = home
        .join(".gemini")
        .join("config")
        .join("projects")
        .join(format!("{trimmed}.json"));
    let Ok(raw) = fs::read_to_string(proj_file) else {
        return (None, None);
    };
    let Ok(val) = serde_json::from_str::<Value>(&raw) else {
        return (None, None);
    };
    let name = val
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut folder_path: Option<PathBuf> = None;
    if let Some(resources) = val
        .pointer("/projectResources/resources")
        .and_then(|v| v.as_array())
    {
        for res in resources {
            let uri = res
                .get("folderUri")
                .or_else(|| res.pointer("/gitFolder/folderUri"))
                .and_then(|v| v.as_str());
            if let Some(u) = uri {
                if let Some(p) = decode_workspace_path(u) {
                    folder_path = Some(p);
                    break;
                }
            }
        }
    }

    (name, folder_path)
}

fn get_running_pids() -> (HashSet<u32>, HashSet<u32>) {
    let mut app_pids = HashSet::new();
    let mut ide_pids = HashSet::new();

    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return (app_pids, ide_pids);
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let name = from_wide_slice(&entry.szExeFile).to_lowercase();
                if name == "antigravity ide.exe" {
                    ide_pids.insert(entry.th32ProcessID);
                } else if name == "antigravity.exe" {
                    app_pids.insert(entry.th32ProcessID);
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }

    (app_pids, ide_pids)
}

fn get_foreground_pid() -> Option<u32> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            None
        } else {
            Some(pid)
        }
    }
}

struct EnumCtx<'a> {
    pids: &'a HashSet<u32>,
    titles: Vec<String>,
}

unsafe extern "system" fn enum_windows_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam as *mut EnumCtx<'_>);
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if !ctx.pids.contains(&pid) {
        return 1;
    }
    let len = GetWindowTextLengthW(hwnd);
    if len > 0 {
        let mut buf = vec![0u16; (len as usize) + 1];
        let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if copied > 0 {
            let text = String::from_utf16_lossy(&buf[..copied as usize])
                .trim()
                .to_string();
            if !text.is_empty() {
                ctx.titles.push(text);
            }
        }
    }
    1
}

fn get_window_titles(pids: &HashSet<u32>) -> Vec<String> {
    if pids.is_empty() {
        return Vec::new();
    }
    let mut ctx = EnumCtx {
        pids,
        titles: Vec::new(),
    };
    unsafe {
        EnumWindows(
            Some(enum_windows_cb),
            (&mut ctx as *mut EnumCtx<'_>) as LPARAM,
        );
    }
    ctx.titles
}

fn split_title_segments(title: &str) -> Vec<&str> {
    let clean = title.trim_start_matches(['●', '•', ' ']).trim();
    if clean.contains(" \u{2014} ") {
        clean
            .split(" \u{2014} ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        clean
            .split(" - ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    }
}

fn find_ide_storage_folder(project_hint: Option<&str>) -> (Option<String>, Option<PathBuf>) {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return (None, None);
    };
    let storage = Path::new(&appdata)
        .join("Antigravity")
        .join("User")
        .join("globalStorage")
        .join("storage.json");
    let Ok(raw) = fs::read_to_string(storage) else {
        return (None, None);
    };
    let Ok(val) = serde_json::from_str::<Value>(&raw) else {
        return (None, None);
    };

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(arr) = val
        .pointer("/windowsState/openedWindows")
        .and_then(|v| v.as_array())
    {
        for win in arr {
            let uri = win
                .get("folder")
                .or_else(|| win.pointer("/workspaceIdentifier/configURIPath"))
                .and_then(|v| v.as_str());
            if let Some(u) = uri {
                if let Some(p) = decode_workspace_path(u) {
                    candidates.push(p);
                }
            }
        }
    }
    if let Some(u) = val
        .pointer("/windowsState/lastActiveWindow/folder")
        .and_then(|v| v.as_str())
    {
        if let Some(p) = decode_workspace_path(u) {
            candidates.push(p);
        }
    }

    if let Some(hint) = project_hint {
        for p in &candidates {
            if folder_name_from_path(p)
                .map(|n| n.eq_ignore_ascii_case(hint))
                .unwrap_or(false)
            {
                return (Some(hint.to_string()), Some(p.clone()));
            }
        }
        return (Some(hint.to_string()), None);
    }

    if let Some(first) = candidates.into_iter().next() {
        let name = folder_name_from_path(&first);
        return (name, Some(first));
    }
    (None, None)
}

fn detect_ide_project(ide_pids: &HashSet<u32>) -> (Option<String>, Option<String>, Option<PathBuf>) {
    let ignored = [
        "welcome",
        "get started",
        "antigravity",
        "settings",
        "keyboard shortcuts",
    ];

    for title in get_window_titles(ide_pids) {
        let mut parts = split_title_segments(&title);
        if parts.is_empty() {
            continue;
        }
        if parts
            .last()
            .map(|s| s.to_lowercase().starts_with("antigravity"))
            .unwrap_or(false)
        {
            parts.pop();
        }
        if parts.is_empty() {
            continue;
        }
        if parts.len() >= 2 {
            let proj_name = parts[parts.len() - 1].to_string();
            let first = parts[0];
            let active_file = if ignored.contains(&first.to_lowercase().as_str()) {
                None
            } else {
                Some(first.to_string())
            };
            let (_, folder) = find_ide_storage_folder(Some(&proj_name));
            return (Some(proj_name), active_file, folder);
        }
        let single = parts[0];
        if !ignored.contains(&single.to_lowercase().as_str()) {
            let (_, folder) = find_ide_storage_folder(Some(single));
            return (Some(single.to_string()), None, folder);
        }
    }

    let (name, folder) = find_ide_storage_folder(None);
    (name, None, folder)
}

fn parse_pbtxt_u64(text: &str, key: &str) -> u64 {
    let Some(idx) = text.find(key) else {
        return 0;
    };
    let rest = &text[idx + key.len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().unwrap_or(0)
}

fn parse_pbtxt_title(text: &str) -> Option<String> {
    let marker = "title:\"";
    let idx = text.find(marker)?;
    let rest = &text[idx + marker.len()..];
    let end = rest.find('"')?;
    let t = rest[..end].trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

struct ViewedConversation {
    id: String,
    title: Option<String>,
}

fn get_most_recently_viewed_conversation() -> Option<ViewedConversation> {
    let home = home_dir()?;
    let ann_dir = home.join(".gemini").join("antigravity").join("annotations");
    let entries = fs::read_dir(ann_dir).ok()?;

    let mut best_score: (u64, u64, u128) = (0, 0, 0);
    let mut best: Option<ViewedConversation> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pbtxt") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let mtime_nanos = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        let Ok(txt) = fs::read_to_string(&path) else {
            continue;
        };
        let secs = parse_pbtxt_u64(&txt, "seconds:");
        let nanos = parse_pbtxt_u64(&txt, "nanos:");
        let score = (secs, nanos, mtime_nanos);

        if best.is_none() || score > best_score {
            best_score = score;
            best = Some(ViewedConversation {
                id: stem.to_string(),
                title: parse_pbtxt_title(&txt),
            });
        }
    }

    best
}

fn read_sidebar_selected_project_id() -> Option<String> {
    let appdata = std::env::var_os("APPDATA")?;
    let path = Path::new(&appdata)
        .join("Antigravity")
        .join("app_storage.json");
    let raw = fs::read_to_string(path).ok()?;
    let val = serde_json::from_str::<Value>(&raw).ok()?;
    val.get("new-convo-last-selected-project")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

struct AppDetection {
    conv_id: Option<String>,
    project_id: Option<String>,
    project: Option<String>,
    detail_line: Option<String>,
    folder_path: Option<PathBuf>,
    is_busy: bool,
}

#[derive(Clone)]
struct CachedTranscript {
    len: u64,
    mtime_nanos: u128,
    timestamps: Vec<i64>,
    interactions: u64,
}

struct CachedProjectStats {
    fingerprint: Vec<(String, u64, u128)>,
    days: u32,
    active_secs: u64,
    interactions: u64,
}

static TRANSCRIPT_CACHE: OnceLock<Mutex<HashMap<String, CachedTranscript>>> = OnceLock::new();
static PROJECT_STATS_CACHE: OnceLock<Mutex<HashMap<String, CachedProjectStats>>> = OnceLock::new();

fn local_utc_offset_secs() -> i64 {
    unsafe {
        let mut tzi: TIME_ZONE_INFORMATION = std::mem::zeroed();
        let mode = GetTimeZoneInformation(&mut tzi);
        let extra = match mode {
            2 => tzi.DaylightBias,
            1 => tzi.StandardBias,
            _ => 0,
        };
        -((tzi.Bias + extra) as i64) * 60
    }
}

fn parse_ascii_digits(bytes: &[u8]) -> Option<i64> {
    let mut val: i64 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        val = val * 10 + (b - b'0') as i64;
    }
    Some(val)
}

fn parse_iso_utc_timestamp(b: &[u8]) -> Option<i64> {
    if b.len() < 19 {
        return None;
    }
    if b[4] != b'-' || b[7] != b'-' || (b[10] != b'T' && b[10] != b' ') || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let y = parse_ascii_digits(&b[0..4])?;
    let m = parse_ascii_digits(&b[5..7])?;
    let d = parse_ascii_digits(&b[8..10])?;
    let hh = parse_ascii_digits(&b[11..13])?;
    let mm = parse_ascii_digits(&b[14..16])?;
    let ss = parse_ascii_digits(&b[17..19])?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 59 {
        return None;
    }

    let y = y - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86_400 + hh * 3_600 + mm * 60 + ss)
}

fn count_subslice(haystack: &[u8], needle: &[u8]) -> u64 {
    let mut count = 0u64;
    let mut i = 0usize;
    while i + needle.len() <= haystack.len() {
        if let Some(rel) = haystack[i..].windows(needle.len()).position(|w| w == needle) {
            count += 1;
            i += rel + needle.len();
        } else {
            break;
        }
    }
    count
}

fn extract_transcript_metrics(path: &Path) -> (Vec<i64>, u64) {
    let Ok(bytes) = fs::read(path) else {
        return (Vec::new(), 0);
    };
    const NEEDLE: &[u8] = b"\"created_at\":\"";
    let mut timestamps = Vec::new();
    let mut i = 0;
    while i + NEEDLE.len() + 19 <= bytes.len() {
        if let Some(rel) = bytes[i..].windows(NEEDLE.len()).position(|w| w == NEEDLE) {
            let start = i + rel + NEEDLE.len();
            if start + 19 <= bytes.len() {
                if let Some(ts) = parse_iso_utc_timestamp(&bytes[start..start + 19]) {
                    timestamps.push(ts);
                }
                i = start + 19;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    let interactions = count_subslice(&bytes, b"\"type\":\"USER_INPUT\"")
        + count_subslice(&bytes, b"\"type\": \"USER_INPUT\"");
    (timestamps, interactions)
}

fn get_cached_transcript_metrics(
    cid: &str,
    path: &Path,
    len: u64,
    mtime_nanos: u128,
) -> (Vec<i64>, u64) {
    let cache = TRANSCRIPT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(entry) = guard.get(cid) {
            if entry.len == len && entry.mtime_nanos == mtime_nanos {
                return (entry.timestamps.clone(), entry.interactions);
            }
        }
    }
    let (timestamps, interactions) = extract_transcript_metrics(path);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(
            cid.to_string(),
            CachedTranscript {
                len,
                mtime_nanos,
                timestamps: timestamps.clone(),
                interactions,
            },
        );
    }
    (timestamps, interactions)
}

fn get_conversation_interactions(conv_id: &str) -> u64 {
    let Some(home) = home_dir() else {
        return 0;
    };
    let tr = home
        .join(".gemini")
        .join("antigravity")
        .join("brain")
        .join(conv_id)
        .join(".system_generated")
        .join("logs")
        .join("transcript.jsonl");
    let Ok(meta) = fs::metadata(&tr) else {
        return 0;
    };
    let len = meta.len();
    let mtime_nanos = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    get_cached_transcript_metrics(conv_id, &tr, len, mtime_nanos).1
}

fn collect_project_metadata_matches(
    project_id: Option<&str>,
    project_name: Option<&str>,
    folder_path: Option<&Path>,
) -> (HashSet<String>, HashSet<String>) {
    let mut pids = HashSet::new();
    let mut folder_norms = HashSet::new();

    if let Some(pid) = project_id {
        let t = pid.trim();
        if !t.is_empty() && t != "outside-of-project" {
            pids.insert(t.to_string());
        }
    }
    if let Some(fp) = folder_path {
        let norm = normalize_path_key(fp);
        if !norm.is_empty() {
            folder_norms.insert(norm);
        }
    }

    let Some(home) = home_dir() else {
        return (pids, folder_norms);
    };
    let projects_dir = home.join(".gemini").join("config").join("projects");
    let Ok(entries) = fs::read_dir(projects_dir) else {
        return (pids, folder_norms);
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(val) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let entry_pid = val
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()));
        let entry_name = val
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let mut entry_folders = Vec::new();
        if let Some(resources) = val
            .pointer("/projectResources/resources")
            .and_then(|v| v.as_array())
        {
            for res in resources {
                if let Some(uri) = res
                    .get("folderUri")
                    .or_else(|| res.pointer("/gitFolder/folderUri"))
                    .and_then(|v| v.as_str())
                {
                    if let Some(p) = decode_workspace_path(uri) {
                        entry_folders.push(normalize_path_key(&p));
                    }
                }
            }
        }

        let pid_matches = entry_pid.as_ref().map(|id| pids.contains(id)).unwrap_or(false);
        let folder_matches = entry_folders.iter().any(|fnorm| folder_norms.contains(fnorm));
        let name_matches = project_name
            .zip(entry_name)
            .map(|(a, b)| a.eq_ignore_ascii_case(b))
            .unwrap_or(false);

        if pid_matches || folder_matches || (folder_norms.is_empty() && name_matches) {
            if let Some(id) = entry_pid {
                pids.insert(id);
            }
            for fnorm in entry_folders {
                if !fnorm.is_empty() {
                    folder_norms.insert(fnorm);
                }
            }
        }
    }

    (pids, folder_norms)
}

fn compute_project_stats(
    project_id: Option<&str>,
    project_name: Option<&str>,
    folder_path: Option<&Path>,
) -> (u32, u64, u64) {
    if project_name.is_none() && folder_path.is_none() && project_id.is_none() {
        return (0, 0, 0);
    }

    let Some(home) = home_dir() else {
        return (0, 0, 0);
    };
    let ag_dir = home.join(".gemini").join("antigravity");
    let db_path = ag_dir.join("conversation_summaries.db");
    if !db_path.exists() {
        return (0, 0, 0);
    }

    let (pids, folder_norms) = collect_project_metadata_matches(project_id, project_name, folder_path);
    if pids.is_empty() && folder_norms.is_empty() && project_name.is_none() {
        return (0, 0, 0);
    }

    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Ok(conn) = Connection::open_with_flags(&db_path, flags) else {
        return (0, 0, 0);
    };
    let _ = conn.busy_timeout(Duration::from_millis(800));

    let Ok(mut stmt) =
        conn.prepare("SELECT conversation_id, project_id, workspace_uris FROM conversation_summaries")
    else {
        return (0, 0, 0);
    };
    let Ok(rows) = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    }) else {
        return (0, 0, 0);
    };

    let mut matched_cids = Vec::new();
    for row in rows.flatten() {
        let (cid, row_pid, ws_uris) = row;
        let pid_hit = !row_pid.is_empty() && pids.contains(row_pid.trim());
        let mut ws_hit = false;
        if !pid_hit {
            let trimmed = ws_uris.trim();
            if !trimmed.is_empty() && trimmed != "[]" {
                let candidates: Vec<String> =
                    if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(trimmed) {
                        arr.into_iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    } else {
                        vec![trimmed.to_string()]
                    };
                for u in candidates {
                    if let Some(p) = decode_workspace_path(&u) {
                        let norm = normalize_path_key(&p);
                        if folder_norms.contains(&norm) {
                            ws_hit = true;
                            break;
                        }
                        if folder_norms.is_empty() {
                            if let (Some(pn), Some(fnm)) = (project_name, folder_name_from_path(&p))
                            {
                                if pn.eq_ignore_ascii_case(&fnm) {
                                    ws_hit = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        if pid_hit || ws_hit {
            matched_cids.push(cid);
        }
    }

    if matched_cids.is_empty() {
        return (0, 0, 0);
    }
    matched_cids.sort();
    matched_cids.dedup();

    let mut files_meta: Vec<(String, PathBuf, u64, u128)> = Vec::new();
    let mut fingerprint: Vec<(String, u64, u128)> = Vec::new();
    for cid in &matched_cids {
        let tr = ag_dir
            .join("brain")
            .join(cid)
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");
        if let Ok(meta) = fs::metadata(&tr) {
            let len = meta.len();
            let mtime_nanos = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            fingerprint.push((cid.clone(), len, mtime_nanos));
            files_meta.push((cid.clone(), tr, len, mtime_nanos));
        }
    }

    if files_meta.is_empty() {
        return (0, 0, 0);
    }

    let cache_key = folder_path
        .map(normalize_path_key)
        .filter(|s| !s.is_empty())
        .or_else(|| project_id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
        .or_else(|| project_name.map(|s| s.to_lowercase()))
        .unwrap_or_else(|| matched_cids.join(","));

    let stats_cache = PROJECT_STATS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = stats_cache.lock() {
        if let Some(cached) = guard.get(&cache_key) {
            if cached.fingerprint == fingerprint {
                return (cached.days, cached.active_secs, cached.interactions);
            }
        }
    }

    let mut all_ts: Vec<i64> = Vec::new();
    let mut total_interactions: u64 = 0;
    for (cid, tr, len, mtime_nanos) in files_meta {
        let (ts, ints) = get_cached_transcript_metrics(&cid, &tr, len, mtime_nanos);
        all_ts.extend(ts);
        total_interactions += ints;
    }
    if all_ts.is_empty() {
        return (0, 0, total_interactions);
    }

    all_ts.sort_unstable();
    all_ts.dedup();

    let offset = local_utc_offset_secs();
    let mut day_buckets: HashSet<i64> = HashSet::new();
    let mut active_secs: u64 = 0;

    for i in 0..all_ts.len() {
        day_buckets.insert((all_ts[i] + offset).div_euclid(86_400));
        if i > 0 {
            let diff = all_ts[i] - all_ts[i - 1];
            if (0..=1_200).contains(&diff) {
                active_secs += diff as u64;
            }
        }
    }

    let days = day_buckets.len() as u32;
    if let Ok(mut guard) = stats_cache.lock() {
        guard.insert(
            cache_key,
            CachedProjectStats {
                fingerprint,
                days,
                active_secs,
                interactions: total_interactions,
            },
        );
    }
    (days, active_secs, total_interactions)
}

fn detect_app_project(fallback_to_last_project: bool) -> AppDetection {
    let viewed = get_most_recently_viewed_conversation();
    let sidebar_proj = read_sidebar_selected_project_id();

    let mut use_sidebar_proj: Option<String> = None;
    if let Ok(mut tracker) = SIDEBAR_TRACKER.lock() {
        let cur_ann = viewed.as_ref().map(|v| v.id.clone());
        let ann_changed = cur_ann != tracker.last_annotation_id && tracker.last_annotation_id.is_some();
        let proj_changed = sidebar_proj != tracker.last_selected_project && tracker.last_selected_project.is_some();

        if proj_changed && !ann_changed {
            tracker.prefer_sidebar_project = true;
        } else if ann_changed {
            tracker.prefer_sidebar_project = false;
        }
        tracker.last_annotation_id = cur_ann;
        tracker.last_selected_project = sidebar_proj.clone();

        if tracker.prefer_sidebar_project {
            use_sidebar_proj = sidebar_proj;
        }
    }

    if let Some(pid) = use_sidebar_proj {
        let (proj_name, folder_path) = read_project_json(&pid);
        if proj_name.is_some() || folder_path.is_some() {
            let final_name = proj_name.or_else(|| folder_path.as_deref().and_then(folder_name_from_path));
            return AppDetection {
                conv_id: None,
                project_id: Some(pid),
                project: final_name,
                detail_line: None,
                folder_path,
                is_busy: false,
            };
        }
    }

    let Some(home) = home_dir() else {
        return AppDetection {
            conv_id: viewed.as_ref().map(|v| v.id.clone()),
            project_id: None,
            project: None,
            detail_line: viewed.and_then(|v| v.title),
            folder_path: None,
            is_busy: false,
        };
    };

    let db_path = home
        .join(".gemini")
        .join("antigravity")
        .join("conversation_summaries.db");
    if !db_path.exists() {
        return AppDetection {
            conv_id: viewed.as_ref().map(|v| v.id.clone()),
            project_id: None,
            project: None,
            detail_line: viewed.and_then(|v| v.title),
            folder_path: None,
            is_busy: false,
        };
    }

    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Ok(conn) = Connection::open_with_flags(&db_path, flags) else {
        return AppDetection {
            conv_id: viewed.as_ref().map(|v| v.id.clone()),
            project_id: None,
            project: None,
            detail_line: viewed.and_then(|v| v.title),
            folder_path: None,
            is_busy: false,
        };
    };
    let _ = conn.busy_timeout(Duration::from_millis(1000));

    type DbRow = (String, String, String, String, i64, String, i64);

    let mut chosen: Option<DbRow> = None;
    if let Some(ref v) = viewed {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT conversation_id, title, workspace_uris, project_id, step_count, status, not_fully_idle
             FROM conversation_summaries
             WHERE conversation_id = ?1
             LIMIT 1",
        ) {
            if let Ok(row) = stmt.query_row([&v.id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            }) {
                chosen = Some(row);
            }
        }
    }

    if chosen.is_none() {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT conversation_id, title, workspace_uris, project_id, step_count, status, not_fully_idle
             FROM conversation_summaries
             ORDER BY last_modified_time DESC
             LIMIT 1",
        ) {
            if let Ok(row) = stmt.query_row([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            }) {
                chosen = Some(row);
            }
        }
    }

    let Some((conv_id, db_title, ws_uris, project_id, _step_count, status, not_fully_idle)) = chosen
    else {
        return AppDetection {
            conv_id: viewed.as_ref().map(|v| v.id.clone()),
            project_id: None,
            project: None,
            detail_line: viewed.and_then(|v| v.title),
            folder_path: None,
            is_busy: false,
        };
    };

    let (proj_json_name, proj_json_path) = read_project_json(&project_id);
    let (ws_name, ws_path) = parse_workspace_uris(&ws_uris);

    let mut folder_path = ws_path.or(proj_json_path);
    let mut project = proj_json_name.or(ws_name);

    if project.is_none() && !conv_id.is_empty() {
        if let Ok(mut fts) = conn.prepare(
            "SELECT project, workspace FROM conversations_fts WHERE conversation_id = ?1 LIMIT 1",
        ) {
            if let Ok((proj_fts, ws_fts)) = fts.query_row([&conv_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }) {
                let p = proj_fts.trim();
                if !p.is_empty() {
                    project = Some(p.to_string());
                } else {
                    let w = percent_decode(ws_fts.trim());
                    if !w.trim().is_empty() {
                        project = Some(w.trim().to_string());
                    }
                }
            }
        }
    }

    let mut resolved_project_id = {
        let p = project_id.trim();
        if p.is_empty() || p == "outside-of-project" {
            None
        } else {
            Some(p.to_string())
        }
    };

    if project.is_none() && fallback_to_last_project {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT workspace_uris, project_id
             FROM conversation_summaries
             WHERE workspace_uris != '' OR (project_id != '' AND project_id != 'outside-of-project')
             ORDER BY last_modified_time DESC
             LIMIT 1",
        ) {
            if let Ok((next_ws, next_pid)) =
                stmt.query_row([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            {
                let (jn, jp) = read_project_json(&next_pid);
                let (wn, wp) = parse_workspace_uris(&next_ws);
                project = jn.or(wn);
                folder_path = wp.or(jp);
                let trimmed_next_pid = next_pid.trim();
                if !trimmed_next_pid.is_empty() && trimmed_next_pid != "outside-of-project" {
                    resolved_project_id = Some(trimmed_next_pid.to_string());
                }
            }
        }
    }

    let clean_db_title = db_title.trim();
    let detail_line = if !clean_db_title.is_empty() {
        Some(clean_db_title.to_string())
    } else {
        viewed.and_then(|v| v.title)
    };

    let is_busy = not_fully_idle != 0 || status.contains("RUNNING");

    AppDetection {
        conv_id: Some(conv_id),
        project_id: resolved_project_id,
        project,
        detail_line,
        folder_path,
        is_busy,
    }
}

pub fn detect_antigravity_state(fallback_to_last_project: bool) -> AntigravityState {
    let (app_pids, ide_pids) = get_running_pids();
    if app_pids.is_empty() && ide_pids.is_empty() {
        return AntigravityState {
            running: false,
            surface: Surface::None,
            project: None,
            detail_line: None,
            active_file: None,
            git_branch: None,
            repo_url: None,
            interactions: 0,
            is_busy: false,
            project_days: 0,
            project_active_secs: 0,
        };
    }

    let fg_pid = get_foreground_pid();
    let prefer_ide = if !ide_pids.is_empty() && app_pids.is_empty() {
        true
    } else if fg_pid.map(|p| ide_pids.contains(&p)).unwrap_or(false) {
        true
    } else if fg_pid.map(|p| app_pids.contains(&p)).unwrap_or(false) {
        false
    } else if !ide_pids.is_empty() {
        detect_ide_project(&ide_pids).0.is_some()
    } else {
        false
    };

    if prefer_ide {
        let (project, active_file, folder_path) = detect_ide_project(&ide_pids);
        let (git_branch, repo_url) = folder_path
            .as_deref()
            .map(read_git_info)
            .unwrap_or((None, None));
        let (project_days, project_active_secs, interactions) =
            compute_project_stats(None, project.as_deref(), folder_path.as_deref());
        return AntigravityState {
            running: true,
            surface: Surface::Ide,
            project,
            detail_line: None,
            active_file,
            git_branch,
            repo_url,
            interactions,
            is_busy: false,
            project_days,
            project_active_secs,
        };
    }

    let app = detect_app_project(fallback_to_last_project);
    let (git_branch, repo_url) = app
        .folder_path
        .as_deref()
        .map(read_git_info)
        .unwrap_or((None, None));
    let (project_days, project_active_secs, proj_interactions) =
        if app.project.is_some() || app.folder_path.is_some() {
            compute_project_stats(
                app.project_id.as_deref(),
                app.project.as_deref(),
                app.folder_path.as_deref(),
            )
        } else {
            (0, 0, 0)
        };
    let interactions = if proj_interactions > 0 {
        proj_interactions
    } else {
        app.conv_id
            .as_deref()
            .map(get_conversation_interactions)
            .unwrap_or(0)
    };

    AntigravityState {
        running: true,
        surface: Surface::App,
        project: app.project,
        detail_line: app.detail_line,
        active_file: None,
        git_branch,
        repo_url,
        interactions,
        is_busy: app.is_busy,
        project_days,
        project_active_secs,
    }
}
