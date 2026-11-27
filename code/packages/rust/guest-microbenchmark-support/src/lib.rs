//! Shared command, envelope, and strict replay support for guest-runtime lanes.
//!
//! The nested VER07 report remains a portable closed object. Runtime-specific
//! facts live next to it in a versioned envelope so another language can extract
//! and replay the unmodified core report.

use coding_adventures_microbenchmark::{Config, Report};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Instant;

pub const ENVELOPE_SCHEMA: &str = "guest-microbenchmark-envelope-v1";
pub const MAX_ENVELOPE_BYTES: usize = 4 * 1024 * 1024;
pub const DEFAULT_EMPTY_ENTRY_SAMPLES: usize = 21;
pub const DEFAULT_EMPTY_ENTRY_TARGET_NS: i64 = 100_000;
pub const DEFAULT_EMPTY_ENTRY_MAX_BATCH: usize = 65_536;

/// A local support error. Workload-specific failures are converted at the boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn error(message: impl Into<String>) -> Error {
    Error(message.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMetadata {
    pub revision: String,
    pub host_runtime: String,
    pub guest_runtime: String,
    pub build_profile: String,
    pub os: String,
    pub arch: String,
    pub source_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GuestEnvelope {
    pub schema: String,
    pub guest: String,
    pub workload: String,
    /// CLI arguments excluding argv[0], which is a machine-specific path.
    pub invocation: Vec<String>,
    pub baseline_variant: String,
    pub candidate_variant: String,
    pub setup_boundary: String,
    pub timing_boundary: String,
    /// Host clock/no-op measurements are separate from guest call measurements.
    pub host_clock_noop: EmptyEntryProbe,
    /// Repeated empty guest calls use the same wrapper path as the main workload.
    pub empty_guest_call: EmptyEntryProbe,
    pub metadata: RuntimeMetadata,
    pub report: Report,
}

/// Raw repeated-entry timing data. Per-entry point estimates are derived by
/// dividing each raw duration by `entries_per_sample`; nothing is subtracted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmptyEntryProbe {
    pub method: String,
    pub target_ns: i64,
    pub max_entries_per_sample: usize,
    pub sample_count: usize,
    pub entries_per_sample: usize,
    pub raw_elapsed_ns: Vec<i64>,
}

impl GuestEnvelope {
    // Keeping the portable report and each envelope component explicit makes
    // accidental omission at call sites visible; the signature mirrors schema.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        guest: &str,
        workload: &str,
        invocation: Vec<String>,
        baseline_variant: &str,
        candidate_variant: &str,
        setup_boundary: &str,
        timing_boundary: &str,
        host_clock_noop: EmptyEntryProbe,
        empty_guest_call: EmptyEntryProbe,
        metadata: RuntimeMetadata,
        report: Report,
    ) -> Result<Self, Error> {
        let value = Self {
            schema: ENVELOPE_SCHEMA.to_owned(),
            guest: guest.to_owned(),
            workload: workload.to_owned(),
            invocation,
            baseline_variant: baseline_variant.to_owned(),
            candidate_variant: candidate_variant.to_owned(),
            setup_boundary: setup_boundary.to_owned(),
            timing_boundary: timing_boundary.to_owned(),
            host_clock_noop,
            empty_guest_call,
            metadata,
            report,
        };
        value.validate(guest, workload)?;
        Ok(value)
    }

    pub fn validate(&self, expected_guest: &str, expected_workload: &str) -> Result<(), Error> {
        for (label, value) in [
            ("guest", self.guest.as_str()),
            ("workload", self.workload.as_str()),
            ("baseline_variant", self.baseline_variant.as_str()),
            ("candidate_variant", self.candidate_variant.as_str()),
            ("setup_boundary", self.setup_boundary.as_str()),
            ("timing_boundary", self.timing_boundary.as_str()),
            (
                "host_clock_noop.method",
                self.host_clock_noop.method.as_str(),
            ),
            (
                "empty_guest_call.method",
                self.empty_guest_call.method.as_str(),
            ),
            ("revision", self.metadata.revision.as_str()),
            ("host_runtime", self.metadata.host_runtime.as_str()),
            ("guest_runtime", self.metadata.guest_runtime.as_str()),
            ("build_profile", self.metadata.build_profile.as_str()),
            ("os", self.metadata.os.as_str()),
            ("arch", self.metadata.arch.as_str()),
            ("source_identity", self.metadata.source_identity.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(error(format!("{label} must not be blank")));
            }
        }
        if self.schema != ENVELOPE_SCHEMA {
            return Err(error("unsupported guest envelope schema"));
        }
        if self.guest != expected_guest || self.workload != expected_workload {
            return Err(error("guest envelope does not match this driver"));
        }
        if self
            .invocation
            .iter()
            .any(|argument| argument.contains('\0'))
        {
            return Err(error(
                "guest envelope invocation contains an invalid argument",
            ));
        }
        for probe in [&self.host_clock_noop, &self.empty_guest_call] {
            if probe.target_ns <= 0
                || probe.max_entries_per_sample == 0
                || probe.sample_count == 0
                || probe.sample_count != probe.raw_elapsed_ns.len()
                || probe.entries_per_sample == 0
                || probe.entries_per_sample > probe.max_entries_per_sample
                || probe.raw_elapsed_ns.iter().any(|duration| *duration < 0)
            {
                return Err(error("empty-entry probe has invalid bounds or samples"));
            }
        }
        // Re-run the portable validator instead of trusting the nested typed value.
        let core = self
            .report
            .to_json()
            .map_err(|e| error(format!("could not serialize nested report: {e}")))?;
        Report::from_json(&core).map_err(|e| error(format!("invalid nested report: {e}")))?;
        Ok(())
    }
}

/// Replay one envelope while rejecting duplicate names before serde can discard them.
pub fn replay_envelope(
    source: &str,
    expected_guest: &str,
    expected_workload: &str,
) -> Result<GuestEnvelope, Error> {
    if source.len() > MAX_ENVELOPE_BYTES {
        return Err(error("guest envelope exceeds 4 MiB"));
    }
    let UniqueJsonValue(value) = serde_json::from_str(source)
        .map_err(|e| error(format!("invalid guest envelope JSON: {e}")))?;
    let envelope: GuestEnvelope =
        serde_json::from_value(value).map_err(|e| error(format!("invalid guest envelope: {e}")))?;
    envelope.validate(expected_guest, expected_workload)?;
    Ok(envelope)
}

/// Replay a portable core report while rejecting duplicate members on its raw JSON text.
pub fn replay_core_report(source: &str) -> Result<Report, Error> {
    if source.len() > MAX_ENVELOPE_BYTES {
        return Err(error("microbenchmark report exceeds 4 MiB"));
    }
    let UniqueJsonValue(value) = serde_json::from_str(source)
        .map_err(|e| error(format!("invalid microbenchmark report JSON: {e}")))?;
    let report: Report = serde_json::from_value(value)
        .map_err(|e| error(format!("invalid microbenchmark report: {e}")))?;
    let canonical = report.to_json().map_err(|e| error(e.to_string()))?;
    Report::from_json(&canonical).map_err(|e| error(format!("invalid microbenchmark report: {e}")))
}

pub fn write_envelope(envelope: &GuestEnvelope, output: &mut impl Write) -> Result<(), Error> {
    serde_json::to_writer_pretty(&mut *output, envelope)
        .map_err(|e| error(format!("could not encode guest envelope: {e}")))?;
    output
        .write_all(b"\n")
        .map_err(|e| error(format!("could not write guest envelope: {e}")))?;
    output
        .flush()
        .map_err(|e| error(format!("could not flush guest envelope: {e}")))
}

pub fn write_outputs(
    envelope: &GuestEnvelope,
    manifest: &mut impl Write,
    report_output: &mut impl Write,
) -> Result<(), Error> {
    write_envelope(envelope, manifest)?;
    let report = envelope
        .report
        .to_json()
        .map_err(|e| error(e.to_string()))?;
    report_output
        .write_all(report.as_bytes())
        .map_err(|e| error(format!("could not write core report: {e}")))?;
    report_output
        .flush()
        .map_err(|e| error(format!("could not flush core report: {e}")))
}

pub fn write_manifest_file(envelope: &GuestEnvelope, path: &str) -> Result<(), Error> {
    let mut file = File::create(path).map_err(|e| error(format!("cannot create manifest: {e}")))?;
    write_envelope(envelope, &mut file)
}

pub fn replay_file(
    path: &str,
    expected_guest: &str,
    expected_workload: &str,
) -> Result<GuestEnvelope, Error> {
    let mut file = File::open(path).map_err(|e| error(format!("cannot open replay file: {e}")))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_ENVELOPE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| error(format!("cannot read replay file: {e}")))?;
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(error("guest envelope exceeds 4 MiB"));
    }
    let source =
        std::str::from_utf8(&bytes).map_err(|e| error(format!("envelope is not UTF-8: {e}")))?;
    replay_envelope(source, expected_guest, expected_workload)
}

pub fn replay_core_file(path: &str) -> Result<Report, Error> {
    let mut file = File::open(path).map_err(|e| error(format!("cannot open report file: {e}")))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_ENVELOPE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| error(format!("cannot read report file: {e}")))?;
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(error("microbenchmark report exceeds 4 MiB"));
    }
    let source =
        std::str::from_utf8(&bytes).map_err(|e| error(format!("report is not UTF-8: {e}")))?;
    replay_core_report(source)
}

/// CLI options shared by both guest executables.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help,
    ReplayCore(PathBuf),
    ReplayEnvelope(PathBuf),
    Run {
        config: Config,
        revision: Option<String>,
        manifest: PathBuf,
    },
}

pub fn parse_args(args: &[String]) -> Result<Command, Error> {
    let mut config = Config::default();
    let mut revision = None;
    let mut replay = None;
    let mut manifest = None;
    let mut index = 1;
    while index < args.len() {
        let flag = &args[index];
        if flag == "--help" || flag == "-h" {
            if args.len() != 2 {
                return Err(error("--help must be used alone"));
            }
            return Ok(Command::Help);
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| error(format!("{flag} requires a value")))?;
        let invalid = |name: &str| error(format!("invalid {name}"));
        match flag.as_str() {
            "--pairs" => config.pairs = value.parse().map_err(|_| invalid("--pairs"))?,
            "--confidence" => {
                config.confidence_percent = value.parse().map_err(|_| invalid("--confidence"))?
            }
            "--factor" => config.minimum_factor = value.parse().map_err(|_| invalid("--factor"))?,
            "--seed" => config.seed = value.parse().map_err(|_| invalid("--seed"))?,
            "--target-ns" => {
                config.target_ns = value.parse().map_err(|_| invalid("--target-ns"))?
            }
            "--max-batch" => {
                config.max_batch = value.parse().map_err(|_| invalid("--max-batch"))?
            }
            "--revision" => {
                if value.trim().is_empty() {
                    return Err(error("--revision must not be blank"));
                }
                revision = Some(value.clone());
            }
            "--replay" => replay = Some(PathBuf::from(value)),
            "--replay-envelope" => replay = Some(PathBuf::from(value)),
            "--manifest" => {
                if value.trim().is_empty() {
                    return Err(error("--manifest path must not be blank"));
                }
                manifest = Some(PathBuf::from(value));
            }
            _ => {
                return Err(error(format!(
                    "unknown option or positional argument: {flag}"
                )))
            }
        }
        index += 2;
    }
    if let Some(path) = replay {
        if revision.is_some()
            || manifest.is_some()
            || config != Config::default()
            || args.len() != 3
        {
            return Err(error("--replay must be used alone"));
        }
        return Ok(if args[1] == "--replay" {
            Command::ReplayCore(path)
        } else {
            Command::ReplayEnvelope(path)
        });
    }
    let manifest = manifest.ok_or_else(|| error("--manifest FILE is required for a guest run"))?;
    config.validate().map_err(|e| error(e.to_string()))?;
    Ok(Command::Run {
        config,
        revision,
        manifest,
    })
}

pub fn select_revision(
    explicit: Option<String>,
    git_revision: Option<&str>,
    github_sha: Option<&str>,
) -> Result<String, Error> {
    if let Some(value) = explicit {
        if value.trim().is_empty() {
            return Err(error("--revision must not be blank"));
        }
        return Ok(value);
    }
    Ok(git_revision
        .filter(|value| !value.trim().is_empty())
        .or_else(|| github_sha.filter(|value| !value.trim().is_empty()))
        .unwrap_or("unknown")
        .to_owned())
}

pub fn runtime_metadata(
    guest_runtime: &str,
    source_identity: &str,
    revision: String,
) -> RuntimeMetadata {
    RuntimeMetadata {
        revision,
        // Runtime metadata must not execute or query a compiler. Callers that
        // need a compiler label can supply one in a future explicit field;
        // until then, `unknown` is truthful and portable.
        host_runtime: "unknown".to_owned(),
        guest_runtime: guest_runtime.to_owned(),
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_owned(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        source_identity: source_identity.to_owned(),
    }
}

/// Convert one process-relative monotonic timestamp to the collector's signed clock.
pub fn elapsed_ns(start: &Instant) -> i64 {
    i64::try_from(start.elapsed().as_nanos()).unwrap_or(-1)
}

/// Calibrate repeated entries and retain raw batch durations. A zero is valid
/// evidence of clock resolution for these diagnostics; main report samples use
/// the collector's stricter positive-duration rule.
pub fn sample_empty_entries(
    method: &str,
    target_ns: i64,
    max_entries_per_sample: usize,
    sample_count: usize,
    now: &mut impl FnMut() -> i64,
    entry: &mut impl FnMut() -> Result<(), Error>,
) -> Result<EmptyEntryProbe, Error> {
    if method.trim().is_empty()
        || target_ns <= 0
        || max_entries_per_sample == 0
        || sample_count == 0
        || sample_count > 1_000
    {
        return Err(error(
            "empty-entry probe configuration is outside its supported range",
        ));
    }
    let mut last_tick = None;
    let mut run_batch = |entries: usize| -> Result<i64, Error> {
        let start = now();
        if start < 0 || last_tick.is_some_and(|last| start < last) {
            return Err(error("empty-entry clock is negative or regressed"));
        }
        last_tick = Some(start);
        for _ in 0..entries {
            entry()?;
        }
        let finish = now();
        if finish < 0 || finish < start || last_tick.is_some_and(|last| finish < last) {
            return Err(error("empty-entry clock is negative or regressed"));
        }
        last_tick = Some(finish);
        Ok(finish - start)
    };
    let mut entries_per_sample = 1usize;
    loop {
        if run_batch(entries_per_sample)? >= target_ns
            || entries_per_sample == max_entries_per_sample
        {
            break;
        }
        entries_per_sample = entries_per_sample
            .checked_mul(2)
            .unwrap_or(max_entries_per_sample)
            .min(max_entries_per_sample);
    }
    let mut raw_elapsed_ns = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        raw_elapsed_ns.push(run_batch(entries_per_sample)?);
    }
    Ok(EmptyEntryProbe {
        method: method.to_owned(),
        target_ns,
        max_entries_per_sample,
        sample_count,
        entries_per_sample,
        raw_elapsed_ns,
    })
}

/// Host-only clock/no-op probe. It is not a guest-entry measurement.
pub fn sample_host_clock_noop(now: &mut impl FnMut() -> i64) -> Result<EmptyEntryProbe, Error> {
    sample_empty_entries(
        "black_box no-op between two host clock reads; no guest runtime call",
        DEFAULT_EMPTY_ENTRY_TARGET_NS,
        DEFAULT_EMPTY_ENTRY_MAX_BATCH,
        DEFAULT_EMPTY_ENTRY_SAMPLES,
        now,
        &mut || {
            std::hint::black_box(());
            Ok(())
        },
    )
}

// A recursive Serde value visitor retains the full JSON syntax rules while
// tracking decoded member names before a map insertion could overwrite them.
struct UniqueJsonValue(Value);

impl<'de> Deserialize<'de> for UniqueJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonValueVisitor)
    }
}

struct UniqueJsonValueVisitor;

impl<'de> Visitor<'de> for UniqueJsonValueVisitor {
    type Value = UniqueJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON with unique object member names")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Bool(value)))
    }
    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Number(Number::from(value))))
    }
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Number(Number::from(value))))
    }
    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(|value| UniqueJsonValue(Value::Number(value)))
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::String(value.to_owned())))
    }
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::String(value)))
    }
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Null))
    }
    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(UniqueJsonValue(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(UniqueJsonValue(Value::Array(values)))
    }
    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut names = HashSet::new();
        while let Some(name) = object.next_key::<String>()? {
            if !names.insert(name.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object member: {name:?}"
                )));
            }
            let UniqueJsonValue(value) = object.next_value()?;
            values.insert(name, value);
        }
        Ok(UniqueJsonValue(Value::Object(values)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_metadata_does_not_query_the_build_compiler() {
        let metadata = runtime_metadata("guest/1", "source", "revision".to_owned());
        assert_eq!(metadata.host_runtime, "unknown");
    }
    use coding_adventures_microbenchmark::{collect, Batch};

    fn sample_report() -> Report {
        let config = Config {
            pairs: 1,
            target_ns: 1,
            max_batch: 1,
            ..Config::default()
        };
        let mut baseline: &mut Batch<'_> = &mut |count| Ok(count as i64);
        let mut candidate: &mut Batch<'_> = &mut |count| Ok(count as i64);
        let mut tick = 0i64;
        let mut now = || {
            tick += 1;
            tick
        };
        collect(config, 1, &mut baseline, &mut candidate, &mut now).unwrap()
    }

    fn probe(method: &str) -> EmptyEntryProbe {
        EmptyEntryProbe {
            method: method.into(),
            target_ns: 1,
            max_entries_per_sample: 1,
            sample_count: 1,
            entries_per_sample: 1,
            raw_elapsed_ns: vec![0],
        }
    }

    fn envelope() -> GuestEnvelope {
        GuestEnvelope::new(
            "test-guest",
            "test-workload",
            vec!["--pairs".into(), "1".into()],
            "vector",
            "string",
            "setup",
            "timed body",
            probe("host noop"),
            probe("empty guest call"),
            RuntimeMetadata {
                revision: "rev".into(),
                host_runtime: "rustc test".into(),
                guest_runtime: "guest 0.1".into(),
                build_profile: "test".into(),
                os: "test-os".into(),
                arch: "test-arch".into(),
                source_identity: "test-source-v1".into(),
            },
            sample_report(),
        )
        .unwrap()
    }

    #[test]
    fn envelope_round_trips_and_validates_nested_report() {
        let encoded = serde_json::to_string(&envelope()).unwrap();
        let decoded = replay_envelope(&encoded, "test-guest", "test-workload").unwrap();
        assert_eq!(decoded.report.pairs.len(), 1);
        assert_eq!(decoded.invocation, vec!["--pairs", "1"]);
        let empty_invocation =
            encoded.replace("\"invocation\":[\"--pairs\",\"1\"]", "\"invocation\":[]");
        assert!(replay_envelope(&empty_invocation, "test-guest", "test-workload").is_ok());
        let missing_invocation = encoded.replace(",\"invocation\":[\"--pairs\",\"1\"]", "");
        assert!(replay_envelope(&missing_invocation, "test-guest", "test-workload").is_err());
        let wrong_type =
            encoded.replace("\"invocation\":[\"--pairs\",\"1\"]", "\"invocation\":[1]");
        assert!(replay_envelope(&wrong_type, "test-guest", "test-workload").is_err());
        let invalid_argument = encoded.replace(
            "\"invocation\":[\"--pairs\",\"1\"]",
            "\"invocation\":[\"\\u0000\"]",
        );
        assert!(replay_envelope(&invalid_argument, "test-guest", "test-workload").is_err());
        assert!(replay_envelope(&encoded, "wrong-guest", "test-workload").is_err());
        assert!(replay_envelope(
            &encoded.replace("test-workload", "other-workload"),
            "test-guest",
            "test-workload"
        )
        .is_err());
    }

    #[test]
    fn envelope_replay_rejects_duplicate_alias_unknown_and_malformed_json() {
        let encoded = serde_json::to_string(&envelope()).unwrap();
        let duplicate = encoded.replacen(
            "\"guest\":\"test-guest\"",
            "\"guest\":\"test-guest\",\"guest\":\"test-guest\"",
            1,
        );
        assert!(replay_envelope(&duplicate, "test-guest", "test-workload").is_err());
        let alias = encoded.replacen(
            "\"guest\":\"test-guest\"",
            "\"guest\":\"test-guest\",\"gues\\u0074\":\"test-guest\"",
            1,
        );
        assert!(replay_envelope(&alias, "test-guest", "test-workload").is_err());
        let nested_duplicate =
            encoded.replace("\"baseline_ns\":", "\"baseline_ns\":0,\"baseline_ns\":");
        assert!(replay_envelope(&nested_duplicate, "test-guest", "test-workload").is_err());
        let unknown = encoded.replacen("\"guest\":", "\"unexpected\":true,\"guest\":", 1);
        assert!(replay_envelope(&unknown, "test-guest", "test-workload").is_err());
        assert!(replay_envelope("{", "test-guest", "test-workload").is_err());
        assert!(replay_envelope(
            &" ".repeat(MAX_ENVELOPE_BYTES + 1),
            "test-guest",
            "test-workload"
        )
        .is_err());
    }

    #[test]
    fn raw_core_report_replay_rejects_duplicates_before_object_materialization() {
        let report = envelope().report.to_json().unwrap();
        let duplicate = report.replacen("\"pairs\": 1,", "\"pairs\": 1, \"pairs\": 1,", 1);
        let alias = report.replacen("\"pairs\": 1,", "\"pairs\": 1, \"pa\\u0069rs\": 1,", 1);
        assert!(replay_core_report(&duplicate).is_err());
        assert!(replay_core_report(&alias).is_err());
        assert!(replay_core_report(&report).is_ok());
        assert!(replay_core_report("{").is_err());
    }

    #[test]
    fn file_replay_preserves_duplicate_detection_on_original_json_text() {
        let core_path =
            std::env::temp_dir().join(format!("guest-core-duplicate-{}.json", std::process::id()));
        let envelope_path = std::env::temp_dir().join(format!(
            "guest-envelope-duplicate-{}.json",
            std::process::id()
        ));
        let core = envelope().report.to_json().unwrap();
        std::fs::write(
            &core_path,
            core.replacen("\"pairs\": 1,", "\"pairs\": 1, \"pairs\": 1,", 1),
        )
        .unwrap();
        let encoded = serde_json::to_string(&envelope()).unwrap();
        std::fs::write(
            &envelope_path,
            encoded.replacen(
                "\"guest\":\"test-guest\"",
                "\"guest\":\"test-guest\",\"guest\":\"test-guest\"",
                1,
            ),
        )
        .unwrap();
        assert!(replay_core_file(core_path.to_str().unwrap()).is_err());
        assert!(replay_file(
            envelope_path.to_str().unwrap(),
            "test-guest",
            "test-workload"
        )
        .is_err());
        std::fs::remove_file(core_path).unwrap();
        std::fs::remove_file(envelope_path).unwrap();
    }

    #[test]
    fn options_enforce_defaults_bounds_and_revision_fallback() {
        let defaults =
            parse_args(&["bench".into(), "--manifest".into(), "guest.json".into()]).unwrap();
        assert_eq!(
            defaults,
            Command::Run {
                config: Config::default(),
                revision: None,
                manifest: PathBuf::from("guest.json")
            }
        );
        assert!(parse_args(&["bench".into()]).is_err());
        assert!(parse_args(&["bench".into(), "--pairs".into(), "10001".into()]).is_err());
        assert!(parse_args(&["bench".into(), "--revision".into(), " ".into()]).is_err());
        assert_eq!(
            select_revision(None, Some(" "), Some("sha")).unwrap(),
            "sha"
        );
        assert_eq!(
            select_revision(None, Some(" "), Some(" ")).unwrap(),
            "unknown"
        );
        assert!(select_revision(Some("\t".into()), Some("sha"), None).is_err());
        assert!(parse_args(&[
            "bench".into(),
            "--help".into(),
            "--pairs".into(),
            "2".into()
        ])
        .is_err());
        assert!(matches!(
            parse_args(&["bench".into(), "--replay".into(), "report.json".into()]).unwrap(),
            Command::ReplayCore(_)
        ));
        assert!(matches!(
            parse_args(&[
                "bench".into(),
                "--replay-envelope".into(),
                "manifest.json".into()
            ])
            .unwrap(),
            Command::ReplayEnvelope(_)
        ));
    }

    #[test]
    fn empty_entry_probe_calibrates_and_preserves_raw_samples_including_zero() {
        let tick = std::cell::Cell::new(0i64);
        let mut calls = 0usize;
        let probe = sample_empty_entries(
            "fake guest entry",
            20,
            8,
            3,
            &mut || {
                let before = tick.get();
                tick.set(before + 1);
                before
            },
            &mut || {
                calls += 1;
                tick.set(tick.get() + 4);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(probe.entries_per_sample, 8);
        assert_eq!(probe.raw_elapsed_ns, vec![33, 33, 33]);
        assert_eq!(calls, 39);
        let mut zero_tick = || 10;
        let zeroes = sample_empty_entries(
            "clock resolution",
            100,
            2,
            2,
            &mut zero_tick,
            &mut || Ok(()),
        )
        .unwrap();
        assert_eq!(zeroes.raw_elapsed_ns, vec![0, 0]);
        assert_eq!(zeroes.entries_per_sample, 2);
        let mut ticks = [3, 2].into_iter();
        assert!(sample_empty_entries(
            "regression",
            1,
            1,
            1,
            &mut || ticks.next().unwrap_or(2),
            &mut || Ok(())
        )
        .is_err());
        let mut progress = 0i64;
        assert!(sample_empty_entries(
            "callback error",
            1,
            1,
            1,
            &mut || {
                progress += 1;
                progress
            },
            &mut || Err(error("fake entry failed")),
        )
        .is_err());
    }

    #[test]
    fn output_write_errors_are_reported() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        assert!(write_envelope(&envelope(), &mut Broken).is_err());
    }
}
