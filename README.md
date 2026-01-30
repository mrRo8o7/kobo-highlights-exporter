# kobo-highlights-exporter

A command-line tool that exports highlights and annotations from a Kobo e-reader's SQLite database into well-structured Markdown files — one per book. Each file preserves the book's table of contents hierarchy, so your highlights appear under the correct chapter and section headings.

The generated Markdown files can be used with note-taking and knowledge management tools such as [Obsidian](https://obsidian.md), making it easy to search, link, and build on your reading notes.

## Features

- Exports all highlighted passages and annotations from your Kobo library
- Organizes highlights under their original chapter/section headings
- Includes personal annotations and highlight timestamps
- Produces one `.md` file per book, named after the book title
- Opens the database in read-only/immutable mode, so your Kobo data is never modified

## Building

You need a working [Rust](https://www.rust-lang.org/tools/install) toolchain.

```sh
cargo build --release
```

The compiled binary will be at `target/release/kobo-highlights-exporter` (or `.exe` on Windows).

## Usage

Connect your Kobo e-reader via USB. The highlights database is located at:

```
<Kobo mount point>/.kobo/KoboReader.sqlite
```

Run the exporter, pointing it at the database file:

```sh
kobo-highlights-exporter /path/to/KoboReader.sqlite
```

By default, Markdown files are written to a `highlights/` directory. You can specify a different output directory with the `-o` flag:

```sh
kobo-highlights-exporter /path/to/KoboReader.sqlite -o ~/my-highlights
```

## Windows context menu integration

On Windows, you can add a right-click context menu entry for `.sqlite` files so you can run the exporter directly from Explorer.

1. Copy the compiled binary (`target\release\kobo-highlights-exporter.exe`) and the icon file (`kobo-highlights-icon.ico`) to `%USERPROFILE%\kobo-highlights-exporter\`.
2. Run the provided `install-kobo-context-menu.bat` script. This creates a registry entry under `HKEY_CURRENT_USER` that adds a **"Run Kobo Highlights Exporter"** option when you right-click any `.sqlite` file.
3. After installation, right-click your `KoboReader.sqlite` file and select **"Run Kobo Highlights Exporter"** to export your highlights.

To uninstall the context menu entry, delete the registry key `HKEY_CURRENT_USER\Software\Classes\SystemFileAssociations\.sqlite\shell\RunKoboHighlights` using `regedit`.

## Output format

Each book produces a Markdown file with the following structure:

```markdown
# Book Title

**Author:** Author Name

---

## Chapter Heading

### Section

> Your highlighted text

**Note:** Your annotation

*2024-01-15T10:30:00*
```

Only chapters and sections that contain highlights are included. Highlights that cannot be matched to a table of contents entry appear under an **Uncategorized** section at the end.