#!/usr/bin/env cargo

// File: src/main.rs
// Crate: download-list
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Date: 2026-05-10
// Description: List, Get last, extract Last file if possible, with colors and emoji
// License: MIT

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use colored::*;
use console::{Emoji, Term};
use indicatif::{ProgressBar, ProgressStyle};
use log::{warn, LevelFilter};

// Custom error types
#[derive(thiserror::Error, Debug)]
pub enum DownloadListError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("No archive files found")]
    NoArchiveFiles,

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Unsupported archive format: {0}")]
    UnsupportedFormat(String),

    #[error("Archive extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Constants for colors and emojis
struct Theme;

impl Theme {
    // ANSI 256 colors
    const CYAN: u8 = 51;
    const GREEN: u8 = 46;
    const YELLOW: u8 = 226;
    const RED: u8 = 196;
    const BLUE: u8 = 21;
    const MAGENTA: u8 = 201;
    const GRAY: u8 = 244;

    // Emoji maps
    const FILE: Emoji<'static, 'static> = Emoji("📄", "");
    const ARCHIVE: Emoji<'static, 'static> = Emoji("📦", "");
    const SUCCESS: Emoji<'static, 'static> = Emoji("✅", "");
    const ERROR: Emoji<'static, 'static> = Emoji("❌", "");
    const INFO: Emoji<'static, 'static> = Emoji("ℹ️", "");
    const WARNING: Emoji<'static, 'static> = Emoji("⚠️", "");
    const FOLDER: Emoji<'static, 'static> = Emoji("📁", "");
    const DOWNLOAD: Emoji<'static, 'static> = Emoji("📥", "");
    const EXTRACTING: Emoji<'static, 'static> = Emoji("🔧", "");
    const PROGRESS: Emoji<'static, 'static> = Emoji("🔄", "");
    const CHECKING: Emoji<'static, 'static> = Emoji("🔍", "");
    const SKIPPED: Emoji<'static, 'static> = Emoji("⏭️", "");
    const CONFLICT: Emoji<'static, 'static> = Emoji("⚡", "");
    const NEW: Emoji<'static, 'static> = Emoji("🆕", "");
    const EXISTING: Emoji<'static, 'static> = Emoji("📌", "");
}

/// Represents a file entry with metadata
#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    name: String,
    size: u64,
    modified: DateTime<Local>,
    is_archive: bool,
}

impl FileEntry {
    fn new(path: PathBuf) -> Result<Self> {
        let metadata = fs::metadata(&path)?;
        let modified = metadata.modified()?;
        let datetime: DateTime<Local> = modified.into();

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let is_archive = is_archive_file(&path);

        Ok(FileEntry {
            path,
            name,
            size: metadata.len(),
            modified: datetime,
            is_archive,
        })
    }

    fn age(&self) -> String {
        let now = Local::now();
        let duration = now.signed_duration_since(self.modified);

        if duration.num_days() > 365 {
            format!(
                "{}y {}mo",
                duration.num_days() / 365,
                (duration.num_days() % 365) / 30
            )
        } else if duration.num_days() > 30 {
            format!(
                "{}mo {}d",
                duration.num_days() / 30,
                duration.num_days() % 30
            )
        } else if duration.num_days() > 0 {
            format!(
                "{}d {}h",
                duration.num_days(),
                duration.num_hours() % 24
            )
        } else if duration.num_hours() > 0 {
            format!(
                "{}h {}m",
                duration.num_hours(),
                duration.num_minutes() % 60
            )
        } else if duration.num_minutes() > 0 {
            format!(
                "{}m {}s",
                duration.num_minutes(),
                duration.num_seconds() % 60
            )
        } else {
            format!("{}s", duration.num_seconds())
        }
    }
}

/// Main CLI structure
#[derive(Parser)]
#[command(
    name = "download-list",
    version = env!("CARGO_PKG_VERSION"),
    author = "Hadi Cahyadi <cumulus13@gmail.com>",
    about = "List, Get last, extract Last file if possible, with colors and emoji",
    long_about = None,
    after_help = "Homepage: https://github.com/cumulus13/download-list"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// No color output
    #[arg(long, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List downloaded files
    #[command(visible_aliases = ["l", "ls"])]
    List {
        /// Show only the last downloaded file
        #[arg(short, long)]
        last: bool,

        /// File pattern to match (glob pattern)
        #[arg(short, long, default_value = "*")]
        pattern: String,

        /// List contents of archive file (like '7z l')
        #[arg(short = 'z', long)]
        list_archive: bool,

        /// Specific archive file to list (used with -z)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Filter by extension
        #[arg(short = 'x', long)]
        extension: Option<String>,

        /// Sort by field (name, size, modified)
        #[arg(short = 's', long, default_value = "modified")]
        sort_by: String,

        /// Reverse sort order
        #[arg(short = 'r', long)]
        reverse: bool,
    },

    /// Extract archive file
    #[command(visible_aliases = ["e", "x"])]
    Extract {
        /// Specific file to extract (default: last archive)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Overwrite existing files without asking
        #[arg(short = 'w', long)]
        overwrite: bool,

        /// Keep original archive after extraction
        #[arg(short = 'k', long)]
        keep: bool,

        /// Delete archive after successful extraction
        #[arg(short = 'd', long, conflicts_with = "keep")]
        delete_after: bool,

        /// Show detailed progress
        #[arg(short = 'e', long)]
        verbose_extract: bool,
    },

    /// Show download directory path
    #[command(visible_aliases = ["p"])]
    Path,

    /// Show configuration and statistics
    #[command(visible_aliases = ["i", "stat"])]
    Info,

    /// Clean old files from download directory
    #[command(visible_aliases = ["c", "clean"])]
    Clean {
        /// Remove files older than specified days
        #[arg(short, long, default_value = "30")]
        older_than: u32,

        /// Dry run (show what would be deleted)
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,

        /// Pattern to match files to clean
        #[arg(short, long)]
        pattern: Option<String>,
    },
}

/// Check if file is an archive
fn is_archive_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let archive_extensions = [
        ".zip", ".rar", ".7z", ".tar", ".tar.gz", ".tar.bz2", ".tar.xz", ".tgz", ".tbz2",
        ".txz",
    ];

    archive_extensions.iter().any(|ext| name.ends_with(ext))
}

/// Get download directory based on platform
fn get_download_path() -> PathBuf {
    if let Some(dirs) = dirs::download_dir() {
        return dirs;
    }

    if let Some(home) = dirs::home_dir() {
        let downloads = home.join("Downloads");
        if downloads.exists() {
            return downloads;
        }
    }

    PathBuf::from(".")
}

/// Format file size in human-readable format
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = size as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

/// Colorize text with hex color
fn colorize(text: &str, color: u8, no_color: bool) -> String {
    if no_color {
        text.to_string()
    } else {
        text.truecolor(
            ((color >> 4) & 0xF) * 17,
            ((color >> 2) & 0x3) * 85,
            (color & 0x3) * 85,
        )
        .to_string()
    }
}

/// Print a timestamped verbose message
fn print_verbose(level: &str, message: &str, verbose: bool, no_color: bool) {
    if !verbose {
        return;
    }

    let timestamp = Local::now().format("%H:%M:%S");
    let level_color = match level {
        "INFO" => Theme::BLUE,
        "SUCCESS" => Theme::GREEN,
        "WARNING" => Theme::YELLOW,
        "ERROR" => Theme::RED,
        "DEBUG" => Theme::GRAY,
        _ => Theme::BLUE,
    };

    let prefix = format!("[{}] {}", timestamp, level);
    println!(
        "{} {}",
        colorize(&prefix, Theme::GRAY, no_color),
        colorize(message, level_color, no_color)
    );
}

/// Get file type description based on extension
fn get_file_type(path: &Path) -> &str {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    match ext.as_str() {
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "Archive",
        "txt" | "md" | "rst" | "log" => "Text",
        "py" | "rs" | "js" | "ts" | "go" | "java" | "cpp" | "c" | "h" => "Code",
        "json" | "yaml" | "yml" | "toml" | "xml" => "Data",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" => "Image",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "Audio",
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "Video",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "Document",
        "exe" | "msi" | "deb" | "rpm" | "appimage" => "Executable",
        "iso" | "img" | "dmg" => "Disk Image",
        _ => "File",
    }
}

/// List files in download directory
fn list_files(
    last: bool,
    pattern: &str,
    extension: Option<&str>,
    sort_by: &str,
    reverse: bool,
    verbose: bool,
    no_color: bool,
) -> Result<Vec<FileEntry>> {
    let download_path = get_download_path();
    print_verbose(
        "INFO",
        &format!("Listing files in: {}", download_path.display()),
        verbose,
        no_color,
    );

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in fs::read_dir(&download_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // Apply pattern filter
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        if pattern != "*" {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if !glob_pattern.matches(&file_name) {
                    continue;
                }
            }
        }

        // Apply extension filter
        if let Some(ext) = extension {
            let file_ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy();
            if file_ext != ext {
                continue;
            }
        }

        if let Ok(entry) = FileEntry::new(path) {
            files.push(entry);
        }
    }

    // Sort files
    match sort_by {
        "name" => {
            files.sort_by(|a, b| a.name.cmp(&b.name));
        }
        "size" => {
            files.sort_by(|a, b| a.size.cmp(&b.size));
        }
        _ => {
            files.sort_by(|a, b| a.modified.cmp(&b.modified));
        }
    }

    if reverse {
        files.reverse();
    }

    // Get last file if requested
    if last && !files.is_empty() {
        files = vec![files.remove(files.len() - 1)];
    }

    Ok(files)
}

/// Display files in a formatted table
fn display_files(files: &[FileEntry], no_color: bool) {
    if files.is_empty() {
        println!(
            "{} {}",
            Theme::INFO,
            colorize("No files found", Theme::YELLOW, no_color)
        );
        return;
    }

    println!(
        "\n{} {}",
        Theme::DOWNLOAD,
        colorize("Downloaded Files:", Theme::CYAN, no_color)
    );
    println!("{}", "─".repeat(100));
    println!(
        "{:<4} {:<40} {:>10} {:<20} {:>10}",
        colorize("No.", Theme::CYAN, no_color),
        colorize("File", Theme::GREEN, no_color),
        colorize("Size", Theme::YELLOW, no_color),
        colorize("Modified", Theme::BLUE, no_color),
        colorize("Age", Theme::MAGENTA, no_color)
    );
    println!("{}", "─".repeat(100));

    for (i, file) in files.iter().enumerate() {
        let emoji = if file.is_archive {
            Theme::ARCHIVE
        } else {
            Theme::FILE
        };

        let name = if file.name.len() > 38 {
            format!("...{}", &file.name[file.name.len() - 35..])
        } else {
            file.name.clone()
        };

        println!(
            "{:<4} {} {:<38} {:>10} {:<20} {:>10}",
            (i + 1).to_string(),
            emoji,
            colorize(&name, Theme::GREEN, no_color),
            colorize(&format_size(file.size), Theme::YELLOW, no_color),
            colorize(
                &file.modified.format("%Y-%m-%d %H:%M").to_string(),
                Theme::BLUE,
                no_color
            ),
            colorize(&file.age(), Theme::MAGENTA, no_color)
        );
    }
    println!("{}", "─".repeat(100));
}

/// List contents of an archive file
fn list_archive_contents(
    file_path: Option<PathBuf>,
    verbose: bool,
    no_color: bool,
) -> Result<()> {
    let file_path = match file_path {
        Some(path) => path,
        None => {
            let files = list_files(false, "*", None, "modified", false, verbose, no_color)?;
            let archive_files: Vec<&FileEntry> =
                files.iter().filter(|f| f.is_archive).collect();

            if archive_files.is_empty() {
                bail!(DownloadListError::NoArchiveFiles);
            }

            archive_files[0].path.clone()
        }
    };

    if !file_path.exists() {
        bail!(DownloadListError::FileNotFound(file_path));
    }

    print_verbose(
        "INFO",
        &format!("Listing contents of: {}", file_path.display()),
        verbose,
        no_color,
    );

    let files = extract_archive_list(&file_path)?;

    println!(
        "\n{} {}",
        Theme::ARCHIVE,
        colorize(
            &format!(
                "Archive: {}",
                file_path.file_name().unwrap_or_default().to_string_lossy()
            ),
            Theme::CYAN,
            no_color
        )
    );
    println!(
        "{} {} | {}",
        Theme::INFO,
        colorize(
            &format!("Total files: {}", files.len()),
            Theme::BLUE,
            no_color
        ),
        colorize(
            &format!(
                "Total size: {}",
                format_size(files.iter().map(|(_, s)| s).sum())
            ),
            Theme::YELLOW,
            no_color
        )
    );
    println!("{}", "─".repeat(100));
    println!(
        "{:<6} {:<70} {:<15}",
        colorize("No.", Theme::CYAN, no_color),
        colorize("Filename", Theme::GREEN, no_color),
        colorize("Type", Theme::BLUE, no_color)
    );
    println!("{}", "─".repeat(100));

    for (i, (name, _size)) in files.iter().enumerate() {
        let path = Path::new(name);
        let file_type = get_file_type(path);
        let display_name = if name.len() > 68 {
            format!("...{}", &name[name.len() - 65..])
        } else {
            name.clone()
        };

        println!(
            "{:<6} {:<70} {:<15}",
            (i + 1).to_string(),
            colorize(&display_name, Theme::GREEN, no_color),
            colorize(file_type, Theme::BLUE, no_color)
        );
    }
    println!("{}", "─".repeat(100));

    Ok(())
}

/// Extract list of files from archive
fn extract_archive_list(archive_path: &Path) -> Result<Vec<(String, u64)>> {
    let extension = archive_path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let mut files = Vec::new();

    // Try 7z first
    if let Ok(output) = Command::new("7z")
        .args(["l", "-slt", &archive_path.to_string_lossy()])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_file = String::new();
            let mut current_size: u64 = 0;

            for line in stdout.lines() {
                if line.starts_with("Path = ") {
                    current_file = line[7..].to_string();
                } else if line.starts_with("Size = ") {
                    if let Ok(size) = line[7..].parse::<u64>() {
                        current_size = size;
                    }
                    if !current_file.is_empty() && !current_file.ends_with('/') {
                        files.push((current_file.clone(), current_size));
                    }
                    current_file.clear();
                }
            }

            if !files.is_empty() {
                return Ok(files);
            }
        }
    }

    // Fallback for zip files
    if extension == "zip" {
        if let Ok(file) = File::open(archive_path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index(i) {
                        if !file.is_dir() {
                            files.push((file.name().to_string(), file.size()));
                        }
                    }
                }
            }
        }
    }

    if files.is_empty() {
        warn!(
            "Cannot list contents of archive: {}",
            archive_path.display()
        );
    }

    Ok(files)
}

/// Extract archive file
fn extract_archive(
    file_path: Option<PathBuf>,
    output_dir: PathBuf,
    overwrite: bool,
    _keep: bool,
    delete_after: bool,
    verbose: bool,
    no_color: bool,
) -> Result<()> {
    let file_path = match file_path {
        Some(path) => {
            if !path.exists() {
                bail!(DownloadListError::FileNotFound(path));
            }
            path
        }
        None => {
            let files = list_files(false, "*", None, "modified", false, verbose, no_color)?;
            let archive_files: Vec<&FileEntry> =
                files.iter().filter(|f| f.is_archive).collect();

            if archive_files.is_empty() {
                bail!(DownloadListError::NoArchiveFiles);
            }

            archive_files[0].path.clone()
        }
    };

    print_verbose(
        "INFO",
        &format!("Extracting: {}", file_path.display()),
        verbose,
        no_color,
    );

    fs::create_dir_all(&output_dir)?;

    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    progress.set_message(format!(
        "Extracting {}...",
        file_path.file_name().unwrap_or_default().to_string_lossy()
    ));

    let extension = file_path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let result = match extension.as_str() {
        "zip" => {
            let mut cmd = Command::new("unzip");
            if overwrite {
                cmd.arg("-o");
            } else {
                cmd.arg("-n");
            }
            cmd.arg(&file_path).arg("-d").arg(&output_dir).status()
        }
        "rar" => {
            let mut cmd = Command::new("unrar");
            cmd.arg("x");
            if overwrite {
                cmd.arg("-o+");
            } else {
                cmd.arg("-o-");
            }
            cmd.arg("-y").arg(&file_path).arg(&output_dir).status()
        }
        "7z" => {
            let mut cmd = Command::new("7z");
            cmd.arg("x");
            if overwrite {
                cmd.arg("-aoa");
            } else {
                cmd.arg("-aos");
            }
            cmd.arg("-y")
                .arg(&file_path)
                .arg(format!("-o{}", output_dir.display()))
                .status()
        }
        _ => {
            progress.finish_and_clear();
            bail!(DownloadListError::UnsupportedFormat(extension));
        }
    };

    progress.finish_and_clear();

    match result {
        Ok(status) if status.success() => {
            println!(
                "{} {}",
                Theme::SUCCESS,
                colorize("Extraction completed successfully", Theme::GREEN, no_color)
            );

            if delete_after {
                print_verbose(
                    "INFO",
                    "Deleting archive after extraction",
                    verbose,
                    no_color,
                );
                fs::remove_file(&file_path)?;
                println!(
                    "{} {}",
                    Theme::SUCCESS,
                    colorize("Archive deleted", Theme::GREEN, no_color)
                );
            }

            Ok(())
        }
        Ok(status) => {
            bail!(DownloadListError::ExtractionFailed(format!(
                "Exit code: {}",
                status
            )));
        }
        Err(e) => {
            bail!(DownloadListError::ExtractionFailed(e.to_string()));
        }
    }
}

/// Clean old files from download directory
fn clean_files(
    older_than: u32,
    dry_run: bool,
    force: bool,
    pattern: Option<String>,
    verbose: bool,
    no_color: bool,
) -> Result<()> {
    let download_path = get_download_path();
    let now = Local::now();
    let mut files_to_delete = Vec::new();
    let mut total_size: u64 = 0;

    print_verbose(
        "INFO",
        &format!("Cleaning files older than {} days", older_than),
        verbose,
        no_color,
    );

    for entry in fs::read_dir(&download_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if let Some(ref pattern) = pattern {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if !glob_pattern.matches(&file_name) {
                    continue;
                }
            }
        }

        if let Ok(metadata) = path.metadata() {
            if let Ok(modified) = metadata.modified() {
                let modified_time: DateTime<Local> = modified.into();
                let age = now.signed_duration_since(modified_time).num_days();

                if age >= older_than as i64 {
                    files_to_delete.push(path.clone());
                    total_size += metadata.len();
                }
            }
        }
    }

    if files_to_delete.is_empty() {
        println!(
            "{} {}",
            Theme::INFO,
            colorize("No files to clean", Theme::GREEN, no_color)
        );
        return Ok(());
    }

    println!(
        "\n{} {}",
        Theme::WARNING,
        colorize(
            &format!(
                "Found {} files to delete ({}):",
                files_to_delete.len(),
                format_size(total_size)
            ),
            Theme::YELLOW,
            no_color
        )
    );

    for file in &files_to_delete {
        let size = fs::metadata(file).map(|m| m.len()).unwrap_or(0);
        println!(
            "  {} {} ({})",
            Theme::FILE,
            colorize(
                &file.file_name().unwrap_or_default().to_string_lossy(),
                Theme::GRAY,
                no_color
            ),
            format_size(size)
        );
    }

    if dry_run {
        println!(
            "\n{} {}",
            Theme::INFO,
            colorize("Dry run - no files were deleted", Theme::CYAN, no_color)
        );
        return Ok(());
    }

    if !force {
        print!(
            "\n{} {}",
            Theme::WARNING,
            colorize("Delete these files? (y/N): ", Theme::YELLOW, no_color)
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() != "y" {
            println!(
                "{} {}",
                Theme::INFO,
                colorize("Operation cancelled", Theme::YELLOW, no_color)
            );
            return Ok(());
        }
    }

    let mut deleted = 0;
    for file in &files_to_delete {
        if let Err(e) = fs::remove_file(file) {
            eprintln!(
                "{} {}: {}",
                Theme::ERROR,
                colorize("Failed to delete", Theme::RED, no_color),
                e
            );
        } else {
            deleted += 1;
            print_verbose(
                "SUCCESS",
                &format!("Deleted: {}", file.display()),
                verbose,
                no_color,
            );
        }
    }

    println!(
        "{} {}",
        Theme::SUCCESS,
        colorize(
            &format!(
                "Deleted {} files ({} freed)",
                deleted,
                format_size(total_size)
            ),
            Theme::GREEN,
            no_color
        )
    );

    Ok(())
}

/// Display download directory info and statistics
fn display_info(verbose: bool, no_color: bool) -> Result<()> {
    let download_path = get_download_path();

    println!(
        "\n{} {}",
        Theme::INFO,
        colorize("Download Directory Info", Theme::CYAN, no_color)
    );
    println!("{}", "─".repeat(60));
    println!(
        "{} {}",
        colorize("Path:", Theme::BLUE, no_color),
        colorize(&download_path.display().to_string(), Theme::GREEN, no_color)
    );

    let mut total_files = 0;
    let mut total_size: u64 = 0;
    let mut archive_count = 0;
    let mut extensions: HashMap<String, (usize, u64)> = HashMap::new();

    if download_path.exists() {
        for entry in fs::read_dir(&download_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(metadata) = path.metadata() {
                    total_files += 1;
                    total_size += metadata.len();

                    if is_archive_file(&path) {
                        archive_count += 1;
                    }

                    let ext = path
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();

                    let entry = extensions.entry(ext).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += metadata.len();
                }
            }
        }
    }

    println!(
        "{} {}",
        colorize("Total Files:", Theme::BLUE, no_color),
        colorize(&total_files.to_string(), Theme::YELLOW, no_color)
    );
    println!(
        "{} {}",
        colorize("Total Size:", Theme::BLUE, no_color),
        colorize(&format_size(total_size), Theme::YELLOW, no_color)
    );
    println!(
        "{} {}",
        colorize("Archives:", Theme::BLUE, no_color),
        colorize(&archive_count.to_string(), Theme::YELLOW, no_color)
    );

    if verbose && !extensions.is_empty() {
        println!(
            "\n{}",
            colorize("File Types:", Theme::CYAN, no_color)
        );
        println!("{}", "─".repeat(60));

        let mut ext_vec: Vec<_> = extensions.iter().collect();
        ext_vec.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

        for (ext, (count, size)) in ext_vec.iter().take(10) {
            println!(
                "  .{:<10} {:>5} files  {}",
                ext,
                count,
                format_size(*size)
            );
        }
    }

    Ok(())
}

/// Main application
struct DownloadList;

impl DownloadList {
    fn run() -> Result<()> {
        env_logger::Builder::new()
            .filter_level(LevelFilter::Info)
            .format_timestamp_secs()
            .init();

        let cli = Cli::parse();
        let no_color = cli.no_color || !Term::stdout().features().colors_supported();

        match cli.command {
            Commands::List {
                last,
                pattern,
                list_archive,
                file,
                extension,
                sort_by,
                reverse,
            } => {
                if list_archive {
                    list_archive_contents(file, cli.verbose, no_color)?;
                } else {
                    let files = list_files(
                        last,
                        &pattern,
                        extension.as_deref(),
                        &sort_by,
                        reverse,
                        cli.verbose,
                        no_color,
                    )?;
                    display_files(&files, no_color);
                }
            }

            Commands::Extract {
                file,
                output,
                overwrite,
                keep,
                delete_after,
                verbose_extract,
            } => {
                extract_archive(
                    file,
                    output,
                    overwrite,
                    keep,
                    delete_after,
                    verbose_extract || cli.verbose,
                    no_color,
                )?;
            }

            Commands::Path => {
                let download_path = get_download_path();
                println!(
                    "{} {}",
                    Theme::FOLDER,
                    colorize("Download directory:", Theme::CYAN, no_color)
                );
                println!(
                    "  {}",
                    colorize(&download_path.display().to_string(), Theme::GREEN, no_color)
                );
            }

            Commands::Info => {
                display_info(cli.verbose, no_color)?;
            }

            Commands::Clean {
                older_than,
                dry_run,
                force,
                pattern,
            } => {
                clean_files(older_than, dry_run, force, pattern, cli.verbose, no_color)?;
            }
        }

        Ok(())
    }
}

fn main() -> ExitCode {
    match DownloadList::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!(
                "{} {}",
                Theme::ERROR,
                colorize(&format!("Error: {}", e), Theme::RED, false)
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_is_archive_file() {
        assert!(is_archive_file(Path::new("test.zip")));
        assert!(is_archive_file(Path::new("test.tar.gz")));
        assert!(!is_archive_file(Path::new("test.txt")));
    }

    #[test]
    fn test_get_file_type() {
        assert_eq!(get_file_type(Path::new("test.zip")), "Archive");
        assert_eq!(get_file_type(Path::new("test.txt")), "Text");
        assert_eq!(get_file_type(Path::new("test.jpg")), "Image");
    }
}