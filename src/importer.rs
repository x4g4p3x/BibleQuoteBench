//! Provenance-preserving import of canonical Bible books from USFM archives.

use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::Path,
    sync::LazyLock,
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::{BibleReference, CorpusLock, ReferenceRecord, SourceArtifact, TranslationSpec};

static FOOTNOTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\\f\s.*?\\f\*").expect("valid footnote regex"));
static CROSS_REFERENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\\x\s.*?\\x\*").expect("valid cross-reference regex"));
static WORD_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\\+?w\s+([^|\\]+)(?:\|[^\\]*)?\\\+?w\*").expect("valid word-marker regex")
});
static NOTE_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\\(?:fe|ef|ex)\s.*?\\(?:fe|ef|ex)\*").expect("valid note regex")
});
static MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\\+?[A-Za-z][A-Za-z0-9+\-]*\*?(?:\s+)?").expect("valid marker regex")
});
static VERSE_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\v\s+([0-9]+(?:-[0-9]+)?[a-z]?)\s*").expect("valid verse-marker regex")
});
static SPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{Z}\t\n]+").expect("valid whitespace regex"));

#[derive(Debug)]
pub struct ImportedCorpus {
    pub records: Vec<ReferenceRecord>,
    pub lock: CorpusLock,
}

/// Imports a USFM directory or zip archive and records byte-level provenance.
///
/// # Errors
///
/// Returns an error if the source cannot be read, contains malformed USFM, or
/// yields no canonical verse records.
pub fn import_usfm(
    source: &Path,
    translation: &TranslationSpec,
    source_url: &str,
) -> Result<ImportedCorpus> {
    let source_bytes = fs::read(source).ok();
    let mut files = if source.is_dir() {
        read_usfm_directory(source)?
    } else {
        read_usfm_zip(source)?
    };
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if files.is_empty() {
        bail!("{} contains no .usfm or .sfm files", source.display());
    }

    let source_sha256 = source_bytes
        .as_deref()
        .map_or_else(|| digest_file_set(&files), sha256_hex);
    let artifacts = files
        .iter()
        .map(|(path, bytes)| SourceArtifact {
            path: path.clone(),
            sha256: sha256_hex(bytes),
            bytes: bytes.len() as u64,
        })
        .collect();

    let mut records = Vec::new();
    for (path, bytes) in &files {
        let text = String::from_utf8(bytes.clone())
            .with_context(|| format!("{path} is not valid UTF-8 USFM"))?;
        records.extend(parse_usfm(&text, &translation.id).with_context(|| path.clone())?);
    }
    records.sort_by(|left, right| {
        reference_sort_key(&left.reference).cmp(&reference_sort_key(&right.reference))
    });
    records.dedup_by(|left, right| {
        left.translation == right.translation && left.reference == right.reference
    });
    if records.is_empty() {
        bail!("{} yielded no verse records", source.display());
    }
    let corpus_sha256 = digest_records(&records)?;

    Ok(ImportedCorpus {
        lock: CorpusLock {
            schema_version: 1,
            translation: translation.id.clone(),
            edition: translation.edition.clone(),
            source_url: source_url.to_owned(),
            source_sha256,
            importer_version: env!("CARGO_PKG_VERSION").to_owned(),
            artifacts,
            reference_count: records.len(),
            corpus_sha256,
        },
        records,
    })
}

fn read_usfm_zip(path: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut archive =
        ZipArchive::new(file).with_context(|| format!("reading {}", path.display()))?;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).context("reading zip entry")?;
        let name = entry.name().replace('\\', "/");
        if entry.is_dir() || !is_usfm_path(Path::new(&name)) {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .with_context(|| name.clone())?;
        files.push((name, bytes));
    }
    Ok(files)
}

fn read_usfm_directory(root: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut pending = vec![root.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in
            fs::read_dir(&directory).with_context(|| format!("reading {}", directory.display()))?
        {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if is_usfm_path(&path) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .expect("walked paths remain below root")
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            Ok((relative, bytes))
        })
        .collect()
}

fn is_usfm_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "usfm" | "sfm"))
}

fn parse_usfm(input: &str, translation: &str) -> Result<Vec<ReferenceRecord>> {
    let mut book = None;
    let mut chapter = 0_u16;
    let mut active: Option<(u16, Option<u16>, String)> = None;
    let mut records = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim_start_matches('\u{feff}').trim();
        if let Some(rest) = line.strip_prefix("\\id ") {
            let code = rest
                .split_whitespace()
                .next()
                .context("missing \\id code")?;
            book = book_name(code);
        } else if let Some(rest) = line.strip_prefix("\\c ") {
            flush_verse(&mut active, &mut records, translation, book, chapter)?;
            chapter = rest
                .split_whitespace()
                .next()
                .context("missing chapter number")?
                .parse()
                .context("invalid chapter number")?;
        } else {
            let markers: Vec<_> = VERSE_MARKER.captures_iter(line).collect();
            if markers.is_empty() {
                if let Some((_, _, text)) = &mut active {
                    if is_verse_continuation(line) {
                        text.push(' ');
                        text.push_str(line);
                    }
                }
                continue;
            }

            if let Some((_, _, text)) = &mut active {
                let prefix = &line[..markers[0].get(0).expect("whole match").start()];
                if !clean_usfm_text(prefix).is_empty() {
                    text.push(' ');
                    text.push_str(prefix);
                }
            }
            for (index, captures) in markers.iter().enumerate() {
                flush_verse(&mut active, &mut records, translation, book, chapter)?;
                let number = captures.get(1).expect("verse capture").as_str();
                let text_start = captures.get(0).expect("whole match").end();
                let text_end = markers
                    .get(index + 1)
                    .map_or(line.len(), |next| next.get(0).expect("whole match").start());
                let (start, end) = parse_verse_number(number)?;
                active = Some((start, end, line[text_start..text_end].to_owned()));
            }
        }
    }
    flush_verse(&mut active, &mut records, translation, book, chapter)?;
    Ok(records)
}

fn flush_verse(
    active: &mut Option<(u16, Option<u16>, String)>,
    records: &mut Vec<ReferenceRecord>,
    translation: &str,
    book: Option<&'static str>,
    chapter: u16,
) -> Result<()> {
    let Some((verse_start, verse_end, raw_text)) = active.take() else {
        return Ok(());
    };
    let Some(book) = book else {
        return Ok(());
    };
    if chapter == 0 {
        bail!("verse appeared before \\c");
    }
    let text = clean_usfm_text(&raw_text);
    if !text.is_empty() {
        records.push(ReferenceRecord {
            translation: translation.to_owned(),
            reference: BibleReference {
                book: book.to_owned(),
                chapter,
                verse_start,
                verse_end,
            },
            text,
        });
    }
    Ok(())
}

fn parse_verse_number(value: &str) -> Result<(u16, Option<u16>)> {
    let numeric = |part: &str| {
        part.trim_end_matches(|character: char| character.is_ascii_alphabetic())
            .parse::<u16>()
            .with_context(|| format!("invalid verse number {value}"))
    };
    if let Some((start, end)) = value.split_once('-') {
        Ok((numeric(start)?, Some(numeric(end)?)))
    } else {
        Ok((numeric(value)?, None))
    }
}

fn is_verse_continuation(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let marker = line
        .strip_prefix('\\')
        .and_then(|rest| rest.split_whitespace().next());
    !matches!(
        marker,
        Some(
            "id" | "ide"
                | "h"
                | "toc1"
                | "toc2"
                | "toc3"
                | "mt"
                | "mt1"
                | "mt2"
                | "s"
                | "s1"
                | "s2"
                | "s3"
                | "r"
                | "d"
                | "cl"
                | "rem"
        )
    )
}

fn clean_usfm_text(raw: &str) -> String {
    let without_notes = FOOTNOTE.replace_all(raw, " ");
    let without_cross_references = CROSS_REFERENCE.replace_all(&without_notes, " ");
    let without_extended_notes = NOTE_MARKER.replace_all(&without_cross_references, " ");
    let with_words = WORD_MARKER.replace_all(&without_extended_notes, "$1");
    let without_markers = MARKER.replace_all(&with_words, " ");
    SPACE
        .replace_all(without_markers.trim(), " ")
        .trim()
        .to_owned()
}

fn digest_file_set(files: &[(String, Vec<u8>)]) -> String {
    let mut digest = Sha256::new();
    for (path, bytes) in files {
        digest.update(path.as_bytes());
        digest.update([0]);
        digest.update(bytes);
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}

fn digest_records(records: &[ReferenceRecord]) -> Result<String> {
    let mut buffer = Cursor::new(Vec::new());
    for record in records {
        serde_json::to_writer(&mut buffer, record)?;
        buffer.get_mut().push(b'\n');
    }
    Ok(sha256_hex(buffer.get_ref()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn reference_sort_key(reference: &BibleReference) -> (usize, u16, u16, u16) {
    (
        book_order(&reference.book),
        reference.chapter,
        reference.verse_start,
        reference.end_verse(),
    )
}

fn book_order(book: &str) -> usize {
    BOOKS
        .iter()
        .position(|(_, name)| *name == book)
        .unwrap_or(usize::MAX)
}

fn book_name(code: &str) -> Option<&'static str> {
    let code = code.trim().to_ascii_uppercase();
    BOOKS
        .iter()
        .find_map(|(candidate, name)| (*candidate == code).then_some(*name))
}

const BOOKS: [(&str, &str); 66] = [
    ("GEN", "Genesis"),
    ("EXO", "Exodus"),
    ("LEV", "Leviticus"),
    ("NUM", "Numbers"),
    ("DEU", "Deuteronomy"),
    ("JOS", "Joshua"),
    ("JDG", "Judges"),
    ("RUT", "Ruth"),
    ("1SA", "1 Samuel"),
    ("2SA", "2 Samuel"),
    ("1KI", "1 Kings"),
    ("2KI", "2 Kings"),
    ("1CH", "1 Chronicles"),
    ("2CH", "2 Chronicles"),
    ("EZR", "Ezra"),
    ("NEH", "Nehemiah"),
    ("EST", "Esther"),
    ("JOB", "Job"),
    ("PSA", "Psalms"),
    ("PRO", "Proverbs"),
    ("ECC", "Ecclesiastes"),
    ("SNG", "Song of Solomon"),
    ("ISA", "Isaiah"),
    ("JER", "Jeremiah"),
    ("LAM", "Lamentations"),
    ("EZK", "Ezekiel"),
    ("DAN", "Daniel"),
    ("HOS", "Hosea"),
    ("JOL", "Joel"),
    ("AMO", "Amos"),
    ("OBA", "Obadiah"),
    ("JON", "Jonah"),
    ("MIC", "Micah"),
    ("NAM", "Nahum"),
    ("HAB", "Habakkuk"),
    ("ZEP", "Zephaniah"),
    ("HAG", "Haggai"),
    ("ZEC", "Zechariah"),
    ("MAL", "Malachi"),
    ("MAT", "Matthew"),
    ("MRK", "Mark"),
    ("LUK", "Luke"),
    ("JHN", "John"),
    ("ACT", "Acts"),
    ("ROM", "Romans"),
    ("1CO", "1 Corinthians"),
    ("2CO", "2 Corinthians"),
    ("GAL", "Galatians"),
    ("EPH", "Ephesians"),
    ("PHP", "Philippians"),
    ("COL", "Colossians"),
    ("1TH", "1 Thessalonians"),
    ("2TH", "2 Thessalonians"),
    ("1TI", "1 Timothy"),
    ("2TI", "2 Timothy"),
    ("TIT", "Titus"),
    ("PHM", "Philemon"),
    ("HEB", "Hebrews"),
    ("JAS", "James"),
    ("1PE", "1 Peter"),
    ("2PE", "2 Peter"),
    ("1JN", "1 John"),
    ("2JN", "2 John"),
    ("3JN", "3 John"),
    ("JUD", "Jude"),
    ("REV", "Revelation"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_cleans_usfm() {
        let input = "\\id JHN\n\\c 3\n\\p\n\\v 16 For God \\wj so loved\\wj* the world. \\f + \\ft A note.\\f*\n";
        let records = parse_usfm(input, "test").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].reference.to_string(), "John 3:16");
        assert_eq!(records[0].text, "For God so loved the world.");
    }

    #[test]
    fn parses_verse_ranges() {
        assert_eq!(parse_verse_number("3-4").unwrap(), (3, Some(4)));
        assert_eq!(parse_verse_number("5a").unwrap(), (5, None));
    }
}
