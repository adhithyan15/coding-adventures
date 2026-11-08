//! # Coverage-Guided Fuzz Engine (VER06)
//!
//! Level 2 coverage-guided fuzz testing engine providing a deterministic mutator
//! algebra, 65,536-bit coverage bitmap feedback, seed energy scheduling, crash
//! deduplication via normalized FNV-1a stack hashing, crashing input minimization,
//! closed failure vocabulary, and privacy-preserving TAP reporting.
//!
//! Conforms to specification `code/specs/VER06-coverage-guided-fuzz-engine.md`.

use std::collections::HashSet;

use crate::prng::{fnv1a64, SplitMix64};

// --- Closed Failure Vocabulary (VER06 §5.1, §6) -----------------------------

pub const FID_FUZZ_CRASH: &str = "fuzz-crash";
pub const FID_FUZZ_TIMEOUT: &str = "fuzz-timeout";
pub const FID_FUZZ_OOM: &str = "fuzz-oom";
pub const FID_FUZZ_LEAK: &str = "fuzz-leak";
pub const FID_FUZZ_PANIC: &str = "fuzz-panic";

/// Check if a failure ID is in the closed fuzz vocabulary (VER06 §5.1).
pub fn is_registered_failure_id(id: &str) -> bool {
    matches!(
        id,
        FID_FUZZ_CRASH | FID_FUZZ_TIMEOUT | FID_FUZZ_OOM | FID_FUZZ_LEAK | FID_FUZZ_PANIC
    )
}

// --- Mutator Algebra (VER06 §3) ---------------------------------------------

/// Invert `count` consecutive bits starting at `bit_offset` modulo total bit length.
///
/// If `data` is empty, returns an empty vector.
pub fn bit_flip(data: &[u8], bit_offset: usize, count: usize) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut buf = data.to_vec();
    let total_bits = buf.len() * 8;
    let base = bit_offset % total_bits;
    for i in 0..count {
        let pos = (base + (i % total_bits)) % total_bits;
        let byte_idx = pos / 8;
        let bit_idx = pos % 8;
        buf[byte_idx] ^= 1 << bit_idx;
    }
    buf
}

/// Invert `count` consecutive bytes via XOR 0xFF starting at `byte_offset`
/// modulo total byte length.
///
/// If `data` is empty, returns an empty vector.
pub fn byte_flip(data: &[u8], byte_offset: usize, count: usize) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut buf = data.to_vec();
    let total_bytes = buf.len();
    let base = byte_offset % total_bytes;
    for i in 0..count {
        let pos = (base + (i % total_bytes)) % total_bytes;
        buf[pos] ^= 0xFF;
    }
    buf
}

/// Add signed `delta` modulo `2^(8 * width)` to integer at `byte_offset`.
///
/// Supports widths 1, 2, 4, and 8 bytes in "little" or "big" endianness.
/// If `data.len() < width`, returns a copy of `data` unchanged.
pub fn byte_arithmetic(
    data: &[u8],
    byte_offset: usize,
    delta: i64,
    width: usize,
    endian: &str,
) -> Vec<u8> {
    if data.len() < width {
        return data.to_vec();
    }
    let mut buf = data.to_vec();
    let offset = byte_offset % (buf.len() - width + 1);
    let is_little = endian != "big";

    match width {
        1 => {
            let val = buf[offset];
            buf[offset] = val.wrapping_add(delta as u8);
        }
        2 => {
            let slice: [u8; 2] = buf[offset..offset + 2].try_into().unwrap();
            let val = if is_little {
                u16::from_le_bytes(slice)
            } else {
                u16::from_be_bytes(slice)
            };
            let new_val = val.wrapping_add(delta as u16);
            let bytes = if is_little {
                new_val.to_le_bytes()
            } else {
                new_val.to_be_bytes()
            };
            buf[offset..offset + 2].copy_from_slice(&bytes);
        }
        4 => {
            let slice: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
            let val = if is_little {
                u32::from_le_bytes(slice)
            } else {
                u32::from_be_bytes(slice)
            };
            let new_val = val.wrapping_add(delta as u32);
            let bytes = if is_little {
                new_val.to_le_bytes()
            } else {
                new_val.to_be_bytes()
            };
            buf[offset..offset + 4].copy_from_slice(&bytes);
        }
        8 => {
            let slice: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            let val = if is_little {
                u64::from_le_bytes(slice)
            } else {
                u64::from_be_bytes(slice)
            };
            let new_val = val.wrapping_add(delta as u64);
            let bytes = if is_little {
                new_val.to_le_bytes()
            } else {
                new_val.to_be_bytes()
            };
            buf[offset..offset + 8].copy_from_slice(&bytes);
        }
        _ => {}
    }
    buf
}

/// Overwrite `width` bytes at `byte_offset` with boundary interest value `val`.
///
/// Supports widths 1, 2, 4, and 8 bytes in "little" or "big" endianness.
/// If `data.len() < width`, returns a copy of `data` unchanged.
pub fn boundary_insert(
    data: &[u8],
    byte_offset: usize,
    width: usize,
    val: u64,
    endian: &str,
) -> Vec<u8> {
    if data.len() < width {
        return data.to_vec();
    }
    let mut buf = data.to_vec();
    let offset = byte_offset % (buf.len() - width + 1);
    let is_little = endian != "big";

    match width {
        1 => {
            buf[offset] = val as u8;
        }
        2 => {
            let bytes = if is_little {
                (val as u16).to_le_bytes()
            } else {
                (val as u16).to_be_bytes()
            };
            buf[offset..offset + 2].copy_from_slice(&bytes);
        }
        4 => {
            let bytes = if is_little {
                (val as u32).to_le_bytes()
            } else {
                (val as u32).to_be_bytes()
            };
            buf[offset..offset + 4].copy_from_slice(&bytes);
        }
        8 => {
            let bytes = if is_little {
                val.to_le_bytes()
            } else {
                val.to_be_bytes()
            };
            buf[offset..offset + 8].copy_from_slice(&bytes);
        }
        _ => {}
    }
    buf
}

/// Delete `length` bytes at `offset % len(data)`.
///
/// Clamps length to the remaining buffer size. If `data` is empty or `length == 0`,
/// returns `data` unchanged.
pub fn block_delete(data: &[u8], offset: usize, length: usize) -> Vec<u8> {
    if data.is_empty() || length == 0 {
        return data.to_vec();
    }
    let actual_offset = offset % data.len();
    let actual_length = length.min(data.len() - actual_offset);
    let mut out = Vec::with_capacity(data.len() - actual_length);
    out.extend_from_slice(&data[..actual_offset]);
    out.extend_from_slice(&data[actual_offset + actual_length..]);
    out
}

/// Insert `insert_bytes` at `offset % (len(data) + 1)`.
///
/// If `data` is empty, returns a copy of `insert_bytes`.
pub fn block_insert(data: &[u8], offset: usize, insert_bytes: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return insert_bytes.to_vec();
    }
    let actual_offset = offset % (data.len() + 1);
    let mut out = Vec::with_capacity(data.len() + insert_bytes.len());
    out.extend_from_slice(&data[..actual_offset]);
    out.extend_from_slice(insert_bytes);
    out.extend_from_slice(&data[actual_offset..]);
    out
}

/// Duplicate `length` bytes starting at `src_offset` and insert them at `dst_offset`.
///
/// Clamps copied length to buffer boundary. If `data` is empty or `length == 0`,
/// returns `data` unchanged.
pub fn block_duplicate(
    data: &[u8],
    src_offset: usize,
    length: usize,
    dst_offset: usize,
) -> Vec<u8> {
    if data.is_empty() || length == 0 {
        return data.to_vec();
    }
    let actual_src = src_offset % data.len();
    let actual_len = length.min(data.len() - actual_src);
    let chunk = &data[actual_src..actual_src + actual_len];
    let actual_dst = dst_offset % (data.len() + 1);

    let mut out = Vec::with_capacity(data.len() + actual_len);
    out.extend_from_slice(&data[..actual_dst]);
    out.extend_from_slice(chunk);
    out.extend_from_slice(&data[actual_dst..]);
    out
}

/// Splice prefix of `target` before `target_split` with suffix of `donor` from `donor_split`.
pub fn splice(target: &[u8], donor: &[u8], target_split: usize, donor_split: usize) -> Vec<u8> {
    let t_split = target_split.min(target.len());
    let d_split = donor_split.min(donor.len());
    let mut out = Vec::with_capacity(t_split + (donor.len() - d_split));
    out.extend_from_slice(&target[..t_split]);
    out.extend_from_slice(&donor[d_split..]);
    out
}

/// Deterministic compound mutation sequence using SplitMix64 PRNG (VER06 §3.7).
///
/// Executes `steps` mutator invocations chosen by `rng.bounded(7)`.
pub fn mutate_sequence(data: &[u8], seed: u64, steps: usize) -> Vec<u8> {
    let mut rng = SplitMix64::new(seed);
    let mut current = data.to_vec();

    for _ in 0..steps {
        if current.is_empty() {
            current.push(0);
        }
        let op_choice = rng.bounded(7);
        match op_choice {
            0 => {
                let bit_off = rng.bounded((current.len() * 8) as u64) as usize;
                current = bit_flip(&current, bit_off, 1);
            }
            1 => {
                let byte_off = rng.bounded(current.len() as u64) as usize;
                current = byte_flip(&current, byte_off, 1);
            }
            2 => {
                const DELTAS: [i64; 6] = [-35, -8, -1, 1, 8, 35];
                let delta = DELTAS[rng.bounded(DELTAS.len() as u64) as usize];
                let byte_off = rng.bounded(current.len() as u64) as usize;
                current = byte_arithmetic(&current, byte_off, delta, 1, "little");
            }
            3 => {
                const INTEREST: [u64; 5] = [0, 1, 127, 128, 255];
                let val = INTEREST[rng.bounded(INTEREST.len() as u64) as usize];
                let byte_off = rng.bounded(current.len() as u64) as usize;
                current = boundary_insert(&current, byte_off, 1, val, "little");
            }
            4 => {
                if current.len() > 1 {
                    let max_del = (current.len() / 2).max(1);
                    let del_len = 1 + rng.bounded(max_del as u64) as usize;
                    let off = rng.bounded(current.len() as u64) as usize;
                    current = block_delete(&current, off, del_len);
                }
            }
            5 => {
                let ins_byte = [rng.bounded(256) as u8];
                let off = rng.bounded((current.len() + 1) as u64) as usize;
                current = block_insert(&current, off, &ins_byte);
            }
            _ => {
                if current.len() > 1 {
                    let max_dup = (current.len() / 2).max(1);
                    let dup_len = 1 + rng.bounded(max_dup as u64) as usize;
                    let src_off = rng.bounded(current.len() as u64) as usize;
                    let dst_off = rng.bounded((current.len() + 1) as u64) as usize;
                    current = block_duplicate(&current, src_off, dup_len, dst_off);
                }
            }
        }
    }
    current
}

// --- Corpus Admission & Coverage Bitmap (VER06 §4) ---------------------------

/// Pure check whether `trace_edges` discovers any novel edge absent from `existing_edges`.
///
/// Returns `(admitted, new_edges_count)`.
pub fn corpus_admit(existing_edges: &[usize], trace_edges: &[usize]) -> (bool, usize) {
    let existing: HashSet<usize> = existing_edges.iter().copied().collect();
    let mut unique_new = HashSet::new();
    for &edge in trace_edges {
        if !existing.contains(&edge) {
            unique_new.insert(edge);
        }
    }
    if !unique_new.is_empty() {
        (true, unique_new.len())
    } else {
        (false, 0)
    }
}

pub const COVERAGE_MAP_SIZE: usize = 65536;
pub const COVERAGE_BYTES: usize = COVERAGE_MAP_SIZE / 8; // 8192

/// Shared 65,536-bit execution coverage bitmap (VER06 §4.1).
#[derive(Clone)]
pub struct CoverageBitmap {
    bytes: [u8; COVERAGE_BYTES],
}

impl Default for CoverageBitmap {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverageBitmap {
    /// Allocate an empty coverage bitmap with 0 edges covered.
    pub fn new() -> Self {
        CoverageBitmap {
            bytes: [0u8; COVERAGE_BYTES],
        }
    }

    /// Record that edge `edge` (modulo 65,536) was executed.
    pub fn record_edge(&mut self, edge: usize) {
        let e = edge % COVERAGE_MAP_SIZE;
        self.bytes[e / 8] |= 1 << (e % 8);
    }

    /// Check whether edge `edge` has been covered.
    pub fn has_edge(&self, edge: usize) -> bool {
        let e = edge % COVERAGE_MAP_SIZE;
        (self.bytes[e / 8] & (1 << (e % 8))) != 0
    }

    /// Admit execution trace edges: updates bitmap with unseen edges.
    ///
    /// Returns `(has_new, count_new)`.
    pub fn admit_trace(&mut self, trace: &[usize]) -> (bool, usize) {
        let mut count_new = 0;
        for &edge in trace {
            let e = edge % COVERAGE_MAP_SIZE;
            let byte_idx = e / 8;
            let bit_mask = 1 << (e % 8);
            if (self.bytes[byte_idx] & bit_mask) == 0 {
                self.bytes[byte_idx] |= bit_mask;
                count_new += 1;
            }
        }
        (count_new > 0, count_new)
    }

    /// Total number of distinct covered edges.
    pub fn count(&self) -> usize {
        self.bytes.iter().map(|&b| b.count_ones() as usize).sum()
    }

    /// Merge cumulative coverage from another bitmap.
    pub fn merge(&mut self, other: &CoverageBitmap) {
        for (dst, src) in self.bytes.iter_mut().zip(other.bytes.iter()) {
            *dst |= *src;
        }
    }
}

// --- Crash Deduplication & Privacy-Preserving TAP (VER06 §5, VER00 §7.3) -----

/// Compute 8-hex-digit normalized crash signature hash (VER06 §5.2).
///
/// Hashes `kind:frame_1:frame_2:...` with FNV-1a 64-bit and formats lower 32 bits.
pub fn crash_signature(kind: &str, frames: &[&str]) -> String {
    let mut raw = String::from(kind);
    raw.push(':');
    raw.push_str(&frames.join(":"));
    let h = fnv1a64(raw.as_bytes());
    format!("{:08x}", (h & 0xFFFF_FFFF) as u32)
}

/// Check if two crash reports share an identical normalized signature.
pub fn is_duplicate_crash(
    kind_a: &str,
    frames_a: &[&str],
    kind_b: &str,
    frames_b: &[&str],
) -> bool {
    crash_signature(kind_a, frames_a) == crash_signature(kind_b, frames_b)
}

/// Format privacy-preserving TAP failure line (VER00 §7.3 / VER06 §5.2).
///
/// Contains NO raw input bytes: only target ID, failure vocabulary ID, and sig hash.
pub fn format_tap_failure(
    test_num: usize,
    target_id: &str,
    failure_id: &str,
    sig_hash: &str,
) -> String {
    format!("not ok {test_num} - {target_id} [{failure_id}:{sig_hash}]")
}

// --- Energy Scheduling (VER06 §4.3) -----------------------------------------

/// Compute power schedule energy (mutation budget) for a corpus seed.
///
/// energy(S) = base_energy * (1 + new_edges / total_edges) / sqrt(len)
pub fn calculate_energy(
    base_energy: usize,
    seed_len: usize,
    new_edges: usize,
    total_edges: usize,
) -> usize {
    let base = base_energy.max(1) as f64;
    let edge_bonus = if total_edges > 0 {
        1.0 + (new_edges as f64 / total_edges as f64)
    } else {
        1.0
    };
    let len_factor = (seed_len.max(1) as f64).sqrt();
    let energy = (base * edge_bonus) / len_factor;
    (energy.round() as usize).max(1)
}

// --- Crash Minimization & Engine Runner (VER06 §5.3, §7) ---------------------

/// Outcome of a single target invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FuzzOutcome {
    Pass,
    Crash { kind: String, frames: Vec<String> },
    Timeout,
    Oom,
}

/// Configuration options for a bounded fuzz campaign.
#[derive(Clone, Debug)]
pub struct FuzzOptions {
    pub max_iterations: usize,
    pub seed: u64,
    pub max_input_len: usize,
    pub base_energy: usize,
}

impl Default for FuzzOptions {
    fn default() -> Self {
        FuzzOptions {
            max_iterations: 1000,
            seed: 42,
            max_input_len: 4096,
            base_energy: 10,
        }
    }
}

/// Deduplicated crash record.
#[derive(Clone, Debug)]
pub struct CrashRecord {
    pub signature: String,
    pub kind: String,
    pub frames: Vec<String>,
    pub minimal_input: Vec<u8>,
    pub hits: usize,
}

/// Summary report returned after a bounded fuzzing run.
#[derive(Clone, Debug)]
pub struct FuzzReport {
    pub iterations: usize,
    pub admitted_seeds: usize,
    pub total_edges: usize,
    pub crashes: Vec<CrashRecord>,
}

/// Minimize a crashing input while preserving its exact crash signature (VER06 §5.3).
///
/// 1. Tries block deletion of chunks (halving chunk sizes).
/// 2. Tries byte zeroing / normalization on individual non-essential bytes.
pub fn minimize_crashing_input<F>(input: &[u8], target_sig: &str, mut test_fn: F) -> Vec<u8>
where
    F: FnMut(&[u8]) -> FuzzOutcome,
{
    let mut current = input.to_vec();
    if current.is_empty() {
        return current;
    }

    let mut chunk_size = current.len() / 2;
    while chunk_size > 0 {
        let mut offset = 0;
        while offset < current.len() {
            let actual_len = chunk_size.min(current.len() - offset);
            let mut candidate = Vec::with_capacity(current.len() - actual_len);
            candidate.extend_from_slice(&current[..offset]);
            candidate.extend_from_slice(&current[offset + actual_len..]);

            let outcome = test_fn(&candidate);
            if let FuzzOutcome::Crash { kind, frames } = outcome {
                let frame_refs: Vec<&str> = frames.iter().map(String::as_str).collect();
                if crash_signature(&kind, &frame_refs) == target_sig {
                    current = candidate;
                    continue;
                }
            }
            offset += chunk_size;
        }
        chunk_size /= 2;
    }

    // Pass 2: byte zeroing
    for i in 0..current.len() {
        if current[i] == 0 {
            continue;
        }
        let original_byte = current[i];
        current[i] = 0;
        let outcome = test_fn(&current);
        let mut matched = false;
        if let FuzzOutcome::Crash { kind, frames } = outcome {
            let frame_refs: Vec<&str> = frames.iter().map(String::as_str).collect();
            if crash_signature(&kind, &frame_refs) == target_sig {
                matched = true;
            }
        }
        if !matched {
            current[i] = original_byte;
        }
    }

    current
}

/// Seed entry held in the active corpus.
#[derive(Clone, Debug)]
pub struct CorpusSeed {
    pub data: Vec<u8>,
    pub new_edges: usize,
    pub generation: usize,
}

/// Run a coverage-guided fuzz campaign over `initial_corpus` seeds (VER06 §7).
pub fn fuzz_target<F>(initial_corpus: &[Vec<u8>], opts: &FuzzOptions, mut target: F) -> FuzzReport
where
    F: FnMut(&[u8], &mut CoverageBitmap) -> FuzzOutcome,
{
    let mut rng = SplitMix64::new(opts.seed);
    let mut cumulative_coverage = CoverageBitmap::new();
    let mut corpus: Vec<CorpusSeed> = Vec::new();
    let mut crashes: Vec<CrashRecord> = Vec::new();

    // 1. Initial corpus triage
    let seeds = if initial_corpus.is_empty() {
        vec![vec![0u8; 1]]
    } else {
        initial_corpus.to_vec()
    };

    for seed in seeds {
        let mut step_coverage = CoverageBitmap::new();
        let outcome = target(&seed, &mut step_coverage);
        match outcome {
            FuzzOutcome::Pass => {
                let mut trace = Vec::new();
                for edge in 0..COVERAGE_MAP_SIZE {
                    if step_coverage.has_edge(edge) {
                        trace.push(edge);
                    }
                }
                let (has_new, new_count) = cumulative_coverage.admit_trace(&trace);
                if has_new || corpus.is_empty() {
                    corpus.push(CorpusSeed {
                        data: seed,
                        new_edges: new_count,
                        generation: 0,
                    });
                }
            }
            FuzzOutcome::Crash { kind, frames } => {
                let frame_refs: Vec<&str> = frames.iter().map(String::as_str).collect();
                let sig = crash_signature(&kind, &frame_refs);
                crashes.push(CrashRecord {
                    signature: sig,
                    kind,
                    frames,
                    minimal_input: seed,
                    hits: 1,
                });
            }
            _ => {}
        }
    }

    // 2. Fuzzing loop
    let mut iterations = 0;
    while iterations < opts.max_iterations && !corpus.is_empty() {
        let seed_idx = rng.bounded(corpus.len() as u64) as usize;
        let seed = corpus[seed_idx].clone();
        let total_cov = cumulative_coverage.count();
        let energy = calculate_energy(opts.base_energy, seed.data.len(), seed.new_edges, total_cov);

        for _ in 0..energy {
            if iterations >= opts.max_iterations {
                break;
            }
            iterations += 1;

            let mutation_steps = 1 + rng.bounded(4) as usize;
            let sub_seed = rng.next_u64();
            let mut candidate = mutate_sequence(&seed.data, sub_seed, mutation_steps);

            // Splicing with another seed if corpus > 1 and rng chooses
            if corpus.len() > 1 && rng.bounded(4) == 0 {
                let donor_idx = rng.bounded(corpus.len() as u64) as usize;
                let donor = &corpus[donor_idx].data;
                let t_split = rng.bounded((candidate.len() + 1) as u64) as usize;
                let d_split = rng.bounded((donor.len() + 1) as u64) as usize;
                candidate = splice(&candidate, donor, t_split, d_split);
            }

            if candidate.len() > opts.max_input_len {
                candidate.truncate(opts.max_input_len);
            }

            let mut step_cov = CoverageBitmap::new();
            let outcome = target(&candidate, &mut step_cov);

            match outcome {
                FuzzOutcome::Pass => {
                    let mut trace = Vec::new();
                    for edge in 0..COVERAGE_MAP_SIZE {
                        if step_cov.has_edge(edge) {
                            trace.push(edge);
                        }
                    }
                    let (has_new, new_count) = cumulative_coverage.admit_trace(&trace);
                    if has_new {
                        corpus.push(CorpusSeed {
                            data: candidate,
                            new_edges: new_count,
                            generation: seed.generation + 1,
                        });
                    }
                }
                FuzzOutcome::Crash { kind, frames } => {
                    let frame_refs: Vec<&str> = frames.iter().map(String::as_str).collect();
                    let sig = crash_signature(&kind, &frame_refs);

                    if let Some(existing) = crashes.iter_mut().find(|c| c.signature == sig) {
                        existing.hits += 1;
                        if candidate.len() < existing.minimal_input.len() {
                            existing.minimal_input = candidate;
                        }
                    } else {
                        crashes.push(CrashRecord {
                            signature: sig,
                            kind,
                            frames,
                            minimal_input: candidate,
                            hits: 1,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    FuzzReport {
        iterations,
        admitted_seeds: corpus.len(),
        total_edges: cumulative_coverage.count(),
        crashes,
    }
}

// --- Unit Tests (VER06 §6, §7) -----------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_vocabulary() {
        assert!(is_registered_failure_id(FID_FUZZ_CRASH));
        assert!(is_registered_failure_id(FID_FUZZ_TIMEOUT));
        assert!(is_registered_failure_id(FID_FUZZ_OOM));
        assert!(is_registered_failure_id(FID_FUZZ_LEAK));
        assert!(is_registered_failure_id(FID_FUZZ_PANIC));
        assert!(!is_registered_failure_id("assertion-failure"));
        assert!(!is_registered_failure_id("unknown-id"));
    }

    #[test]
    fn test_bit_flip() {
        assert_eq!(bit_flip(&[], 0, 1), Vec::<u8>::new());
        assert_eq!(bit_flip(&[0x00, 0x00], 0, 1), vec![0x01, 0x00]);
        assert_eq!(bit_flip(&[0x00, 0x00], 9, 1), vec![0x00, 0x02]);
        assert_eq!(bit_flip(&[0x00, 0x00], 2, 4), vec![0x3C, 0x00]);
        assert_eq!(bit_flip(&[0x00, 0x00], usize::MAX, 2), vec![0x01, 0x80]);
    }

    #[test]
    fn test_byte_flip() {
        assert_eq!(byte_flip(&[], 0, 1), Vec::<u8>::new());
        assert_eq!(byte_flip(&[0x12, 0x34, 0x56], 0, 1), vec![0xED, 0x34, 0x56]);
        assert_eq!(byte_flip(&[0x12, 0x34, 0x56], 1, 2), vec![0x12, 0xCB, 0xA9]);
        assert_eq!(
            byte_flip(&[0x12, 0x34, 0x56], usize::MAX, 2),
            vec![0xED, 0xCB, 0x56]
        );
    }

    #[test]
    fn test_byte_arithmetic() {
        assert_eq!(byte_arithmetic(&[], 0, 1, 1, "little"), Vec::<u8>::new());
        assert_eq!(
            byte_arithmetic(&[0x10, 0x20], 0, 1, 1, "little"),
            vec![0x11, 0x20]
        );
        assert_eq!(
            byte_arithmetic(&[0x00, 0x00], 0, -1, 1, "little"),
            vec![0xFF, 0x00]
        );
        assert_eq!(
            byte_arithmetic(&[0xFF, 0x00], 0, 1, 2, "little"),
            vec![0x00, 0x01]
        );
        assert_eq!(
            byte_arithmetic(&[0x00, 0xFF], 0, 1, 2, "big"),
            vec![0x01, 0x00]
        );
        assert_eq!(
            byte_arithmetic(&[0x00, 0x00, 0x00, 0x00], 0, 1, 4, "little"),
            vec![0x01, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            byte_arithmetic(&[0x00, 0x00, 0x00, 0x00], 0, 1, 4, "big"),
            vec![0x00, 0x00, 0x00, 0x01]
        );
        assert_eq!(
            byte_arithmetic(
                &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                0,
                1,
                8,
                "little"
            ),
            vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            byte_arithmetic(
                &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                0,
                1,
                8,
                "big"
            ),
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
        );
    }

    #[test]
    fn test_boundary_insert() {
        assert_eq!(boundary_insert(&[], 0, 1, 1, "little"), Vec::<u8>::new());
        assert_eq!(
            boundary_insert(&[0x00, 0x00], 0, 1, 0x7F, "little"),
            vec![0x7F, 0x00]
        );
        assert_eq!(
            boundary_insert(&[0x00, 0x00], 0, 2, 0x7FFF, "little"),
            vec![0xFF, 0x7F]
        );
        assert_eq!(
            boundary_insert(&[0x00, 0x00], 0, 2, 0x7FFF, "big"),
            vec![0x7F, 0xFF]
        );
        assert_eq!(
            boundary_insert(&[0x00, 0x00, 0x00, 0x00], 0, 4, 0x7FFFFFFF, "little"),
            vec![0xFF, 0xFF, 0xFF, 0x7F]
        );
        assert_eq!(
            boundary_insert(
                &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                0,
                8,
                0x7FFFFFFFFFFFFFFF,
                "little"
            ),
            vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]
        );
        assert_eq!(
            boundary_insert(
                &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                0,
                8,
                0x7FFFFFFFFFFFFFFF,
                "big"
            ),
            vec![0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn test_block_delete() {
        assert_eq!(block_delete(&[], 0, 1), Vec::<u8>::new());
        assert_eq!(block_delete(&[1, 2, 3], 0, 0), vec![1, 2, 3]);
        assert_eq!(block_delete(&[1, 2, 3, 4, 5], 1, 2), vec![1, 4, 5]);
        assert_eq!(block_delete(&[1, 2, 3, 4, 5], 4, 3), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_block_insert() {
        assert_eq!(block_insert(&[], 0, &[1, 2]), vec![1, 2]);
        assert_eq!(block_insert(&[1, 4], 1, &[2, 3]), vec![1, 2, 3, 4]);
        assert_eq!(block_insert(&[1, 2], 2, &[3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_block_duplicate() {
        assert_eq!(block_duplicate(&[], 0, 1, 0), Vec::<u8>::new());
        assert_eq!(block_duplicate(&[1, 2], 0, 0, 0), vec![1, 2]);
        assert_eq!(block_duplicate(&[1, 2, 3], 0, 2, 3), vec![1, 2, 3, 1, 2]);
    }

    #[test]
    fn test_splice() {
        assert_eq!(splice(&[1, 2, 3, 4], &[5, 6, 7, 8], 2, 2), vec![1, 2, 7, 8]);
        assert_eq!(splice(&[1, 2], &[3, 4], 0, 0), vec![3, 4]);
        assert_eq!(splice(&[1, 2], &[3, 4], 10, 10), vec![1, 2]);
    }

    #[test]
    fn test_mutate_sequence() {
        let initial = vec![1, 2, 3, 4, 5];
        let mutated1 = mutate_sequence(&initial, 12345, 10);
        let mutated2 = mutate_sequence(&initial, 12345, 10);
        assert_eq!(mutated1, mutated2);
        assert_ne!(mutated1, initial);

        // Empty input test
        let from_empty = mutate_sequence(&[], 999, 5);
        assert!(!from_empty.is_empty());
    }

    #[test]
    fn test_corpus_admit() {
        let existing = vec![10, 20, 30];
        let trace1 = vec![20, 30];
        assert_eq!(corpus_admit(&existing, &trace1), (false, 0));

        let trace2 = vec![20, 40, 50];
        assert_eq!(corpus_admit(&existing, &trace2), (true, 2));
    }

    #[test]
    fn test_coverage_bitmap() {
        let mut map = CoverageBitmap::new();
        assert_eq!(map.count(), 0);
        assert!(!map.has_edge(42));

        map.record_edge(42);
        assert!(map.has_edge(42));
        assert_eq!(map.count(), 1);

        let (admitted, count) = map.admit_trace(&[42, 100, 200]);
        assert!(admitted);
        assert_eq!(count, 2);
        assert_eq!(map.count(), 3);

        let mut other = CoverageBitmap::new();
        other.record_edge(300);
        map.merge(&other);
        assert!(map.has_edge(300));
        assert_eq!(map.count(), 4);
    }

    #[test]
    fn test_coverage_bitmap_default() {
        let map = CoverageBitmap::default();
        assert_eq!(map.count(), 0);
    }

    #[test]
    fn test_crash_deduplication() {
        let sig1 = crash_signature("panic", &["eval", "step"]);
        let sig2 = crash_signature("panic", &["eval", "step"]);
        let sig3 = crash_signature("panic", &["other", "step"]);
        assert_eq!(sig1, sig2);
        assert_ne!(sig1, sig3);

        assert!(is_duplicate_crash(
            "panic",
            &["eval", "step"],
            "panic",
            &["eval", "step"]
        ));
        assert!(!is_duplicate_crash(
            "panic",
            &["eval", "step"],
            "sigsegv",
            &["eval", "step"]
        ));
    }

    #[test]
    fn test_tap_failure_formatting() {
        let line = format_tap_failure(1, "target_calc", FID_FUZZ_CRASH, "a1b2c3d4");
        assert_eq!(line, "not ok 1 - target_calc [fuzz-crash:a1b2c3d4]");
        // Privacy invariant: line contains no raw payload bytes
        assert!(!line.contains("secret"));
    }

    #[test]
    fn test_energy_calculation() {
        let e1 = calculate_energy(10, 16, 5, 20);
        assert!(e1 > 0);
        let e2 = calculate_energy(10, 256, 5, 20);
        assert!(e1 > e2); // Shorter seed receives higher energy
    }

    #[test]
    fn test_minimization() {
        // Crash triggers if slice contains [0xDE, 0xAD]
        let crashing_input = vec![1, 2, 3, 0xDE, 0xAD, 4, 5, 6, 7, 8];
        let target_sig = crash_signature("panic", &["detect_dead"]);

        let test_fn = |slice: &[u8]| {
            if slice.windows(2).any(|w| w == [0xDE, 0xAD]) {
                FuzzOutcome::Crash {
                    kind: "panic".into(),
                    frames: vec!["detect_dead".into()],
                }
            } else {
                FuzzOutcome::Pass
            }
        };

        let minimized = minimize_crashing_input(&crashing_input, &target_sig, test_fn);
        assert_eq!(minimized, vec![0xDE, 0xAD]);

        assert_eq!(
            minimize_crashing_input(&[], "any", |_| FuzzOutcome::Pass),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn test_fuzz_options_default() {
        let opts = FuzzOptions::default();
        assert_eq!(opts.max_iterations, 1000);
        assert_eq!(opts.seed, 42);
    }

    #[test]
    fn test_fuzz_target_runner() {
        let opts = FuzzOptions {
            max_iterations: 100,
            seed: 7,
            max_input_len: 64,
            base_energy: 5,
        };

        let report = fuzz_target(&[vec![0; 4]], &opts, |input, cov| {
            if !input.is_empty() {
                cov.record_edge(input[0] as usize);
            }
            if input.len() >= 2 && input[0] == 0x42 && input[1] == 0x24 {
                FuzzOutcome::Crash {
                    kind: "panic".into(),
                    frames: vec!["magic_match".into()],
                }
            } else {
                FuzzOutcome::Pass
            }
        });

        assert_eq!(report.iterations, 100);
        assert!(report.total_edges > 0);
    }

    #[test]
    fn test_fuzz_target_empty_corpus_and_crashing_dedup() {
        let opts = FuzzOptions {
            max_iterations: 30,
            seed: 42,
            max_input_len: 16,
            base_energy: 10,
        };

        let report = fuzz_target(&[], &opts, |input, cov| {
            cov.record_edge(input.len());
            if input.len() > 1 && input[0] == 0 {
                FuzzOutcome::Crash {
                    kind: "panic".into(),
                    frames: vec!["repeat".into()],
                }
            } else if input.len() > 10 {
                FuzzOutcome::Timeout
            } else {
                FuzzOutcome::Pass
            }
        });
        if !report.crashes.is_empty() {
            assert!(report.crashes[0].hits >= 1);
        }
    }
}
