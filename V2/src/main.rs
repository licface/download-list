// download-list — dwn
// Author  : Hadi Cahyadi <cumulus13@gmail.com>
// Homepage: https://github.com/cumulus13/download-list
// License : MIT

#![allow(dead_code)]

use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use unicode_width::UnicodeWidthStr;

// ── Colour palette (24-bit hex ANSI) ─────────────────────────────────────────
mod color {
    #[inline]
    pub fn fg(r: u8, g: u8, b: u8, text: &str) -> String {
        format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
    }
    pub fn cyan(t: &str) -> String {
        fg(0x4D, 0xD0, 0xE1, t)
    }
    pub fn green(t: &str) -> String {
        fg(0x66, 0xBB, 0x6A, t)
    }
    pub fn yellow(t: &str) -> String {
        fg(0xFF, 0xD5, 0x4F, t)
    }
    pub fn red(t: &str) -> String {
        fg(0xEF, 0x53, 0x50, t)
    }
    pub fn blue(t: &str) -> String {
        fg(0x42, 0xA5, 0xF5, t)
    }
    pub fn magenta(t: &str) -> String {
        fg(0xAB, 0x47, 0xBC, t)
    }
    pub fn gray(t: &str) -> String {
        fg(0x78, 0x90, 0x9C, t)
    }
    pub fn white(t: &str) -> String {
        fg(0xEC, 0xEF, 0xF1, t)
    }
    pub fn orange(t: &str) -> String {
        fg(0xFF, 0xA7, 0x26, t)
    }
    pub fn teal(t: &str) -> String {
        fg(0x26, 0xA6, 0x9A, t)
    }
}

// ── Emoji ─────────────────────────────────────────────────────────────────────
const E_FILE: &str = "📄";
const E_ARCHIVE: &str = "📦";
const E_SUCCESS: &str = "✅";
const E_ERROR: &str = "❌";
const E_INFO: &str = "ℹ️ ";
const E_WARNING: &str = "⚠️ ";
const E_FOLDER: &str = "📁";
const E_DOWNLOAD: &str = "📥";
const E_EXTRACT: &str = "🔧";
const E_SEARCH: &str = "🔍";
const E_SKIP: &str = "⏭️ ";
const E_NEW: &str = "🆕";
const E_EXISTING: &str = "📌";
const E_IMAGE: &str = "🖼️ ";
const E_VIDEO: &str = "🎬";
const E_AUDIO: &str = "🎵";
const E_TEXT: &str = "📝";
const E_DOC: &str = "📋";
const E_MOVE: &str = "📦";

// ── CLI ───────────────────────────────────────────────────────────────────────
#[derive(Parser)]
#[command(
    name  = "dwn",
    about = "📥 download-list — manage & extract files in your Downloads folder",
    long_about = concat!(
        "📥  download-list  v", env!("CARGO_PKG_VERSION"), "\n",
        "    Manage & extract files in your Downloads folder\n\n",
        "    Author  : Hadi Cahyadi <cumulus13@gmail.com>\n",
        "    Homepage: https://github.com/cumulus13/download-list"
    ),
    after_help = "EXAMPLES:\n  \
      dwn list                              List all downloaded files\n  \
      dwn list -p \"*.pdf\"                   List only PDF files\n  \
      dwn list -p \"*\" -F \"!temp*\"          List all except temp files\n  \
      dwn list -p \"*\" -F \"/regex:.*202[0-9].*/\"  List files with years 2020-2029\n  \
      dwn list -p \"*.{jpg,png,gif}\"         List images using alternation\n  \
      dwn list --last                       Show only the newest file\n  \
      dwn list -z                           Inspect contents of the newest archive\n  \
      dwn list -z -f foo.7z                 Inspect a specific archive\n  \
      dwn list -v                           Verbose output\n  \
      dwn list --asc                        Sort oldest first\n  \
      \n  \
      dwn extract                           Extract the newest archive\n  \
      dwn extract -f foo.zip                Extract a specific archive\n  \
      dwn extract -F \"*.txt\"               Extract only .txt files from archive\n  \
      dwn extract -F \"!*.exe\"              Extract everything except .exe files\n  \
      dwn extract -F \"/regex:.*readme.*/i\"  Extract files containing 'readme' (case-insensitive)\n  \
      dwn extract --overwrite               Overwrite all existing files\n  \
      dwn extract --verbose                 Show per-file details\n  \
      \n  \
      dwn move -d ~/Documents               Move all files to Documents\n  \
      dwn move -p \"*.pdf\" -d ~/PDFs        Move all PDFs to PDFs folder\n  \
      dwn move -p \"*\" -F \"!temp*\" -d ~/archive  Move all except temp files\n  \
      dwn move -p \"*.{jpg,png}\" -d ~/Pictures    Move images using alternation\n  \
      dwn move -p \"*\" -F \"/regex:.*\\.(pdf|doc)$/\" -d ~/docs  Regex filter\n  \
      dwn move --last -d ~/Desktop           Move newest file to Desktop\n  \
      dwn move --oldest -d ~/old             Move oldest file\n  \
      dwn move -f -d ~/archive --force       Move and overwrite existing\n  \
      dwn move --dry-run -d ~/backup         Preview what would be moved\n  \
      \n  \
      dwn path                              Print the Downloads directory path\n  \
      \n  \
      PATTERN SYNTAX:\n  \
      Wildcards:   * (any), ? (single char)\n  \
      Char class:  [abc], [0-9], [!xyz]\n  \
      Alternation: {jpg,png,gif}\n  \
      Regex:       /regex:pattern/ or /r:pattern/\n  \
      Negation:    !pattern (as filter)",
    version,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 📋  List files in the Downloads directory
    #[command(alias = "l")]
    List {
        #[arg(short, long)]
        last: bool,
        #[arg(short, long, default_value = "*")]
        pattern: String,
        /// Filter files by name pattern (supports wildcards, glob patterns, and regex)
        #[arg(short = 'F', long)]
        filter: Option<String>,
        /// Inspect the contents of an archive (like `7z l`)
        #[arg(short = 'z', long = "list-archive")]
        list_archive: bool,
        /// Archive to inspect (used with -z)
        #[arg(short, long)]
        file: Option<PathBuf>,
        #[arg(short, long)]
        verbose: bool,
        /// Sort oldest-first (ascending by modification time)
        #[arg(long, conflicts_with = "desc")]
        asc: bool,
        /// Sort newest-first (descending) — this is the default
        #[arg(long, conflicts_with = "asc")]
        desc: bool,
    },
    /// 🔧  Extract an archive from the Downloads directory
    #[command(alias = "e")]
    Extract {
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Filter files within archive to extract (supports wildcards, glob patterns, and regex)
        #[arg(short = 'F', long)]
        filter: Option<String>,
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Overwrite all existing files without asking
        #[arg(short = 'w', long)]
        overwrite: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    /// 📦  Move files from Downloads to another directory
    #[command(alias = "m")]
    Move {
        /// Source file(s) to move (supports wildcards, glob patterns, and regex)
        #[arg(short, long, default_value = "*")]
        pattern: String,
        /// Filter files by name pattern (supports wildcards, glob patterns, and regex)
        #[arg(short = 'F', long)]
        filter: Option<String>,
        /// Destination directory
        #[arg(short, long)]
        dest: PathBuf,
        /// Move only the newest file matching the pattern
        #[arg(short, long)]
        last: bool,
        /// Move only the oldest file matching the pattern
        #[arg(short = 'O', long)]
        oldest: bool,
        /// Force overwrite existing files at destination
        #[arg(short, long)]
        force: bool,
        /// Dry run - show what would be moved without actually moving
        #[arg(short = 'n', long = "dry-run")]
        dry_run: bool,
        #[arg(short, long)]
        verbose: bool,
        /// Sort oldest-first (ascending by modification time)
        #[arg(long, conflicts_with = "desc")]
        asc: bool,
        /// Sort newest-first (descending) — this is the default
        #[arg(long, conflicts_with = "asc")]
        desc: bool,
    },
    /// 📁  Show the Downloads directory path
    #[command(alias = "p")]
    Path,
}

// ── Archive extensions ────────────────────────────────────────────────────────
const ARCHIVE_EXTS: &[&str] = &[
    ".zip", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz", ".tar", ".7z", ".rar",
];

fn is_archive(p: &Path) -> bool {
    let n = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    ARCHIVE_EXTS.iter().any(|e| n.ends_with(e))
}

// ── Downloads dir ─────────────────────────────────────────────────────────────
fn downloads_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    if let Ok(xdg) = std::env::var("XDG_DOWNLOAD_DIR") {
        let p = PathBuf::from(&xdg);
        if p.is_absolute() {
            let _ = fs::create_dir_all(&p);
            return p;
        }
    }
    let home = home_dir();
    let dl = home.join("Downloads");
    let _ = fs::create_dir_all(&dl);
    dl
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

// ── Pattern matching system ───────────────────────────────────────────────────
#[derive(Debug, Clone)]
enum PatternType {
    Glob(String),
    Regex(Regex),
    Negate(Box<PatternType>),
    Alternation(Vec<PatternType>),
    CharClass {
        chars: Vec<char>,
        ranges: Vec<(char, char)>,
        negated: bool,
    },
}

impl PatternType {
    fn from_str(pattern: &str) -> Result<Self, String> {
        let pattern = pattern.trim();

        if pattern.is_empty() {
            return Ok(PatternType::Glob("*".to_string()));
        }

        // Check for negation
        if pattern.starts_with('!') {
            let inner = PatternType::from_str(&pattern[1..])?;
            return Ok(PatternType::Negate(Box::new(inner)));
        }

        // Check for explicit regex pattern
        if pattern.starts_with("/regex:") || pattern.starts_with("/r:") {
            let regex_str = if pattern.starts_with("/regex:") {
                &pattern[7..]
            } else {
                &pattern[3..]
            };

            // Remove trailing / if present
            let regex_str = regex_str.strip_suffix('/').unwrap_or(regex_str);

            Regex::new(regex_str)
                .map(PatternType::Regex)
                .map_err(|e| format!("Invalid regex pattern '{}': {}", regex_str, e))
        } else if pattern.contains('{') && pattern.contains('}') {
            Self::parse_alternation(pattern)
        } else if pattern.contains('[') && pattern.contains(']') {
            Self::parse_char_class(pattern)
        } else {
            Ok(PatternType::Glob(pattern.to_string()))
        }
    }

    fn parse_alternation(pattern: &str) -> Result<Self, String> {
        if let Some(start) = pattern.find('{') {
            if let Some(end) = pattern[start..].find('}') {
                let end = start + end;
                let prefix = &pattern[..start];
                let suffix = &pattern[end + 1..];
                let alternatives = &pattern[start + 1..end];

                let mut patterns = Vec::new();
                for alt in alternatives.split(',') {
                    let alt = alt.trim();
                    if alt.is_empty() {
                        continue;
                    }
                    let full_pattern = format!("{}{}{}", prefix, alt, suffix);
                    patterns.push(PatternType::from_str(&full_pattern)?);
                }

                if patterns.is_empty() {
                    return Ok(PatternType::Glob(pattern.to_string()));
                }

                return Ok(PatternType::Alternation(patterns));
            }
        }
        Ok(PatternType::Glob(pattern.to_string()))
    }

    fn parse_char_class(pattern: &str) -> Result<Self, String> {
        let mut chars = Vec::new();
        let mut ranges = Vec::new();
        let mut negated = false;
        let mut in_class = false;
        let mut class_content = String::new();
        let mut start_idx = 0;

        for (i, ch) in pattern.char_indices() {
            if ch == '[' && !in_class {
                in_class = true;
                start_idx = i;
                // Check for negation
                if pattern[i + 1..].starts_with('!') || pattern[i + 1..].starts_with('^') {
                    negated = true;
                    class_content.push(pattern.chars().nth(i + 1).unwrap_or('!'));
                }
            } else if ch == ']' && in_class {
                let content = if negated {
                    &class_content[1..]
                } else {
                    &class_content
                };
                let bytes: Vec<char> = content.chars().collect();
                let mut j = 0;

                while j < bytes.len() {
                    if j + 2 < bytes.len() && bytes[j + 1] == '-' {
                        ranges.push((bytes[j], bytes[j + 2]));
                        j += 3;
                    } else {
                        chars.push(bytes[j]);
                        j += 1;
                    }
                }

                return Ok(PatternType::CharClass {
                    chars,
                    ranges,
                    negated,
                });
            } else if in_class {
                class_content.push(ch);
            }
        }

        Ok(PatternType::Glob(pattern.to_string()))
    }

    fn matches(&self, text: &str) -> bool {
        match self {
            PatternType::Glob(pattern) => glob_match(pattern, text),
            PatternType::Regex(re) => re.is_match(text),
            PatternType::Negate(inner) => !inner.matches(text),
            PatternType::Alternation(patterns) => patterns.iter().any(|p| p.matches(text)),
            PatternType::CharClass {
                chars,
                ranges,
                negated,
            } => {
                if text.is_empty() {
                    return *negated;
                }

                let ch = text.chars().next().unwrap();
                let in_class = chars.contains(&ch)
                    || ranges.iter().any(|(start, end)| ch >= *start && ch <= *end);

                if *negated {
                    !in_class
                } else {
                    in_class
                }
            }
        }
    }
}

// ── File listing ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    modified: u64,
    size: u64,
}

impl FileEntry {
    fn is_archive(&self) -> bool {
        is_archive(&self.path)
    }
}

fn list_downloads(pattern: &str) -> Vec<FileEntry> {
    let dir = downloads_dir();
    let mut out: Vec<FileEntry> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if pattern != "*" && !glob_match(pattern, name) {
                continue;
            }
            if let Ok(meta) = p.metadata() {
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                out.push(FileEntry {
                    path: p,
                    modified,
                    size: meta.len(),
                });
            }
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

fn list_downloads_filtered(pattern: &str, filter: &Option<String>) -> Vec<FileEntry> {
    let mut files = list_downloads("*");

    let pattern_type = match PatternType::from_str(pattern) {
        Ok(pt) => pt,
        Err(e) => {
            eprintln!(
                "{E_WARNING} {}",
                color::yellow(&format!("Invalid pattern '{}': {}", pattern, e))
            );
            return Vec::new();
        }
    };

    let filter_type = filter
        .as_ref()
        .and_then(|f| match PatternType::from_str(f) {
            Ok(pt) => Some(pt),
            Err(e) => {
                eprintln!(
                    "{E_WARNING} {}",
                    color::yellow(&format!("Invalid filter '{}': {}", f, e))
                );
                None
            }
        });

    files.retain(|file| {
        let name = file.path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let pattern_match = pattern_type.matches(name);
        let filter_match = if let Some(ref ft) = filter_type {
            ft.matches(name)
        } else {
            true
        };

        pattern_match && filter_match
    });

    files
}

fn filter_archive_entries(
    entries: &mut Vec<ArchiveEntry>,
    filter: &Option<String>,
) -> Result<(), String> {
    if let Some(filter_str) = filter {
        let filter_type = PatternType::from_str(filter_str)
            .map_err(|e| format!("Invalid filter '{}': {}", filter_str, e))?;

        let before_count = entries.len();
        entries.retain(|entry| filter_type.matches(&entry.name));
        let after_count = entries.len();

        if after_count == 0 {
            return Err(format!(
                "No files matching filter '{}' in archive (had {} total files)",
                filter_str, before_count
            ));
        }
    }
    Ok(())
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let (pb, tb) = (pattern.as_bytes(), text.as_bytes());
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut spi, mut sti) = (usize::MAX, 0usize);
    while ti < tb.len() {
        if pi < pb.len() && (pb[pi] == b'?' || pb[pi].eq_ignore_ascii_case(&tb[ti])) {
            pi += 1;
            ti += 1;
        } else if pi < pb.len() && pb[pi] == b'*' {
            spi = pi;
            sti = ti;
            pi += 1;
        } else if spi != usize::MAX {
            pi = spi + 1;
            sti += 1;
            ti = sti;
        } else {
            return false;
        }
    }
    while pi < pb.len() && pb[pi] == b'*' {
        pi += 1;
    }
    pi == pb.len()
}

// ── Formatting helpers ────────────────────────────────────────────────────────
fn terminal_width() -> usize {
    use terminal_size::{terminal_size, Width};
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(100)
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut n = bytes as f64;
    for unit in &["B", "KB", "MB", "GB", "TB"] {
        if n < 1024.0 {
            return format!("{n:.1} {unit}");
        }
        n /= 1024.0;
    }
    format!("{n:.1} PB")
}

fn format_ts(ts: u64) -> String {
    let secs = ts % 60;
    let mins = (ts / 60) % 60;
    let hours = (ts / 3600) % 24;
    let (y, mo, d) = days_to_ymd(ts / 86400);
    format!("{y:04}-{mo:02}-{d:02} {hours:02}:{mins:02}:{secs:02}")
}

fn days_to_ymd(days: u64) -> (u64, u8, u8) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u8, d as u8)
}

fn format_age(secs: u64) -> String {
    if secs < 60 {
        return format!("{secs}s");
    }
    if secs < 3600 {
        return format!("{}m", secs / 60);
    }
    if secs < 86400 {
        return format!("{}h", secs / 3600);
    }
    if secs < 86400 * 30 {
        return format!("{}d", secs / 86400);
    }
    if secs < 86400 * 365 {
        return format!("{}mo", secs / (86400 * 30));
    }
    format!("{}y", secs / (86400 * 365))
}

fn age_str(modified: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_age(now.saturating_sub(modified))
}

fn file_emoji(p: &Path) -> &'static str {
    if is_archive(p) {
        return E_ARCHIVE;
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" => E_IMAGE,
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => E_VIDEO,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => E_AUDIO,
        "txt" | "md" | "py" | "js" | "ts" | "rs" | "c" | "cpp" | "h" | "go" | "rb" | "sh"
        | "yaml" | "toml" | "json" => E_TEXT,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => E_DOC,
        _ => E_FILE,
    }
}

fn type_label(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if ARCHIVE_EXTS.iter().any(|e| lower.ends_with(e)) {
        return "Archive";
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" => "Image",
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "Video",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "Audio",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "Document",
        "txt" | "md" | "py" | "js" | "ts" | "rs" | "json" | "xml" | "yaml" | "toml" | "c"
        | "cpp" | "h" | "go" => "Text",
        _ => "File",
    }
}

fn type_emoji(t: &str) -> &'static str {
    match t {
        "Archive" => "📦",
        "Image" => E_IMAGE,
        "Video" => E_VIDEO,
        "Audio" => E_AUDIO,
        "Document" => E_DOC,
        "Text" => E_TEXT,
        _ => E_FILE,
    }
}

// ── Table renderer ────────────────────────────────────────────────────────────
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut esc = false;
    for c in s.chars() {
        if esc {
            if c == 'm' {
                esc = false;
            }
        } else if c == '\x1b' {
            esc = true;
        } else {
            out.push(c);
        }
    }
    out
}

fn vis(s: &str) -> usize {
    UnicodeWidthStr::width(strip_ansi(s).as_str())
}

/// Truncate a string's visible content to max_w chars, appending '…'.
fn trunc_vis(s: &str, max_w: usize) -> String {
    let plain = strip_ansi(s);
    if UnicodeWidthStr::width(plain.as_str()) <= max_w {
        return plain;
    }
    if max_w == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in plain.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if used + w + 1 > max_w {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

/// Re-apply the first ANSI colour escape in `original` to `text`.
fn reapply_color(original: &str, text: &str) -> String {
    if original.starts_with('\x1b') {
        if let Some(end) = original.find('m') {
            let esc = &original[..=end];
            return format!("{esc}{text}\x1b[0m");
        }
    }
    text.to_string()
}

fn print_table(title: Option<&str>, headers: &[&str], rows: &[Vec<String>]) {
    let n = headers.len();
    let tw = terminal_width();

    // Natural widths
    let mut nat: Vec<usize> = headers.iter().map(|h| vis(h)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < n {
                nat[i] = nat[i].max(vis(cell));
            }
        }
    }

    // Overhead: leading "│" + per-column " content │" = n*3 + 1
    let overhead = n * 3 + 1;
    let budget = tw.saturating_sub(overhead);

    // Fixed columns: those whose natural width fits within FIXED_MAX
    const FIXED_MAX: usize = 22;
    let fixed_total: usize = nat
        .iter()
        .map(|&w| if w <= FIXED_MAX { w } else { 0 })
        .sum();
    let flex_count = nat.iter().filter(|&&w| w > FIXED_MAX).count();
    let flex_budget = if flex_count > 0 {
        budget.saturating_sub(fixed_total) / flex_count
    } else {
        0
    };

    let mut widths: Vec<usize> = (0..n)
        .map(|i| {
            let natural = nat[i];
            let header_min = vis(headers[i]);
            if natural > FIXED_MAX {
                flex_budget.max(header_min).min(natural)
            } else {
                natural.max(header_min)
            }
        })
        .collect();

    // Make sure flex cols are at least as wide as their header
    for i in 0..n {
        widths[i] = widths[i].max(vis(headers[i]));
    }

    let total: usize = widths.iter().sum::<usize>() + overhead;
    let sep_len = total.min(tw);
    let sep = color::gray(&"─".repeat(sep_len));

    if let Some(t) = title {
        let lines: Vec<&str> = t.split('\n').collect();
        for line in lines {
            let tv = vis(line);
            let pad = if sep_len > tv { (sep_len - tv) / 2 } else { 0 };
            println!("{}{}", " ".repeat(pad), color::cyan(line));
        }
    }
    println!("{sep}");

    // Header
    print!("{}", color::gray("│"));
    for (i, h) in headers.iter().enumerate() {
        let pad = widths[i].saturating_sub(vis(h));
        print!(
            " {}{} {}",
            color::white(h),
            " ".repeat(pad),
            color::gray("│")
        );
    }
    println!();
    println!("{sep}");

    // Data rows
    for row in rows {
        print!("{}", color::gray("│"));
        for (i, cell) in row.iter().enumerate() {
            if i >= n {
                break;
            }
            let cv = vis(cell);
            let (display, actual_w) = if cv > widths[i] {
                let t = trunc_vis(cell, widths[i]);
                let aw = vis(&t);
                (reapply_color(cell, &t), aw)
            } else {
                let plain = strip_ansi(cell);
                (reapply_color(cell, &plain), cv)
            };
            let pad = widths[i].saturating_sub(actual_w);
            print!(" {}{} {}", display, " ".repeat(pad), color::gray("│"));
        }
        println!();
    }
    println!("{sep}");
}

// ── Commands ──────────────────────────────────────────────────────────────────
fn cmd_path() {
    println!("{E_FOLDER} {}", color::cyan("Downloads directory:"));
    println!("  {}", color::green(&downloads_dir().display().to_string()));
}

fn cmd_list(last: bool, pattern: &str, filter: &Option<String>, asc: bool) {
    let mut files = list_downloads_filtered(pattern, filter);

    if files.is_empty() {
        let filter_msg = filter
            .as_ref()
            .map(|f| format!(" with filter '{}'", f))
            .unwrap_or_default();
        println!(
            "{E_INFO} {}",
            color::yellow(&format!(
                "No files matching pattern '{}'{} found",
                pattern, filter_msg
            ))
        );
        return;
    }

    // Sort based on asc/desc
    if asc {
        files.sort_by(|a, b| a.modified.cmp(&b.modified));
    } else {
        files.sort_by(|a, b| b.modified.cmp(&a.modified));
    }

    if last {
        files.truncate(1);
    }

    let sort_indicator = if asc {
        format!("  {}", color::teal("↑ oldest first"))
    } else {
        format!("  {}", color::magenta("↓ newest first"))
    };

    let dl = downloads_dir();
    let mut title = format!(
        "{E_DOWNLOAD} Downloads  —  {}{sort_indicator}",
        color::gray(&dl.display().to_string())
    );

    // Add pattern info to title if not default
    if pattern != "*" {
        title.push_str(&format!(
            "\n  {} {}",
            color::cyan("Pattern:"),
            color::blue(pattern)
        ));
    }
    if let Some(ref f) = filter {
        title.push_str(&format!(
            "\n  {} {}",
            color::cyan("Filter:"),
            color::magenta(f)
        ));
    }

    let headers = ["#", " ", "Filename", "Size", "Modified", "Age"];
    let mut rows: Vec<Vec<String>> = Vec::new();

    for (i, f) in files.iter().enumerate() {
        let name = f.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let emoji = file_emoji(&f.path);
        let name_c = if f.is_archive() {
            color::yellow(name)
        } else {
            color::green(name)
        };
        rows.push(vec![
            color::gray(&(i + 1).to_string()),
            emoji.to_string(),
            name_c,
            color::orange(&format_size(f.size)),
            color::blue(&format_ts(f.modified)),
            color::magenta(&age_str(f.modified)),
        ]);
    }

    print_table(Some(&title), &headers, &rows);
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    println!(
        "  {} files  •  total {}",
        color::cyan(&files.len().to_string()),
        color::orange(&format_size(total_size))
    );
}

fn cmd_list_archive(file: Option<PathBuf>, verbose: bool) {
    let path = match resolve_archive(file, verbose) {
        Some(p) => p,
        None => process::exit(1),
    };
    vlog(
        verbose,
        "INFO",
        &format!("{E_SEARCH} Inspecting: {}", path.display()),
    );

    let entries = match read_archive_entries(&path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "{E_ERROR} {}",
                color::red(&format!("Cannot read archive: {e}"))
            );
            process::exit(1);
        }
    };

    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    println!();
    println!(
        "{E_ARCHIVE}  {}",
        color::cyan(&path.file_name().unwrap_or_default().to_string_lossy())
    );
    println!(
        "  {E_INFO} {} files   {}",
        color::blue(&entries.len().to_string()),
        color::orange(&format_size(total_size))
    );

    let title = format!(
        "Contents of  {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    let headers = ["#", "T", "Filename", "Size", "Compressed"];
    let mut rows: Vec<Vec<String>> = Vec::new();

    for (i, e) in entries.iter().enumerate() {
        let lbl = type_label(&e.name);
        let display = if e.name.len() > 64 {
            format!("…{}", &e.name[e.name.len() - 63..])
        } else {
            e.name.clone()
        };
        rows.push(vec![
            color::gray(&(i + 1).to_string()),
            type_emoji(lbl).to_string(),
            color::green(&display),
            color::orange(&if e.size > 0 {
                format_size(e.size)
            } else {
                "—".into()
            }),
            color::teal(&if e.compressed > 0 {
                format_size(e.compressed)
            } else {
                "—".into()
            }),
        ]);
    }
    print_table(Some(&title), &headers, &rows);
}

// ── Extract mode ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    OverwriteAll,
    AllNew,
    OnlyNew,
    AllExist,
}

fn cmd_extract(
    file: Option<PathBuf>,
    filter: &Option<String>,
    output: PathBuf,
    overwrite: bool,
    verbose: bool,
) {
    let path = match resolve_archive(file, verbose) {
        Some(p) => p,
        None => process::exit(1),
    };

    let file_size = match fs::metadata(&path) {
        Ok(m) => m.len(),
        Err(e) => {
            eprintln!("{E_ERROR} {}", color::red(&e.to_string()));
            process::exit(1);
        }
    };
    if file_size == 0 {
        eprintln!("{E_ERROR} {}", color::red("File is empty"));
        process::exit(1);
    }
    vlog(
        verbose,
        "INFO",
        &format!("File size: {}", format_size(file_size)),
    );

    let _ = fs::create_dir_all(&output);
    let test = output.join(".dwn_write_test");
    match fs::File::create(&test) {
        Ok(_) => {
            let _ = fs::remove_file(&test);
        }
        Err(_) => {
            eprintln!(
                "{E_ERROR} {}",
                color::red(&format!("No write permission: {}", output.display()))
            );
            process::exit(1);
        }
    }

    vlog(verbose, "INFO", "Analysing archive contents…");
    let mut entries = match read_archive_entries(&path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{E_ERROR} {}", color::red(&e));
            process::exit(1);
        }
    };
    if entries.is_empty() {
        eprintln!("{E_WARNING} {}", color::yellow("Archive is empty"));
        process::exit(1);
    }

    // Apply filter if provided
    if let Err(e) = filter_archive_entries(&mut entries, filter) {
        eprintln!("{E_WARNING} {}", color::yellow(&e));
        process::exit(1);
    }

    vlog(
        verbose,
        "INFO",
        &format!(
            "{} files  ({})",
            entries.len(),
            format_size(entries.iter().map(|e| e.size).sum::<u64>())
        ),
    );

    let (existing, new_files): (Vec<_>, Vec<_>) =
        entries.iter().partition(|e| output.join(&e.name).exists());

    if verbose {
        if !existing.is_empty() {
            vlog(verbose, "INFO", &format!("{} existing:", existing.len()));
            for e in existing.iter().take(5) {
                vlog(verbose, "INFO", &format!("  {E_EXISTING} {}", e.name));
            }
            if existing.len() > 5 {
                vlog(
                    verbose,
                    "INFO",
                    &format!("  … and {} more", existing.len() - 5),
                );
            }
        }
        if !new_files.is_empty() {
            vlog(verbose, "INFO", &format!("{} new:", new_files.len()));
            for e in new_files.iter().take(5) {
                vlog(verbose, "INFO", &format!("  {E_NEW} {}", e.name));
            }
            if new_files.len() > 5 {
                vlog(
                    verbose,
                    "INFO",
                    &format!("  … and {} more", new_files.len() - 5),
                );
            }
        }
    }

    let mode = if overwrite {
        Mode::OverwriteAll
    } else if existing.is_empty() {
        Mode::AllNew
    } else if !new_files.is_empty() {
        Mode::OnlyNew
    } else {
        Mode::AllExist
    };

    if mode == Mode::AllExist {
        let msg = format!(
            "All {} files already exist. Use --overwrite to re-extract.",
            entries.len()
        );
        println!("{E_INFO} {}", color::yellow(&msg));
        if is_tty() && !overwrite {
            print!("{E_WARNING} Overwrite all? [y/N] ");
            io::stdout().flush().ok();
            let mut r = String::new();
            io::stdin().read_line(&mut r).ok();
            if matches!(r.trim().to_lowercase().as_str(), "y" | "yes") {
                do_extract(
                    &path,
                    &output,
                    Mode::OverwriteAll,
                    &new_files.iter().map(|e| e.name.clone()).collect::<Vec<_>>(),
                    existing.len(),
                    verbose,
                );
            }
        }
        return;
    }

    let new_names: Vec<String> = new_files.iter().map(|e| e.name.clone()).collect();
    do_extract(&path, &output, mode, &new_names, existing.len(), verbose);
}

fn do_extract(
    path: &Path,
    output: &Path,
    mode: Mode,
    new_files: &[String],
    existing_count: usize,
    verbose: bool,
) {
    let fname = path.file_name().unwrap_or_default().to_string_lossy();
    let verb = match mode {
        Mode::OverwriteAll => "Overwriting",
        Mode::AllNew => "Extracting",
        Mode::OnlyNew => "Extracting new files from",
        Mode::AllExist => return,
    };
    println!(
        "{E_EXTRACT} {} {}",
        color::cyan(verb),
        color::yellow(&fname)
    );

    match extract_archive(path, output, mode, new_files, verbose) {
        Ok(count) => {
            let msg = match mode {
                Mode::OverwriteAll => format!("Overwrote {count} files from {fname}"),
                Mode::AllNew => format!("Extracted {count} files from {fname}"),
                Mode::OnlyNew => format!(
                    "Extracted {count} new files (skipped {existing_count} existing) from {fname}"
                ),
                Mode::AllExist => return,
            };
            println!("{E_SUCCESS} {}", color::green(&msg));
            println!(
                "  {E_FOLDER} {}",
                color::gray(&output.display().to_string())
            );
        }
        Err(e) => {
            eprintln!(
                "{E_ERROR} {}",
                color::red(&format!("Extraction failed: {e}"))
            );
            process::exit(1);
        }
    }
}

// ── Move command ──────────────────────────────────────────────────────────────
fn cmd_move(
    pattern: &str,
    filter: &Option<String>,
    dest: &Path,
    last: bool,
    oldest: bool,
    force: bool,
    dry_run: bool,
    verbose: bool,
    asc: bool,
) {
    // Validate destination
    if !dest.exists() {
        eprintln!(
            "{E_ERROR} {}",
            color::red(&format!("Destination does not exist: {}", dest.display()))
        );
        process::exit(1);
    }
    if !dest.is_dir() {
        eprintln!(
            "{E_ERROR} {}",
            color::red(&format!(
                "Destination is not a directory: {}",
                dest.display()
            ))
        );
        process::exit(1);
    }

    // Test write permission at destination
    let test = dest.join(".dwn_write_test");
    match fs::File::create(&test) {
        Ok(_) => {
            let _ = fs::remove_file(&test);
        }
        Err(_) => {
            eprintln!(
                "{E_ERROR} {}",
                color::red(&format!(
                    "No write permission at destination: {}",
                    dest.display()
                ))
            );
            process::exit(1);
        }
    }

    // Get files matching pattern and filter
    let mut files = list_downloads_filtered(pattern, filter);

    if files.is_empty() {
        let filter_msg = filter
            .as_ref()
            .map(|f| format!(" with filter '{}'", f))
            .unwrap_or_default();
        println!(
            "{E_INFO} {}",
            color::yellow(&format!(
                "No files matching pattern '{}'{} found",
                pattern, filter_msg
            ))
        );
        return;
    }

    // Sort based on asc/desc
    if asc {
        files.sort_by(|a, b| a.modified.cmp(&b.modified));
    } else {
        files.sort_by(|a, b| b.modified.cmp(&a.modified));
    }

    // Apply last/oldest selection
    if last {
        files = vec![files[0].clone()];
    } else if oldest {
        files = vec![files[files.len() - 1].clone()];
    }

    // Display header
    let dl = downloads_dir();
    println!();
    let mut title = format!(
        "{E_MOVE} Moving files from {}",
        color::gray(&dl.display().to_string())
    );
    title.push_str(&format!(
        "\n  {} {}",
        color::cyan("To:"),
        color::green(&dest.display().to_string())
    ));

    if pattern != "*" {
        title.push_str(&format!(
            "\n  {} {}",
            color::cyan("Pattern:"),
            color::blue(pattern)
        ));
    }
    if let Some(ref f) = filter {
        title.push_str(&format!(
            "\n  {} {}",
            color::cyan("Filter:"),
            color::magenta(f)
        ));
    }

    let sort_indicator = if asc {
        format!("  {}", color::teal("↑ oldest first"))
    } else {
        format!("  {}", color::magenta("↓ newest first"))
    };
    title.push_str(&format!("\n{}", sort_indicator));

    // Check for conflicts
    let mut to_move: Vec<(&FileEntry, bool)> = Vec::new();
    let mut conflicts: Vec<&FileEntry> = Vec::new();

    for file in &files {
        let dest_path = dest.join(file.path.file_name().unwrap_or_default());
        let will_overwrite = dest_path.exists();

        if will_overwrite && !force {
            conflicts.push(file);
        } else {
            to_move.push((file, will_overwrite));
        }
    }

    // Show warnings for conflicts
    if !conflicts.is_empty() {
        for file in &conflicts {
            let name = file
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");
            eprintln!(
                "{E_WARNING} {} {}",
                color::yellow(&format!("File exists at destination: {}", name)),
                color::gray("(use --force to overwrite)")
            );
        }
        println!();
    }

    if to_move.is_empty() {
        println!(
            "{E_INFO} {}",
            color::yellow("No files to move (all would cause conflicts)")
        );
        println!("{}", color::gray("Use --force to overwrite existing files"));
        return;
    }

    // Show summary table
    let headers = ["#", " ", "Filename", "Size", "Modified", "Status"];
    let mut rows: Vec<Vec<String>> = Vec::new();

    for (i, (file, overwriting)) in to_move.iter().enumerate() {
        let name = file
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        let emoji = file_emoji(&file.path);
        let name_c = if file.is_archive() {
            color::yellow(name)
        } else if *overwriting {
            color::orange(name)
        } else {
            color::green(name)
        };

        let status = if *overwriting {
            format!("{} overwrite", color::orange("🔄"))
        } else {
            format!("{} new", color::green("✓"))
        };

        rows.push(vec![
            color::gray(&(i + 1).to_string()),
            emoji.to_string(),
            name_c,
            color::orange(&format_size(file.size)),
            color::blue(&age_str(file.modified)),
            status,
        ]);
    }

    print_table(Some(&title), &headers, &rows);

    let total_size: u64 = to_move.iter().map(|(f, _)| f.size).sum();
    println!(
        "  {} files  •  total {}",
        color::cyan(&to_move.len().to_string()),
        color::orange(&format_size(total_size))
    );

    // Dry run
    if dry_run {
        println!();
        println!(
            "{E_INFO} {}",
            color::yellow("Dry run — no files were actually moved")
        );
        return;
    }

    // Confirmation prompt if not forced and there are overwrites
    let has_overwrites = to_move.iter().any(|(_, ow)| *ow);
    if has_overwrites && !force && is_tty() {
        print!("{E_WARNING} Some files will be overwritten. Continue? [y/N] ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("{E_INFO} {}", color::yellow("Move cancelled"));
            return;
        }
    }

    // Perform the move
    println!();
    vlog(
        verbose,
        "INFO",
        &format!("Moving {} file(s)...", to_move.len()),
    );

    let mut moved = 0u64;
    let mut errors: Vec<String> = Vec::new();

    for (file, overwriting) in &to_move {
        let name = file
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        let dest_path = dest.join(file.path.file_name().unwrap_or_default());

        // Handle overwrite by removing existing file first
        if *overwriting {
            if let Err(e) = fs::remove_file(&dest_path) {
                let err_msg = format!("Failed to remove existing '{}': {}", name, e);
                eprintln!("{E_ERROR} {}", color::red(&err_msg));
                errors.push(err_msg);
                continue;
            }
        }

        match fs::rename(&file.path, &dest_path) {
            Ok(_) => {
                let status = if *overwriting { "overwritten" } else { "moved" };
                vlog(
                    verbose,
                    "SUCCESS",
                    &format!(
                        "{} {} ({})",
                        if *overwriting { "🔄" } else { "✓" },
                        name,
                        status
                    ),
                );
                moved += 1;
            }
            Err(e) => {
                let err_msg = format!("Failed to move '{}': {}", name, e);
                eprintln!("{E_ERROR} {}", color::red(&err_msg));
                errors.push(err_msg);
            }
        }
    }

    // Summary
    println!();
    if moved > 0 {
        println!(
            "{E_SUCCESS} {}",
            color::green(&format!(
                "Successfully moved {} file(s) to {}",
                moved,
                dest.display()
            ))
        );
    }

    if !errors.is_empty() {
        eprintln!(
            "{E_ERROR} {} {}",
            color::red(&format!("{} error(s) occurred:", errors.len())),
            color::gray("see above for details")
        );
        process::exit(1);
    }
}

// ── Archive entry struct ──────────────────────────────────────────────────────
#[derive(Debug, Clone)]
struct ArchiveEntry {
    name: String,
    size: u64,
    compressed: u64,
}

// ── Archive reading ───────────────────────────────────────────────────────────
fn read_archive_entries(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let n = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    if n.ends_with(".zip") {
        read_zip(path)
    } else if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
        read_tar(path, 'z')
    } else if n.ends_with(".tar.bz2") || n.ends_with(".tbz2") {
        read_tar(path, 'j')
    } else if n.ends_with(".tar.xz") || n.ends_with(".txz") {
        read_tar(path, 'J')
    } else if n.ends_with(".tar") {
        read_tar(path, ' ')
    } else if n.ends_with(".7z") {
        read_7z(path)
    } else if n.ends_with(".rar") {
        read_rar(path)
    } else {
        Err(format!("Unsupported: {}", path.display()))
    }
}

fn read_zip(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for i in 0..z.len() {
        let f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_file() {
            out.push(ArchiveEntry {
                name: f.name().to_string(),
                size: f.size(),
                compressed: f.compressed_size(),
            });
        }
    }
    Ok(out)
}

fn read_tar(path: &Path, comp: char) -> Result<Vec<ArchiveEntry>, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    macro_rules! parse {
        ($rd:expr) => {{
            let mut arc = tar::Archive::new($rd);
            let mut out = Vec::new();
            for e in arc.entries().map_err(|e| e.to_string())? {
                let e = e.map_err(|e| e.to_string())?;
                if e.header().entry_type().is_file() {
                    let size = e.header().size().unwrap_or(0);
                    let name = e
                        .path()
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .into_owned();
                    out.push(ArchiveEntry {
                        name,
                        size,
                        compressed: 0,
                    });
                }
            }
            Ok(out)
        }};
    }
    match comp {
        'z' => parse!(flate2::read::GzDecoder::new(f)),
        'j' => parse!(bzip2::read::BzDecoder::new(f)),
        'J' => parse!(xz2::read::XzDecoder::new(f)),
        _ => parse!(f),
    }
}

fn read_7z(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let out = process::Command::new("7z")
        .args(["l", "-slt", &path.display().to_string()])
        .output()
        .map_err(|_| "7z not found — install p7zip to work with .7z files".to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    let mut cur_name = String::new();
    let mut cur_size = 0u64;
    let mut cur_csize = 0u64;
    let mut is_dir = false;
    for line in text.lines() {
        if line.starts_with("----------") {
            if !cur_name.is_empty() && !is_dir {
                entries.push(ArchiveEntry {
                    name: cur_name.clone(),
                    size: cur_size,
                    compressed: cur_csize,
                });
            }
            cur_name.clear();
            cur_size = 0;
            cur_csize = 0;
            is_dir = false;
        } else if let Some(v) = line.strip_prefix("Path = ") {
            cur_name = v.to_string();
        } else if let Some(v) = line.strip_prefix("Size = ") {
            cur_size = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("Packed Size = ") {
            cur_csize = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("Attributes = ") {
            is_dir = v.contains('D');
        }
    }
    if !cur_name.is_empty() && !is_dir {
        entries.push(ArchiveEntry {
            name: cur_name,
            size: cur_size,
            compressed: cur_csize,
        });
    }
    Ok(entries)
}

fn read_rar(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let out = process::Command::new("unrar")
        .args(["l", &path.display().to_string()])
        .output()
        .map_err(|_| "unrar not found — install `unrar` to work with .rar files".to_string())?;
    if !out.status.success() {
        return Err("unrar exited with error".to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    let mut in_sec = false;
    for line in text.lines() {
        if line.starts_with("---") {
            in_sec = !in_sec;
            continue;
        }
        if !in_sec || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(size) = parts[1].replace(',', "").parse::<u64>() {
                let name = parts[3..].join(" ");
                if !name.ends_with('/') {
                    entries.push(ArchiveEntry {
                        name,
                        size,
                        compressed: 0,
                    });
                }
            }
        }
    }
    Ok(entries)
}

// ── Archive extraction ────────────────────────────────────────────────────────
fn should_write(dest: &Path, mode: Mode) -> bool {
    match mode {
        Mode::OverwriteAll => true,
        Mode::AllNew | Mode::OnlyNew => !dest.exists(),
        Mode::AllExist => false,
    }
}

fn extract_archive(
    path: &Path,
    output: &Path,
    mode: Mode,
    _new: &[String],
    verbose: bool,
) -> Result<usize, String> {
    let n = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    if n.ends_with(".zip") {
        extract_zip(path, output, mode, verbose)
    } else if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
        extract_tar(path, output, mode, 'z', verbose)
    } else if n.ends_with(".tar.bz2") || n.ends_with(".tbz2") {
        extract_tar(path, output, mode, 'j', verbose)
    } else if n.ends_with(".tar.xz") || n.ends_with(".txz") {
        extract_tar(path, output, mode, 'J', verbose)
    } else if n.ends_with(".tar") {
        extract_tar(path, output, mode, ' ', verbose)
    } else if n.ends_with(".7z") {
        extract_7z(path, output, mode, verbose)
    } else if n.ends_with(".rar") {
        extract_rar(path, output, mode)
    } else {
        Err(format!("Unsupported: {}", path.display()))
    }
}

fn extract_zip(path: &Path, output: &Path, mode: Mode, verbose: bool) -> Result<usize, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut count = 0;
    for i in 0..z.len() {
        let mut file = z.by_index(i).map_err(|e| e.to_string())?;
        if file.is_dir() {
            continue;
        }
        let dest = output.join(file.name());
        if !should_write(&dest, mode) {
            vlog(verbose, "DEBUG", &format!("{E_SKIP} {}", file.name()));
            continue;
        }
        if let Some(p) = dest.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&dest).map_err(|e| e.to_string())?;
        io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
        vlog(verbose, "DEBUG", &format!("{E_NEW} {}", file.name()));
        count += 1;
    }
    Ok(count)
}

fn extract_tar(
    path: &Path,
    output: &Path,
    mode: Mode,
    comp: char,
    verbose: bool,
) -> Result<usize, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut count = 0usize;
    macro_rules! do_extract {
        ($rd:expr) => {{
            let mut arc = tar::Archive::new($rd);
            for entry in arc.entries().map_err(|e| e.to_string())? {
                let mut entry = entry.map_err(|e| e.to_string())?;
                if !entry.header().entry_type().is_file() {
                    continue;
                }
                let name = entry
                    .path()
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .into_owned();
                let dest = output.join(&name);
                if !should_write(&dest, mode) {
                    vlog(verbose, "DEBUG", &format!("{E_SKIP} {name}"));
                    continue;
                }
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                fs::write(&dest, &buf).map_err(|e| e.to_string())?;
                vlog(verbose, "DEBUG", &format!("{E_NEW} {name}"));
                count += 1;
            }
        }};
    }
    match comp {
        'z' => {
            do_extract!(flate2::read::GzDecoder::new(f));
        }
        'j' => {
            do_extract!(bzip2::read::BzDecoder::new(f));
        }
        'J' => {
            do_extract!(xz2::read::XzDecoder::new(f));
        }
        _ => {
            do_extract!(f);
        }
    }
    Ok(count)
}

fn extract_7z(path: &Path, output: &Path, mode: Mode, _verbose: bool) -> Result<usize, String> {
    let flag = match mode {
        Mode::OverwriteAll => "-aoa",
        Mode::OnlyNew => "-aos",
        _ => "-aos",
    };
    let out_arg = format!("-o{}", output.display());
    let res = process::Command::new("7z")
        .args(["x", "-y", flag, &out_arg, &path.display().to_string()])
        .output()
        .map_err(|_| "7z not found — install p7zip to extract .7z files".to_string())?;
    if res.status.success() {
        let text = String::from_utf8_lossy(&res.stdout);
        let count = text
            .lines()
            .filter(|l| l.trim_start().starts_with("Extracting"))
            .count();
        Ok(count.max(1))
    } else {
        Err(String::from_utf8_lossy(&res.stderr).to_string())
    }
}

fn extract_rar(path: &Path, output: &Path, mode: Mode) -> Result<usize, String> {
    let flag = match mode {
        Mode::OverwriteAll => "-o+",
        _ => "-o-",
    };
    let res = process::Command::new("unrar")
        .args([
            "x",
            "-y",
            flag,
            &path.display().to_string(),
            &output.display().to_string(),
        ])
        .output()
        .map_err(|_| "unrar not found — install `unrar` to extract .rar files".to_string())?;
    if res.status.success() {
        let text = String::from_utf8_lossy(&res.stdout);
        Ok(text.lines().filter(|l| l.contains("Extracting")).count())
    } else {
        Err(String::from_utf8_lossy(&res.stderr).to_string())
    }
}

// ── Resolve archive from args or auto-detect ──────────────────────────────────
fn resolve_archive(file: Option<PathBuf>, verbose: bool) -> Option<PathBuf> {
    if let Some(p) = file {
        if p.exists() {
            return Some(p);
        }
        eprintln!(
            "{E_ERROR} {}",
            color::red(&format!("File not found: {}", p.display()))
        );
        return None;
    }
    let all = list_downloads("*");
    let archives: Vec<_> = all.into_iter().filter(|f| f.is_archive()).collect();
    if archives.is_empty() {
        eprintln!(
            "{E_ERROR} {}",
            color::red("No archive files found in Downloads")
        );
        return None;
    }
    let name = archives[0]
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    vlog(verbose, "INFO", &format!("Using newest archive: {name}"));
    Some(archives[0].path.clone())
}

// ── Verbose logger ────────────────────────────────────────────────────────────
fn vlog(verbose: bool, level: &str, msg: &str) {
    if !verbose {
        return;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            let s = d.as_secs();
            format!("{:02}:{:02}:{:02}", (s / 3600) % 24, (s / 60) % 60, s % 60)
        })
        .unwrap_or_else(|_| "??:??:??".into());
    let lc = match level {
        "INFO" => color::blue("INFO"),
        "SUCCESS" => color::green("SUCCESS"),
        "WARNING" => color::yellow("WARN"),
        "ERROR" => color::red("ERROR"),
        _ => color::gray("DEBUG"),
    };
    eprintln!("{} {}: {}", color::gray(&format!("[{now}]")), lc, msg);
}

// ── TTY detection ─────────────────────────────────────────────────────────────
fn is_tty() -> bool {
    #[cfg(unix)]
    {
        extern "C" {
            fn isatty(fd: i32) -> i32;
        }
        unsafe { isatty(0) != 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

// ── main ──────────────────────────────────────────────────────────────────────
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Path => cmd_path(),
        Commands::List {
            last,
            pattern,
            filter,
            list_archive,
            file,
            verbose,
            asc,
            desc: _,
        } => {
            if list_archive {
                cmd_list_archive(file, verbose);
            } else {
                cmd_list(last, &pattern, &filter, asc);
            }
        }
        Commands::Extract {
            file,
            filter,
            output,
            overwrite,
            verbose,
        } => {
            cmd_extract(file, &filter, output, overwrite, verbose);
        }
        Commands::Move {
            pattern,
            filter,
            dest,
            last,
            oldest,
            force,
            dry_run,
            verbose,
            asc,
            desc: _,
        } => {
            cmd_move(
                &pattern, &filter, &dest, last, oldest, force, dry_run, verbose, asc,
            );
        }
    }
}
