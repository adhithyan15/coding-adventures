//! D3b generic/shape-only red scaffold. No replay admission is implemented or
//! wired into production publication yet. Retain a compiling semantic red first.
use super::source_domain::ShapeAdmitted;
use coding_adventures_javascript_tokens::EsVersion;
use lexer::token::Token;
use std::mem::size_of;
use std::path::PathBuf;
#[derive(Clone, Copy)]
pub(super) struct ReplayLimits {
    source_capacity: usize,
    index_bytes: usize,
    fingerprint_bytes: usize,
    work: usize,
}
impl Default for ReplayLimits {
    fn default() -> Self {
        Self {
            source_capacity: 64 * 1024 * 1024,
            index_bytes: 16 * 1024 * 1024,
            fingerprint_bytes: 16 * 1024 * 1024,
            work: 64_000_000,
        }
    }
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Cost {
    source_capacity: usize,
    index_bytes: usize,
    fingerprint_bytes: usize,
    work: usize,
}
struct TrustedInput {
    source: String,
    root: u64,
    fingerprints: Option<Vec<[u8; 32]>>,
}
pub(super) struct SourceCatalog {
    inputs: Vec<TrustedInput>,
    edition: EsVersion,
    limits: ReplayLimits,
    used: Cost,
}
struct Work {
    used: usize,
    limit: usize,
}
fn fingerprint(_token: &Token, _work: &mut Work) -> Result<[u8; 32], String> {
    Ok([0; 32])
}
fn catalog_storage(count: usize) -> Result<usize, String> {
    count
        .checked_mul(size_of::<TrustedInput>())
        .ok_or_else(|| "catalog overflow".into())
}
fn fingerprint_storage(count: usize) -> Result<usize, String> {
    count
        .checked_mul(size_of::<[u8; 32]>())
        .ok_or_else(|| "fingerprint overflow".into())
}
impl SourceCatalog {
    pub(super) fn new(
        _expected: usize,
        edition: EsVersion,
        limits: ReplayLimits,
    ) -> Result<Self, String> {
        Ok(Self {
            inputs: Vec::new(),
            edition,
            limits,
            used: Cost::default(),
        })
    }
    pub(super) fn retain(
        &mut self,
        source: String,
        root: &str,
        tokens: Option<&[Token]>,
    ) -> Result<&str, String> {
        self.inputs.push(TrustedInput {
            source,
            root: u64::from_str_radix(&root[4..], 16).map_err(|_| "root")?,
            fingerprints: tokens.map(|_| Vec::new()),
        });
        Ok(&self.inputs.last().ok_or("missing input")?.source)
    }
    pub(super) fn validate(
        &self,
        shape: &ShapeAdmitted<'_>,
        roots: &[String],
        paths: &[PathBuf],
    ) -> Result<(), String> {
        self.validate_core(shape, roots, paths).map(|_| ())
    }
    fn validate_core(
        &self,
        shape: &ShapeAdmitted<'_>,
        _roots: &[String],
        _paths: &[PathBuf],
    ) -> Result<usize, String> {
        let _ = shape.log();
        Ok(0)
    }
}
#[cfg(test)]
#[path = "source_replay_tests.rs"]
mod tests;
