//! Native host-clocked entry point for a prepared WebAssembly guest module.
use coding_adventures_guest_microbenchmark_support::{
    elapsed_ns, parse_args, replay_core_file, replay_file, runtime_metadata, sample_empty_entries,
    sample_host_clock_noop, select_revision, write_outputs, Command, Error as SupportError,
    GuestEnvelope, DEFAULT_EMPTY_ENTRY_SAMPLES,
};
use coding_adventures_microbenchmark::Report;
use coding_adventures_wasm_guest_microbenchmark::{WasmGuestBenchmark, WORKLOAD_ID};
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    match run(&env::args().collect::<Vec<_>>(), &mut io::stdout()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("wasm-guest-microbenchmark: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String], stdout: &mut impl Write) -> Result<(), String> {
    let command = parse_args(args).map_err(|error| error.to_string())?;
    match command {
        Command::Help => {
            writeln!(stdout, "Usage: wasm-guest-microbenchmark --manifest FILE [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID]")
                .map_err(|error| error.to_string())?;
        }
        Command::ReplayCore(path) => {
            let report =
                replay_core_file(&path.to_string_lossy()).map_err(|error| error.to_string())?;
            write_replay_result(stdout, report).map_err(|error| error.to_string())?;
        }
        Command::ReplayEnvelope(path) => {
            let envelope = replay_file(&path.to_string_lossy(), "wasm-runtime", WORKLOAD_ID)
                .map_err(|error| error.to_string())?;
            write_replay_result(stdout, envelope.report).map_err(|error| error.to_string())?;
        }
        Command::Run {
            config,
            revision,
            manifest,
        } => {
            let revision = select_revision(
                revision,
                option_env!("GIT_COMMIT"),
                option_env!("GITHUB_SHA"),
            )
            .map_err(|error| error.to_string())?;
            let mut guest = WasmGuestBenchmark::prepare().map_err(|error| error.to_string())?;
            let clock_origin = Instant::now();
            let mut now = || elapsed_ns(&clock_origin);

            // Encoding, validation, instance construction, and warm checks are outside timing.
            let report = guest
                .collect(config.clone(), &mut now)
                .map_err(|error| error.to_string())?;
            let host_clock_noop =
                sample_host_clock_noop(&mut now).map_err(|error| error.to_string())?;
            let mut empty_entry = || {
                guest
                    .empty_batch(1)
                    .map_err(|error| SupportError(error.to_string()))
                    .and_then(|checksum| {
                        if checksum == 0 {
                            Ok(())
                        } else {
                            Err(SupportError(
                                "empty WASM guest returned nonzero checksum".to_owned(),
                            ))
                        }
                    })
            };
            let empty_guest_call = sample_empty_entries(
                "WasmRuntime::call on prepared `empty` export; includes per-call engine reconstruction and state transfer",
                config.target_ns,
                config.max_batch,
                DEFAULT_EMPTY_ENTRY_SAMPLES,
                &mut now,
                &mut empty_entry,
            )
            .map_err(|error| error.to_string())?;

            let envelope = GuestEnvelope::new(
                "wasm-runtime",
                WORKLOAD_ID,
                args.to_vec(),
                "recursive sum(32..0)",
                "guest loop sum(32..0)",
                "encode, parse, validate, instantiate all variants and verify checksum before collection",
                "host monotonic Instant around WasmRuntime::call; includes per-call engine setup/state transfer and guest instruction execution",
                host_clock_noop,
                empty_guest_call,
                runtime_metadata(
                    "wasm-runtime/0.6.38",
                    "embedded WebAssembly 1.0 module; recursive/loop sum 1..=32; checksum 528",
                    revision,
                ),
                report,
            )
            .map_err(|error| error.to_string())?;

            let mut manifest_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&manifest)
                .map_err(|error| {
                    format!("cannot create manifest {}: {error}", manifest.display())
                })?;
            write_outputs(&envelope, &mut manifest_file, stdout)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn write_replay_result(stdout: &mut impl Write, report: Report) -> io::Result<()> {
    writeln!(
        stdout,
        "replay valid: {} pairs, ratio={}",
        report.pairs.len(),
        report.analysis.ratio
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_wasm_guest_microbenchmark::GUEST_CHECKSUM;

    #[test]
    fn cli_rejects_out_of_range_pair_count_before_guest_setup() {
        let args = vec![
            "wasm-guest-microbenchmark".into(),
            "--pairs".into(),
            "10001".into(),
            "--manifest".into(),
            "not-created.json".into(),
        ];
        let mut output = Vec::new();
        assert!(run(&args, &mut output).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn default_workload_checksum_matches_guest_variants() {
        let mut guest = WasmGuestBenchmark::prepare().unwrap();
        assert_eq!(guest.recursive_batch(1).unwrap(), GUEST_CHECKSUM);
        assert_eq!(guest.loop_batch(1).unwrap(), GUEST_CHECKSUM);
    }
}
