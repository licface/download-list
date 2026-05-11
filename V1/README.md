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
```

---

### `dwn extract`

```
dwn extract [OPTIONS]

Options:
  -f, --file <FILE>     Archive to extract (default: newest archive)
  -o, --output <DIR>    Output directory (default: current dir)
  -w, --overwrite       Overwrite all existing files without asking
  -v, --verbose         Show per-file progress and timestamps
```

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

## License

MIT © Hadi Cahyadi
