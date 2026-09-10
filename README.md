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
  -p, --pattern <PAT>   Filter (default: *) — see matching rules below
  -z, --list-archive    Inspect archive contents (like `7z l`)
  -f, --file <FILE>     Archive to inspect (used with -z)
  -v, --verbose         Verbose output
      --asc             Sort oldest-first (default)
      --desc            Sort newest-first
```

**Sort order:** oldest-first (`--asc`) is the default. Pass `--desc` for
newest-first.

**Pattern matching:** if `--pattern` contains no `*`/`?`, it's treated as a
substring search — `-p xmap` matches `xmaps_with_bmkg.zip` anywhere the text
`xmap` appears, not just a file literally named `xmap`. Once you include a
wildcard, it becomes a real glob and must match the whole name — `-p "*.zip"`
matches anything ending in `.zip`.

**Examples:**
```bash
dwn list                      # List all files, oldest first
dwn list --desc               # List all files, newest first
dwn list --last               # Show only the newest file
dwn list --pattern xmap       # Substring match: anything containing "xmap"
dwn list --pattern "*.zip"    # Glob match: only ZIP files
dwn list -z                   # Inspect the newest archive
dwn list -z -f myfile.tar.gz  # Inspect a specific archive
dwn list -z -p png            # Inspect contents, filtered to entries containing "png"
dwn list -z -p xmap           # No -f given: picks the newest archive named *xmap*
                               # and shows its full contents (see below)
```

**Archive auto-selection with `-z` (and `extract`):** if you don't pass `-f`,
`--pattern` is first tried against **archive filenames** in Downloads — the
newest archive whose name matches is opened. Only if no archive filename
matches does the pattern fall back to filtering the *contents* of the
newest archive overall (the original behavior). This means `-p` does double
duty depending on whether `-f` is given:

- No `-f`: `-p` picks *which archive* to open (falls back to content
  filtering only if no archive name matches).
- With `-f`: `-p` filters the *contents* of that specific archive, as
  documented above — filename matching doesn't apply since you've already
  named the file.

If an archive was selected by filename match and none of its internal
entries also happen to match the same pattern, the full listing (or, for
`extract`, the full contents) is shown/extracted rather than erroring —
the pattern already did its job picking the archive.

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
and its basename, so both `-p png` (substring) and `-p "assets/*"` (glob)
work as expected — see the matching rules under `dwn list` above; they're
identical here. Conflict detection (existing vs. new files) and the
interactive overwrite prompt are both computed only over the entries that
match the pattern. See "Archive auto-selection" above — the same rule
applies here when `-f` is omitted.

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

### v1.0.8

- **Fixed (critical):** extraction of an entire archive would abort with a
  raw OS error and extract **zero files** if even one entry's name used a
  character that's illegal on Windows (`< > : " | ? *`) — legal on the
  Linux/macOS system that built the archive, but not on Windows. Some tools
  even embed synthetic debug-only entries like Python's `<frozen
  importlib>` that trigger this. Two independent fixes:
  - Path components are now sanitised on Windows (illegal characters
    replaced, trailing dots/spaces trimmed) so these entries extract
    successfully instead of failing outright.
  - Belt and suspenders: a single entry that still fails to write for any
    reason (odd permissions, a path that's too long, etc.) is now skipped
    with a warning instead of aborting extraction of the rest of the
    archive. Previously one bad entry meant *nothing* got extracted, even
    when the other 99% of the archive was perfectly fine.

### v1.0.7

- **Fixed (critical):** `extract -p PATTERN` without `-f`, when the pattern
  matched an archive's *filename* but nothing inside it, correctly listed
  and counted "extracting all N entries" — then extracted **zero files**.
  The v1.4.0 archive-selection fallback (above) only updated the `entries`
  list used for display/counting; it never told `extract_zip`/`extract_tar`/
  `extract_7z`/`extract_rar` to stop re-applying the original pattern as a
  second, independent content filter. Those functions don't see `entries` —
  they take the pattern string and filter the archive again themselves, so
  `-p vmmon` (matched by filename) still filtered every internal file by
  "contains vmmon" and matched none of them. Fixed by forwarding `"*"` to
  the actual extraction step whenever the fallback was used, instead of the
  original pattern.

### v1.0.6

- **Fixed:** `list -z -p PATTERN` (and `extract -p PATTERN`) without `-f`
  always opened the newest archive **overall**, then searched *inside* it
  for the pattern — so `dwn list -z -p xmap` would fail with "No entries
  match pattern" unless the newest archive in Downloads happened to be about
  xmap, even when an archive literally named `xmaps_with_bmkg.zip` existed.
  Auto-selection now tries the pattern against archive **filenames** first
  and opens the newest match; it only falls back to "newest archive overall,
  filter its contents" if no archive filename matches. If an archive is
  selected by filename and none of its internal entries also match, the
  whole archive is shown/extracted rather than erroring, since the pattern
  already did its job picking the file.

### v1.0.5

Full self-audit of every command after two rounds of bugs slipping through
one at a time — these four did not need a bug report to find:

- **Fixed:** `is_tty()` was hard-coded to `false` on Windows, so the
  interactive "overwrite existing files? [y/N]" prompt in `extract` could
  never appear on Windows even in a real terminal — it silently did nothing.
  Now uses the C runtime's `_isatty` on Windows, same approach `isatty` uses
  on Unix.
- **Fixed:** `list -z` could panic on archive entries with non-ASCII names
  over 64 characters — the long-name truncation sliced by raw byte offset,
  which can land mid-character on multi-byte UTF-8 and crash. Now truncates
  by character count.
- **Fixed:** `.7z` archives — `list -z`, and `extract`'s conflict detection
  and `-p` pattern filter — only ever saw the *last* file in the archive,
  plus a bogus extra "entry" for the archive itself. `7z l -slt` output has
  exactly one `----------` separator in total (between the archive header
  and the file list), not one per file; the parser treated every occurrence
  as an entry boundary, so it never advanced past entry #1 in the header and
  entry #2 at EOF. Rewritten to skip the single header separator and then
  read the actual blank-line-delimited per-file blocks.
- **Fixed:** `.rar` archives — `unrar l`'s columns are `Attributes Size Date
  Time Name`, with Date and Time as two separate tokens. The parser started
  the filename at the Time column instead of Name, so every parsed name came
  out prefixed with a bogus time (`"12:00 realfile.txt"`). This silently
  broke `-p` pattern filtering and existing/new detection for `.rar` files
  (actual extraction via the real `unrar` binary was unaffected, since it
  doesn't go through this parser).

### v1.0.3

- **Fixed:** `--pattern` (on `list`, `list -z`, and `extract`) demanded an
  exact, anchored glob match — `-p xmap` returned "No files found" even
  though files like `xmaps_with_bmkg.zip` existed, because `xmap` has no
  `*`/`?` and so had to equal the *entire* filename. Patterns with no
  wildcard are now treated as a substring search (`*xmap*`); patterns that
  already contain `*`/`?` are left alone and still do a real anchored glob
  match.
- **Changed:** `list`'s default sort order is now oldest-first (`--asc`,
  matching the flag's own description, which already said "this is the
  default" while the code did the opposite). Pass `--desc` for newest-first.

### v1.0.2

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
