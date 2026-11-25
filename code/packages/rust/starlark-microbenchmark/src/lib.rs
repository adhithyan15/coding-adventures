//! Execute the actual Starlark interpreter on equivalent source-compiled and
//! hand-assembled bytecode. Source compilation is deliberately outside timing.

use coding_adventures_guest_microbenchmark_support::{
    parse_args, replay_core_file, replay_file, runtime_metadata, sample_empty_entries,
    sample_host_clock_noop, select_revision, write_outputs, Command, Error as SupportError,
    GuestEnvelope, DEFAULT_EMPTY_ENTRY_MAX_BATCH, DEFAULT_EMPTY_ENTRY_SAMPLES,
    DEFAULT_EMPTY_ENTRY_TARGET_NS,
};
use coding_adventures_microbenchmark::{collect, Batch, Config, Error, Report};
use starlark_interpreter::{
    compile_source, interpret_bytecode, CodeObject, Instruction, Op, Operand, Value,
};
use std::fs::File;
use std::io::Write;

pub const GUEST_ID: &str = "starlark-vm";
pub const WORKLOAD_ID: &str = "starlark-bytecode-integer-add-chain-256";
pub const ADDITIONS: usize = 256;
pub const MAX_BATCH: usize = 65_536;
pub const BASELINE_VARIANT: &str = "stub-source-compiled-bytecode";
pub const CANDIDATE_VARIANT: &str = "hand-assembled-operand-stack-bytecode";

fn support_error(error: impl std::fmt::Display) -> SupportError {
    SupportError(error.to_string())
}

pub fn source_program(additions: usize) -> Result<String, SupportError> {
    if !(1..=ADDITIONS).contains(&additions) {
        return Err(SupportError(format!(
            "addition count must be from 1 through {ADDITIONS}"
        )));
    }
    let mut source = String::with_capacity(10 + additions * 22);
    source.push_str("total = 0\n");
    for _ in 0..additions {
        source.push_str("total = total + 1\n");
    }
    Ok(source)
}

pub fn stack_bytecode(additions: usize) -> Result<CodeObject, SupportError> {
    if !(1..=ADDITIONS).contains(&additions) {
        return Err(SupportError(format!(
            "addition count must be from 1 through {ADDITIONS}"
        )));
    }
    let mut instructions = Vec::with_capacity(additions * 2 + 2);
    instructions.push(Instruction {
        opcode: Op::LoadConst as u8,
        operand: Some(Operand::Index(0)),
    });
    for _ in 0..additions {
        instructions.push(Instruction {
            opcode: Op::LoadConst as u8,
            operand: Some(Operand::Index(1)),
        });
        instructions.push(Instruction {
            opcode: Op::Add as u8,
            operand: None,
        });
    }
    instructions.push(Instruction {
        opcode: Op::StoreName as u8,
        operand: Some(Operand::Index(0)),
    });
    instructions.push(Instruction {
        opcode: Op::Halt as u8,
        operand: None,
    });
    Ok(CodeObject {
        instructions,
        constants: vec![Value::Int(0), Value::Int(1)],
        names: vec!["total".into()],
    })
}

pub struct PreparedPrograms {
    source_compiled: CodeObject,
    stack: CodeObject,
    expected: i64,
}

pub fn prepare_programs(additions: usize) -> Result<PreparedPrograms, SupportError> {
    let source = source_program(additions)?;
    let source_compiled = compile_source(&source).map_err(support_error)?;
    let stack = stack_bytecode(additions)?;
    let expected = i64::try_from(additions).map_err(support_error)?;
    for code in [&source_compiled, &stack] {
        let result = interpret_bytecode(code).map_err(support_error)?;
        if result.get_int("total") != Some(expected) {
            return Err(SupportError(
                "Starlark bytecode produced an unexpected total".into(),
            ));
        }
    }
    Ok(PreparedPrograms {
        source_compiled,
        stack,
        expected,
    })
}

pub fn execute_batch(
    programs: &PreparedPrograms,
    operations: usize,
    candidate: bool,
) -> Result<i64, Error> {
    if operations > MAX_BATCH {
        return Err(Error(format!("operations exceed {MAX_BATCH}")));
    }
    let code = if candidate {
        &programs.stack
    } else {
        &programs.source_compiled
    };
    let mut checksum = 0i64;
    for _ in 0..operations {
        let actual = execute_one(code, Some(programs.expected))?;
        checksum = checked_checksum_add(checksum, actual)?;
    }
    Ok(checksum)
}

fn execute_one(code: &CodeObject, expected: Option<i64>) -> Result<i64, Error> {
    let result = interpret_bytecode(code).map_err(|e| Error(e.to_string()))?;
    match expected {
        Some(expected) => {
            let actual = result
                .get_int("total")
                .ok_or_else(|| Error("Starlark result did not contain integer total".into()))?;
            if actual != expected {
                return Err(Error("Starlark result checksum mismatch".into()));
            }
            Ok(actual)
        }
        None => {
            if !result.output.is_empty() || !result.variables.is_empty() {
                return Err(Error(
                    "Halt-only Starlark call returned unexpected state".into(),
                ));
            }
            Ok(0)
        }
    }
}

fn checked_checksum_add(total: i64, next: i64) -> Result<i64, Error> {
    total
        .checked_add(next)
        .ok_or_else(|| Error("Starlark batch checksum overflow".into()))
}

pub fn collect_programs(
    config: Config,
    programs: &PreparedPrograms,
    now: &mut impl FnMut() -> i64,
) -> Result<Report, Error> {
    if config.max_batch > MAX_BATCH {
        return Err(Error(format!("max_batch must not exceed {MAX_BATCH}")));
    }
    let mut baseline: &mut Batch<'_> = &mut |n| execute_batch(programs, n, false);
    let mut candidate: &mut Batch<'_> = &mut |n| execute_batch(programs, n, true);
    collect(
        config,
        programs.expected,
        &mut baseline,
        &mut candidate,
        now,
    )
}

pub fn run(
    args: &[String],
    output: &mut impl Write,
    now: &mut impl FnMut() -> i64,
) -> Result<(), SupportError> {
    match parse_args(args)? {
        Command::Help => {
            writeln!(output, "Usage: starlark-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID] --manifest FILE | --replay REPORT | --replay-envelope MANIFEST")
                .map_err(|e| SupportError(format!("could not write help: {e}")))?;
            output
                .flush()
                .map_err(|e| SupportError(format!("could not flush help: {e}")))
        }
        Command::ReplayCore(path) => {
            let path = path
                .to_str()
                .ok_or_else(|| SupportError("report path is not valid UTF-8".into()))?;
            let report = replay_core_file(path)?;
            writeln!(
                output,
                "replay valid: {} Starlark report pairs",
                report.pairs.len()
            )
            .map_err(|e| SupportError(e.to_string()))?;
            output.flush().map_err(|e| SupportError(e.to_string()))
        }
        Command::ReplayEnvelope(path) => {
            let path = path
                .to_str()
                .ok_or_else(|| SupportError("manifest path is not valid UTF-8".into()))?;
            let envelope = replay_file(path, GUEST_ID, WORKLOAD_ID)?;
            writeln!(
                output,
                "replay valid: {} Starlark envelope pairs",
                envelope.report.pairs.len()
            )
            .map_err(|e| SupportError(e.to_string()))?;
            output.flush().map_err(|e| SupportError(e.to_string()))
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
            let programs = prepare_programs(ADDITIONS)?;
            let empty_code = CodeObject {
                instructions: vec![Instruction {
                    opcode: Op::Halt as u8,
                    operand: None,
                }],
                constants: vec![],
                names: vec![],
            };
            let host_clock_noop = sample_host_clock_noop(now)?;
            let empty_guest_call = sample_empty_entries(
                "interpret_bytecode on a Halt-only code object through the same host call wrapper",
                DEFAULT_EMPTY_ENTRY_TARGET_NS,
                DEFAULT_EMPTY_ENTRY_MAX_BATCH,
                DEFAULT_EMPTY_ENTRY_SAMPLES,
                now,
                &mut || {
                    execute_one(&empty_code, None)
                        .map(|_| ())
                        .map_err(support_error)
                },
            )?;
            let report = collect_programs(config, &programs, now).map_err(support_error)?;
            let envelope = GuestEnvelope::new(
                GUEST_ID, WORKLOAD_ID, args.iter().skip(1).cloned().collect(), BASELINE_VARIANT, CANDIDATE_VARIANT,
                "Generate source and compile with the current stub compiler; assemble both CodeObjects and validate outputs before collection.",
                "Fresh interpreter setup, bytecode dispatch for 256 integer additions, result extraction and checksum; excludes source generation, compilation, bytecode assembly, and CodeObject setup.",
                host_clock_noop, empty_guest_call,
                runtime_metadata("starlark-interpreter 0.1.0", "source-compiled-add-chain-256-v1;stack-bytecode-add-chain-256-v1", revision), report,
            )?;
            let path = manifest
                .to_str()
                .ok_or_else(|| SupportError("manifest path is not valid UTF-8".into()))?;
            let mut manifest_file = File::create(path)
                .map_err(|e| SupportError(format!("cannot create manifest: {e}")))?;
            write_outputs(&envelope, &mut manifest_file, output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_compiler_and_stack_bytecode_execute_same_result() {
        assert_eq!(
            source_program(ADDITIONS).unwrap().as_bytes(),
            include_bytes!("../testdata/add-chain-256.star")
        );
        let programs = prepare_programs(ADDITIONS).unwrap();
        assert_eq!(
            execute_batch(&programs, 1, false).unwrap(),
            ADDITIONS as i64
        );
        assert_eq!(execute_batch(&programs, 1, true).unwrap(), ADDITIONS as i64);
        assert_eq!(execute_batch(&programs, 0, true).unwrap(), 0);
        assert!(execute_batch(&programs, usize::MAX, false).is_err());
        assert!(source_program(0).is_err());
        assert!(stack_bytecode(ADDITIONS + 1).is_err());
        assert!(checked_checksum_add(i64::MAX, 1).is_err());
    }

    #[test]
    fn generated_addition_lengths_match_for_compiled_and_assembled_bytecode() {
        for additions in (1..=ADDITIONS).step_by(11) {
            let programs = prepare_programs(additions).unwrap();
            assert_eq!(
                execute_batch(&programs, 1, false).unwrap(),
                additions as i64
            );
            assert_eq!(execute_batch(&programs, 1, true).unwrap(), additions as i64);
        }
    }

    #[test]
    fn fake_clock_collection_rejects_nonprogressing_and_accepts_ordered_batches() {
        let programs = prepare_programs(3).unwrap();
        let report = collect_programs(
            Config {
                pairs: 3,
                target_ns: 1,
                max_batch: 1,
                ..Config::default()
            },
            &programs,
            &mut {
                let mut tick = 0;
                move || {
                    tick += 1;
                    tick
                }
            },
        )
        .unwrap();
        assert_eq!(report.pairs.len(), 3);
        assert!(report
            .pairs
            .iter()
            .all(|p| p.baseline_ns > 0 && p.candidate_ns > 0));
        assert!(collect_programs(
            Config {
                max_batch: MAX_BATCH + 1,
                ..Config::default()
            },
            &programs,
            &mut || 0
        )
        .is_err());
        let mut ticks = [3, 4, 5, 2].into_iter();
        assert!(collect_programs(
            Config {
                pairs: 1,
                target_ns: 1,
                max_batch: 1,
                ..Config::default()
            },
            &programs,
            &mut || ticks.next().unwrap_or(2),
        )
        .is_err());
    }

    #[test]
    fn entrypoint_writes_portable_core_report_and_strict_manifest() {
        let manifest =
            std::env::temp_dir().join(format!("starlark-guest-{}.json", std::process::id()));
        let mut stdout = Vec::new();
        let mut tick = 0i64;
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
                manifest.to_string_lossy().into_owned(),
            ],
            &mut stdout,
            &mut || {
                tick += 100_000;
                tick
            },
        )
        .unwrap();
        let report = coding_adventures_guest_microbenchmark_support::replay_core_report(
            std::str::from_utf8(&stdout).unwrap(),
        )
        .unwrap();
        let envelope = std::fs::read_to_string(&manifest).unwrap();
        let replay = coding_adventures_guest_microbenchmark_support::replay_envelope(
            &envelope,
            GUEST_ID,
            WORKLOAD_ID,
        )
        .unwrap();
        assert_eq!(replay.report, report);
        assert_eq!(
            replay.invocation.first().map(String::as_str),
            Some("--pairs")
        );
        let report_path =
            std::env::temp_dir().join(format!("starlark-core-{}.json", std::process::id()));
        std::fs::write(&report_path, &stdout).unwrap();
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
                manifest.to_string_lossy().into_owned(),
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
        impl std::io::Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        assert!(run(&["bench".into(), "--help".into()], &mut Broken, &mut || 0).is_err());
        std::fs::remove_file(report_path).unwrap();
        std::fs::remove_file(manifest).unwrap();
    }
}
