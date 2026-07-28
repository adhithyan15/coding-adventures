//! `sir-bench` — run the benchmark corpus and print a Markdown report.
//!
//! ```text
//! cargo run --release -p sir-bench          # every program, every backend
//! ```
//!
//! Each program is lowered once; every available backend is emitted, compiled
//! (if native), and timed (median of N runs after warmup).  A backend whose
//! toolchain is absent — or that a v0 backend does not yet accept — is reported
//! as a skip, so the report is honest about what actually ran on this host.

use sir_bench::{corpus, lower, markdown_report, measure, Sample, Target};

fn main() {
    println!("# SIR cross-backend performance benchmarks\n");
    println!(
        "How fast is the code each backend generates from the SAME Ruby program, \
         lowered through the Semantic IR?  `run ms` is the number to read: the \
         generated program's own execution time.\n"
    );

    // Report which toolchains are present, so a reader knows why a row skipped.
    let present: Vec<&str> = Target::all()
        .iter()
        .filter(|t| t.available())
        .map(|t| t.tag())
        .collect();
    println!("Toolchains available on this host: {}\n", present.join(", "));

    for bench in corpus() {
        let module = match lower(bench.name, bench.ruby) {
            Ok(m) => m,
            Err(e) => {
                println!("### `{}`\n\n_frontend failed: {}_\n", bench.name, e);
                continue;
            }
        };

        let results: Vec<(Target, Sample)> = Target::all()
            .iter()
            .map(|&t| (t, measure(&bench, &module, t)))
            .collect();

        // The expected output, taken from any cell that ran (they must agree —
        // conformance guarantees it; here we just surface it for context).
        let expected = results.iter().find_map(|(_, s)| match s {
            Sample::Ran { stdout, .. } => Some(stdout.clone()),
            _ => None,
        });

        print!(
            "{}",
            markdown_report(&bench, expected.as_deref(), &results)
        );
    }
}
