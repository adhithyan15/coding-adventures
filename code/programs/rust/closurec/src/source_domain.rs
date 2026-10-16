//! Generic-only D3a red scaffold. This is not source-shape admission and is not
//! called at publication. Replace only after retaining compiling semantic red.
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_tokens::EsVersion;
use serde_json::Value;
const NS: &str = "closurec_source_v1";
#[derive(Clone, Copy)]
pub(super) struct ShapeLimits { work: usize, index_bytes: usize }
impl Default for ShapeLimits {
    fn default() -> Self { Self { work: 64_000_000, index_bytes: 64 * 1024 * 1024 } }
}
pub(super) fn validate_shape(log: &CVLog, _limits: ShapeLimits) -> Result<(), String> {
    log.validate_graph()
}
fn validate_shape_core(log: &CVLog, limits: ShapeLimits) -> Result<(usize, usize), String> {
    validate_shape(log, limits)?;
    Ok((limits.work, limits.index_bytes))
}
fn compact_index(_id: &str, _count: usize) -> Result<usize, String> { Err("red scaffold".into()) }
fn index_storage(count: usize) -> Result<usize, String> {
    count.checked_mul(std::mem::size_of::<Option<&Value>>()).ok_or_else(|| "overflow".into())
}
#[cfg(test)]
#[path = "source_domain_tests.rs"]
mod tests;
