//! A small caller-owned ABI around the shared paired microbenchmark core.
//!
//! The module deliberately owns no clock or workload implementation. The host
//! supplies both through two imports, while this crate validates bounded JSON,
//! enforces the shared collection protocol, and returns the portable report.

#![cfg_attr(
    not(target_arch = "wasm32"),
    allow(dead_code, unused_imports, unused_mut)
)]

#[cfg(target_arch = "wasm32")]
use coding_adventures_microbenchmark::collect;
use coding_adventures_microbenchmark::{
    analyze, Analysis, Config, Error as CoreError, Pair, Report,
};
use serde::{Deserialize, Serialize};
use std::alloc::{alloc_zeroed, dealloc as system_dealloc, Layout};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_ANALYSIS_PAIRS: usize = 10_000;
const MAX_BATCH: usize = 65_536;
const MAX_TOTAL_ENTRIES: u64 = 1_000_000;
const MAX_JSON_DEPTH: usize = 16;
const MAX_JSON_NODES: usize = 100_000;
const MAX_ALLOCATION_RECORDS: usize = 32;
const MAX_LIVE_BYTES: usize = 8 * 1024 * 1024;
const MAX_INPUT_LIVE_BYTES: usize = 7 * 1024 * 1024;

const ERR_INVALID_INPUT: &str = "invalid_input";
const ERR_BOUNDS: &str = "bounds_exceeded";
const ERR_HOST: &str = "host_callback_failed";
const ERR_CLOCK: &str = "clock_failure";
const ERR_CHECKSUM: &str = "checksum_mismatch";
const ERR_REPORT: &str = "invalid_report";
const ERR_SERIALIZE: &str = "serialization_failed";
const ERR_INTERNAL: &str = "internal_error";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AllocationKind {
    Input,
    Result,
}

#[derive(Clone, Copy)]
struct Record {
    address: usize,
    length: usize,
    kind: Option<AllocationKind>,
}

impl Record {
    const EMPTY: Self = Self {
        address: 0,
        length: 0,
        kind: None,
    };
}

struct Registry {
    records: [Record; MAX_ALLOCATION_RECORDS],
    live_bytes: usize,
    input_bytes: usize,
}

impl Registry {
    const fn new() -> Self {
        Self {
            records: [Record::EMPTY; MAX_ALLOCATION_RECORDS],
            live_bytes: 0,
            input_bytes: 0,
        }
    }

    fn insert(&mut self, address: usize, length: usize, kind: AllocationKind) -> bool {
        let Some(slot_index) = self.records.iter().position(|entry| entry.kind.is_none()) else {
            return false;
        };
        let Some(live_bytes) = self.live_bytes.checked_add(length) else {
            return false;
        };
        if live_bytes > MAX_LIVE_BYTES {
            return false;
        }
        if kind == AllocationKind::Input {
            let Some(input_bytes) = self.input_bytes.checked_add(length) else {
                return false;
            };
            if input_bytes > MAX_INPUT_LIVE_BYTES {
                return false;
            }
            self.input_bytes = input_bytes;
        }
        self.live_bytes = live_bytes;
        self.records[slot_index] = Record {
            address,
            length,
            kind: Some(kind),
        };
        true
    }

    fn remove(&mut self, address: usize, length: usize) -> Option<AllocationKind> {
        let index = self.records.iter().position(|entry| {
            entry.kind.is_some() && entry.address == address && entry.length == length
        })?;
        let record = self.records[index];
        let kind = record.kind?;
        self.live_bytes = self.live_bytes.checked_sub(record.length)?;
        if kind == AllocationKind::Input {
            self.input_bytes = self.input_bytes.checked_sub(record.length)?;
        }
        self.records[index] = Record::EMPTY;
        Some(kind)
    }

    fn has_exact_live_span(&self, address: usize, length: usize) -> bool {
        self.records
            .iter()
            .any(|entry| entry.kind.is_some() && entry.address == address && entry.length == length)
    }
}

static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
static BUSY: AtomicBool = AtomicBool::new(false);

struct BusyGuard;

impl BusyGuard {
    fn enter() -> Option<Self> {
        BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Self)
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone, Copy)]
struct AbiError {
    code: &'static str,
    message: &'static str,
}

impl AbiError {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionRequest {
    config: Config,
    expected_checksum: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AnalysisRequest {
    confidence_percent: u8,
    minimum_factor: f64,
    pairs: Vec<Pair>,
}

#[derive(Serialize)]
struct Success<'a, T> {
    ok: bool,
    value: &'a T,
}

#[derive(Serialize)]
struct Failure<'a> {
    ok: bool,
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
}

fn locked_registry() -> std::sync::MutexGuard<'static, Registry> {
    REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn allocation_layout(length: usize) -> Option<Layout> {
    Layout::from_size_align(length, 8).ok()
}

fn allocate_registered(length: usize, kind: AllocationKind) -> Option<usize> {
    if length == 0 || length > MAX_OUTPUT_BYTES {
        return None;
    }
    let layout = allocation_layout(length)?;
    let pointer = unsafe { alloc_zeroed(layout) };
    if pointer.is_null() {
        return None;
    }
    let address = pointer as usize;
    if !locked_registry().insert(address, length, kind) {
        unsafe { system_dealloc(pointer, layout) };
        return None;
    }
    Some(address)
}

fn release_registered(address: usize, length: usize) -> bool {
    let kind = locked_registry().remove(address, length);
    let Some(_kind) = kind else {
        return false;
    };
    let Some(layout) = allocation_layout(length) else {
        return false;
    };
    unsafe { system_dealloc(address as *mut u8, layout) };
    true
}

#[cfg(target_arch = "wasm32")]
fn memory_bytes() -> Option<usize> {
    core::arch::wasm32::memory_size(0).checked_mul(65_536)
}

#[cfg(not(target_arch = "wasm32"))]
fn memory_bytes() -> Option<usize> {
    Some(usize::MAX)
}

fn copy_live_input(pointer: i32, length: i32) -> Result<Vec<u8>, AbiError> {
    let address = pointer as u32 as usize;
    let length = length as u32 as usize;
    if address == 0 || length == 0 || length > MAX_INPUT_BYTES {
        return Err(AbiError::new(
            ERR_BOUNDS,
            "input pointer or length is outside ABI limits",
        ));
    }
    let end = address.checked_add(length).ok_or(AbiError::new(
        ERR_BOUNDS,
        "input range overflows address space",
    ))?;
    if end > memory_bytes().ok_or(AbiError::new(ERR_BOUNDS, "cannot inspect module memory"))? {
        return Err(AbiError::new(
            ERR_BOUNDS,
            "input range is outside module memory",
        ));
    }
    if !locked_registry().has_exact_live_span(address, length) {
        return Err(AbiError::new(
            ERR_INVALID_INPUT,
            "input must match a live allocation",
        ));
    }
    let bytes = unsafe { std::slice::from_raw_parts(address as *const u8, length) };
    let mut copy = Vec::new();
    copy.try_reserve_exact(length)
        .map_err(|_| AbiError::new(ERR_INTERNAL, "could not copy bounded input"))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

/// Count JSON nesting and values without allocating, before serde materializes
/// caller-controlled containers. Syntax and UTF-8 are fully checked by serde.
fn preflight_json(bytes: &[u8]) -> Result<(), AbiError> {
    if bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
        return Err(AbiError::new(
            ERR_BOUNDS,
            "JSON input size is outside ABI limits",
        ));
    }
    std::str::from_utf8(bytes)
        .map_err(|_| AbiError::new(ERR_INVALID_INPUT, "JSON input must be UTF-8"))?;
    let mut depth = 0_usize;
    let mut nodes = 0_usize;
    let mut index = 0_usize;
    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\n' | b'\r' | b'\t' | b':' | b',' => index += 1,
            b'{' | b'[' => {
                nodes = nodes
                    .checked_add(1)
                    .ok_or(AbiError::new(ERR_BOUNDS, "JSON is too complex"))?;
                depth += 1;
                if depth > MAX_JSON_DEPTH || nodes > MAX_JSON_NODES {
                    return Err(AbiError::new(
                        ERR_BOUNDS,
                        "JSON nesting or value limit exceeded",
                    ));
                }
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            b'"' => {
                nodes = nodes
                    .checked_add(1)
                    .ok_or(AbiError::new(ERR_BOUNDS, "JSON is too complex"))?;
                if nodes > MAX_JSON_NODES {
                    return Err(AbiError::new(ERR_BOUNDS, "JSON value limit exceeded"));
                }
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    match bytes[index] {
                        b'\\' => index = index.saturating_add(2),
                        b'"' => {
                            index += 1;
                            closed = true;
                            break;
                        }
                        _ => index += 1,
                    }
                }
                if !closed {
                    return Err(AbiError::new(ERR_INVALID_INPUT, "unterminated JSON string"));
                }
            }
            _ => {
                nodes = nodes
                    .checked_add(1)
                    .ok_or(AbiError::new(ERR_BOUNDS, "JSON is too complex"))?;
                if nodes > MAX_JSON_NODES {
                    return Err(AbiError::new(ERR_BOUNDS, "JSON value limit exceeded"));
                }
                while index < bytes.len()
                    && !matches!(
                        bytes[index],
                        b' ' | b'\n' | b'\r' | b'\t' | b',' | b']' | b'}'
                    )
                {
                    index += 1;
                }
            }
        }
    }
    if depth != 0 {
        return Err(AbiError::new(ERR_INVALID_INPUT, "unclosed JSON container"));
    }
    Ok(())
}

fn parse_json<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, AbiError> {
    preflight_json(bytes)?;
    serde_json::from_slice(bytes)
        .map_err(|_| AbiError::new(ERR_INVALID_INPUT, "JSON does not match the expected schema"))
}

fn error_for_core(error: &CoreError) -> AbiError {
    let message = error.0.as_str();
    if message.contains("clock") || message.contains("duration") {
        AbiError::new(
            ERR_CLOCK,
            "host clock was negative, regressed, or had invalid resolution",
        )
    } else if message.contains("checksum") {
        AbiError::new(
            ERR_CHECKSUM,
            "workload checksum did not match the expected value",
        )
    } else if message.contains("max batch") || message.contains("batch size") {
        AbiError::new(ERR_BOUNDS, "collection settings exceed the ABI bounds")
    } else {
        AbiError::new(ERR_INVALID_INPUT, "shared collection rejected the request")
    }
}

fn encode_success<T: Serialize>(value: &T) -> Result<Vec<u8>, AbiError> {
    let output = serde_json::to_vec(&Success { ok: true, value })
        .map_err(|_| AbiError::new(ERR_SERIALIZE, "could not encode successful result"))?;
    if output.len() > MAX_OUTPUT_BYTES {
        return Err(AbiError::new(
            ERR_BOUNDS,
            "result JSON exceeds output limit",
        ));
    }
    Ok(output)
}

fn encode_failure(error: AbiError) -> Vec<u8> {
    serde_json::to_vec(&Failure {
        ok: false,
        error: ErrorBody {
            code: error.code,
            message: error.message,
        },
    })
    .unwrap_or_else(|_| {
        b"{\"ok\":false,\"error\":{\"code\":\"internal_error\",\"message\":\"response encoding failed\"}}".to_vec()
    })
}

fn result_pointer(bytes: Vec<u8>) -> i64 {
    if bytes.is_empty() || bytes.len() > MAX_OUTPUT_BYTES || bytes.len() > u32::MAX as usize {
        return 0;
    }
    let Some(address) = allocate_registered(bytes.len(), AllocationKind::Result) else {
        return 0;
    };
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
    let packed = ((bytes.len() as u64) << 32) | address as u32 as u64;
    packed as i64
}

fn respond<T: Serialize>(result: Result<T, AbiError>) -> i64 {
    match result {
        Ok(value) => match encode_success(&value) {
            Ok(bytes) => result_pointer(bytes),
            Err(error) => result_pointer(encode_failure(error)),
        },
        Err(error) => result_pointer(encode_failure(error)),
    }
}

// `now_ns` is the caller's monotonic host clock. No system clock is linked in.
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "microbenchmark_host")]
extern "C" {
    fn now_ns() -> i64;
    fn run_batch(variant: i32, entries: i32, checksum_out: *mut i64) -> i32;
}

#[derive(Default)]
struct CallbackState {
    remaining: u64,
    failure: Option<AbiError>,
}

#[cfg(target_arch = "wasm32")]
impl CallbackState {
    fn run(&mut self, variant: i32, entries: usize) -> Result<i64, CoreError> {
        let entries_u64 = u64::try_from(entries)
            .map_err(|_| CoreError("entry count exceeds ABI range".to_owned()))?;
        if entries == 0 || entries > MAX_BATCH || entries_u64 > self.remaining {
            self.failure = Some(AbiError::new(
                ERR_BOUNDS,
                "collection entry budget exceeded",
            ));
            return Err(CoreError("collection entry budget exceeded".to_owned()));
        }
        self.remaining -= entries_u64;
        let mut checksum = 0_i64;
        let status = unsafe { run_batch(variant, entries as i32, &mut checksum) };
        if status != 0 {
            self.failure = Some(AbiError::new(ERR_HOST, "host workload callback failed"));
            return Err(CoreError("host workload callback failed".to_owned()));
        }
        Ok(checksum)
    }
}

fn collect_request(bytes: &[u8]) -> Result<Report, AbiError> {
    let request: CollectionRequest = parse_json(bytes)?;
    request
        .config
        .validate()
        .map_err(|_| AbiError::new(ERR_INVALID_INPUT, "collection configuration is invalid"))?;
    if request.config.max_batch > MAX_BATCH {
        return Err(AbiError::new(ERR_BOUNDS, "max_batch exceeds ABI limit"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let state = std::cell::RefCell::new(CallbackState {
            remaining: MAX_TOTAL_ENTRIES,
            failure: None,
        });
        let mut baseline = |entries| state.borrow_mut().run(0, entries);
        let mut candidate = |entries| state.borrow_mut().run(1, entries);
        let mut clock = || unsafe { now_ns() };
        let result = collect(
            request.config,
            request.expected_checksum,
            &mut baseline,
            &mut candidate,
            &mut clock,
        );
        if let Err(error) = result {
            if let Some(callback_error) = state.borrow().failure {
                return Err(callback_error);
            }
            return Err(error_for_core(&error));
        }
        result.map_err(|error| error_for_core(&error))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = request;
        Err(AbiError::new(
            ERR_INTERNAL,
            "host imports are only available in the WASM artifact",
        ))
    }
}

fn analyze_request(bytes: &[u8]) -> Result<Analysis, AbiError> {
    let request: AnalysisRequest = parse_json(bytes)?;
    if request.pairs.is_empty() || request.pairs.len() > MAX_ANALYSIS_PAIRS {
        return Err(AbiError::new(
            ERR_BOUNDS,
            "analysis pair count exceeds ABI limits",
        ));
    }
    analyze(
        &request.pairs,
        request.confidence_percent,
        request.minimum_factor,
    )
    .map_err(|_| AbiError::new(ERR_INVALID_INPUT, "shared analysis rejected the request"))
}

fn replay_request(bytes: &[u8]) -> Result<Report, AbiError> {
    preflight_json(bytes)?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AbiError::new(ERR_INVALID_INPUT, "report JSON must be UTF-8"))?;
    Report::from_json(text).map_err(|_| AbiError::new(ERR_REPORT, "report is invalid or forged"))
}

fn collect_operation(pointer: i32, length: i32) -> i64 {
    let Some(_busy) = BusyGuard::enter() else {
        return 0;
    };
    let result = copy_live_input(pointer, length).and_then(|bytes| collect_request(&bytes));
    respond(result)
}

/// ABI version returned to independent host runtimes.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn abi_version() -> i32 {
    1
}

/// Allocate a zeroed request buffer owned by the host until `dealloc`.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn alloc(byte_len: i32) -> i32 {
    let Some(_busy) = BusyGuard::enter() else {
        return 0;
    };
    let length = byte_len as u32 as usize;
    if length == 0 || length > MAX_INPUT_BYTES {
        return 0;
    }
    allocate_registered(length, AllocationKind::Input)
        .and_then(|address| i32::try_from(address).ok())
        .unwrap_or(0)
}

/// Free a live input or result by its exact pointer and length.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn dealloc(pointer: i32, byte_len: i32) -> i32 {
    let Some(_busy) = BusyGuard::enter() else {
        return 2;
    };
    let address = pointer as u32 as usize;
    let length = byte_len as u32 as usize;
    if address == 0 || length == 0 {
        return 1;
    }
    if release_registered(address, length) {
        0
    } else {
        1
    }
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn collect_json(request_ptr: i32, request_len: i32) -> i64 {
    collect_operation(request_ptr, request_len)
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn analyze_json(request_ptr: i32, request_len: i32) -> i64 {
    let Some(_busy) = BusyGuard::enter() else {
        return 0;
    };
    let result =
        copy_live_input(request_ptr, request_len).and_then(|bytes| analyze_request(&bytes));
    respond(result)
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn replay_json(request_ptr: i32, request_len: i32) -> i64 {
    let Some(_busy) = BusyGuard::enter() else {
        return 0;
    };
    let result = copy_live_input(request_ptr, request_len).and_then(|bytes| replay_request(&bytes));
    respond(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_microbenchmark::{Analysis, Pair};

    #[test]
    fn preflight_accepts_supported_report_depth_and_pair_shape() {
        let json = br#"{"config":{"pairs":1,"confidence_percent":95,"minimum_factor":1.05,"seed":0,"target_ns":1000,"max_batch":10},"batch_size":1,"pairs":[{"baseline_ns":10,"candidate_ns":9,"order":"baseline-first"}],"analysis":{"ratio":0.9,"interval":null,"verdict":"inconclusive"}}"#;
        assert!(preflight_json(json).is_ok());
    }

    #[test]
    fn preflight_rejects_excessive_depth_and_values_without_allocating() {
        let deep = format!(
            "{}0{}",
            "[".repeat(MAX_JSON_DEPTH + 1),
            "]".repeat(MAX_JSON_DEPTH + 1)
        );
        assert_eq!(
            preflight_json(deep.as_bytes()).unwrap_err().code,
            ERR_BOUNDS
        );

        let many = format!("[{}]", vec!["0"; MAX_JSON_NODES].join(","));
        assert_eq!(
            preflight_json(many.as_bytes()).unwrap_err().code,
            ERR_BOUNDS
        );
    }

    #[test]
    fn preflight_checks_utf8_size_and_string_escapes() {
        assert_eq!(preflight_json(&[0xff]).unwrap_err().code, ERR_INVALID_INPUT);
        assert_eq!(
            preflight_json(&vec![b' '; MAX_INPUT_BYTES + 1])
                .unwrap_err()
                .code,
            ERR_BOUNDS
        );
        assert!(preflight_json(br#"{"escaped":"quote: \\" and slash: \\\\"}"#).is_ok());
        assert_eq!(
            preflight_json(br#"{"unterminated":"x}"#).unwrap_err().code,
            ERR_INVALID_INPUT
        );
    }

    #[test]
    fn analysis_request_uses_shared_rank_statistics() {
        let json = br#"{"confidence_percent":95,"minimum_factor":1.05,"pairs":[{"baseline_ns":100,"candidate_ns":50,"order":"baseline-first"},{"baseline_ns":100,"candidate_ns":50,"order":"candidate-first"},{"baseline_ns":100,"candidate_ns":50,"order":"baseline-first"}]}"#;
        let analysis = analyze_request(json).unwrap();
        assert_eq!(analysis.ratio, 0.5);
        assert_eq!(analysis.verdict, "inconclusive");
    }

    #[test]
    fn analysis_request_rejects_duplicate_unknown_and_oversized_fields() {
        let duplicate = br#"{"confidence_percent":95,"confidence_percent":95,"minimum_factor":1.05,"pairs":[{"baseline_ns":2,"candidate_ns":1,"order":"baseline-first"}]}"#;
        assert_eq!(
            analyze_request(duplicate).unwrap_err().code,
            ERR_INVALID_INPUT
        );
        let unknown =
            br#"{"confidence_percent":95,"minimum_factor":1.05,"pairs":[],"unexpected":true}"#;
        assert_eq!(
            analyze_request(unknown).unwrap_err().code,
            ERR_INVALID_INPUT
        );
        let pairs = vec![
            Pair {
                baseline_ns: 1,
                candidate_ns: 1,
                order: "baseline-first".into(),
            };
            MAX_ANALYSIS_PAIRS + 1
        ];
        let json = serde_json::to_vec(&AnalysisRequest {
            confidence_percent: 95,
            minimum_factor: 1.05,
            pairs,
        })
        .unwrap();
        assert_eq!(analyze_request(&json).unwrap_err().code, ERR_BOUNDS);
    }

    #[test]
    fn replay_rejects_forged_analysis_and_accepts_core_report() {
        let report = Report {
            config: Config {
                pairs: 1,
                confidence_percent: 95,
                minimum_factor: 1.05,
                seed: 0,
                target_ns: 100,
                max_batch: 1,
            },
            batch_size: 1,
            pairs: vec![Pair {
                baseline_ns: 10,
                candidate_ns: 9,
                order: "baseline-first".into(),
            }],
            analysis: analyze(
                &[Pair {
                    baseline_ns: 10,
                    candidate_ns: 9,
                    order: "baseline-first".into(),
                }],
                95,
                1.05,
            )
            .unwrap(),
        };
        let json = report.to_json().unwrap();
        assert!(replay_request(json.as_bytes()).is_ok());
        let forged = json.replace("0.9", "2.0");
        assert_eq!(
            replay_request(forged.as_bytes()).unwrap_err().code,
            ERR_REPORT
        );
        assert_eq!(replay_request(b"{]").unwrap_err().code, ERR_REPORT);
    }

    #[test]
    fn registry_rejects_invalid_free_and_enforces_caps() {
        let mut registry = Registry::new();
        assert!(registry.insert(16, 8, AllocationKind::Input));
        assert!(!registry.insert(24, MAX_INPUT_LIVE_BYTES, AllocationKind::Input));
        assert!(registry.has_exact_live_span(16, 8));
        assert_eq!(registry.remove(16, 7), None);
        assert_eq!(registry.live_bytes, 8);
        assert_eq!(registry.remove(16, 8), Some(AllocationKind::Input));
        assert!(!registry.has_exact_live_span(16, 8));
    }

    #[test]
    fn registry_rejects_the_thirty_third_live_allocation() {
        let mut registry = Registry::new();
        for index in 0..MAX_ALLOCATION_RECORDS {
            assert!(registry.insert(16 + index * 8, 1, AllocationKind::Input));
        }
        assert!(!registry.insert(16 + MAX_ALLOCATION_RECORDS * 8, 1, AllocationKind::Input));
        assert_eq!(registry.live_bytes, MAX_ALLOCATION_RECORDS);
        assert_eq!(registry.input_bytes, MAX_ALLOCATION_RECORDS);
    }

    #[test]
    fn success_and_failure_responses_are_structured_json() {
        let analysis = Analysis {
            ratio: 1.0,
            interval: None,
            verdict: "inconclusive".into(),
        };
        let success = encode_success(&analysis).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&success).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["value"]["verdict"], "inconclusive");
        let failure = encode_failure(AbiError::new(ERR_BOUNDS, "limit"));
        let parsed: serde_json::Value = serde_json::from_slice(&failure).unwrap();
        assert_eq!(parsed["error"]["code"], ERR_BOUNDS);
    }
}
