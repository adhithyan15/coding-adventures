//! CLI entry point for the sharded echo benchmark.
//!
//! Running the full sweep is intentionally opt-in (it spins up real servers and
//! saturates cores for a few seconds per shard count), so it is *not* what CI
//! runs — CI runs the fast `#[test]` smoke instead.  Trigger the full sweep with:
//!
//! ```text
//! cargo run -p sharded-echo-bench --release
//! # or, to confirm intent in a scripted context:
//! SHARDED_BENCH_FULL=1 cargo run -p sharded-echo-bench --release
//! ```
//!
//! Tunables via env vars: `BENCH_CLIENTS` (default 64), `BENCH_PAYLOAD` (bytes,
//! default 64), `BENCH_SECONDS` (per shard count, default 3).

use std::time::Duration;

use sharded_echo_bench::{format_table, run_sweep};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let clients = env_usize("BENCH_CLIENTS", 64);
    let payload = env_usize("BENCH_PAYLOAD", 64);
    let seconds = env_usize("BENCH_SECONDS", 3);
    let duration = Duration::from_secs(seconds as u64);

    println!(
        "sharded-echo-bench — loopback echo, {clients} clients, {payload}B payload, \
         {seconds}s per shard count\n\
         (measures the runtime, not a NIC; SO_REUSEPORT balance is even on Linux, \
         skewed on macOS/BSD)\n"
    );

    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    println!("available_parallelism = {cores} core(s)\n");

    let results = run_sweep(clients, payload, duration);
    print!("{}", format_table(&results));

    // A one-line takeaway: speedup of the largest shard count vs a single shard.
    if let (Some(first), Some(last)) = (results.first(), results.last()) {
        if first.req_per_s > 0.0 {
            println!(
                "\n{}-shard throughput is {:.2}x the single-shard baseline.",
                last.worker_count,
                last.req_per_s / first.req_per_s
            );
        }
    }
}
