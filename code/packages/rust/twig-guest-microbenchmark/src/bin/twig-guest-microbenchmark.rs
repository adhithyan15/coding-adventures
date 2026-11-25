//! Native host-clocked entry point for the prepared Twig VM guest.
use coding_adventures_guest_microbenchmark_support::{
    elapsed_ns, parse_args, replay_core_file, replay_file, runtime_metadata, sample_empty_entries,
    sample_host_clock_noop, select_revision, write_outputs, Command, Error as SupportError,
    GuestEnvelope, DEFAULT_EMPTY_ENTRY_SAMPLES,
};
use coding_adventures_microbenchmark::Report;
use coding_adventures_twig_guest_microbenchmark::{TwigGuestBenchmark, WORKLOAD_ID};
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    match run(&env::args().collect::<Vec<_>>(), &mut io::stdout()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("twig-guest-microbenchmark: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String], stdout: &mut impl Write) -> Result<(), String> {
    let command = parse_args(args).map_err(|error| error.to_string())?;
    match command {
        Command::Help => {
            writeln!(stdout, "Usage: twig-guest-microbenchmark --manifest FILE [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID]")
                .map_err(|error| error.to_string())?;
        }
        Command::ReplayCore(path) => {
            let report =
                replay_core_file(&path.to_string_lossy()).map_err(|error| error.to_string())?;
            write_replay_result(stdout, report).map_err(|error| error.to_string())?;
        }
        Command::ReplayEnvelope(path) => {
            let envelope = replay_file(&path.to_string_lossy(), "twig-vm", WORKLOAD_ID)
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
            let guest = TwigGuestBenchmark::prepare().map_err(|error| error.to_string())?;
            let clock_origin = Instant::now();
            let mut now = || elapsed_ns(&clock_origin);

            // Prepare and verify the compiled guest before the shared collector's first read.
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
                                "empty Twig guest returned nonzero checksum".to_owned(),
                            ))
                        }
                    })
            };
            let empty_guest_call = sample_empty_entries(
                "TwigVM::dispatch::run on the precompiled `0` module, one guest entry per operation",
                config.target_ns,
                config.max_batch,
                DEFAULT_EMPTY_ENTRY_SAMPLES,
                &mut now,
                &mut empty_entry,
            )
            .map_err(|error| error.to_string())?;

            let envelope = GuestEnvelope::new(
                "twig-vm",
                WORKLOAD_ID,
                args.to_vec(),
                "recursive sum(32..0)",
                "accumulator recursive sum(32..0)",
                "TwigVM::compile and warm checksum verification complete before collection",
                "host monotonic Instant around dispatch::run for one compiled Twig IIR guest per operation",
                host_clock_noop,
                empty_guest_call,
                runtime_metadata(
                    "twig-vm/0.25.0",
                    "embedded source constants; sum of integers 1..=32; checksum 528",
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
    use coding_adventures_twig_guest_microbenchmark::GUEST_CHECKSUM;

    #[test]
    fn cli_rejects_out_of_range_pair_count_before_guest_setup() {
        let args = vec![
            "twig-guest-microbenchmark".into(),
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
        let guest = TwigGuestBenchmark::prepare().unwrap();
        assert_eq!(guest.baseline_batch(1).unwrap(), GUEST_CHECKSUM);
        assert_eq!(guest.candidate_batch(1).unwrap(), GUEST_CHECKSUM);
    }
}
