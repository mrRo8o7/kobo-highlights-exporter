# kobo-highlights-exporter

A tool that exports highlights and annotations from a Kobo e-reader into well-structured Markdown files — one per book. Each file preserves the book's table of contents hierarchy, so your highlights appear under the correct chapter and section headings.

The generated Markdown files can be used with note-taking and knowledge management tools such as [Obsidian](https://obsidian.md), making it easy to search, link, and build on your reading notes.

## Features

- Exports all highlighted passages and annotations from your Kobo library
- Organizes highlights under their original chapter/section headings
- Includes personal annotations and highlight timestamps
- Produces one `.md` file per book, named after the book title
- Opens the database in read-only/immutable mode, so your Kobo data is never modified

## Installation

### Windows (installer)

Download `kobo-highlights-exporter-installer.exe` from the [Releases](https://github.com/mrRo8o7/kobo-highlights-exporter/releases) page and run it. The installer will set up the tool and add a right-click menu entry for exporting highlights directly from File Explorer (see [Windows right-click menu](#windows-right-click-menu) below). To remove the tool later, use **"Add or Remove Programs"** in Windows Settings.

### macOS / Linux

Download the archive that matches your system from the [Releases](https://github.com/mrRo8o7/kobo-highlights-exporter/releases) page:

| Operating system       | File to download                                              |
|------------------------|---------------------------------------------------------------|
| macOS (Apple Silicon)  | `kobo-highlights-exporter-aarch64-apple-darwin.tar.gz`        |
| macOS (Intel)          | `kobo-highlights-exporter-x86_64-apple-darwin.tar.gz`         |
| Linux                  | `kobo-highlights-exporter-x86_64-unknown-linux-gnu.tar.gz`    |

Extract the archive by opening a terminal and running:

```sh
tar xzf kobo-highlights-exporter-*.tar.gz
```

You can move the extracted file to any folder you like. Placing it somewhere in your `PATH` (e.g. `/usr/local/bin`) lets you run it from any directory.

### Build from source

If you prefer to compile the tool yourself, you need a working [Rust](https://www.rust-lang.org/tools/install) toolchain.

```sh
cargo build --release
```

The compiled binary will be at `target/release/kobo-highlights-exporter` (or `.exe` on Windows).

## Usage

1. Connect your Kobo e-reader to your computer via USB.
2. Find the highlights database on the device. It is located at:

   ```
   <Kobo drive>/.kobo/KoboReader.sqlite
   ```

   On Windows this will be something like `E:\.kobo\KoboReader.sqlite` (the drive letter depends on your system). On macOS look under `/Volumes/KOBOeReader/`.

3. Run the exporter and point it at the database file:

   ```sh
   kobo-highlights-exporter /path/to/KoboReader.sqlite
   ```

   By default, Markdown files are written to a `highlights/` folder next to the database. You can choose a different output folder with the `-o` flag:

   ```sh
   kobo-highlights-exporter /path/to/KoboReader.sqlite -o ~/my-highlights
   ```

## Windows right-click menu

If you installed using the Windows installer, a right-click menu entry is automatically set up. To use it:

1. Connect your Kobo e-reader via USB.
2. Open the Kobo drive in File Explorer and navigate to the `.kobo` folder (this is a hidden folder — you may need to enable "Show hidden files" in Explorer's **View** menu).
3. Right-click on `KoboReader.sqlite` and select **"Run Kobo Highlights Exporter"**.
4. Your highlights will be exported as Markdown files into a `highlights/` folder next to the database file.

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
