//! File loading and WAV file exporting (MUS01 / MUS04 / Stage 8).

use std::fs;
use std::path::{Component, Path, PathBuf};

use wav_sink::write_wav;

use crate::error::MusicMachineError;
use crate::limits::*;
use crate::portable_parser::parse_portable_score;
use crate::render::{render_portable_score_to_pcm, render_score_to_pcm};
use crate::score_parser::parse_score;
use crate::types::*;

/// Reject a path if it is a symbolic link.
fn reject_symlink(p: &Path) -> Result<(), MusicMachineError> {
    match fs::symlink_metadata(p) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(MusicMachineError::InvalidArgument(
                "symlinks are not permitted for score files".into(),
            ))
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(MusicMachineError::Io(err.to_string())),
    }
}

/// Validate an input file path against null bytes, parent directory traversal (..), and symlinks (CWE-22).
fn validate_input_path(path: &Path) -> Result<(), MusicMachineError> {
    let path_str = path.to_string_lossy();
    if path_str.is_empty() {
        return Err(MusicMachineError::InvalidArgument(
            "file path is empty".into(),
        ));
    }
    if path_str.contains('\0') {
        return Err(MusicMachineError::InvalidArgument(
            "file path contains null byte".into(),
        ));
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(MusicMachineError::InvalidArgument(
            "path contains a '..' component".into(),
        ));
    }
    for ancestor in path.ancestors() {
        if !ancestor.as_os_str().is_empty() {
            reject_symlink(ancestor)?;
        }
    }
    if !path.is_file() {
        return Err(MusicMachineError::Io(format!(
            "score file not found or not a regular file: {}",
            path.display()
        )));
    }
    let metadata = fs::metadata(path).map_err(|e| MusicMachineError::Io(e.to_string()))?;
    if metadata.len() > (MAX_SCORE_BYTES as u64) {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "file size {} exceeds MAX_SCORE_BYTES={MAX_SCORE_BYTES}",
            metadata.len()
        )));
    }
    Ok(())
}

/// Read and parse a beginner text score file.
pub fn parse_score_file(path: impl AsRef<Path>) -> Result<TextScore, MusicMachineError> {
    let p = path.as_ref();
    validate_input_path(p)?;
    let content = fs::read_to_string(p).map_err(|e| MusicMachineError::Io(e.to_string()))?;
    parse_score(&content)
}

/// Read and parse a portable multi-track score file.
pub fn parse_portable_score_file(path: impl AsRef<Path>) -> Result<PortableScore, MusicMachineError> {
    let p = path.as_ref();
    validate_input_path(p)?;
    let content = fs::read_to_string(p).map_err(|e| MusicMachineError::Io(e.to_string()))?;
    parse_portable_score(&content)
}

/// Render a beginner text score and write the output to a standard RIFF/WAVE file.
pub fn render_score_to_wav_file(
    score: &TextScore,
    path: impl AsRef<Path>,
    max_sample_count: usize,
) -> Result<PathBuf, MusicMachineError> {
    let rendered = render_score_to_pcm(score, max_sample_count)?;
    let out_path = write_wav(path, &rendered.pcm_buffer, None)?;
    Ok(out_path)
}

/// Render a portable multi-track score and write the output to a standard RIFF/WAVE file.
pub fn render_portable_score_to_wav_file(
    score: &PortableScore,
    path: impl AsRef<Path>,
    max_sample_count: usize,
) -> Result<PathBuf, MusicMachineError> {
    let rendered = render_portable_score_to_pcm(score, max_sample_count)?;
    let out_path = write_wav(path, &rendered.pcm_buffer, None)?;
    Ok(out_path)
}
