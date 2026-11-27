//! Paired batch collection and replayable analysis for VER07.
//!
//! A benchmark comparison is a fixed plan: verify both variants, calibrate a
//! common batch size, then time adjacent baseline/candidate batches in a
//! balanced seeded order. Raw pairs are retained so a report can be replayed
//! without trusting its precomputed conclusion.

use coding_adventures_rank_statistics::{median, median_interval, Interval};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::HashSet;
use std::fmt;

pub const MAX_PAIRS: usize = 10_000;
pub const MAX_I64: i64 = i64::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// A JSON value decoded while rejecting duplicate decoded object member names.
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
        formatter.write_str("a JSON value with unique object member names")
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
            .map(|number| UniqueJsonValue(Value::Number(number)))
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

fn error(message: impl Into<String>) -> Error {
    Error(message.into())
}

fn valid_workload_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\0')
        && !value
            .bytes()
            .all(|byte| byte == 0x20 || (0x09..=0x0d).contains(&byte))
}

fn valid_metadata_text(value: &str) -> bool {
    !value.is_empty() && !value.contains('\0')
}

fn validate_projected_metadata(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Error> {
    const GROUP: [&str; 6] = [
        "workload",
        "invocation",
        "revision",
        "runtime",
        "os",
        "arch",
    ];
    let present = GROUP
        .iter()
        .filter(|key| object.contains_key(**key))
        .count();
    let has_profile = object.contains_key("profile");
    if present == 0 {
        return if has_profile {
            Err(error(
                "profile metadata requires the complete metadata group",
            ))
        } else {
            Ok(())
        };
    }
    if present != GROUP.len() {
        return Err(error("report metadata group must contain all six fields"));
    }

    let workload = object
        .get("workload")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| error("workload metadata must be a string"))?;
    if !valid_workload_identifier(workload) {
        return Err(error(
            "workload must be a nonblank UTF-8 identifier without NUL",
        ));
    }

    for key in ["revision", "runtime", "os", "arch"] {
        let value = object
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| error(format!("{key} metadata must be a string")))?;
        if !valid_metadata_text(value) {
            return Err(error(format!(
                "{key} metadata must be non-empty and contain no NUL"
            )));
        }
    }

    let invocation = object
        .get("invocation")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| error("invocation metadata must be an array of strings"))?;
    if invocation.iter().any(|item| match item.as_str() {
        Some(value) => value.contains('\0'),
        None => true,
    }) {
        return Err(error(
            "invocation metadata must contain UTF-8 strings without NUL",
        ));
    }

    if has_profile {
        let profile = object
            .get("profile")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| error("profile metadata must be a string"))?;
        if !valid_metadata_text(profile) {
            return Err(error(
                "profile metadata must be non-empty and contain no NUL",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub pairs: usize,
    pub confidence_percent: u8,
    pub minimum_factor: f64,
    pub seed: i64,
    pub target_ns: i64,
    pub max_batch: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pairs: 40,
            confidence_percent: 95,
            minimum_factor: 1.05,
            seed: 0,
            target_ns: 100_000,
            max_batch: 65_536,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), Error> {
        if !(1..=MAX_PAIRS).contains(&self.pairs) {
            return Err(error("pairs must be from 1 through 10000"));
        }
        if !(1..=99).contains(&self.confidence_percent) {
            return Err(error("confidence must be from 1 through 99"));
        }
        if !self.minimum_factor.is_finite() || self.minimum_factor <= 1.0 {
            return Err(error("factor must be finite and greater than 1"));
        }
        if self.target_ns <= 0 || self.max_batch == 0 || self.max_batch as u128 > i64::MAX as u128 {
            return Err(error(
                "target and max batch must be positive signed 64-bit integers",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub baseline_ns: i64,
    pub candidate_ns: i64,
    pub order: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RatioInterval(pub f64, pub f64);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Analysis {
    pub ratio: f64,
    pub interval: Option<RatioInterval>,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub config: Config,
    pub batch_size: usize,
    pub pairs: Vec<Pair>,
    pub analysis: Analysis,
}

/// Analyze paired observations using VER07b's exact interval and verdict rules.
pub fn analyze(
    pairs: &[Pair],
    confidence_percent: u8,
    minimum_factor: f64,
) -> Result<Analysis, Error> {
    if pairs.is_empty() {
        return Err(error("at least one pair is required"));
    }
    if !(1..=99).contains(&confidence_percent) {
        return Err(error("confidence must be from 1 through 99"));
    }
    if !minimum_factor.is_finite() || minimum_factor <= 1.0 {
        return Err(error("factor must be finite and greater than 1"));
    }
    if pairs.len() > MAX_PAIRS {
        return Err(error("pairs must contain at most 10000 observations"));
    }

    let mut ratios = Vec::with_capacity(pairs.len());
    for (index, pair) in pairs.iter().enumerate() {
        if pair.baseline_ns <= 0 || pair.candidate_ns <= 0 {
            return Err(error(format!("pair {index} durations must be positive")));
        }
        if pair.order != "baseline-first" && pair.order != "candidate-first" {
            return Err(error(format!("pair {index} has invalid order")));
        }
        // The contract explicitly converts both i64 values before division.
        ratios.push((pair.candidate_ns as f64) / (pair.baseline_ns as f64));
    }

    let ratio = median(&ratios).map_err(|e| error(e.to_string()))?;
    let interval =
        median_interval(&ratios, confidence_percent).map_err(|e| error(e.to_string()))?;
    let mut result = Analysis {
        ratio,
        interval: interval.map(|i| RatioInterval(i.low, i.high)),
        verdict: "inconclusive".to_owned(),
    };
    if let Some(Interval { low, high }) = interval {
        let faster_boundary = 1.0 / minimum_factor;
        match () {
            _ if high < faster_boundary => result.verdict = "faster".to_owned(),
            _ if low > minimum_factor => result.verdict = "slower".to_owned(),
            _ if low >= faster_boundary && high <= minimum_factor => {
                result.verdict = "equivalent".to_owned()
            }
            _ => {}
        }
    }
    Ok(result)
}

impl Report {
    /// Serialize the full report as stable, pretty JSON followed by a newline.
    pub fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string_pretty(self)
            .map(|mut json| {
                json.push('\n');
                json
            })
            .map_err(|e| error(format!("could not serialize report: {e}")))
    }

    /// Parse and reanalyze raw pairs, rejecting a stale or forged analysis.
    pub fn from_json(json: &str) -> Result<Self, Error> {
        let UniqueJsonValue(mut document) =
            serde_json::from_str(json).map_err(|e| error(format!("invalid report JSON: {e}")))?;
        // The executable adds descriptive metadata to the same report object.
        // Remove only those documented fields; serde rejects everything else.
        if let Some(object) = document.as_object_mut() {
            validate_projected_metadata(object)?;
            if object
                .get("pairs")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|pairs| pairs.len() > MAX_PAIRS)
            {
                return Err(error("report pair count exceeds 10000"));
            }
            for key in [
                "workload",
                "invocation",
                "revision",
                "runtime",
                "profile",
                "os",
                "arch",
            ] {
                object.remove(key);
            }
        }
        let report: Self = serde_json::from_value(document)
            .map_err(|e| error(format!("invalid report JSON: {e}")))?;
        report.config.validate()?;
        if report.batch_size == 0 || report.batch_size > report.config.max_batch {
            return Err(error("batch size must be positive and within max_batch"));
        }
        if report.pairs.len() != report.config.pairs {
            return Err(error("report pair count does not match config"));
        }
        let mut baseline_first = 0;
        for (index, pair) in report.pairs.iter().enumerate() {
            if pair.baseline_ns <= 0 || pair.candidate_ns <= 0 {
                return Err(error(format!("pair {index} durations must be positive")));
            }
            if pair.order != "baseline-first" && pair.order != "candidate-first" {
                return Err(error(format!("pair {index} has invalid order")));
            }
            if pair.order == "baseline-first" {
                baseline_first += 1;
            }
        }
        if baseline_first != report.config.pairs.div_ceil(2) {
            return Err(error("report pair order counts are not balanced"));
        }
        let expected = analyze(
            &report.pairs,
            report.config.confidence_percent,
            report.config.minimum_factor,
        )?;
        if report.analysis != expected {
            return Err(error("report analysis does not match raw pairs"));
        }
        Ok(report)
    }
}

pub type Batch<'a> = dyn FnMut(usize) -> Result<i64, Error> + 'a;
pub type Clock<'a> = dyn FnMut() -> i64 + 'a;

/// Verify, calibrate and collect adjacent pairs. Any failure discards the report.
pub fn collect(
    config: Config,
    expected_checksum: i64,
    baseline: &mut Batch<'_>,
    candidate: &mut Batch<'_>,
    now: &mut Clock<'_>,
) -> Result<Report, Error> {
    config.validate()?;

    let expected_for = |size: usize| -> Result<i64, Error> {
        let size =
            i64::try_from(size).map_err(|_| error("batch size exceeds signed 64-bit range"))?;
        expected_checksum
            .checked_mul(size)
            .ok_or_else(|| error("checksum arithmetic overflow"))
    };
    let verify = |batch: &mut Batch<'_>, size: usize| -> Result<(), Error> {
        let expected = expected_for(size)?;
        let got = batch(size)?;
        if got != expected {
            return Err(error(format!(
                "checksum mismatch: got {got}, want {expected}"
            )));
        }
        Ok(())
    };

    // Warm and establish correctness before the first clock read.
    verify(baseline, 1)?;
    verify(candidate, 1)?;

    let mut last = None;
    let mut read_clock = || -> Result<i64, Error> {
        let tick = now();
        if tick < 0 || last.is_some_and(|previous| tick < previous) {
            return Err(error("clock is negative or regressed"));
        }
        last = Some(tick);
        Ok(tick)
    };
    let mut measure =
        |batch: &mut Batch<'_>, size: usize, calibration: bool| -> Result<i64, Error> {
            let expected = expected_for(size)?;
            let start = read_clock()?;
            let got = batch(size)?;
            let end = read_clock()?;
            if got != expected {
                return Err(error(format!(
                    "checksum mismatch: got {got}, want {expected}"
                )));
            }
            let duration = end - start;
            if duration < 0 || (!calibration && duration == 0) {
                return Err(error(
                    "measured duration must be positive and clock must not regress",
                ));
            }
            Ok(duration)
        };

    let mut batch_size = 1usize;
    loop {
        let baseline_ns = measure(baseline, batch_size, true)?;
        let candidate_ns = measure(candidate, batch_size, true)?;
        if baseline_ns >= config.target_ns && candidate_ns >= config.target_ns {
            break;
        }
        if batch_size == config.max_batch {
            return Err(error("calibration limit exhausted"));
        }
        batch_size = batch_size
            .checked_mul(2)
            .unwrap_or(config.max_batch)
            .min(config.max_batch);
    }

    let mut first_flags: Vec<bool> = (0..config.pairs)
        .map(|index| index < config.pairs.div_ceil(2))
        .collect();
    let mut rng = SplitMix64::new(config.seed as u64);
    for index in (1..first_flags.len()).rev() {
        let swap = rng.bounded((index + 1) as u64) as usize;
        first_flags.swap(index, swap);
    }

    let mut pairs = Vec::with_capacity(config.pairs);
    for baseline_first in first_flags {
        let (baseline_ns, candidate_ns) = if baseline_first {
            let baseline_ns = measure(baseline, batch_size, false)?;
            let candidate_ns = measure(candidate, batch_size, false)?;
            (baseline_ns, candidate_ns)
        } else {
            let candidate_ns = measure(candidate, batch_size, false)?;
            let baseline_ns = measure(baseline, batch_size, false)?;
            (baseline_ns, candidate_ns)
        };
        pairs.push(Pair {
            baseline_ns,
            candidate_ns,
            order: if baseline_first {
                "baseline-first"
            } else {
                "candidate-first"
            }
            .to_owned(),
        });
    }
    let analysis = analyze(&pairs, config.confidence_percent, config.minimum_factor)?;
    Ok(Report {
        config,
        batch_size,
        pairs,
        analysis,
    })
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn bounded(&mut self, bound: u64) -> u64 {
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next();
            if value >= threshold {
                return value % bound;
            }
        }
    }
}

pub const ARRAY_LEN: usize = 4096;
pub const ARRAY_CHECKSUM: i64 = 511_533;

/// Construct the shared indexed-array workload before any timed batch.
pub fn indexed_array() -> Vec<i64> {
    (0..ARRAY_LEN)
        .map(|index| ((17 * index + 3) % 251) as i64)
        .collect()
}

pub fn indexed_array_by_index(values: &[i64], batches: usize) -> Result<i64, Error> {
    sum_batches(values, batches, true)
}

pub fn indexed_array_by_iteration(values: &[i64], batches: usize) -> Result<i64, Error> {
    sum_batches(values, batches, false)
}

fn sum_batches(values: &[i64], batches: usize, by_index: bool) -> Result<i64, Error> {
    let mut total = 0i64;
    if by_index {
        let values = std::hint::black_box(values);
        for _ in 0..batches {
            // This loop deliberately contrasts indexed loads with the native
            // iteration body below; the range shape is the workload variant.
            #[allow(clippy::needless_range_loop)]
            for index in 0..values.len() {
                total = total
                    .checked_add(std::hint::black_box(values[index]))
                    .ok_or_else(|| error("array checksum overflow"))?;
            }
        }
    } else {
        let values = std::hint::black_box(values);
        for _ in 0..batches {
            for value in values {
                total = total
                    .checked_add(std::hint::black_box(*value))
                    .ok_or_else(|| error("array checksum overflow"))?;
            }
        }
    }
    Ok(std::hint::black_box(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(baseline: i64, candidate: i64) -> Pair {
        Pair {
            baseline_ns: baseline,
            candidate_ns: candidate,
            order: "baseline-first".to_owned(),
        }
    }

    fn fake_clock(durations: &[i64]) -> impl FnMut() -> i64 + '_ {
        let mut tick = 0;
        let mut index = 0;
        move || {
            let delta = durations.get(index / 2).copied().unwrap_or(1);
            tick += delta;
            index += 1;
            tick
        }
    }

    fn scripted_clock(ticks: Vec<i64>) -> impl FnMut() -> i64 {
        let mut ticks = ticks.into_iter();
        move || ticks.next().expect("test supplied all clock reads")
    }

    #[test]
    fn portable_fixture_covers_verdicts_boundaries_and_insufficient_pairs() {
        #[derive(Deserialize)]
        struct Fixture {
            confidence_percent: u8,
            minimum_factor: f64,
            cases: Vec<Case>,
        }
        #[derive(Deserialize)]
        struct Case {
            name: String,
            baseline_ns: i64,
            candidate_ns: Vec<i64>,
            ratio: f64,
            interval: Option<[f64; 2]>,
            verdict: String,
        }
        let fixture: Fixture =
            serde_json::from_str(include_str!("../testdata/microbenchmark-pairs-v1.json")).unwrap();
        assert_eq!(fixture.cases.len(), 6);
        for case in fixture.cases {
            let pairs: Vec<_> = case
                .candidate_ns
                .iter()
                .map(|value| pair(case.baseline_ns, *value))
                .collect();
            let got = analyze(&pairs, fixture.confidence_percent, fixture.minimum_factor).unwrap();
            assert!((got.ratio - case.ratio).abs() <= 1e-12, "{}", case.name);
            assert_eq!(got.verdict, case.verdict, "{}", case.name);
            assert_eq!(
                got.interval.map(|i| [i.0, i.1]),
                case.interval,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn packaged_fixture_matches_canonical_when_built_in_repository() {
        use std::path::Path;

        let package = Path::new(env!("CARGO_MANIFEST_DIR"));
        if let Some(repository) = package.ancestors().find(|path| path.join(".git").exists()) {
            let canonical = repository.join("code/specs/fixtures/microbenchmark-pairs-v1.json");
            assert!(canonical.is_file(), "canonical VER07 fixture is missing");
            assert_eq!(
                std::fs::read(canonical).unwrap(),
                include_bytes!("../testdata/microbenchmark-pairs-v1.json"),
                "update the package-local fixture from the canonical VER07 fixture"
            );
        }
    }

    #[test]
    fn collector_warms_calibrates_balances_orders_and_repeats_seed() {
        let config = Config {
            pairs: 5,
            seed: 99,
            target_ns: 5,
            max_batch: 8,
            ..Config::default()
        };
        let batch = |n: usize| Ok(7 * n as i64);
        let mut clock1 = fake_clock(&[5; 20]);
        let mut baseline1 = batch;
        let mut candidate1 = batch;
        let first = collect(
            config.clone(),
            7,
            &mut baseline1,
            &mut candidate1,
            &mut clock1,
        )
        .unwrap();
        let mut clock2 = fake_clock(&[5; 20]);
        let mut baseline2 = batch;
        let mut candidate2 = batch;
        let second = collect(config, 7, &mut baseline2, &mut candidate2, &mut clock2).unwrap();
        assert_eq!(first.batch_size, 1);
        assert_eq!(first.pairs, second.pairs);
        assert_eq!(
            first
                .pairs
                .iter()
                .filter(|p| p.order == "baseline-first")
                .count(),
            3
        );
        assert_eq!(
            first
                .pairs
                .iter()
                .filter(|p| p.order == "candidate-first")
                .count(),
            2
        );
        assert!(first
            .pairs
            .iter()
            .all(|p| p.baseline_ns == 5 && p.candidate_ns == 5));
    }

    #[test]
    fn paired_callbacks_are_adjacent_and_match_the_recorded_order() {
        let config = Config {
            pairs: 3,
            target_ns: 1,
            max_batch: 1,
            seed: 11,
            ..Config::default()
        };
        let calls = std::cell::RefCell::new(Vec::new());
        let mut baseline = |n| {
            calls.borrow_mut().push(("baseline", n));
            Ok(n as i64)
        };
        let mut candidate = |n| {
            calls.borrow_mut().push(("candidate", n));
            Ok(n as i64)
        };
        let mut now = scripted_clock((0..20).collect());
        let report = collect(config, 1, &mut baseline, &mut candidate, &mut now).unwrap();
        let measured = &calls.borrow()[4..]; // warmup and calibration precede pairs
        assert_eq!(measured.len(), 6);
        for (index, pair) in report.pairs.iter().enumerate() {
            let expected = if pair.order == "baseline-first" {
                [("baseline", 1), ("candidate", 1)]
            } else {
                [("candidate", 1), ("baseline", 1)]
            };
            assert_eq!(&measured[index * 2..index * 2 + 2], &expected);
        }
    }

    #[test]
    fn collector_doubles_to_the_first_common_calibration_size() {
        let config = Config {
            pairs: 1,
            target_ns: 8,
            max_batch: 4,
            ..Config::default()
        };
        let calls = std::cell::RefCell::new(Vec::new());
        let mut baseline = |n| {
            calls.borrow_mut().push(("baseline", n));
            Ok(n as i64)
        };
        let mut candidate = |n| {
            calls.borrow_mut().push(("candidate", n));
            Ok(n as i64)
        };
        let mut now = scripted_clock(vec![0, 5, 5, 10, 10, 18, 18, 26, 26, 29, 29, 33]);
        let report = collect(config, 1, &mut baseline, &mut candidate, &mut now).unwrap();
        assert_eq!(report.batch_size, 2);
        assert_eq!(
            *calls.borrow(),
            [
                ("baseline", 1),
                ("candidate", 1), // warm
                ("baseline", 1),
                ("candidate", 1), // calibration 1
                ("baseline", 2),
                ("candidate", 2), // calibration 2
                ("baseline", 2),
                ("candidate", 2), // measured pair
            ]
        );
    }

    #[test]
    fn collector_rejects_warmup_timed_calibration_and_clock_failures() {
        let config = Config {
            pairs: 1,
            target_ns: 5,
            max_batch: 2,
            ..Config::default()
        };
        let mut clock_reads = 0;
        let mut now = || {
            clock_reads += 1;
            clock_reads
        };
        let mut bad_warm = |_| Ok(0);
        let mut good = |n| Ok(n as i64);
        assert!(collect(config.clone(), 1, &mut bad_warm, &mut good, &mut now).is_err());
        assert_eq!(clock_reads, 0, "warmup must fail before starting the clock");

        let mut calls = 0;
        let mut failing = |_| {
            calls += 1;
            if calls == 1 {
                Ok(1)
            } else {
                Err(error("workload failed"))
            }
        };
        let mut good = |n| Ok(n as i64);
        assert!(collect(config.clone(), 1, &mut failing, &mut good, &mut || 1).is_err());

        let mut good = |n| Ok(n as i64);
        let mut good2 = |n| Ok(n as i64);
        let mut regressing = scripted_clock(vec![5, 4]);
        assert!(
            collect(config.clone(), 1, &mut good, &mut good2, &mut regressing)
                .unwrap_err()
                .0
                .contains("regressed")
        );

        let mut good = |n| Ok(n as i64);
        let mut good2 = |n| Ok(n as i64);
        let mut negative = scripted_clock(vec![-1]);
        assert!(
            collect(config.clone(), 1, &mut good, &mut good2, &mut negative)
                .unwrap_err()
                .0
                .contains("negative")
        );

        let short = Config {
            target_ns: 9,
            ..config
        };
        let mut good = |n| Ok(n as i64);
        let mut good2 = |n| Ok(n as i64);
        let mut tiny = scripted_clock(vec![0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8]);
        assert!(collect(short, 1, &mut good, &mut good2, &mut tiny)
            .unwrap_err()
            .0
            .contains("calibration limit"));
    }

    #[test]
    fn collector_rejects_timed_zero_duration_checksum_and_overflow_without_partial_report() {
        let one_pair = Config {
            pairs: 1,
            target_ns: 1,
            max_batch: 1,
            ..Config::default()
        };
        let mut good = |n| Ok(n as i64);
        let mut good2 = |n| Ok(n as i64);
        let mut zero_timed = scripted_clock(vec![0, 1, 1, 2, 2, 2]);
        assert!(
            collect(one_pair.clone(), 1, &mut good, &mut good2, &mut zero_timed)
                .unwrap_err()
                .0
                .contains("positive")
        );

        let mut calls = 0;
        let mut wrong_after_warmup = |n| {
            calls += 1;
            Ok(if calls == 1 { n as i64 } else { 99 })
        };
        let mut good2 = |n| Ok(n as i64);
        assert!(collect(
            one_pair.clone(),
            1,
            &mut wrong_after_warmup,
            &mut good2,
            &mut || 0
        )
        .unwrap_err()
        .0
        .contains("checksum mismatch"));

        let config = Config {
            target_ns: 10,
            max_batch: 2,
            ..one_pair
        };
        let calls = std::cell::RefCell::new(Vec::new());
        let mut huge = |n| {
            calls.borrow_mut().push(n);
            Ok(i64::MAX)
        };
        let mut huge2 = |n| {
            calls.borrow_mut().push(n);
            Ok(i64::MAX)
        };
        let mut tick = 0;
        let mut now = || {
            tick += 1;
            tick
        };
        assert!(collect(config, i64::MAX, &mut huge, &mut huge2, &mut now)
            .unwrap_err()
            .0
            .contains("overflow"));
        assert!(
            !calls.borrow().contains(&2),
            "overflow must be detected before the oversized batch runs"
        );
    }

    #[test]
    fn report_round_trip_replays_and_rejects_forged_analysis() {
        let config = Config {
            pairs: 1,
            target_ns: 1,
            max_batch: 1,
            ..Config::default()
        };
        let batch = |n: usize| Ok(n as i64);
        let mut baseline = batch;
        let mut candidate = batch;
        let mut ticks = [0, 2, 3, 5, 6, 8, 9, 11].into_iter();
        let report = collect(config, 1, &mut baseline, &mut candidate, &mut || {
            ticks.next().unwrap()
        })
        .unwrap();
        let json = report.to_json().unwrap();
        assert_eq!(Report::from_json(&json).unwrap(), report);
        let mut forged: serde_json::Value = serde_json::from_str(&json).unwrap();
        forged["analysis"]["verdict"] = serde_json::Value::String("faster".to_owned());
        assert!(Report::from_json(&forged.to_string()).is_err());

        let mut unbalanced: serde_json::Value = serde_json::from_str(&json).unwrap();
        unbalanced["pairs"][0]["order"] = serde_json::Value::String("candidate-first".to_owned());
        assert!(Report::from_json(&unbalanced.to_string()).is_err());

        let mut generic_metadata: serde_json::Value = serde_json::from_str(&json).unwrap();
        generic_metadata["workload"] = serde_json::Value::String("dependent-link-order".to_owned());
        generic_metadata["invocation"] = serde_json::json!([""]);
        generic_metadata["revision"] = serde_json::json!("r");
        generic_metadata["runtime"] = serde_json::json!("Rust");
        generic_metadata["os"] = serde_json::json!("test");
        generic_metadata["arch"] = serde_json::json!("test");
        assert_eq!(
            Report::from_json(&generic_metadata.to_string()).unwrap(),
            report
        );

        let mut empty_invocation: serde_json::Value = serde_json::from_str(&json).unwrap();
        empty_invocation["workload"] = serde_json::json!("dependent-link-order");
        empty_invocation["invocation"] = serde_json::json!([]);
        empty_invocation["revision"] = serde_json::json!("r");
        empty_invocation["runtime"] = serde_json::json!("Rust");
        empty_invocation["os"] = serde_json::json!("test");
        empty_invocation["arch"] = serde_json::json!("test");
        empty_invocation["profile"] = serde_json::json!("release");
        assert_eq!(
            Report::from_json(&empty_invocation.to_string()).unwrap(),
            report
        );

        for value in [
            serde_json::Value::String(String::new()),
            serde_json::Value::String(" \t\r\n".to_owned()),
            serde_json::Value::String("bad\0id".to_owned()),
            serde_json::Value::Null,
            serde_json::Value::Number(7.into()),
            serde_json::Value::Bool(true),
            serde_json::json!([]),
            serde_json::json!({}),
        ] {
            let mut malformed_metadata = empty_invocation.clone();
            malformed_metadata["workload"] = value;
            assert!(Report::from_json(&malformed_metadata.to_string()).is_err());
        }
        let invalid_unicode = json.replace("\"analysis\"", "\"workload\":\"\\ud800\",\"analysis\"");
        assert!(Report::from_json(&invalid_unicode).is_err());
        let mut unknown: serde_json::Value = serde_json::from_str(&json).unwrap();
        unknown["surprise"] = serde_json::Value::Bool(true);
        assert!(Report::from_json(&unknown.to_string()).is_err());
    }

    #[test]
    fn raw_json_rejects_duplicate_decoded_members_but_allows_distinct_scopes() {
        let core = serde_json::json!({
            "config": { "pairs": 1, "confidence_percent": 95, "minimum_factor": 1.05,
                "seed": 0, "target_ns": 1, "max_batch": 1 },
            "batch_size": 1,
            "pairs": [{ "baseline_ns": 1, "candidate_ns": 1, "order": "baseline-first" }],
            "analysis": { "ratio": 1.0, "interval": null, "verdict": "inconclusive" },
            "workload": "generic-workload", "invocation": [], "revision": "shared",
            "runtime": "shared", "os": "test", "arch": "test"
        });
        let valid = core.to_string();
        assert!(Report::from_json(&valid).is_ok());
        // The same member name appears in root and config scopes (`pairs`).

        let invalid = [
            valid.replace("\"pairs\":1", "\"pairs\":1,\"pairs\":1"),
            valid.replace("\"baseline_ns\":1", "\"baseline_ns\":1,\"baseline_ns\":1"),
            valid.replace(
                "\"verdict\":\"inconclusive\"",
                "\"verdict\":\"inconclusive\",\"verdict\":\"inconclusive\"",
            ),
            valid.replace(
                "\"revision\":\"shared\"",
                "\"revision\":\"shared\",\"revision\":\"other\"",
            ),
            valid.replace("\"pairs\":1", "\"pairs\":1,\"pa\\u0069rs\":1"),
        ];
        for duplicate in invalid {
            assert!(
                Report::from_json(&duplicate).is_err(),
                "accepted duplicate: {duplicate}"
            );
        }
    }

    #[test]
    fn raw_json_requires_one_complete_document() {
        let valid = serde_json::json!({
            "config": { "pairs": 1, "confidence_percent": 95, "minimum_factor": 1.05,
                "seed": 0, "target_ns": 1, "max_batch": 1 },
            "batch_size": 1,
            "pairs": [{ "baseline_ns": 1, "candidate_ns": 1, "order": "baseline-first" }],
            "analysis": { "ratio": 1.0, "interval": null, "verdict": "inconclusive" }
        })
        .to_string();
        assert!(Report::from_json(&valid).is_ok());

        let truncated = &valid[..valid.len() - 1];
        for invalid in [
            truncated.to_owned(),
            format!("{valid} trailing"),
            format!("{valid} {{}}"),
        ] {
            assert!(
                Report::from_json(&invalid).is_err(),
                "accepted incomplete or multiple JSON values: {invalid}"
            );
        }
    }

    #[test]
    fn raw_json_preserves_signed_64_bit_endpoints() {
        let extreme = r#"{"config":{"pairs":1,"confidence_percent":95,"minimum_factor":1.05,"seed":-9223372036854775808,"target_ns":9223372036854775807,"max_batch":1},"batch_size":1,"pairs":[{"baseline_ns":9223372036854775807,"candidate_ns":9223372036854775807,"order":"baseline-first"}],"analysis":{"ratio":1.0,"interval":null,"verdict":"inconclusive"}}"#;
        let replayed = Report::from_json(extreme).unwrap();
        assert_eq!(replayed.config.seed, i64::MIN);
        assert_eq!(replayed.config.target_ns, i64::MAX);
        assert_eq!(replayed.pairs[0].baseline_ns, i64::MAX);
        assert_eq!(replayed.pairs[0].candidate_ns, i64::MAX);
    }

    #[test]
    fn raw_json_accepts_maximum_balanced_report_with_reused_pair_member_names() {
        let config = Config {
            pairs: MAX_PAIRS,
            target_ns: 1,
            max_batch: 1,
            ..Config::default()
        };
        let pairs = (0..MAX_PAIRS)
            .map(|index| Pair {
                baseline_ns: 1,
                candidate_ns: 1,
                order: if index % 2 == 0 {
                    "baseline-first".to_owned()
                } else {
                    "candidate-first".to_owned()
                },
            })
            .collect::<Vec<_>>();
        let analysis = analyze(&pairs, config.confidence_percent, config.minimum_factor).unwrap();
        let report = Report {
            config,
            batch_size: 1,
            pairs,
            analysis,
        };

        let text = report.to_json().unwrap();
        let replayed = Report::from_json(&text).unwrap();
        assert_eq!(replayed, report);
        assert_eq!(replayed.pairs.len(), MAX_PAIRS);
    }

    #[test]
    fn raw_json_keeps_signed_64_bit_endpoints() {
        let extreme = r#"{"config":{"pairs":1,"confidence_percent":95,"minimum_factor":1.05,"seed":-9223372036854775808,"target_ns":9223372036854775807,"max_batch":1},"batch_size":1,"pairs":[{"baseline_ns":9223372036854775807,"candidate_ns":9223372036854775807,"order":"baseline-first"}],"analysis":{"ratio":1.0,"interval":null,"verdict":"inconclusive"}}"#;
        let replayed = Report::from_json(extreme).unwrap();
        assert_eq!(replayed.config.seed, i64::MIN);
        assert_eq!(replayed.config.target_ns, i64::MAX);
        assert_eq!(replayed.pairs[0].baseline_ns, i64::MAX);
    }

    #[test]
    fn raw_json_rejects_oversized_pair_array_before_typed_conversion() {
        let document = serde_json::json!({
            "pairs": vec![serde_json::Value::Null; MAX_PAIRS + 1]
        });
        let error = Report::from_json(&document.to_string()).unwrap_err();
        assert!(error.to_string().contains("pair count exceeds 10000"));
    }

    #[test]
    fn report_replay_validates_optional_metadata_as_a_complete_group() {
        let core = serde_json::json!({
            "config": { "pairs": 1, "confidence_percent": 95, "minimum_factor": 1.05,
                "seed": 0, "target_ns": 1, "max_batch": 1 },
            "batch_size": 1,
            "pairs": [{ "baseline_ns": 1, "candidate_ns": 1, "order": "baseline-first" }],
            "analysis": { "ratio": 1.0, "interval": null, "verdict": "inconclusive" }
        });
        let mut complete = core.clone();
        complete["workload"] = serde_json::json!("generic-workload");
        complete["invocation"] = serde_json::json!([]);
        complete["revision"] = serde_json::json!("r");
        complete["runtime"] = serde_json::json!("Rust");
        complete["os"] = serde_json::json!("test");
        complete["arch"] = serde_json::json!("test");
        assert!(Report::from_json(&complete.to_string()).is_ok());

        for field in [
            "workload",
            "invocation",
            "revision",
            "runtime",
            "os",
            "arch",
        ] {
            let mut partial = complete.clone();
            partial.as_object_mut().unwrap().remove(field);
            assert!(
                Report::from_json(&partial.to_string()).is_err(),
                "accepted missing {field}"
            );
        }
        let mut profile_only = core.clone();
        profile_only["profile"] = serde_json::json!("release");
        assert!(Report::from_json(&profile_only.to_string()).is_err());

        for field in ["revision", "runtime", "os", "arch"] {
            for invalid in [
                serde_json::Value::Null,
                serde_json::json!(""),
                serde_json::json!("bad\0value"),
            ] {
                let mut malformed = complete.clone();
                malformed[field] = invalid.clone();
                assert!(
                    Report::from_json(&malformed.to_string()).is_err(),
                    "accepted {field}={invalid:?}"
                );
            }
        }
        for field in ["revision", "runtime", "os", "arch", "profile"] {
            let mut whitespace = complete.clone();
            whitespace[field] = serde_json::json!(" ");
            assert!(
                Report::from_json(&whitespace.to_string()).is_ok(),
                "rejected non-empty whitespace {field}"
            );
        }
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(""),
            serde_json::json!("bad\0value"),
        ] {
            let mut malformed = complete.clone();
            malformed["profile"] = invalid.clone();
            assert!(
                Report::from_json(&malformed.to_string()).is_err(),
                "accepted profile={invalid:?}"
            );
        }
        let mut nul_arg = complete.clone();
        nul_arg["invocation"] = serde_json::json!(["bad\0arg"]);
        assert!(Report::from_json(&nul_arg.to_string()).is_err());
        let mut empty_arg = complete;
        empty_arg["invocation"] = serde_json::json!([""]);
        assert!(Report::from_json(&empty_arg.to_string()).is_ok());
    }

    #[test]
    fn report_replay_rejects_oversized_raw_pair_array_before_typed_conversion() {
        let document = serde_json::json!({
            "pairs": vec![serde_json::Value::Null; MAX_PAIRS + 1]
        });
        let error = Report::from_json(&document.to_string()).unwrap_err();
        assert!(error.to_string().contains("pair count exceeds 10000"));
    }

    #[test]
    fn rejects_config_pairs_and_observation_boundaries() {
        for config in [
            Config {
                pairs: 0,
                ..Config::default()
            },
            Config {
                pairs: 10_001,
                ..Config::default()
            },
            Config {
                confidence_percent: 0,
                ..Config::default()
            },
            Config {
                confidence_percent: 100,
                ..Config::default()
            },
            Config {
                minimum_factor: 1.0,
                ..Config::default()
            },
            Config {
                minimum_factor: f64::NAN,
                ..Config::default()
            },
            Config {
                minimum_factor: f64::INFINITY,
                ..Config::default()
            },
            Config {
                target_ns: 0,
                ..Config::default()
            },
            Config {
                target_ns: -1,
                ..Config::default()
            },
            Config {
                max_batch: 0,
                ..Config::default()
            },
        ] {
            assert!(config.validate().is_err());
        }
        for pairs in [
            vec![],
            vec![pair(0, 1)],
            vec![pair(1, -1)],
            vec![Pair {
                order: "bad".to_owned(),
                ..pair(1, 1)
            }],
        ] {
            assert!(analyze(&pairs, 95, 1.05).is_err());
        }
        assert!(analyze(&[pair(1, 1)], 0, 1.05).is_err());
        assert!(analyze(&[pair(1, 1)], 100, 1.05).is_err());
        assert!(analyze(&[pair(1, 1)], 95, f64::NAN).is_err());
        assert!(analyze(&[pair(1, 1)], 95, 1.0).is_err());
    }

    #[test]
    fn analysis_accepts_maximum_pairs_and_rejects_excess_before_pair_validation() {
        let maximum = vec![pair(1, 1); MAX_PAIRS];
        assert!(analyze(&maximum, 95, 1.05).is_ok());

        let oversized = vec![
            Pair {
                baseline_ns: 0,
                candidate_ns: 0,
                order: "invalid".to_owned(),
            };
            MAX_PAIRS + 1
        ];
        let error = analyze(&oversized, 95, 1.05).unwrap_err();
        assert!(error.to_string().contains("at most 10000"));
        let invalid_confidence = analyze(&oversized, 0, 1.05).unwrap_err();
        assert!(invalid_confidence.to_string().contains("confidence"));
    }

    #[test]
    fn generated_ratio_sets_keep_intervals_inside_the_sample() {
        let mut state = 7u64;
        for n in 1..=80 {
            let mut pairs = Vec::new();
            for _ in 0..n {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let duration = 1 + (state % 10_000) as i64;
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                pairs.push(pair(duration, 1 + (state % 10_000) as i64));
            }
            let result = analyze(&pairs, 95, 1.05).unwrap();
            if let Some(interval) = result.interval {
                let min = pairs
                    .iter()
                    .map(|p| p.candidate_ns as f64 / p.baseline_ns as f64)
                    .fold(f64::INFINITY, f64::min);
                let max = pairs
                    .iter()
                    .map(|p| p.candidate_ns as f64 / p.baseline_ns as f64)
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!(min <= interval.0 && interval.0 <= interval.1 && interval.1 <= max);
            }
        }
    }

    #[test]
    fn indexed_array_pair_has_expected_checksum_and_same_work() {
        let values = indexed_array();
        assert_eq!(values.len(), ARRAY_LEN);
        assert_eq!(values.iter().sum::<i64>(), ARRAY_CHECKSUM);
        assert_eq!(
            indexed_array_by_index(&values, 2).unwrap(),
            ARRAY_CHECKSUM * 2
        );
        assert_eq!(
            indexed_array_by_iteration(&values, 2).unwrap(),
            ARRAY_CHECKSUM * 2
        );
    }
}
