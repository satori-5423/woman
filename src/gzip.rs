use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::Read;
use std::io::Write;
use std::path::Path;

/// Read a man page file, decompressing it if it is gzip-compressed.
/// Falls back to reading as plain text when the file is not a valid gzip stream
/// (e.g., fish shell's uncompressed man pages).
pub fn read_gzip_file(path: &Path) -> Result<String, String> {
    let mut bytes = Vec::new();

    // Try gzip first.
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;
    let mut decoder = GzDecoder::new(file);
    match decoder.read_to_end(&mut bytes) {
        Ok(_) => {
            // Valid gzip stream — even a decompressed-empty one is honored, so
            // a gzip file can never be misread as raw binary fallback.
            return Ok(String::from_utf8_lossy(&bytes).into_owned());
        }
        Err(_) => {
            // Not a valid gzip stream — fall through to plain-text fallback.
            drop(decoder);
        }
    }

    // Plain text fallback.
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;
    bytes.clear();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Compress a string as gzip and write it to the given path.
/// Creates parent directories as needed.
pub fn write_gzip_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create cache parent directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let file = std::fs::File::create(path)
        .map_err(|e| format!("Failed to create file {}: {}", path.display(), e))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to compress and write content: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to finalize gzip file: {}", e))?;
    Ok(())
}
