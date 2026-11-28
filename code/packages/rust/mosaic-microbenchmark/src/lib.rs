//! Measure Mosaic's real IR tree walk using two equivalent event-log sinks.
//!
//! The guest VM resolves Mosaic values and calls a renderer for each node.
//! This experiment compares how those same callbacks accumulate their event
//! stream. It is intentionally not labeled as an array or cache benchmark.

use coding_adventures_guest_microbenchmark_support::{
    parse_args, replay_core_file, replay_file, runtime_metadata, sample_empty_entries,
    sample_host_clock_noop, select_revision, write_outputs, Command, Error as SupportError,
    GuestEnvelope, DEFAULT_EMPTY_ENTRY_MAX_BATCH, DEFAULT_EMPTY_ENTRY_SAMPLES,
    DEFAULT_EMPTY_ENTRY_TARGET_NS,
};
use coding_adventures_microbenchmark::{collect, Batch, Config, Error, Report};
use mosaic_analyzer::analyze;
use mosaic_analyzer::{MosaicSlot, MosaicType};
use mosaic_vm::{EmitResult, MosaicRenderer, MosaicVM, ResolvedProperty};
use std::fs::File;
use std::io::Write;

pub const GUEST_ID: &str = "mosaic-vm";
pub const WORKLOAD_ID: &str = "mosaic-ir-tree-walk-event-stream-4096";
pub const LEAF_COUNT: usize = 4096;
pub const MAX_BATCH: usize = 65_536;
pub const BASELINE_VARIANT: &str = "vector-event-log";
pub const CANDIDATE_VARIANT: &str = "string-event-log";
const EXPECTED_COMPONENT: &str = "TreeWalkBench";

fn support_error(error: impl std::fmt::Display) -> SupportError {
    SupportError(error.to_string())
}

/// Build deterministic Mosaic source before timing. Each leaf has both a
/// string and dimension property so the tree walk exercises value resolution.
pub fn component_source(leaves: usize) -> Result<String, SupportError> {
    if !(1..=LEAF_COUNT).contains(&leaves) {
        return Err(SupportError(format!(
            "leaf count must be from 1 through {LEAF_COUNT}"
        )));
    }
    let capacity = 64 + leaves * 52;
    let mut source = String::with_capacity(capacity);
    source.push_str("component TreeWalkBench { Column {\n");
    for _ in 0..leaves {
        source.push_str("Text { content: \"node\"; padding: 16dp; }\n");
    }
    source.push_str("} }");
    Ok(source)
}

/// Prepared guest input. Parsing and analysis are intentionally complete before collection.
pub struct PreparedTree {
    vm: MosaicVM,
    expected_checksum: i64,
    #[cfg(test)]
    expected_output: String,
}

pub fn prepare_tree(leaves: usize) -> Result<PreparedTree, SupportError> {
    let source = component_source(leaves)?;
    let file = analyze(&source).map_err(support_error)?;
    let vm = MosaicVM::new(file);
    let vector_result = vm
        .run(EventRenderer::new(VectorSink::default()))
        .map_err(support_error)?;
    let string_result = vm
        .run(EventRenderer::new(StringSink::default()))
        .map_err(support_error)?;
    if vector_result != string_result || vector_result.component_name != EXPECTED_COMPONENT {
        return Err(SupportError(
            "renderer sinks produced different Mosaic output".into(),
        ));
    }
    let expected_checksum = stream_checksum(vector_result.output.as_bytes());
    Ok(PreparedTree {
        vm,
        expected_checksum,
        #[cfg(test)]
        expected_output: vector_result.output,
    })
}

/// Execute one batch against the chosen sink. A zero batch is the empty checksum;
/// values above the documented bound are rejected before any guest work begins.
pub fn render_batch(
    tree: &PreparedTree,
    operations: usize,
    use_string_sink: bool,
) -> Result<i64, Error> {
    if operations > MAX_BATCH {
        return Err(Error(format!("operations exceed {MAX_BATCH}")));
    }
    let mut checksum = 0i64;
    for _ in 0..operations {
        let result = if use_string_sink {
            tree.vm.run(EventRenderer::new(StringSink::default()))
        } else {
            tree.vm.run(EventRenderer::new(VectorSink::default()))
        }
        .map_err(|e| Error(e.to_string()))?;
        if result.component_name != EXPECTED_COMPONENT {
            return Err(Error(
                "Mosaic renderer returned an unexpected component".into(),
            ));
        }
        checksum = checked_checksum_add(checksum, stream_checksum(result.output.as_bytes()))?;
    }
    Ok(checksum)
}

pub fn collect_tree(
    config: Config,
    tree: &PreparedTree,
    now: &mut impl FnMut() -> i64,
) -> Result<Report, Error> {
    if config.max_batch > MAX_BATCH {
        return Err(Error(format!("max_batch must not exceed {MAX_BATCH}")));
    }
    let mut baseline: &mut Batch<'_> = &mut |operations| render_batch(tree, operations, false);
    let mut candidate: &mut Batch<'_> = &mut |operations| render_batch(tree, operations, true);
    collect(
        config,
        tree.expected_checksum,
        &mut baseline,
        &mut candidate,
        now,
    )
}

fn stream_checksum(bytes: &[u8]) -> i64 {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        value ^= u64::from(byte);
        value = value.wrapping_mul(0x0000_0100_0000_01b3);
    }
    ((value & i64::MAX as u64).max(1)) as i64
}

fn checked_checksum_add(total: i64, next: i64) -> Result<i64, Error> {
    total
        .checked_add(next)
        .ok_or_else(|| Error("Mosaic event checksum overflow".into()))
}

trait EventSink {
    fn push(&mut self, event: String);
    fn finish(self) -> String;
}

#[derive(Default)]
struct VectorSink(Vec<String>);

impl EventSink for VectorSink {
    fn push(&mut self, event: String) {
        self.0.push(event);
    }
    fn finish(self) -> String {
        self.0.join("\n")
    }
}

#[derive(Default)]
struct StringSink {
    output: String,
}

impl EventSink for StringSink {
    fn push(&mut self, event: String) {
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output.push_str(&event);
    }
    fn finish(self) -> String {
        self.output
    }
}

struct EventRenderer<S> {
    sink: S,
    component_name: String,
}

impl<S> EventRenderer<S> {
    fn new(sink: S) -> Self {
        Self {
            sink,
            component_name: String::new(),
        }
    }
}

impl<S: EventSink> MosaicRenderer for EventRenderer<S> {
    fn begin_component(&mut self, name: &str, slots: &[MosaicSlot]) {
        self.component_name = name.to_owned();
        self.sink
            .push(format!("begin-component|{name}|slots={slots:?}"));
    }
    fn end_component(&mut self) {
        self.sink.push("end-component".into());
    }
    fn begin_node(&mut self, tag: &str, is_primitive: bool, properties: &[ResolvedProperty]) {
        self.sink.push(format!(
            "begin-node|{tag}|primitive={is_primitive}|props={properties:?}"
        ));
    }
    fn end_node(&mut self, tag: &str) {
        self.sink.push(format!("end-node|{tag}"));
    }
    fn render_slot_child(&mut self, slot_name: &str, slot_type: &MosaicType) {
        self.sink
            .push(format!("slot-child|{slot_name}|{slot_type:?}"));
    }
    fn begin_when(&mut self, slot_name: &str) {
        self.sink.push(format!("begin-when|{slot_name}"));
    }
    fn end_when(&mut self) {
        self.sink.push("end-when".into());
    }
    fn begin_each(&mut self, slot_name: &str, item_name: &str, element_type: &MosaicType) {
        self.sink.push(format!(
            "begin-each|{slot_name}|{item_name}|{element_type:?}"
        ));
    }
    fn end_each(&mut self) {
        self.sink.push("end-each".into());
    }
    fn emit(self) -> EmitResult {
        EmitResult {
            output: self.sink.finish(),
            component_name: self.component_name,
        }
    }
}

pub fn run(
    args: &[String],
    output: &mut impl Write,
    now: &mut impl FnMut() -> i64,
) -> Result<(), SupportError> {
    match parse_args(args)? {
        Command::Help => {
            writeln!(output, "Usage: mosaic-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID] --manifest FILE | --replay REPORT | --replay-envelope MANIFEST")
                .map_err(|e| SupportError(format!("could not write help: {e}")))?;
            output
                .flush()
                .map_err(|e| SupportError(format!("could not flush help: {e}")))
        }
        Command::ReplayCore(path) => {
            let path = path
                .to_str()
                .ok_or_else(|| SupportError("replay path is not valid UTF-8".into()))?;
            let report = replay_core_file(path)?;
            writeln!(
                output,
                "replay valid: {} Mosaic report pairs",
                report.pairs.len()
            )
            .map_err(|e| SupportError(format!("could not write replay result: {e}")))?;
            output
                .flush()
                .map_err(|e| SupportError(format!("could not flush replay result: {e}")))
        }
        Command::ReplayEnvelope(path) => {
            let path = path
                .to_str()
                .ok_or_else(|| SupportError("manifest path is not valid UTF-8".into()))?;
            let envelope = replay_file(path, GUEST_ID, WORKLOAD_ID)?;
            writeln!(
                output,
                "replay valid: {} Mosaic envelope pairs",
                envelope.report.pairs.len()
            )
            .map_err(|e| SupportError(format!("could not write replay result: {e}")))?;
            output
                .flush()
                .map_err(|e| SupportError(format!("could not flush replay result: {e}")))
        }
        Command::Run {
            config,
            revision,
            manifest,
        } => {
            let revision = select_revision(
                revision,
                std::env::var("GIT_REVISION").ok().as_deref(),
                std::env::var("GITHUB_SHA").ok().as_deref(),
            )?;
            let tree = prepare_tree(LEAF_COUNT)?;
            let host_clock_noop = sample_host_clock_noop(now)?;
            let empty_tree = prepare_tree(1)?;
            let empty_guest_call = sample_empty_entries(
                "one-node MosaicVM::run through the vector event renderer and checksum wrapper",
                DEFAULT_EMPTY_ENTRY_TARGET_NS,
                DEFAULT_EMPTY_ENTRY_MAX_BATCH,
                DEFAULT_EMPTY_ENTRY_SAMPLES,
                now,
                &mut || {
                    let result = render_batch(&empty_tree, 1, false).map_err(support_error)?;
                    if result != empty_tree.expected_checksum {
                        return Err(SupportError("empty-tree renderer checksum mismatch".into()));
                    }
                    Ok(())
                },
            )?;
            let report = collect_tree(config, &tree, now).map_err(support_error)?;
            let envelope = GuestEnvelope::new(
                GUEST_ID, WORKLOAD_ID, args.iter().skip(1).cloned().collect(), BASELINE_VARIANT, CANDIDATE_VARIANT,
                "Generate source, parse and analyze once, and allocate the MosaicVM before collection.",
                "MosaicVM depth-first tree walk, property resolution, renderer callbacks, event-log output, and per-operation checksum; excludes source setup and analysis.",
                host_clock_noop,
                empty_guest_call,
                runtime_metadata("mosaic-vm 0.1.0 + mosaic-analyzer 0.1.0", "generated-4096-text-tree-with-dimensions-v1", revision),
                report,
            )?;
            let manifest_path = manifest
                .to_str()
                .ok_or_else(|| SupportError("manifest path is not valid UTF-8".into()))?;
            let mut manifest_file = File::create(manifest_path)
                .map_err(|e| SupportError(format!("cannot create manifest: {e}")))?;
            write_outputs(&envelope, &mut manifest_file, output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_sinks_produce_the_same_guest_output_and_checksum() {
        assert_eq!(
            component_source(LEAF_COUNT).unwrap().as_bytes(),
            include_bytes!("../testdata/mosaic-tree-4096.mosaic")
        );
        let tree = prepare_tree(8).unwrap();
        let vector = tree
            .vm
            .run(EventRenderer::new(VectorSink::default()))
            .unwrap();
        let string = tree
            .vm
            .run(EventRenderer::new(StringSink::default()))
            .unwrap();
        assert_eq!(vector, string);
        assert_eq!(tree.expected_output, vector.output);
        assert_eq!(
            render_batch(&tree, 1, false).unwrap(),
            tree.expected_checksum
        );
        assert_eq!(
            render_batch(&tree, 1, true).unwrap(),
            tree.expected_checksum
        );
        assert_eq!(render_batch(&tree, 0, true).unwrap(), 0);
        assert!(render_batch(&tree, usize::MAX, true).is_err());
        assert!(checked_checksum_add(i64::MAX, 1).is_err());
        assert_eq!(checked_checksum_add(10, 20).unwrap(), 30);
        assert!(component_source(0).is_err());
        assert!(component_source(LEAF_COUNT + 1).is_err());
    }

    #[test]
    fn generated_small_tree_sizes_keep_the_two_guest_paths_equivalent() {
        for leaves in (1..=128).step_by(7) {
            let tree = prepare_tree(leaves).unwrap();
            assert_eq!(
                render_batch(&tree, 1, false).unwrap(),
                tree.expected_checksum
            );
            assert_eq!(
                render_batch(&tree, 1, true).unwrap(),
                tree.expected_checksum
            );
        }
    }

    #[test]
    fn fake_clock_collection_uses_shared_default_order_and_positive_pairs() {
        let tree = prepare_tree(4).unwrap();
        let config = Config {
            pairs: 3,
            target_ns: 1,
            max_batch: 1,
            ..Config::default()
        };
        let mut tick = 0i64;
        let report = collect_tree(config, &tree, &mut || {
            let current = tick;
            tick += 10;
            current
        })
        .unwrap();
        assert_eq!(report.pairs.len(), 3);
        assert!(report
            .pairs
            .iter()
            .all(|pair| pair.baseline_ns > 0 && pair.candidate_ns > 0));
        assert!(collect_tree(
            Config {
                max_batch: MAX_BATCH + 1,
                ..Config::default()
            },
            &tree,
            &mut || 0
        )
        .is_err());
        let mut ticks = [10, 11, 12, 9].into_iter();
        assert!(collect_tree(
            Config {
                pairs: 1,
                max_batch: 1,
                target_ns: 1,
                ..Config::default()
            },
            &tree,
            &mut || ticks.next().unwrap_or(9)
        )
        .is_err());
    }

    #[test]
    fn entrypoint_builds_and_replays_only_its_guest_envelope() {
        let mut bytes = Vec::new();
        let mut tick = 0i64;
        let manifest_path =
            std::env::temp_dir().join(format!("mosaic-guest-manifest-{}.json", std::process::id()));
        run(
            &[
                "bench".into(),
                "--pairs".into(),
                "1".into(),
                "--target-ns".into(),
                "1".into(),
                "--max-batch".into(),
                "1".into(),
                "--revision".into(),
                "test".into(),
                "--manifest".into(),
                manifest_path.to_string_lossy().into_owned(),
            ],
            &mut bytes,
            &mut || {
                tick += 100_000;
                tick
            },
        )
        .unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        let report =
            coding_adventures_guest_microbenchmark_support::replay_core_report(&text).unwrap();
        assert_eq!(report.pairs.len(), 1);
        let manifest = std::fs::read_to_string(&manifest_path).unwrap();
        let replay = coding_adventures_guest_microbenchmark_support::replay_envelope(
            &manifest,
            GUEST_ID,
            WORKLOAD_ID,
        )
        .unwrap();
        assert_eq!(replay.report, report);
        assert_eq!(replay.metadata.revision, "test");
        assert_eq!(
            replay.invocation.first().map(String::as_str),
            Some("--pairs")
        );
        assert!(
            coding_adventures_guest_microbenchmark_support::replay_envelope(
                &manifest,
                "starlark-vm",
                WORKLOAD_ID
            )
            .is_err()
        );
        let report_path =
            std::env::temp_dir().join(format!("mosaic-core-{}.json", std::process::id()));
        std::fs::write(&report_path, text).unwrap();
        let mut replay_output = Vec::new();
        run(
            &[
                "bench".into(),
                "--replay".into(),
                report_path.to_string_lossy().into_owned(),
            ],
            &mut replay_output,
            &mut || panic!("replay must not read time"),
        )
        .unwrap();
        replay_output.clear();
        run(
            &[
                "bench".into(),
                "--replay-envelope".into(),
                manifest_path.to_string_lossy().into_owned(),
            ],
            &mut replay_output,
            &mut || panic!("replay must not read time"),
        )
        .unwrap();
        replay_output.clear();
        run(
            &["bench".into(), "--help".into()],
            &mut replay_output,
            &mut || panic!("help must not read time"),
        )
        .unwrap();
        assert!(String::from_utf8(replay_output).unwrap().contains("Usage:"));
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        assert!(run(&["bench".into(), "--help".into()], &mut Broken, &mut || 0).is_err());
        std::fs::remove_file(report_path).unwrap();
        std::fs::remove_file(manifest_path).unwrap();
    }
}
