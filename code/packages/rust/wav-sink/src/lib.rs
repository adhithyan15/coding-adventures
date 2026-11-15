//! # WAV Sink (MUS01 Stage 8)
//!
//! The only stage of the note-to-sound chain that touches the file system:
//!
//! ```text
//! ... -> PCM audio -> WAV bytes -> file
//! ```
//!
//! [`write_wav`] and [`read_wav`] move [`PcmAudio`] buffers to and from
//! RIFF/WAVE files using the `pcm-audio` codec, behind hardened path and size
//! checks.
//!
//! ## Security
//!
//! **CWE-22 (path traversal)** - [`validate_wav_path`] rejects:
//!
//! - empty paths and paths containing NUL bytes,
//! - any `..` component,
//! - extensions other than `.wav` / `.wave` (case-insensitive),
//! - symbolic links, via `symlink_metadata`, on the target and on every
//!   existing ancestor (below `base_dir` when one is given; the caller-supplied
//!   `base_dir` itself is trusted but must be a directory),
//! - targets that exist but are not regular files,
//! - when `base_dir` is given: any path that does not lexically live inside it.
//!   Relative paths are interpreted relative to `base_dir`.
//!
//! Files are additionally re-checked after opening with `metadata().is_file()`
//! so a path swapped for a directory or device between validation and open is
//! refused.
//!
//! **CWE-400 (resource exhaustion)** - reads never buffer more than
//! [`MAX_WAV_BYTES`] (64 MiB): the file length is checked up front and the read
//! itself is bounded with `take(MAX_WAV_BYTES + 1)`. Writes refuse to emit
//! output larger than the same limit.
//!
//! Without a `base_dir`, every existing ancestor of the path must be a
//! non-symlink, so prefer passing a `base_dir` for paths under e.g. `/tmp` on
//! platforms where that is itself a symlink.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use pcm_audio::{PcmAudio, PcmError};

/// Maximum WAV file size read or written by this crate: 64 MiB (CWE-400).
pub const MAX_WAV_BYTES: usize = pcm_audio::MAX_WAV_BYTES;

/// Accepted WAV file extensions (compared case-insensitively).
pub const WAV_EXTENSIONS: [&str; 2] = ["wav", "wave"];

/// Errors produced by the WAV sink.
#[derive(Debug, Clone, PartialEq)]
pub enum WavSinkError {
    /// The path failed a CWE-22 validation rule.
    InvalidPath(String),
    /// An I/O operation failed.
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
        message: String,
    },
    /// The WAV data would exceed (or does exceed) [`MAX_WAV_BYTES`].
    OversizedWav { size: usize, max: usize },
    /// The PCM codec rejected the audio or the WAV bytes.
    Pcm(PcmError),
}

impl fmt::Display for WavSinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(reason) => write!(f, "invalid WAV path: {reason}"),
            Self::Io {
                operation, message, ..
            } => write!(f, "I/O error during {operation}: {message}"),
            Self::OversizedWav { size, max } => {
                write!(f, "WAV data of {size} bytes exceeds the {max}-byte limit")
            }
            Self::Pcm(error) => write!(f, "PCM codec error: {error}"),
        }
    }
}

impl Error for WavSinkError {}

impl From<PcmError> for WavSinkError {
    fn from(error: PcmError) -> Self {
        Self::Pcm(error)
    }
}

fn io_error(operation: &'static str, error: &io::Error) -> WavSinkError {
    WavSinkError::Io {
        operation,
        kind: error.kind(),
        message: error.to_string(),
    }
}

fn invalid(reason: &str) -> WavSinkError {
    WavSinkError::InvalidPath(reason.to_string())
}

/// Reject `path` if it exists as a symbolic link. Missing paths are fine.
fn reject_symlink(path: &Path) -> Result<(), WavSinkError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(invalid("symbolic links are not allowed")),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("symlink_metadata", &error)),
    }
}

/// Validate `path` as a safe WAV destination or source (CWE-22) and return the
/// path that should be opened.
///
/// See the [crate documentation](crate) for the exact rules. `base_dir`, when
/// provided, must be an existing directory; relative `path`s are resolved
/// against it and the result must remain inside it.
pub fn validate_wav_path(path: &Path, base_dir: Option<&Path>) -> Result<PathBuf, WavSinkError> {
    let text = path.as_os_str().to_string_lossy();
    if text.is_empty() {
        return Err(invalid("path is empty"));
    }
    if text.contains('\0') {
        return Err(invalid("path contains a NUL byte"));
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(invalid("path contains a '..' component"));
    }
    let extension_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| WAV_EXTENSIONS.iter().any(|w| e.eq_ignore_ascii_case(w)))
        .unwrap_or(false);
    if !extension_ok {
        return Err(invalid("path must have a .wav or .wave extension"));
    }
    if path.file_name().is_none() {
        return Err(invalid("path has no file name"));
    }

    let resolved = match base_dir {
        Some(base) => {
            let base_text = base.as_os_str().to_string_lossy();
            if base_text.is_empty() || base_text.contains('\0') {
                return Err(invalid("base_dir is empty or contains a NUL byte"));
            }
            if base.components().any(|c| matches!(c, Component::ParentDir)) {
                return Err(invalid("base_dir contains a '..' component"));
            }
            let base_abs = if base.is_absolute() {
                base.to_path_buf()
            } else {
                std::env::current_dir()
                    .map_err(|e| io_error("current_dir", &e))?
                    .join(base)
            };
            match fs::metadata(&base_abs) {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => return Err(invalid("base_dir is not a directory")),
                Err(error) => return Err(io_error("base_dir metadata", &error)),
            }
            let candidate = if path.is_absolute() {
                path.to_path_buf()
            } else {
                base_abs.join(path)
            };
            let relative = candidate
                .strip_prefix(&base_abs)
                .map_err(|_| invalid("path escapes base_dir"))?
                .to_path_buf();
            // Check every component below base_dir, target included.
            let mut walked = base_abs;
            for component in relative.components() {
                match component {
                    Component::Normal(part) => walked.push(part),
                    _ => return Err(invalid("path escapes base_dir")),
                }
                reject_symlink(&walked)?;
            }
            candidate
        }
        None => {
            // No trusted root: every existing ancestor and the target must be
            // a non-symlink.
            let mut ancestors: Vec<&Path> = path
                .ancestors()
                .filter(|a| !a.as_os_str().is_empty())
                .collect();
            ancestors.reverse();
            for ancestor in ancestors {
                reject_symlink(ancestor)?;
            }
            path.to_path_buf()
        }
    };

    match fs::symlink_metadata(&resolved) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err(invalid("target exists but is not a regular file")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("symlink_metadata", &error)),
    }
    Ok(resolved)
}

/// Serialize `audio` into canonical RIFF/WAVE bytes (no file access), refusing
/// output larger than [`MAX_WAV_BYTES`].
pub fn to_wav_bytes(audio: &PcmAudio) -> Result<Vec<u8>, WavSinkError> {
    let bytes = audio.to_wav_bytes()?;
    if bytes.len() > MAX_WAV_BYTES {
        return Err(WavSinkError::OversizedWav {
            size: bytes.len(),
            max: MAX_WAV_BYTES,
        });
    }
    Ok(bytes)
}

/// Write `audio` as a WAV file at `path` and return the path actually written.
///
/// The destination directory must already exist. An existing regular file is
/// overwritten. See [`validate_wav_path`] for the path rules.
pub fn write_wav(
    path: impl AsRef<Path>,
    audio: &PcmAudio,
    base_dir: Option<&Path>,
) -> Result<PathBuf, WavSinkError> {
    let resolved = validate_wav_path(path.as_ref(), base_dir)?;
    // Encode first so a codec failure never touches (or truncates) the target.
    let bytes = to_wav_bytes(audio)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&resolved)
        .map_err(|e| io_error("open", &e))?;
    let meta = file.metadata().map_err(|e| io_error("metadata", &e))?;
    if !meta.is_file() {
        return Err(invalid("opened target is not a regular file"));
    }
    file.set_len(0).map_err(|e| io_error("truncate", &e))?;
    file.write_all(&bytes).map_err(|e| io_error("write", &e))?;
    file.flush().map_err(|e| io_error("flush", &e))?;
    Ok(resolved)
}

/// Read raw bytes from an already-opened regular file, never buffering more
/// than [`MAX_WAV_BYTES`] (CWE-400).
fn read_bounded(file: File) -> Result<Vec<u8>, WavSinkError> {
    let meta = file.metadata().map_err(|e| io_error("metadata", &e))?;
    if !meta.is_file() {
        return Err(invalid("opened target is not a regular file"));
    }
    let declared = usize::try_from(meta.len()).unwrap_or(usize::MAX);
    if declared > MAX_WAV_BYTES {
        return Err(WavSinkError::OversizedWav {
            size: declared,
            max: MAX_WAV_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(declared);
    // The file may grow after the metadata check; bound the read itself.
    file.take(MAX_WAV_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error("read", &e))?;
    if bytes.len() > MAX_WAV_BYTES {
        return Err(WavSinkError::OversizedWav {
            size: bytes.len(),
            max: MAX_WAV_BYTES,
        });
    }
    Ok(bytes)
}

/// Read and decode the WAV file at `path` into a [`PcmAudio`].
///
/// See [`validate_wav_path`] for the path rules and [`MAX_WAV_BYTES`] for the
/// size bound. Malformed content yields [`WavSinkError::Pcm`], never a panic.
pub fn read_wav(path: impl AsRef<Path>, base_dir: Option<&Path>) -> Result<PcmAudio, WavSinkError> {
    let resolved = validate_wav_path(path.as_ref(), base_dir)?;
    let file = File::open(&resolved).map_err(|e| io_error("open", &e))?;
    let bytes = read_bounded(file)?;
    Ok(PcmAudio::from_wav_bytes(&bytes)?)
}

/// Decode WAV bytes already in memory (bounded by [`MAX_WAV_BYTES`]).
pub fn from_wav_bytes(bytes: &[u8]) -> Result<PcmAudio, WavSinkError> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err(WavSinkError::OversizedWav {
            size: bytes.len(),
            max: MAX_WAV_BYTES,
        });
    }
    Ok(PcmAudio::from_wav_bytes(bytes)?)
}
