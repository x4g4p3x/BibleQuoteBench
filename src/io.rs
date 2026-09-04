//! Strict JSON, JSONL, and text file helpers with contextual diagnostics.

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use anyhow::{Context, Result, bail};
use serde::{Serialize, de::DeserializeOwned};

/// Reads one JSON document.
///
/// # Errors
///
/// Returns an error when the file cannot be opened or does not match `T`.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("parsing JSON from {}", path.display()))
}

/// Reads non-empty lines as independent JSON values.
///
/// # Errors
///
/// Returns an error with a line number when reading or deserialization fails.
pub fn read_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut records = Vec::new();

    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = index + 1;
        let line = line.with_context(|| format!("reading {}:{line_number}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str(&line)
            .with_context(|| format!("parsing {}:{line_number}", path.display()))?;
        records.push(record);
    }

    Ok(records)
}

/// Writes records as newline-delimited JSON to a file or standard output.
///
/// # Errors
///
/// Returns an error when the destination cannot be created, serialized, or written.
pub fn write_jsonl<T: Serialize>(path: Option<&Path>, records: &[T]) -> Result<()> {
    match path {
        Some(path) => {
            let file =
                File::create(path).with_context(|| format!("creating {}", path.display()))?;
            write_records(BufWriter::new(file), records)
        }
        None => write_records(BufWriter::new(std::io::stdout().lock()), records),
    }
}

/// Writes one pretty-printed JSON document.
///
/// # Errors
///
/// Returns an error when the destination cannot be created, serialized, or written.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value).context("serializing JSON document")?;
    writer.write_all(b"\n").context("writing JSON document")?;
    writer.flush().context("flushing JSON output")
}

/// Writes UTF-8 text to a file.
///
/// # Errors
///
/// Returns an error when the destination cannot be written.
pub fn write_text(path: &Path, value: &str) -> Result<()> {
    std::fs::write(path, value).with_context(|| format!("writing {}", path.display()))
}

fn write_records<T: Serialize, W: Write>(mut writer: W, records: &[T]) -> Result<()> {
    for record in records {
        serde_json::to_writer(&mut writer, record).context("serializing JSONL record")?;
        writer.write_all(b"\n").context("writing JSONL record")?;
    }
    writer.flush().context("flushing JSONL output")
}

/// Rejects an empty collection with a path-aware diagnostic.
///
/// # Errors
///
/// Returns an error when `records` is empty.
pub fn ensure_nonempty<T>(records: &[T], kind: &str, path: &Path) -> Result<()> {
    if records.is_empty() {
        bail!("{} contains no {kind}", path.display());
    }
    Ok(())
}
