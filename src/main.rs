use clap::Parser;
use rusqlite::{Connection, Result as SqlResult};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kobo-highlights-exporter")]
#[command(about = "Export Kobo highlights and annotations to Markdown")]
struct Cli {
    /// Path to the KoboReader.sqlite file
    db_path: PathBuf,

    /// Output directory for Markdown files
    #[arg(short, long, default_value = "highlights")]
    output_dir: PathBuf,
}

struct Book {
    content_id: String,
    title: String,
    author: Option<String>,
}

struct TocEntry {
    title: String,
    /// ContentID with the trailing "-N" suffix stripped, used for matching bookmarks.
    match_id: String,
    /// TOC depth level extracted from the trailing "-N" suffix (1 = top, 2 = part, 3 = section, etc.)
    depth: u32,
}

struct Highlight {
    text: String,
    annotation: Option<String>,
    chapter_content_id: String,
    date_created: Option<String>,
}

fn query_books(conn: &Connection) -> SqlResult<Vec<Book>> {
    let mut stmt = conn.prepare(
        "SELECT ContentID, Title, Attribution
         FROM content
         WHERE BookID IS NULL AND ContentType = 6
         ORDER BY Title",
    )?;

    let books = stmt
        .query_map([], |row| {
            Ok(Book {
                content_id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;

    Ok(books)
}

/// Strip the trailing "-N" (digits) suffix from a ContentID.
/// E.g. "...xhtml#chapter01_4-2" → "...xhtml#chapter01_4"
///       "...Cover.xhtml-1"      → "...Cover.xhtml"
fn strip_suffix(content_id: &str) -> String {
    if let Some(pos) = content_id.rfind('-') {
        let after = &content_id[pos + 1..];
        if !after.is_empty() && after.chars().all(|c| c.is_ascii_digit()) {
            return content_id[..pos].to_string();
        }
    }
    content_id.to_string()
}

/// Extract the depth level from the trailing "-N" suffix of a ContentID.
/// E.g. "...xhtml#chapter01_4-2" → 2, "...Cover.xhtml-1" → 1
/// Returns 1 if no suffix is found (treat as top-level).
fn extract_depth(content_id: &str) -> u32 {
    if let Some(pos) = content_id.rfind('-') {
        let after = &content_id[pos + 1..];
        if !after.is_empty() && after.chars().all(|c| c.is_ascii_digit()) {
            return after.parse().unwrap_or(1);
        }
    }
    1
}

/// Fetch only ContentType=899 entries (the real TOC) ordered by VolumeIndex.
/// The trailing "-N" suffix on the ContentID encodes the TOC depth level.
fn query_toc(conn: &Connection, book_content_id: &str) -> SqlResult<Vec<TocEntry>> {
    let mut stmt = conn.prepare(
        "SELECT ContentID, Title
         FROM content
         WHERE BookID = ?1
           AND ContentType = 899
         ORDER BY VolumeIndex",
    )?;

    let entries: Vec<TocEntry> = stmt
        .query_map([book_content_id], |row| {
            let content_id: String = row.get(0)?;
            let title: String = row.get(1)?;
            Ok((content_id, title))
        })?
        .collect::<SqlResult<Vec<_>>>()?
        .into_iter()
        .map(|(content_id, title)| {
            let match_id = strip_suffix(&content_id);
            let depth = extract_depth(&content_id);
            TocEntry {
                title,
                match_id,
                depth,
            }
        })
        .collect();

    Ok(entries)
}

fn query_highlights(conn: &Connection, book_content_id: &str) -> SqlResult<Vec<Highlight>> {
    let mut stmt = conn.prepare(
        "SELECT Text, Annotation, ContentID, ChapterProgress, DateCreated
         FROM Bookmark
         WHERE VolumeID = ?1
           AND Text IS NOT NULL
           AND Text != ''
         ORDER BY ContentID, ChapterProgress",
    )?;

    let highlights = stmt
        .query_map([book_content_id], |row| {
            Ok(Highlight {
                text: row.get(0)?,
                annotation: row.get(1)?,
                chapter_content_id: row.get(2)?,
                date_created: row.get(4)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;

    Ok(highlights)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .trim()
        .to_string()
}

fn format_highlight(h: &Highlight) -> String {
    let mut out = String::new();

    for line in h.text.lines() {
        out.push_str(&format!("> {}\n", line));
    }

    if let Some(ref note) = h.annotation {
        if !note.is_empty() {
            out.push_str(&format!("\n**Note:** {}\n", note));
        }
    }

    if let Some(ref date) = h.date_created {
        out.push_str(&format!("\n*{date}*\n"));
    }

    out
}

/// Assign highlights to TOC entries.
///
/// Matching strategy: the bookmark's ContentID equals a TOC entry's match_id
/// (which is the 899 ContentID with the trailing "-N" suffix stripped).
///
/// Example:
///   Bookmark:  ...Chapter01.xhtml#chapter01_4
///   TOC entry: ...Chapter01.xhtml#chapter01_4-2  →  match_id: ...Chapter01.xhtml#chapter01_4
///   → MATCH
fn assign_highlights<'a>(
    toc: &[TocEntry],
    highlights: &'a [Highlight],
) -> (HashMap<usize, Vec<&'a Highlight>>, Vec<&'a Highlight>) {
    // Map from match_id → TOC entry index
    let mut match_index: HashMap<&str, usize> = HashMap::new();
    for (i, entry) in toc.iter().enumerate() {
        match_index.entry(&entry.match_id).or_insert(i);
    }

    let mut assigned: HashMap<usize, Vec<&'a Highlight>> = HashMap::new();
    let mut uncategorized: Vec<&'a Highlight> = Vec::new();

    for h in highlights {
        if let Some(&idx) = match_index.get(h.chapter_content_id.as_str()) {
            assigned.entry(idx).or_default().push(h);
        } else {
            uncategorized.push(h);
        }
    }

    (assigned, uncategorized)
}

fn generate_markdown(book: &Book, toc: &[TocEntry], highlights: &[Highlight]) -> String {
    let mut md = String::new();

    // Header
    md.push_str(&format!("# {}\n\n", book.title));
    if let Some(ref author) = book.author {
        if !author.is_empty() {
            md.push_str(&format!("**Author:** {author}\n\n"));
        }
    }
    md.push_str("---\n\n");

    let (assigned, uncategorized) = assign_highlights(toc, highlights);

    // Determine which ancestor headings need to be emitted.
    // For each TOC entry with highlights, mark all ancestor entries (entries at
    // shallower depth that precede it) as needed.
    let mut heading_needed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (i, _entry) in toc.iter().enumerate() {
        if assigned.contains_key(&i) {
            heading_needed.insert(i);
            // Walk backwards to find and mark all ancestor headings
            let current_depth = toc[i].depth;
            let mut need_depth = current_depth;
            for j in (0..i).rev() {
                if toc[j].depth < need_depth {
                    heading_needed.insert(j);
                    need_depth = toc[j].depth;
                    if need_depth <= 1 {
                        break;
                    }
                }
            }
        }
    }

    // Walk TOC in VolumeIndex order
    for (i, entry) in toc.iter().enumerate() {
        if !heading_needed.contains(&i) || entry.title.is_empty() {
            continue;
        }

        // depth 1 → ## (2 hashes), depth 2 → ### (3 hashes), etc.
        // # is reserved for the book title, so heading level = depth + 1
        let hashes = "#".repeat((entry.depth + 1) as usize);
        md.push_str(&format!("{hashes} {}\n\n", entry.title));

        if let Some(hl) = assigned.get(&i) {
            for h in hl {
                md.push_str(&format_highlight(h));
                md.push('\n');
            }
        }
    }

    if !uncategorized.is_empty() {
        md.push_str("## Uncategorized\n\n");
        for h in &uncategorized {
            md.push_str(&format_highlight(h));
            md.push('\n');
        }
    }

    md
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if !cli.db_path.exists() {
        eprintln!("Error: database file not found: {}", cli.db_path.display());
        std::process::exit(1);
    }

    let uri = format!("file:{}?immutable=1", cli.db_path.display());
    let conn = Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )?;

    let books = query_books(&conn)?;
    eprintln!("Found {} books in database", books.len());

    fs::create_dir_all(&cli.output_dir)?;

    let mut exported = 0;
    for book in &books {
        let highlights = query_highlights(&conn, &book.content_id)?;
        if highlights.is_empty() {
            continue;
        }

        let toc = query_toc(&conn, &book.content_id)?;
        let md = generate_markdown(book, &toc, &highlights);

        let filename = format!("{}.md", sanitize_filename(&book.title));
        let path = cli.output_dir.join(&filename);
        fs::write(&path, &md)?;

        eprintln!(
            "  Exported: {} ({} highlights)",
            book.title,
            highlights.len()
        );
        exported += 1;
    }

    eprintln!(
        "Done. Exported {} books to {}",
        exported,
        cli.output_dir.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- strip_suffix ---

    #[test]
    fn strip_suffix_removes_trailing_digits() {
        assert_eq!(
            strip_suffix("book.epub!OPS!xhtml/Chapter01.xhtml#chapter01_4-2"),
            "book.epub!OPS!xhtml/Chapter01.xhtml#chapter01_4"
        );
    }

    #[test]
    fn strip_suffix_removes_single_digit() {
        assert_eq!(
            strip_suffix("book.epub!OPS!xhtml/Cover.xhtml-1"),
            "book.epub!OPS!xhtml/Cover.xhtml"
        );
    }

    #[test]
    fn strip_suffix_no_suffix_unchanged() {
        assert_eq!(
            strip_suffix("book.epub!OPS!xhtml/Chapter01.xhtml#chapter01_4"),
            "book.epub!OPS!xhtml/Chapter01.xhtml#chapter01_4"
        );
    }

    #[test]
    fn strip_suffix_dash_not_followed_by_digits() {
        assert_eq!(
            strip_suffix("some-path/file.xhtml#section-abc"),
            "some-path/file.xhtml#section-abc"
        );
    }

    #[test]
    fn strip_suffix_empty_string() {
        assert_eq!(strip_suffix(""), "");
    }

    // --- extract_depth ---

    #[test]
    fn extract_depth_single_digit() {
        assert_eq!(extract_depth("book.epub!xhtml/Cover.xhtml-1"), 1);
    }

    #[test]
    fn extract_depth_multi_digit() {
        assert_eq!(extract_depth("book.epub!xhtml/Chapter01.xhtml#ch01_4-2"), 2);
    }

    #[test]
    fn extract_depth_deep_level() {
        assert_eq!(extract_depth("book.epub!Text/wahl.html#sigil_toc_id_6-4"), 4);
    }

    #[test]
    fn extract_depth_no_suffix_defaults_to_1() {
        assert_eq!(extract_depth("book.epub!xhtml/Chapter01.xhtml#ch01_4"), 1);
    }

    #[test]
    fn extract_depth_dash_not_digits() {
        assert_eq!(extract_depth("some-path/file.xhtml#section-abc"), 1);
    }

    // --- sanitize_filename ---

    #[test]
    fn sanitize_filename_keeps_alphanumeric_spaces_dashes() {
        assert_eq!(sanitize_filename("Hello World - 2024"), "Hello World - 2024");
    }

    #[test]
    fn sanitize_filename_removes_special_chars() {
        assert_eq!(sanitize_filename("Book: A «Story»!"), "Book A Story");
    }

    #[test]
    fn sanitize_filename_trims_whitespace() {
        assert_eq!(sanitize_filename("  Hello  "), "Hello");
    }

    // --- format_highlight ---

    #[test]
    fn format_highlight_text_only() {
        let h = Highlight {
            text: "Some highlighted text".into(),
            annotation: None,
            chapter_content_id: String::new(),
            date_created: None,
        };
        assert_eq!(format_highlight(&h), "> Some highlighted text\n");
    }

    #[test]
    fn format_highlight_with_annotation() {
        let h = Highlight {
            text: "Highlighted".into(),
            annotation: Some("My note".into()),
            chapter_content_id: String::new(),
            date_created: None,
        };
        let result = format_highlight(&h);
        assert!(result.contains("> Highlighted\n"));
        assert!(result.contains("**Note:** My note"));
    }

    #[test]
    fn format_highlight_with_date() {
        let h = Highlight {
            text: "Text".into(),
            annotation: None,
            chapter_content_id: String::new(),
            date_created: Some("2024-01-15T10:30:00".into()),
        };
        let result = format_highlight(&h);
        assert!(result.contains("*2024-01-15T10:30:00*"));
    }

    #[test]
    fn format_highlight_multiline_text() {
        let h = Highlight {
            text: "Line one\nLine two".into(),
            annotation: None,
            chapter_content_id: String::new(),
            date_created: None,
        };
        assert_eq!(format_highlight(&h), "> Line one\n> Line two\n");
    }

    #[test]
    fn format_highlight_empty_annotation_skipped() {
        let h = Highlight {
            text: "Text".into(),
            annotation: Some(String::new()),
            chapter_content_id: String::new(),
            date_created: None,
        };
        assert!(!format_highlight(&h).contains("**Note:**"));
    }

    // --- assign_highlights ---

    fn make_toc(entries: &[(&str, &str, u32)]) -> Vec<TocEntry> {
        entries
            .iter()
            .map(|(title, match_id, depth)| TocEntry {
                title: title.to_string(),
                match_id: match_id.to_string(),
                depth: *depth,
            })
            .collect()
    }

    fn make_highlight(text: &str, content_id: &str) -> Highlight {
        Highlight {
            text: text.into(),
            annotation: None,
            chapter_content_id: content_id.into(),
            date_created: None,
        }
    }

    #[test]
    fn assign_highlights_exact_match() {
        let toc = make_toc(&[
            ("Chapter I", "book!ch01.xhtml#ch01", 1),
            ("Section 1", "book!ch01.xhtml#ch01_1", 2),
        ]);
        let highlights = vec![make_highlight("hello", "book!ch01.xhtml#ch01_1")];

        let (assigned, uncategorized) = assign_highlights(&toc, &highlights);
        assert_eq!(assigned.get(&1).unwrap().len(), 1);
        assert!(uncategorized.is_empty());
    }

    #[test]
    fn assign_highlights_unmatched_goes_to_uncategorized() {
        let toc = make_toc(&[("Chapter I", "book!ch01.xhtml#ch01", 1)]);
        let highlights = vec![make_highlight("hello", "book!ch99.xhtml#unknown")];

        let (assigned, uncategorized) = assign_highlights(&toc, &highlights);
        assert!(assigned.is_empty());
        assert_eq!(uncategorized.len(), 1);
    }

    #[test]
    fn assign_highlights_multiple_to_same_section() {
        let toc = make_toc(&[("Section", "book!ch01.xhtml#sec1", 3)]);
        let highlights = vec![
            make_highlight("first", "book!ch01.xhtml#sec1"),
            make_highlight("second", "book!ch01.xhtml#sec1"),
        ];

        let (assigned, _) = assign_highlights(&toc, &highlights);
        assert_eq!(assigned.get(&0).unwrap().len(), 2);
    }

    // --- generate_markdown ---

    #[test]
    fn generate_markdown_basic_structure() {
        let book = Book {
            content_id: "book1".into(),
            title: "Test Book".into(),
            author: Some("Author Name".into()),
        };
        let toc = make_toc(&[
            ("Chapter I", "book!ch01.xhtml#ch01", 1),
            ("Section 1", "book!ch01.xhtml#sec1", 2),
        ]);
        let highlights = vec![make_highlight("Important text", "book!ch01.xhtml#sec1")];

        let md = generate_markdown(&book, &toc, &highlights);
        assert!(md.starts_with("# Test Book\n"));
        assert!(md.contains("**Author:** Author Name"));
        assert!(md.contains("## Chapter I\n"));
        assert!(md.contains("### Section 1\n"));
        assert!(md.contains("> Important text\n"));
    }

    #[test]
    fn generate_markdown_parent_chapter_emitted_for_subsection_highlights() {
        let toc = make_toc(&[
            ("KAPITEL I", "book!ch01.xhtml#ch01", 1),
            ("1. Abschnitt", "book!ch01.xhtml#ch01_1", 2),
            ("KAPITEL II", "book!ch02.xhtml#ch02", 1),
            ("1. Abschnitt", "book!ch02.xhtml#ch02_1", 2),
        ]);
        let book = Book {
            content_id: "b".into(),
            title: "T".into(),
            author: None,
        };
        // Only highlight in chapter I sub-section
        let highlights = vec![make_highlight("text", "book!ch01.xhtml#ch01_1")];

        let md = generate_markdown(&book, &toc, &highlights);
        // Chapter I heading must appear even though only sub-section has highlights
        assert!(md.contains("## KAPITEL I\n"));
        assert!(md.contains("### 1. Abschnitt\n"));
        // Chapter II should NOT appear (no highlights)
        assert!(!md.contains("## KAPITEL II"));
    }

    #[test]
    fn generate_markdown_multi_level_hierarchy() {
        let toc = make_toc(&[
            ("The Enchanted Forest", "book!forest.html#id_1", 1),
            ("I. The Crystal Cave", "book!forest.html#id_2", 2),
            ("1. The Hidden Door", "book!forest.html#id_3", 3),
            ("a) The Silver Key", "book!forest.html#id_4", 4),
            ("II. The Mountain Pass", "book!forest.html#id_5", 2),
        ]);
        let book = Book {
            content_id: "b".into(),
            title: "T".into(),
            author: None,
        };
        let highlights = vec![make_highlight("deep text", "book!forest.html#id_4")];

        let md = generate_markdown(&book, &toc, &highlights);
        // All ancestors should be emitted (depth+1 = heading level)
        assert!(md.contains("## The Enchanted Forest\n"));
        assert!(md.contains("### I. The Crystal Cave\n"));
        assert!(md.contains("#### 1. The Hidden Door\n"));
        assert!(md.contains("##### a) The Silver Key\n"));
        // Section II should NOT appear (no highlights)
        assert!(!md.contains("II. The Mountain Pass"));
    }

    #[test]
    fn generate_markdown_separate_files_hierarchy() {
        // Each entry in its own file, depth from suffix
        let toc = make_toc(&[
            ("Part One: The Dawn", "book!_1h_1.xhtml", 2),
            ("1. The Awakening", "book!_1h_2.xhtml", 3),
            ("2. The First Light", "book!_1h_3.xhtml", 3),
            ("Part Two: The Dusk", "book!_1h_7.xhtml", 2),
            ("1. The Fading Star", "book!_1h_8.xhtml", 3),
        ]);
        let book = Book {
            content_id: "b".into(),
            title: "T".into(),
            author: None,
        };
        let highlights = vec![make_highlight("text", "book!_1h_2.xhtml")];

        let md = generate_markdown(&book, &toc, &highlights);
        assert!(md.contains("### Part One: The Dawn\n"));
        assert!(md.contains("#### 1. The Awakening\n"));
        // Part Two should NOT appear
        assert!(!md.contains("Part Two: The Dusk"));
        // Other sections without highlights should not appear
        assert!(!md.contains("2. The First Light"));
    }

    #[test]
    fn generate_markdown_uncategorized_section() {
        let book = Book {
            content_id: "b".into(),
            title: "T".into(),
            author: None,
        };
        let toc = make_toc(&[("Ch", "book!ch01.xhtml#ch01", 1)]);
        let highlights = vec![make_highlight("orphan", "book!unknown.xhtml#x")];

        let md = generate_markdown(&book, &toc, &highlights);
        assert!(md.contains("## Uncategorized\n"));
        assert!(md.contains("> orphan\n"));
    }

    #[test]
    fn generate_markdown_no_uncategorized_when_all_matched() {
        let book = Book {
            content_id: "b".into(),
            title: "T".into(),
            author: None,
        };
        let toc = make_toc(&[("Ch", "book!ch01.xhtml#ch01", 1)]);
        let highlights = vec![make_highlight("matched", "book!ch01.xhtml#ch01")];

        let md = generate_markdown(&book, &toc, &highlights);
        assert!(!md.contains("Uncategorized"));
    }

    #[test]
    fn generate_markdown_highlight_with_annotation_and_date() {
        let book = Book {
            content_id: "b".into(),
            title: "T".into(),
            author: None,
        };
        let toc = make_toc(&[("Ch", "id", 1)]);
        let highlights = vec![Highlight {
            text: "highlighted".into(),
            annotation: Some("my note".into()),
            chapter_content_id: "id".into(),
            date_created: Some("2024-06-01".into()),
        }];

        let md = generate_markdown(&book, &toc, &highlights);
        assert!(md.contains("> highlighted\n"));
        assert!(md.contains("**Note:** my note"));
        assert!(md.contains("*2024-06-01*"));
    }

    // --- DB integration test with in-memory SQLite ---

    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE content (
                ContentID TEXT NOT NULL,
                ContentType TEXT NOT NULL,
                BookID TEXT,
                Title TEXT,
                Attribution TEXT,
                VolumeIndex INTEGER DEFAULT 0
            );
            CREATE TABLE Bookmark (
                BookmarkID TEXT NOT NULL,
                VolumeID TEXT NOT NULL,
                ContentID TEXT NOT NULL,
                Text TEXT,
                Annotation TEXT,
                DateCreated TEXT,
                ChapterProgress REAL DEFAULT 0,
                Hidden BOOL DEFAULT 0
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn db_query_books() {
        let conn = create_test_db();
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title, Attribution)
             VALUES ('book1', '6', NULL, 'Blue Lantern', 'Nora Finch')",
            [],
        )
        .unwrap();

        let books = query_books(&conn).unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Blue Lantern");
        assert_eq!(books[0].author.as_deref(), Some("Nora Finch"));
    }

    #[test]
    fn db_query_books_skips_chapters() {
        let conn = create_test_db();
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title)
             VALUES ('book1', '6', NULL, 'Book')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title)
             VALUES ('ch1', '9', 'book1', 'Chapter file')",
            [],
        )
        .unwrap();

        let books = query_books(&conn).unwrap();
        assert_eq!(books.len(), 1);
    }

    #[test]
    fn db_query_toc_hierarchy() {
        let conn = create_test_db();
        // Chapter file (type 9) — should be ignored
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title, VolumeIndex)
             VALUES ('book!Chapter01.xhtml', '9', 'book1', 'Chapter01.xhtml', 0)",
            [],
        )
        .unwrap();
        // Chapter heading (type 899, depth 2 = part level)
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title, VolumeIndex)
             VALUES ('book!Chapter01.xhtml#ch01-2', '899', 'book1', 'I. KAPITEL', 0)",
            [],
        )
        .unwrap();
        // Sub-section (type 899, depth 3 = section level)
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title, VolumeIndex)
             VALUES ('book!Chapter01.xhtml#ch01_1-3', '899', 'book1', '1. Section', 1)",
            [],
        )
        .unwrap();

        let toc = query_toc(&conn, "book1").unwrap();
        assert_eq!(toc.len(), 2);

        assert_eq!(toc[0].title, "I. KAPITEL");
        assert_eq!(toc[0].depth, 2);
        assert_eq!(toc[0].match_id, "book!Chapter01.xhtml#ch01");

        assert_eq!(toc[1].title, "1. Section");
        assert_eq!(toc[1].depth, 3);
        assert_eq!(toc[1].match_id, "book!Chapter01.xhtml#ch01_1");
    }

    #[test]
    fn db_query_highlights() {
        let conn = create_test_db();
        conn.execute(
            "INSERT INTO Bookmark (BookmarkID, VolumeID, ContentID, Text, Annotation, DateCreated, ChapterProgress)
             VALUES ('bm1', 'book1', 'book!ch01.xhtml#sec1', 'highlighted text', 'my note', '2024-01-15', 0.5)",
            [],
        )
        .unwrap();
        // Dogear bookmark (no text) — should be excluded
        conn.execute(
            "INSERT INTO Bookmark (BookmarkID, VolumeID, ContentID, Text, ChapterProgress)
             VALUES ('bm2', 'book1', 'book!ch01.xhtml#sec2', NULL, 0.8)",
            [],
        )
        .unwrap();

        let highlights = query_highlights(&conn, "book1").unwrap();
        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].text, "highlighted text");
        assert_eq!(highlights[0].annotation.as_deref(), Some("my note"));
    }

    #[test]
    fn db_end_to_end() {
        let conn = create_test_db();

        // Book
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title, Attribution)
             VALUES ('book1', '6', NULL, 'The Paper Orchard', 'Samir Hale')",
            [],
        )
        .unwrap();

        // TOC entries (899 only matter)
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title, VolumeIndex)
             VALUES ('book1!ch01.xhtml#ch01-1', '899', 'book1', 'I. Chapter Seven', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO content (ContentID, ContentType, BookID, Title, VolumeIndex)
             VALUES ('book1!ch01.xhtml#ch01_1-2', '899', 'book1', '1. Abschnitt', 1)",
            [],
        )
        .unwrap();

        // Highlight matching the sub-section
        conn.execute(
            "INSERT INTO Bookmark (BookmarkID, VolumeID, ContentID, Text, ChapterProgress)
             VALUES ('bm1', 'book1', 'book1!ch01.xhtml#ch01_1', 'A curious passage about seasons', 0.1)",
            [],
        )
        .unwrap();

        let books = query_books(&conn).unwrap();
        assert_eq!(books.len(), 1);

        let toc = query_toc(&conn, &books[0].content_id).unwrap();
        let highlights = query_highlights(&conn, &books[0].content_id).unwrap();
        let md = generate_markdown(&books[0], &toc, &highlights);

        assert!(md.contains("# The Paper Orchard\n"));
        assert!(md.contains("**Author:** Samir Hale"));
        assert!(md.contains("## I. Chapter Seven\n"));
        assert!(md.contains("### 1. Abschnitt\n"));
        assert!(md.contains("> A curious passage about seasons\n"));
        assert!(!md.contains("Uncategorized"));
    }
}
