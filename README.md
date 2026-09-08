# download-list

**`dwn`** — A fast, colourful CLI tool to list, inspect, and extract files in your Downloads folder.

```
Author  : Hadi Cahyadi <cumulus13@gmail.com>
Homepage: https://github.com/cumulus13/download-list
License : MIT
```

## Features

- 📋 **List** all files in `~/Downloads` with size, date, and age
- 🔍 **Inspect** archive contents (`-z`) without extracting — like `7z l`
- 🔧 **Extract** the newest archive (or a specific one) with smart conflict handling:
  - Skips existing files by default
  - Only extracts new files when partially extracted
  - `--overwrite` flag to force re-extraction
- 🎨 **24-bit hex colour** output — full RGB ANSI palette, no palette approximation
- 📦 **Archive support**: `.zip`, `.tar.gz`, `.tgz`, `.tar.bz2`, `.tbz2`, `.tar.xz`, `.txz`, `.tar`, `.7z`\*, `.rar`\*
- 🐧 Respects `$XDG_DOWNLOAD_DIR` on Linux
- Zero external runtime dependencies (pure Rust for zip/tar/bz2/xz)

> \* `.7z` requires `p7zip` / `7z` in `$PATH`; `.rar` requires `unrar` in `$PATH`

---

## Installation

### From source

```bash
git clone https://github.com/cumulus13/download-list
cd download-list
cargo build --release
# Binary is at: target/release/dwn
sudo cp target/release/dwn /usr/local/bin/
```

### Requirements

| Crate         | Purpose             |
|---------------|---------------------|
| `clap` 4.4    | Argument parsing    |
| `zip` 0.6     | ZIP read/extract    |
| `tar` 0.4     | TAR read/extract    |
| `flate2`      | GZ decompression    |
| `bzip2`       | BZ2 decompression   |
| `xz2`         | XZ decompression    |
| `terminal_size` | Terminal width    |
| `unicode-width` | Display alignment |

`.7z` and `.rar` are handled by shelling out to the `7z` / `unrar` binaries
at runtime — no Rust crate is required for them, and none is bundled.

---

## Usage

```
dwn <COMMAND>
```

### Commands

| Command     | Alias | Description                                  |
|-------------|-------|----------------------------------------------|
| `list`      | `l`   | List files in the Downloads directory        |
| `extract`   | `e`   | Extract an archive from Downloads            |
| `path`      | `p`   | Print the Downloads directory path           |

---

### `dwn list`

```
dwn list [OPTIONS]

Options:
  -l, --last            Show only the single newest file
  -p, --pattern <PAT>   Glob filter (default: *)
  -z, --list-archive    Inspect archive contents (like `7z l`)
  -f, --file <FILE>     Archive to inspect (used with -z)
  -v, --verbose         Verbose output
```

**Examples:**
```bash
dwn list                      # List all files
dwn list --last               # Show only the newest file
dwn list --pattern "*.zip"    # Show only ZIP files
dwn list -z                   # Inspect the newest archive
dwn list -z -f myfile.tar.gz  # Inspect a specific archive
dwn list -z -p "*.png"        # Inspect contents, filtered to *.png entries
```

---

### `dwn extract`

```
dwn extract [OPTIONS]

Options:
  -f, --file <FILE>     Archive to extract (default: newest archive)
  -o, --output <DIR>    Output directory (default: current dir)
  -w, --overwrite       Overwrite all existing files without asking
  -p, --pattern <PAT>   Only extract entries matching this glob (default: *)
  -v, --verbose         Show per-file progress and timestamps
```

`--pattern` matches against both the entry's full path inside the archive
and its basename, so both `-p "*.png"` and `-p "assets/*"` work as expected.
It supports `*` and `?` wildcards, same as `list --pattern`. Conflict
detection (existing vs. new files) and the interactive overwrite prompt are
both computed only over the entries that match the pattern.

**Smart conflict logic:**

| Situation                       | Behaviour                                 |
|---------------------------------|-------------------------------------------|
| No files exist yet              | Extract all                               |
| Some files exist, some are new  | Extract only new, skip existing           |
| All files already exist         | Ask interactively (or use `--overwrite`)  |
| `--overwrite` flag set          | Always extract all                        |

**Examples:**
```bash
dwn extract                          # Extract newest archive → current dir
dwn extract -f archive.zip           # Extract specific file
dwn extract -f archive.zip -o ~/out  # Extract to ~/out
dwn extract --overwrite              # Force overwrite everything
dwn extract --verbose                # Show timestamped per-file log
dwn extract -p "*.pdf"               # Extract only PDF entries
dwn extract -p "images/*" -o ~/pics  # Extract only the images/ folder
```

---

### `dwn path`

```bash
dwn path   # Prints the Downloads directory path
```

---

## Colour Palette

All output uses true 24-bit RGB ANSI colours — no 8-colour or 256-colour fallback:

| Colour    | Hex       | Used for                          |
|-----------|-----------|-----------------------------------|
| Cyan      | `#4DD0E1` | Titles, headings, info labels     |
| Green     | `#66BB6A` | Regular files, success messages   |
| Yellow    | `#FFD54F` | Archive files, warnings           |
| Orange    | `#FFA726` | File sizes                        |
| Blue      | `#42A5F5` | Modification timestamps           |
| Magenta   | `#AB47BC` | Age strings                       |
| Teal      | `#26A69A` | Compressed sizes                  |
| Gray      | `#78909C` | Table borders, verbose timestamps |
| Red       | `#EF5350` | Errors                            |
| White     | `#ECEFF1` | Table headers                     |

---

## Emoji Key

| Emoji | Meaning            |
|-------|--------------------|
| 📥    | Downloads          |
| 📦    | Archive file       |
| 📄    | Generic file       |
| 📝    | Text / source code |
| 🖼️    | Image              |
| 🎬    | Video              |
| 🎵    | Audio              |
| 📋    | Document           |
| 📁    | Folder / output dir |
| ✅    | Success            |
| ❌    | Error              |
| ⚠️    | Warning            |
| ℹ️    | Info               |
| 🔧    | Extracting         |
| 🔍    | Inspecting         |
| 🆕    | New file           |
| 📌    | Existing file      |
| ⏭️    | Skipped            |

---

## Platform Support

| Platform | Downloads path                                     |
|----------|----------------------------------------------------|
| Linux    | `$XDG_DOWNLOAD_DIR` → `~/Downloads`               |
| macOS    | `~/Downloads`                                      |
| Windows  | `%USERPROFILE%\Downloads`                          |

---

## Changelog

### v1.1.1

- **Fixed:** `extract` had no way to select which files to pull out of an
  archive — the underlying plumbing existed (an unused parameter) but was
  never wired up. Added a real `-p/--pattern` glob filter, applied
  consistently to conflict detection, the extraction itself, and the
  ZIP/TAR/7z/RAR backends.
- **Fixed:** `list -z --pattern` silently ignored the pattern when combined
  with `-z`; archive-content filtering now works.
- **Fixed:** a "zip-slip" path-traversal issue — a crafted `.zip`/`.tar*`
  archive containing entries like `../../etc/passwd` could previously write
  outside the requested output directory. Entry paths are now sanitised
  before any file is written, and unsafe entries are skipped with a warning.
- **Fixed:** removed the unused `unrar` crate dependency (the tool already
  shells out to the `unrar` binary; the crate wasn't referenced anywhere in
  the code, and its C++ build step was a needless source of build failures)
  along with over a dozen other dependencies declared in `Cargo.toml` but
  never used (`colored`, `chrono`, `indicatif`, `console`, `dirs`, `anyhow`,
  `thiserror`, `serde`, `serde_json`, `regex`, `tempfile`, `glob`, `log`,
  `env_logger`). Build times should be noticeably faster.

## License

MIT © Hadi Cahyadi

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)