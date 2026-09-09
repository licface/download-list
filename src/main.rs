// download-list — dwn
// Author  : Hadi Cahyadi <cumulus13@gmail.com>
// Homepage: https://github.com/cumulus13/download-list
// License : MIT

#![allow(dead_code)]

use clap::{Parser, Subcommand};
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
    pub fn cyan(t: &str)    -> String { fg(0x4D, 0xD0, 0xE1, t) }
    pub fn green(t: &str)   -> String { fg(0x66, 0xBB, 0x6A, t) }
    pub fn yellow(t: &str)  -> String { fg(0xFF, 0xD5, 0x4F, t) }
    pub fn red(t: &str)     -> String { fg(0xEF, 0x53, 0x50, t) }
    pub fn blue(t: &str)    -> String { fg(0x42, 0xA5, 0xF5, t) }
    pub fn magenta(t: &str) -> String { fg(0xAB, 0x47, 0xBC, t) }
    pub fn gray(t: &str)    -> String { fg(0x78, 0x90, 0x9C, t) }
    pub fn white(t: &str)   -> String { fg(0xEC, 0xEF, 0xF1, t) }
    pub fn orange(t: &str)  -> String { fg(0xFF, 0xA7, 0x26, t) }
    pub fn teal(t: &str)    -> String { fg(0x26, 0xA6, 0x9A, t) }
}

// ── Emoji ─────────────────────────────────────────────────────────────────────
const E_FILE:     &str = "📄";
const E_ARCHIVE:  &str = "📦";
const E_SUCCESS:  &str = "✅";
const E_ERROR:    &str = "❌";
const E_INFO:     &str = "ℹ️ ";
const E_WARNING:  &str = "⚠️ ";
const E_FOLDER:   &str = "📁";
const E_DOWNLOAD: &str = "📥";
const E_EXTRACT:  &str = "🔧";
const E_SEARCH:   &str = "🔍";
const E_SKIP:     &str = "⏭️ ";
const E_NEW:      &str = "🆕";
const E_EXISTING: &str = "📌";
const E_IMAGE:    &str = "🖼️ ";
const E_VIDEO:    &str = "🎬";
const E_AUDIO:    &str = "🎵";
const E_TEXT:     &str = "📝";
const E_DOC:      &str = "📋";

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
      dwn list                    List all downloaded files (oldest first)\n  \
      dwn list --desc             List newest first\n  \
      dwn list --last             Show only the newest file\n  \
      dwn list -p xmap            Substring match: finds *xmap* anywhere in the name\n  \
      dwn list -p \"*.zip\"         Exact glob match (already has a wildcard)\n  \
      dwn list -z                 Inspect contents of the newest archive\n  \
      dwn list -z -f foo.7z       Inspect a specific archive\n  \
      dwn list -z -p xmap         No -f: opens the newest archive named *xmap*\n  \
      dwn extract                 Extract the newest archive\n  \
      dwn extract -f foo.zip      Extract a specific archive\n  \
      dwn extract -p \"*.txt\"      Extract only files matching a pattern\n  \
      dwn extract --overwrite     Overwrite all existing files\n  \
      dwn extract --verbose       Show per-file details\n  \
      dwn path                    Print the Downloads directory path",
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
        /// Inspect the contents of an archive (like `7z l`)
        #[arg(short = 'z', long = "list-archive")]
        list_archive: bool,
        /// Archive to inspect (used with -z)
        #[arg(short, long)]
        file: Option<PathBuf>,
        #[arg(short, long)]
        verbose: bool,
        /// Sort oldest-first (ascending by modification time) — this is the default
        #[arg(long, conflicts_with = "desc")]
        asc: bool,
        /// Sort newest-first (descending)
        #[arg(long, conflicts_with = "asc")]
        desc: bool,
    },
    /// 🔧  Extract an archive from the Downloads directory
    #[command(alias = "e")]
    Extract {
        #[arg(short, long)]
        file: Option<PathBuf>,
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Overwrite all existing files without asking
        #[arg(short = 'w', long)]
        overwrite: bool,
        /// Only extract entries matching this glob (e.g. "*.txt", "docs/*")
        #[arg(short, long, default_value = "*")]
        pattern: String,
        #[arg(short, long)]
        verbose: bool,
    },
    /// 📁  Show the Downloads directory path
    #[command(alias = "p")]
    Path,
}

// ── Archive extensions ────────────────────────────────────────────────────────
const ARCHIVE_EXTS: &[&str] = &[
    ".zip", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2",
    ".tar.xz", ".txz", ".tar", ".7z", ".rar",
];

fn is_archive(p: &Path) -> bool {
    let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    ARCHIVE_EXTS.iter().any(|e| n.ends_with(e))
}

// ── Downloads dir ─────────────────────────────────────────────────────────────
fn downloads_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    if let Ok(xdg) = std::env::var("XDG_DOWNLOAD_DIR") {
        let p = PathBuf::from(&xdg);
        if p.is_absolute() { let _ = fs::create_dir_all(&p); return p; }
    }
    let home = home_dir();
    let dl = home.join("Downloads");
    let _ = fs::create_dir_all(&dl);
    dl
}

fn home_dir() -> PathBuf {
    std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

// ── File listing ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
struct FileEntry {
    path:     PathBuf,
    modified: u64,
    size:     u64,
}

impl FileEntry {
    fn is_archive(&self) -> bool { is_archive(&self.path) }
}

fn list_downloads(pattern: &str) -> Vec<FileEntry> {
    let dir = downloads_dir();
    let glob = as_glob(pattern);
    let mut out: Vec<FileEntry> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_file() { continue; }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !glob_match(&glob, name) { continue; }
            if let Ok(meta) = p.metadata() {
                let modified = meta.modified().ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()).unwrap_or(0);
                out.push(FileEntry { path: p, modified, size: meta.len() });
            }
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.modified));
    out
}

/// Turn a user-supplied `-p/--pattern` value into an actual glob. If the
/// person didn't include any wildcard characters, treat it as a substring
/// search (`*pattern*`) instead of demanding an exact full-name match —
/// `-p xmap` should find `xmaps_with_bmkg.zip`, not just a file literally
/// named `xmap`. Patterns that already contain `*`/`?` are left untouched so
/// people can still anchor an exact match when they want to.
fn as_glob(pattern: &str) -> String {
    if pattern.is_empty() || pattern == "*" {
        "*".to_string()
    } else if pattern.contains('*') || pattern.contains('?') {
        pattern.to_string()
    } else {
        format!("*{pattern}*")
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let (pb, tb) = (pattern.as_bytes(), text.as_bytes());
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut spi, mut sti) = (usize::MAX, 0usize);
    while ti < tb.len() {
        if pi < pb.len() && (pb[pi] == b'?' || pb[pi].eq_ignore_ascii_case(&tb[ti])) {
            pi += 1; ti += 1;
        } else if pi < pb.len() && pb[pi] == b'*' {
            spi = pi; sti = ti; pi += 1;
        } else if spi != usize::MAX {
            pi = spi + 1; sti += 1; ti = sti;
        } else { return false; }
    }
    while pi < pb.len() && pb[pi] == b'*' { pi += 1; }
    pi == pb.len()
}

// ── Formatting helpers ────────────────────────────────────────────────────────
fn terminal_width() -> usize {
    use terminal_size::{terminal_size, Width};
    terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(100)
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 { return "0 B".to_string(); }
    let mut n = bytes as f64;
    for unit in &["B","KB","MB","GB","TB"] {
        if n < 1024.0 { return format!("{n:.1} {unit}"); }
        n /= 1024.0;
    }
    format!("{n:.1} PB")
}

fn format_ts(ts: u64) -> String {
    let secs  = ts % 60;
    let mins  = (ts / 60) % 60;
    let hours = (ts / 3600) % 24;
    let (y, mo, d) = days_to_ymd(ts / 86400);
    format!("{y:04}-{mo:02}-{d:02} {hours:02}:{mins:02}:{secs:02}")
}

fn days_to_ymd(days: u64) -> (u64, u8, u8) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp = (5*doy + 2) / 153;
    let d = doy - (153*mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u8, d as u8)
}

fn format_age(secs: u64) -> String {
    if secs < 60          { return format!("{secs}s"); }
    if secs < 3600        { return format!("{}m", secs/60); }
    if secs < 86400       { return format!("{}h", secs/3600); }
    if secs < 86400*30    { return format!("{}d", secs/86400); }
    if secs < 86400*365   { return format!("{}mo", secs/(86400*30)); }
    format!("{}y", secs/(86400*365))
}

fn age_str(modified: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format_age(now.saturating_sub(modified))
}

fn file_emoji(p: &Path) -> &'static str {
    if is_archive(p) { return E_ARCHIVE; }
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg"|"jpeg"|"png"|"gif"|"bmp"|"svg"|"webp"|"ico" => E_IMAGE,
        "mp4"|"avi"|"mkv"|"mov"|"wmv"|"flv"|"webm"       => E_VIDEO,
        "mp3"|"wav"|"flac"|"aac"|"ogg"|"m4a"             => E_AUDIO,
        "txt"|"md"|"py"|"js"|"ts"|"rs"|"c"|"cpp"|"h"
            |"go"|"rb"|"sh"|"yaml"|"toml"|"json"         => E_TEXT,
        "pdf"|"doc"|"docx"|"xls"|"xlsx"|"ppt"|"pptx"    => E_DOC,
        _ => E_FILE,
    }
}

fn type_label(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if ARCHIVE_EXTS.iter().any(|e| lower.ends_with(e)) { return "Archive"; }
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg"|"jpeg"|"png"|"gif"|"bmp"|"svg"|"webp" => "Image",
        "mp4"|"avi"|"mkv"|"mov"|"wmv"|"flv"|"webm"  => "Video",
        "mp3"|"wav"|"flac"|"aac"|"ogg"|"m4a"        => "Audio",
        "pdf"|"doc"|"docx"|"xls"|"xlsx"|"ppt"|"pptx"=> "Document",
        "txt"|"md"|"py"|"js"|"ts"|"rs"|"json"|"xml"
            |"yaml"|"toml"|"c"|"cpp"|"h"|"go"       => "Text",
        _ => "File",
    }
}

fn type_emoji(t: &str) -> &'static str {
    match t { "Archive"=>"📦","Image"=>E_IMAGE,"Video"=>E_VIDEO,
              "Audio"=>E_AUDIO,"Document"=>E_DOC,"Text"=>E_TEXT,_=>E_FILE }
}

// ── Table renderer ────────────────────────────────────────────────────────────
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut esc = false;
    for c in s.chars() {
        if esc { if c=='m' { esc=false; } }
        else if c=='\x1b' { esc=true; }
        else { out.push(c); }
    }
    out
}

fn vis(s: &str) -> usize { UnicodeWidthStr::width(strip_ansi(s).as_str()) }

/// Truncate a string's visible content to max_w chars, appending '…'.
fn trunc_vis(s: &str, max_w: usize) -> String {
    let plain = strip_ansi(s);
    if UnicodeWidthStr::width(plain.as_str()) <= max_w { return plain; }
    if max_w == 0 { return String::new(); }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in plain.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if used + w + 1 > max_w { break; }
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
            if i < n { nat[i] = nat[i].max(vis(cell)); }
        }
    }

    // Overhead: leading "│" + per-column " content │" = n*3 + 1
    let overhead = n * 3 + 1;
    let budget = tw.saturating_sub(overhead);

    // Fixed columns: those whose natural width fits within FIXED_MAX
    const FIXED_MAX: usize = 22;
    let fixed_total: usize = nat.iter().map(|&w| if w <= FIXED_MAX { w } else { 0 }).sum();
    let flex_count = nat.iter().filter(|&&w| w > FIXED_MAX).count();
    let flex_budget = budget.saturating_sub(fixed_total).checked_div(flex_count).unwrap_or(0);

    let mut widths: Vec<usize> = (0..n).map(|i| {
        let natural = nat[i];
        let header_min = vis(headers[i]);
        if natural > FIXED_MAX {
            flex_budget.max(header_min).min(natural)
        } else {
            natural.max(header_min)
        }
    }).collect();

    // Make sure flex cols are at least as wide as their header
    for i in 0..n { widths[i] = widths[i].max(vis(headers[i])); }

    let total: usize = widths.iter().sum::<usize>() + overhead;
    let sep_len = total.min(tw);
    let sep = color::gray(&"─".repeat(sep_len));

    if let Some(t) = title {
        let tv = vis(t);
        let pad = if sep_len > tv { (sep_len - tv) / 2 } else { 0 };
        println!("{}{}", " ".repeat(pad), color::cyan(t));
    }
    println!("{sep}");

    // Header
    print!("{}", color::gray("│"));
    for (i, h) in headers.iter().enumerate() {
        let pad = widths[i].saturating_sub(vis(h));
        print!(" {}{} {}", color::white(h), " ".repeat(pad), color::gray("│"));
    }
    println!();
    println!("{sep}");

    // Data rows
    for row in rows {
        print!("{}", color::gray("│"));
        for (i, cell) in row.iter().enumerate() {
            if i >= n { break; }
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

fn cmd_list(last: bool, pattern: &str, asc: bool) {
    let mut files = list_downloads(pattern);
    if files.is_empty() { println!("{E_INFO} {}", color::yellow("No files found")); return; }

    // list_downloads returns newest-first. `--last` always means "the single
    // newest matching file", regardless of display sort order — so select it
    // here, before any reversal for --asc, not after.
    if last { files.truncate(1); }

    if asc { files.reverse(); }

    let sort_indicator = if asc {
        format!("  {}", color::teal("↑ oldest first"))
    } else {
        format!("  {}", color::magenta("↓ newest first"))
    };

    let dl = downloads_dir();
    let title = format!("{E_DOWNLOAD} Downloads  —  {}{sort_indicator}", color::gray(&dl.display().to_string()));

    let headers = ["#", " ", "Filename", "Size", "Modified", "Age"];
    let mut rows: Vec<Vec<String>> = Vec::new();

    for (i, f) in files.iter().enumerate() {
        let name = f.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let emoji = file_emoji(&f.path);
        let name_c = if f.is_archive() { color::yellow(name) } else { color::green(name) };
        rows.push(vec![
            color::gray(&(i+1).to_string()),
            emoji.to_string(),
            name_c,
            color::orange(&format_size(f.size)),
            color::blue(&format_ts(f.modified)),
            color::magenta(&age_str(f.modified)),
        ]);
    }

    print_table(Some(&title), &headers, &rows);
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    println!("  {} files  •  total {}", color::cyan(&files.len().to_string()), color::orange(&format_size(total_size)));
}

fn cmd_list_archive(file: Option<PathBuf>, pattern: &str, verbose: bool) {
    let (path, matched_by_name) = match resolve_archive(file, pattern, verbose) { Some(p)=>p, None=>process::exit(1) };
    vlog(verbose, "INFO", &format!("{E_SEARCH} Inspecting: {}", path.display()));

    let all_entries = match read_archive_entries(&path) {
        Ok(e) => e,
        Err(e) => { eprintln!("{E_ERROR} {}", color::red(&format!("Cannot read archive: {e}"))); process::exit(1); }
    };
    if all_entries.is_empty() {
        eprintln!("{E_WARNING} {}", color::yellow("Archive is empty"));
        process::exit(1);
    }

    let filtered: Vec<ArchiveEntry> = all_entries.iter().filter(|e| entry_matches(pattern, &e.name)).cloned().collect();
    // If the pattern already did its job picking *which archive* to open
    // (no -f given), don't also demand it match something *inside* that
    // archive — show everything. Only treat an empty content-filter as an
    // error when the pattern was the sole basis for filtering (i.e. -f was
    // given, or no archive filename matched it).
    let entries: Vec<ArchiveEntry> = if filtered.is_empty() && matched_by_name {
        vlog(verbose, "INFO", &format!("Pattern '{pattern}' matched the archive itself; showing all {} entries", all_entries.len()));
        all_entries
    } else {
        filtered
    };
    if entries.is_empty() {
        eprintln!("{E_WARNING} {}", color::yellow(&format!("No entries match pattern '{pattern}'")));
        process::exit(1);
    }

    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    println!();
    println!("{E_ARCHIVE}  {}", color::cyan(&path.file_name().unwrap_or_default().to_string_lossy()));
    println!("  {E_INFO} {} files   {}", color::blue(&entries.len().to_string()), color::orange(&format_size(total_size)));

    let title = format!("Contents of  {}", path.file_name().unwrap_or_default().to_string_lossy());
    let headers = ["#", "T", "Filename", "Size", "Compressed"];
    let mut rows: Vec<Vec<String>> = Vec::new();

    for (i, e) in entries.iter().enumerate() {
        let lbl = type_label(&e.name);
        // Truncate by character count, not byte offset — slicing raw bytes
        // can land mid-character on multi-byte UTF-8 names and panic.
        let display = if e.name.chars().count() > 64 {
            let total = e.name.chars().count();
            let tail: String = e.name.chars().skip(total.saturating_sub(63)).collect();
            format!("…{tail}")
        } else {
            e.name.clone()
        };
        rows.push(vec![
            color::gray(&(i+1).to_string()),
            type_emoji(lbl).to_string(),
            color::green(&display),
            color::orange(&if e.size>0 { format_size(e.size) } else { "—".into() }),
            color::teal(&if e.compressed>0 { format_size(e.compressed) } else { "—".into() }),
        ]);
    }
    print_table(Some(&title), &headers, &rows);
}

// ── Extract mode ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode { OverwriteAll, AllNew, OnlyNew, AllExist }

fn cmd_extract(file: Option<PathBuf>, output: PathBuf, overwrite: bool, pattern: &str, verbose: bool) {
    let (path, matched_by_name) = match resolve_archive(file, pattern, verbose) { Some(p)=>p, None=>process::exit(1) };

    let file_size = match fs::metadata(&path) {
        Ok(m) => m.len(),
        Err(e) => { eprintln!("{E_ERROR} {}", color::red(&e.to_string())); process::exit(1); }
    };
    if file_size == 0 { eprintln!("{E_ERROR} {}", color::red("File is empty")); process::exit(1); }
    vlog(verbose, "INFO", &format!("File size: {}", format_size(file_size)));

    let _ = fs::create_dir_all(&output);
    let test = output.join(".dwn_write_test");
    match fs::File::create(&test) {
        Ok(_) => { let _ = fs::remove_file(&test); }
        Err(_) => { eprintln!("{E_ERROR} {}", color::red(&format!("No write permission: {}", output.display()))); process::exit(1); }
    }

    vlog(verbose, "INFO", "Analysing archive contents…");
    let all_entries = match read_archive_entries(&path) {
        Ok(e) => e,
        Err(e) => { eprintln!("{E_ERROR} {}", color::red(&e)); process::exit(1); }
    };
    if all_entries.is_empty() { eprintln!("{E_WARNING} {}", color::yellow("Archive is empty")); process::exit(1); }
    let total_entries = all_entries.len();

    // Apply the entry-name filter (if any) before doing anything else, so
    // counts, conflict detection, and the extraction itself all agree.
    let filtered: Vec<ArchiveEntry> = all_entries.iter().filter(|e| entry_matches(pattern, &e.name)).cloned().collect();
    // If the pattern already did its job picking *which archive* to open
    // (no -f given), don't also demand it match something *inside* that
    // archive before extracting anything — extract everything from it.
    let mut used_fallback = false;
    let entries: Vec<ArchiveEntry> = if filtered.is_empty() && matched_by_name {
        used_fallback = true;
        vlog(verbose, "INFO", &format!("Pattern '{pattern}' matched the archive itself; extracting all {} entries", all_entries.len()));
        all_entries
    } else {
        filtered
    };
    if entries.is_empty() {
        eprintln!("{E_WARNING} {}", color::yellow(&format!("No entries match pattern '{pattern}'")));
        process::exit(1);
    }
    if pattern != "*" && !pattern.is_empty() && !used_fallback {
        vlog(verbose, "INFO", &format!("Pattern '{pattern}' matched {} of {} entries", entries.len(), total_entries));
    }
    // The actual extraction (extract_zip/tar/7z/rar) independently re-applies
    // this pattern per-entry — it doesn't know about the archive-selection
    // fallback above. If we fell back to "extract the whole archive", the
    // pattern must NOT be forwarded as a content filter too, or every entry
    // gets filtered out again down there (e.g. pattern "vmmon" matching the
    // archive's filename but none of its internal file names) and nothing
    // gets extracted despite `entries` correctly listing everything.
    let extract_pattern: &str = if used_fallback { "*" } else { pattern };

    vlog(verbose, "INFO", &format!("{} files  ({})", entries.len(), format_size(entries.iter().map(|e|e.size).sum::<u64>())));

    let (existing, new_files): (Vec<&ArchiveEntry>, Vec<&ArchiveEntry>) = entries.iter()
        .partition(|e| safe_join(&output, &e.name).map(|d| d.exists()).unwrap_or(true));

    if verbose {
        if !existing.is_empty() {
            vlog(verbose, "INFO", &format!("{} existing:", existing.len()));
            for e in existing.iter().take(5) { vlog(verbose,"INFO",&format!("  {E_EXISTING} {}",e.name)); }
            if existing.len()>5 { vlog(verbose,"INFO",&format!("  … and {} more",existing.len()-5)); }
        }
        if !new_files.is_empty() {
            vlog(verbose, "INFO", &format!("{} new:", new_files.len()));
            for e in new_files.iter().take(5) { vlog(verbose,"INFO",&format!("  {E_NEW} {}",e.name)); }
            if new_files.len()>5 { vlog(verbose,"INFO",&format!("  … and {} more",new_files.len()-5)); }
        }
    }

    let mode = if overwrite { Mode::OverwriteAll }
    else if existing.is_empty() { Mode::AllNew }
    else if !new_files.is_empty() { Mode::OnlyNew }
    else { Mode::AllExist };

    if mode == Mode::AllExist {
        let msg = format!("All {} matching files already exist. Use --overwrite to re-extract.", entries.len());
        println!("{E_INFO} {}", color::yellow(&msg));
        if is_tty() && !overwrite {
            print!("{E_WARNING} Overwrite all? [y/N] ");
            io::stdout().flush().ok();
            let mut r = String::new();
            io::stdin().read_line(&mut r).ok();
            if matches!(r.trim().to_lowercase().as_str(),"y"|"yes") {
                do_extract(&path, &output, Mode::OverwriteAll, extract_pattern, existing.len(), verbose);
            }
        }
        return;
    }

    do_extract(&path, &output, mode, extract_pattern, existing.len(), verbose);
}

fn do_extract(path: &Path, output: &Path, mode: Mode, pattern: &str, existing_count: usize, verbose: bool) {
    let fname = path.file_name().unwrap_or_default().to_string_lossy();
    let verb = match mode {
        Mode::OverwriteAll => "Overwriting",
        Mode::AllNew       => "Extracting",
        Mode::OnlyNew      => "Extracting new files from",
        Mode::AllExist     => return,
    };
    if pattern != "*" && !pattern.is_empty() {
        println!("{E_EXTRACT} {} {} {}", color::cyan(verb), color::yellow(&fname), color::gray(&format!("(pattern: {pattern})")));
    } else {
        println!("{E_EXTRACT} {} {}", color::cyan(verb), color::yellow(&fname));
    }

    match extract_archive(path, output, mode, pattern, verbose) {
        Ok(count) => {
            let msg = match mode {
                Mode::OverwriteAll => format!("Overwrote {count} files from {fname}"),
                Mode::AllNew       => format!("Extracted {count} files from {fname}"),
                Mode::OnlyNew      => format!("Extracted {count} new files (skipped {existing_count} existing) from {fname}"),
                Mode::AllExist     => return,
            };
            println!("{E_SUCCESS} {}", color::green(&msg));
            println!("  {E_FOLDER} {}", color::gray(&output.display().to_string()));
        }
        Err(e) => { eprintln!("{E_ERROR} {}", color::red(&format!("Extraction failed: {e}"))); process::exit(1); }
    }
}

// ── Archive entry struct ──────────────────────────────────────────────────────
#[derive(Debug, Clone)]
struct ArchiveEntry { name: String, size: u64, compressed: u64 }

/// Does an archive entry name match a user-supplied glob pattern?
/// Matches against the full (normalised) path as well as the basename, so
/// both `-p "*.txt"` and `-p "docs/*"` behave the way people expect.
fn entry_matches(pattern: &str, name: &str) -> bool {
    if pattern.is_empty() || pattern == "*" { return true; }
    let glob = as_glob(pattern);
    let norm = name.replace('\\', "/");
    if glob_match(&glob, &norm) { return true; }
    let base = norm.rsplit('/').next().unwrap_or(&norm);
    glob_match(&glob, base)
}

/// Safely join an archive entry name onto the output directory, rejecting
/// absolute paths and `..` components ("zip-slip") that would otherwise let
/// a crafted archive write outside the extraction target.
fn safe_join(output: &Path, name: &str) -> Option<PathBuf> {
    let norm = name.replace('\\', "/");
    let mut result = output.to_path_buf();
    let mut any = false;
    for comp in Path::new(&norm).components() {
        match comp {
            std::path::Component::Normal(c) => { result.push(c); any = true; }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return None;
            }
        }
    }
    if !any { return None; }
    Some(result)
}

// ── Archive reading ───────────────────────────────────────────────────────────
fn read_archive_entries(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let n = path.file_name().and_then(|n|n.to_str()).unwrap_or("").to_lowercase();
    if n.ends_with(".zip")                              { read_zip(path) }
    else if n.ends_with(".tar.gz")||n.ends_with(".tgz") { read_tar(path, 'z') }
    else if n.ends_with(".tar.bz2")||n.ends_with(".tbz2"){ read_tar(path, 'j') }
    else if n.ends_with(".tar.xz")||n.ends_with(".txz") { read_tar(path, 'J') }
    else if n.ends_with(".tar")                         { read_tar(path, ' ') }
    else if n.ends_with(".7z")                          { read_7z(path) }
    else if n.ends_with(".rar")                         { read_rar(path) }
    else { Err(format!("Unsupported: {}", path.display())) }
}

fn read_zip(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let f = fs::File::open(path).map_err(|e|e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e|e.to_string())?;
    let mut out = Vec::new();
    for i in 0..z.len() {
        let f = z.by_index(i).map_err(|e|e.to_string())?;
        if f.is_file() { out.push(ArchiveEntry { name: f.name().to_string(), size: f.size(), compressed: f.compressed_size() }); }
    }
    Ok(out)
}

fn read_tar(path: &Path, comp: char) -> Result<Vec<ArchiveEntry>, String> {
    let f = fs::File::open(path).map_err(|e|e.to_string())?;
    macro_rules! parse {
        ($rd:expr) => {{
            let mut arc = tar::Archive::new($rd);
            let mut out = Vec::new();
            for e in arc.entries().map_err(|e|e.to_string())? {
                let e = e.map_err(|e|e.to_string())?;
                if e.header().entry_type().is_file() {
                    let size = e.header().size().unwrap_or(0);
                    let name = e.path().map_err(|e|e.to_string())?.to_string_lossy().into_owned();
                    out.push(ArchiveEntry { name, size, compressed:0 });
                }
            }
            Ok(out)
        }};
    }
    match comp {
        'z' => parse!(flate2::read::GzDecoder::new(f)),
        'j' => parse!(bzip2::read::BzDecoder::new(f)),
        'J' => parse!(xz2::read::XzDecoder::new(f)),
        _   => parse!(f),
    }
}

fn read_7z(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    // Use 7z CLI to list contents
    let out = process::Command::new("7z")
        .args(["l", "-slt", &path.display().to_string()])
        .output()
        .map_err(|_| "7z not found — install p7zip to work with .7z files".to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);

    // `-slt` output has exactly ONE "----------" separator in the whole
    // output — between the archive-level header block and the per-file
    // blocks — not one per file. Per-file blocks are delimited by blank
    // lines instead. Skip past that single separator, then parse
    // blank-line-delimited blocks for the actual entries.
    let body = match text.split_once("----------") {
        Some((_, rest)) => rest,
        None => return Ok(Vec::new()),
    };

    let mut entries = Vec::new();
    let mut cur_name: Option<String> = None;
    let mut cur_size = 0u64;
    let mut cur_csize = 0u64;
    let mut is_dir = false;

    for line in body.lines() {
        if line.trim().is_empty() {
            if let Some(name) = cur_name.take() {
                if !is_dir { entries.push(ArchiveEntry { name, size: cur_size, compressed: cur_csize }); }
            }
            cur_size = 0; cur_csize = 0; is_dir = false;
            continue;
        }
        if let Some(v) = line.strip_prefix("Path = ") {
            cur_name = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("Size = ") {
            cur_size = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("Packed Size = ") {
            cur_csize = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("Attributes = ") {
            is_dir = v.contains('D');
        }
    }
    if let Some(name) = cur_name.take() {
        if !is_dir { entries.push(ArchiveEntry { name, size: cur_size, compressed: cur_csize }); }
    }
    Ok(entries)
}

fn read_rar(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let out = process::Command::new("unrar")
        .args(["l", &path.display().to_string()])
        .output()
        .map_err(|_| "unrar not found — install `unrar` to work with .rar files".to_string())?;
    if !out.status.success() { return Err("unrar exited with error".to_string()); }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    let mut in_sec = false;
    for line in text.lines() {
        if line.starts_with("---") { in_sec = !in_sec; continue; }
        if !in_sec || line.trim().is_empty() { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Columns are: Attributes  Size  Date  Time  Name — Date and Time
        // are two separate whitespace-separated tokens, so the name starts
        // at index 4, not 3 (which is just the Time column).
        if parts.len() >= 5 {
            if let Ok(size) = parts[1].replace(',', "").parse::<u64>() {
                let name = parts[4..].join(" ");
                if !name.ends_with('/') { entries.push(ArchiveEntry { name, size, compressed:0 }); }
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

fn extract_archive(path: &Path, output: &Path, mode: Mode, pattern: &str, verbose: bool) -> Result<usize, String> {
    let n = path.file_name().and_then(|n|n.to_str()).unwrap_or("").to_lowercase();
    if n.ends_with(".zip")                               { extract_zip(path, output, mode, pattern, verbose) }
    else if n.ends_with(".tar.gz")||n.ends_with(".tgz")  { extract_tar(path, output, mode, 'z', pattern, verbose) }
    else if n.ends_with(".tar.bz2")||n.ends_with(".tbz2"){ extract_tar(path, output, mode, 'j', pattern, verbose) }
    else if n.ends_with(".tar.xz")||n.ends_with(".txz")  { extract_tar(path, output, mode, 'J', pattern, verbose) }
    else if n.ends_with(".tar")                          { extract_tar(path, output, mode, ' ', pattern, verbose) }
    else if n.ends_with(".7z")                           { extract_7z(path, output, mode, pattern, verbose) }
    else if n.ends_with(".rar")                          { extract_rar(path, output, mode, pattern) }
    else { Err(format!("Unsupported: {}", path.display())) }
}

fn extract_zip(path: &Path, output: &Path, mode: Mode, pattern: &str, verbose: bool) -> Result<usize, String> {
    let f = fs::File::open(path).map_err(|e|e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e|e.to_string())?;
    let mut count = 0;
    for i in 0..z.len() {
        let mut file = z.by_index(i).map_err(|e|e.to_string())?;
        if file.is_dir() { continue; }
        let name = file.name().to_string();
        if !entry_matches(pattern, &name) { continue; }
        let dest = match safe_join(output, &name) {
            Some(d) => d,
            None => { eprintln!("{E_WARNING} {}", color::yellow(&format!("Skipping unsafe path in archive: {name}"))); continue; }
        };
        if !should_write(&dest, mode) { vlog(verbose,"DEBUG",&format!("{E_SKIP} {name}")); continue; }
        if let Some(p) = dest.parent() { fs::create_dir_all(p).map_err(|e|e.to_string())?; }
        let mut out = fs::File::create(&dest).map_err(|e|e.to_string())?;
        io::copy(&mut file, &mut out).map_err(|e|e.to_string())?;
        vlog(verbose,"DEBUG",&format!("{E_NEW} {name}"));
        count += 1;
    }
    Ok(count)
}

fn extract_tar(path: &Path, output: &Path, mode: Mode, comp: char, pattern: &str, verbose: bool) -> Result<usize, String> {
    let f = fs::File::open(path).map_err(|e|e.to_string())?;
    let mut count = 0usize;
    macro_rules! do_extract {
        ($rd:expr) => {{
            let mut arc = tar::Archive::new($rd);
            for entry in arc.entries().map_err(|e|e.to_string())? {
                let mut entry = entry.map_err(|e|e.to_string())?;
                if !entry.header().entry_type().is_file() { continue; }
                let name = entry.path().map_err(|e|e.to_string())?.to_string_lossy().into_owned();
                if !entry_matches(pattern, &name) { continue; }
                let dest = match safe_join(output, &name) {
                    Some(d) => d,
                    None => { eprintln!("{E_WARNING} {}", color::yellow(&format!("Skipping unsafe path in archive: {name}"))); continue; }
                };
                if !should_write(&dest, mode) { vlog(verbose,"DEBUG",&format!("{E_SKIP} {name}")); continue; }
                if let Some(p) = dest.parent() { fs::create_dir_all(p).map_err(|e|e.to_string())?; }
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e|e.to_string())?;
                fs::write(&dest, &buf).map_err(|e|e.to_string())?;
                vlog(verbose,"DEBUG",&format!("{E_NEW} {name}"));
                count += 1;
            }
        }};
    }
    match comp {
        'z' => { do_extract!(flate2::read::GzDecoder::new(f)); }
        'j' => { do_extract!(bzip2::read::BzDecoder::new(f)); }
        'J' => { do_extract!(xz2::read::XzDecoder::new(f)); }
        _   => { do_extract!(f); }
    }
    Ok(count)
}

fn extract_7z(path: &Path, output: &Path, mode: Mode, pattern: &str, _verbose: bool) -> Result<usize, String> {
    let flag = match mode {
        Mode::OverwriteAll => "-aoa",
        Mode::OnlyNew      => "-aos",
        _                  => "-aos",
    };
    let out_arg = format!("-o{}", output.display());
    let mut args: Vec<String> = vec!["x".into(), "-y".into(), flag.into(), out_arg, path.display().to_string()];
    if pattern != "*" && !pattern.is_empty() {
        // Recurse into subdirectories so the mask also matches nested files.
        args.push("-r".into());
        args.push(pattern.to_string());
    }
    let res = process::Command::new("7z")
        .args(&args)
        .output()
        .map_err(|_| "7z not found — install p7zip to extract .7z files".to_string())?;
    if res.status.success() {
        let text = String::from_utf8_lossy(&res.stdout);
        // Count "Extracting" lines in 7z output
        let count = text.lines().filter(|l| l.trim_start().starts_with("Extracting")).count();
        if count == 0 && pattern != "*" && !pattern.is_empty() {
            return Err(format!("No entries matched pattern '{pattern}'"));
        }
        Ok(count.max(1))
    } else {
        Err(String::from_utf8_lossy(&res.stderr).to_string())
    }
}

fn extract_rar(path: &Path, output: &Path, mode: Mode, pattern: &str) -> Result<usize, String> {
    let flag = match mode { Mode::OverwriteAll => "-o+", _ => "-o-" };
    let mut args: Vec<String> = vec!["x".into(), "-y".into(), flag.into()];
    if pattern != "*" && !pattern.is_empty() {
        // Recurse into subdirectories so the mask also matches nested files.
        args.push("-r".into());
    }
    args.push(path.display().to_string());
    if pattern != "*" && !pattern.is_empty() {
        args.push(pattern.to_string());
    }
    args.push(output.display().to_string());
    let res = process::Command::new("unrar")
        .args(&args)
        .output()
        .map_err(|_| "unrar not found — install `unrar` to extract .rar files".to_string())?;
    if res.status.success() {
        let text = String::from_utf8_lossy(&res.stdout);
        let count = text.lines().filter(|l| l.contains("Extracting")).count();
        if count == 0 && pattern != "*" && !pattern.is_empty() {
            return Err(format!("No entries matched pattern '{pattern}'"));
        }
        Ok(count)
    } else {
        Err(String::from_utf8_lossy(&res.stderr).to_string())
    }
}

// ── Resolve archive from args or auto-detect ──────────────────────────────────
/// Resolve which archive to operate on. If `file` is given, use it. Otherwise
/// auto-detect: if `pattern` isn't the default `*`, first try to find the
/// newest archive whose *filename* matches it — that's what people mean by
/// `dwn list -z -p xmap` when they haven't said `-f`: "find the archive
/// named like xmap and show me what's in it," not "open whatever the newest
/// archive happens to be and search its contents for xmap." Only fall back
/// to "just the newest archive overall" if no filename matches.
/// Returns `(path, matched_by_filename)` so callers know whether the
/// pattern already did its job picking the archive (in which case an empty
/// content-filter result shouldn't be treated as an error).
fn resolve_archive(file: Option<PathBuf>, pattern: &str, verbose: bool) -> Option<(PathBuf, bool)> {
    if let Some(p) = file {
        if p.exists() { return Some((p, false)); }
        eprintln!("{E_ERROR} {}", color::red(&format!("File not found: {}", p.display())));
        return None;
    }
    let all = list_downloads("*");
    let archives: Vec<_> = all.into_iter().filter(|f| f.is_archive()).collect();
    if archives.is_empty() {
        eprintln!("{E_ERROR} {}", color::red("No archive files found in Downloads"));
        return None;
    }

    if pattern != "*" && !pattern.is_empty() {
        if let Some(m) = archives.iter().find(|f| {
            let name = f.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            entry_matches(pattern, name)
        }) {
            let name = m.path.file_name().unwrap_or_default().to_string_lossy().to_string();
            vlog(verbose, "INFO", &format!("Using archive matching '{pattern}': {name}"));
            return Some((m.path.clone(), true));
        }
    }

    let name = archives[0].path.file_name().unwrap_or_default().to_string_lossy().to_string();
    vlog(verbose, "INFO", &format!("Using newest archive: {name}"));
    Some((archives[0].path.clone(), false))
}

// ── Verbose logger ────────────────────────────────────────────────────────────
fn vlog(verbose: bool, level: &str, msg: &str) {
    if !verbose { return; }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| {
        let s = d.as_secs();
        format!("{:02}:{:02}:{:02}", (s/3600)%24, (s/60)%60, s%60)
    }).unwrap_or_else(|_| "??:??:??".into());
    let lc = match level {
        "INFO"    => color::blue("INFO"),
        "SUCCESS" => color::green("SUCCESS"),
        "WARNING" => color::yellow("WARN"),
        "ERROR"   => color::red("ERROR"),
        _         => color::gray("DEBUG"),
    };
    eprintln!("{} {}: {}", color::gray(&format!("[{now}]")), lc, msg);
}

// ── TTY detection ─────────────────────────────────────────────────────────────
fn is_tty() -> bool {
    #[cfg(unix)]
    { extern "C" { fn isatty(fd: i32) -> i32; } unsafe { isatty(0) != 0 } }
    #[cfg(windows)]
    { extern "C" { fn _isatty(fd: i32) -> i32; } unsafe { _isatty(0) != 0 } }
    #[cfg(not(any(unix, windows)))]
    { false }
}

// ── main ──────────────────────────────────────────────────────────────────────
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Path => cmd_path(),
        Commands::List { last, pattern, list_archive, file, verbose, asc: _, desc } => {
            // Ascending (oldest-first) is the default; `--desc` opts into
            // newest-first. `--asc` is accepted too, purely for symmetry —
            // it's already the default, so it's a no-op.
            if list_archive { cmd_list_archive(file, &pattern, verbose); }
            else { cmd_list(last, &pattern, !desc); }
        }
        Commands::Extract { file, output, overwrite, pattern, verbose } => {
            cmd_extract(file, output, overwrite, &pattern, verbose);
        }
    }
}