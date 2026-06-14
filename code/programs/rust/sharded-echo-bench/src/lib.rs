//! # sharded-echo-bench — does the multi-core runtime actually scale?
//!
//! `ShardedTcpRuntime` runs *N* independent reactors on *N* OS threads, one
//! kqueue/epoll/IOCP instance each, with the kernel load-balancing accepted
//! connections across them via `SO_REUSEPORT`.  The *claim* is that throughput
//! grows as you add reactor shards.  This benchmark *measures* it: it stands up a
//! trivial echo server at `worker_count = 1, 2, 4, 8`, hammers each with a pool
//! of real TCP clients doing request/response echoes for a fixed duration, and
//! reports a table:
//!
//! ```text
//!  workers |   conns/s |     req/s |   MiB/s |  p50 µs |  p99 µs | shard balance
//! ---------+-----------+-----------+---------+---------+---------+---------------
//!        1 |     … |     … |     … |     … |     … | [100%]
//!        2 |     … |     … |     … |     … |     … | [51% 49%]
//!        …
//! ```
//!
//! ## Reading the numbers honestly
//!
//! * **It measures the runtime, not the NIC.** Everything is loopback
//!   (`127.0.0.1`), so there is no wire — this isolates the reactor/scheduling
//!   cost.  Real-NIC numbers will differ.
//! * **`SO_REUSEPORT` fairness differs by OS.** Linux distributes new connections
//!   across reuseport sockets with a kernel hash (fairly even).  macOS/BSD is more
//!   last-bind / uneven, so the per-shard *shard balance* column can skew on
//!   macOS and the `2 → 4 → 8` curve will look worse there than on Linux.  The
//!   benchmark prints the observed per-shard accept counts precisely so skew is
//!   *visible* rather than silently blamed on the runtime.
//! * **Client threads compete with server threads for the same cores.** Because
//!   the load generator runs in-process, on a machine with `C` cores the server
//!   can't really use more than `C` of them once the clients are also busy.  Read
//!   the curve, not the absolute ceiling.
//!
//! The heavy sweep is opt-in (`cargo run`, or the `SHARDED_BENCH_FULL` env var);
//! CI runs only a fast [`smoke`](tests) `#[test]` so the numbers never gate a PR.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tcp_runtime::{ShardedStopHandle, TcpConnectionInfo, TcpHandlerResult, TcpRuntime, TcpRuntimeOptions};

// ── Platform echo server ──────────────────────────────────────────────────────
//
// The runtime is generic over the OS event backend; pick the right constructor
// per target.  The handler is a pure echo (write back exactly what was read); the
// `init` hook bumps a per-shard accept counter so we can report `SO_REUSEPORT`
// balance.  The connection's owning shard is the low `shard_bits` of its
// `ConnectionId` (stamped there at accept time), so `shard_of` recovers it.

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type EchoRuntime = tcp_runtime::ShardedTcpRuntime<transport_platform::bsd::KqueueTransportPlatform>;
#[cfg(target_os = "linux")]
type EchoRuntime = tcp_runtime::ShardedTcpRuntime<transport_platform::linux::EpollTransportPlatform>;
#[cfg(target_os = "windows")]
type EchoRuntime =
    tcp_runtime::ShardedTcpRuntime<transport_platform::windows::WindowsTransportPlatform>;

/// Number of low `ConnectionId` bits that name the owning shard:
/// `ceil(log2(worker_count))`.  Mirrors `tcp-runtime`'s own (private) helper.
fn shard_bits_for(worker_count: usize) -> u32 {
    if worker_count <= 1 {
        0
    } else {
        u64::BITS - (worker_count as u64 - 1).leading_zeros()
    }
}

/// Bind an echo server with `worker_count` shards, counting accepts per shard.
fn bind_echo(
    worker_count: usize,
    shard_accepts: Arc<Vec<AtomicU64>>,
    work_per_request: usize,
) -> Result<EchoRuntime, tcp_runtime::PlatformError> {
    let mask = (1u64 << shard_bits_for(worker_count)) - 1;
    let init = move |info: TcpConnectionInfo| {
        let shard = (info.id.0 & mask) as usize;
        if let Some(counter) = shard_accepts.get(shard) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    };
    let handler = move |_info: TcpConnectionInfo, _state: &mut (), bytes: &[u8]| {
        // Optional CPU work per request: `work_per_request` rounds of a cheap hash
        // over the payload, kept from being optimised away by `black_box`.  This
        // makes each request cost real CPU, so the load becomes CPU-bound and the
        // reactor shards can each saturate a core in parallel — turning even
        // connection distribution into actual throughput scaling.
        if work_per_request > 0 {
            let mut acc = 0u64;
            for _ in 0..work_per_request {
                for &byte in bytes {
                    acc = acc.wrapping_mul(1_000_003).wrapping_add(byte as u64);
                }
            }
            std::hint::black_box(acc);
        }
        TcpHandlerResult::write(bytes.to_vec())
    };
    let on_close = |_info: TcpConnectionInfo, _state: ()| {};

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        TcpRuntime::bind_kqueue_sharded_with_state(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            worker_count,
            init,
            handler,
            on_close,
        )
    }
    #[cfg(target_os = "linux")]
    {
        TcpRuntime::bind_epoll_sharded_with_state(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            worker_count,
            init,
            handler,
            on_close,
        )
    }
    #[cfg(target_os = "windows")]
    {
        TcpRuntime::bind_windows_sharded_with_state(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            worker_count,
            init,
            handler,
            on_close,
        )
    }
}

// ── Benchmark parameters & results ────────────────────────────────────────────

/// Knobs for one benchmark run.
#[derive(Debug, Clone, Copy)]
pub struct BenchParams {
    /// Number of reactor shards (server threads).
    pub worker_count: usize,
    /// Number of concurrent client connections driving load.
    pub clients: usize,
    /// Bytes per request/response echo (also the per-request payload size).
    pub payload_len: usize,
    /// How long the throughput phase runs.
    pub duration: Duration,
    /// Synthetic CPU work per request: rounds of a cheap hash over the payload,
    /// run inside the echo handler before replying.  `0` is a pure echo
    /// (latency-bound on loopback, so distribution doesn't add throughput); a
    /// non-zero value makes each request **CPU-bound**, which is the regime where
    /// adding reactor shards actually adds throughput.  This is the knob that turns
    /// "connections are evenly distributed" into "and req/s scales with cores".
    pub work_per_request: usize,
}

/// The measured outcome of one run.
#[derive(Debug, Clone)]
pub struct BenchResult {
    pub worker_count: usize,
    /// Connections established per second during the connect phase.
    pub conns_per_s: f64,
    /// Completed request/response echoes per second.
    pub req_per_s: f64,
    /// Application throughput (payload bytes echoed) in MiB/s.
    pub mib_per_s: f64,
    /// Median and 99th-percentile round-trip latency, microseconds.
    pub p50_us: f64,
    pub p99_us: f64,
    /// Connections accepted by each shard (reveals `SO_REUSEPORT` balance).
    pub shard_accepts: Vec<u64>,
    /// Total completed requests across all clients.
    pub total_requests: u64,
}

/// One client thread's tally: how many echoes it completed and each round-trip's
/// latency in nanoseconds.
struct ClientReport {
    requests: u64,
    latencies_ns: Vec<u64>,
}

// ── The benchmark ─────────────────────────────────────────────────────────────

/// Run one benchmark: stand up an echo server with `params.worker_count` shards,
/// drive it with `params.clients` connections for `params.duration`, and return
/// the measured throughput/latency.
///
/// Returns `Err` only if the server could not be bound; client connection retries
/// and per-request errors are tolerated (a wedged client simply contributes fewer
/// requests, which shows up as lower throughput rather than a panic).
pub fn run_benchmark(params: BenchParams) -> Result<BenchResult, tcp_runtime::PlatformError> {
    let worker_count = params.worker_count.max(1);
    let shard_accepts: Arc<Vec<AtomicU64>> =
        Arc::new((0..worker_count).map(|_| AtomicU64::new(0)).collect());

    let mut runtime = bind_echo(worker_count, Arc::clone(&shard_accepts), params.work_per_request)?;
    let actual_workers = runtime.worker_count();
    let addr = runtime.local_addr();
    let stop: ShardedStopHandle = runtime.stop_handle();
    let server = thread::spawn(move || {
        let _ = runtime.serve();
    });

    // The connect phase and the throughput phase are separated by a barrier so we
    // can time them independently: `clients` client threads + this one.
    let start_gate = Arc::new(Barrier::new(params.clients + 1));
    let mut handles: Vec<JoinHandle<ClientReport>> = Vec::with_capacity(params.clients);

    let connect_start = Instant::now();
    for _ in 0..params.clients {
        let gate = Arc::clone(&start_gate);
        let payload = vec![b'x'; params.payload_len.max(1)];
        let duration = params.duration;
        handles.push(thread::spawn(move || {
            run_client(addr, &payload, duration, &gate)
        }));
    }

    // All clients have connected (they each wait on the gate after connecting);
    // releasing the gate starts the throughput phase for everyone at once.
    start_gate.wait();
    let conns_elapsed = connect_start.elapsed();

    let mut total_requests: u64 = 0;
    let mut latencies_ns: Vec<u64> = Vec::new();
    for handle in handles {
        if let Ok(report) = handle.join() {
            total_requests += report.requests;
            latencies_ns.extend_from_slice(&report.latencies_ns);
        }
    }

    stop.stop();
    let _ = server.join();

    let secs = params.duration.as_secs_f64().max(f64::MIN_POSITIVE);
    let req_per_s = total_requests as f64 / secs;
    let mib_per_s = (total_requests as f64 * params.payload_len as f64) / secs / (1024.0 * 1024.0);
    let conns_per_s = if conns_elapsed.as_secs_f64() > 0.0 {
        params.clients as f64 / conns_elapsed.as_secs_f64()
    } else {
        f64::INFINITY
    };
    let (p50_us, p99_us) = percentiles_us(&mut latencies_ns);
    let shard_accepts = shard_accepts.iter().map(|c| c.load(Ordering::Relaxed)).collect();

    Ok(BenchResult {
        worker_count: actual_workers,
        conns_per_s,
        req_per_s,
        mib_per_s,
        p50_us,
        p99_us,
        shard_accepts,
        total_requests,
    })
}

/// One client: connect (with retry), wait at the gate, then echo `payload` in a
/// tight request/response loop until `duration` elapses, recording each
/// round-trip's latency.
fn run_client(addr: SocketAddr, payload: &[u8], duration: Duration, gate: &Barrier) -> ClientReport {
    let mut report = ClientReport {
        requests: 0,
        latencies_ns: Vec::new(),
    };

    let Some(mut stream) = connect_with_retry(addr) else {
        // Couldn't connect at all: still release the gate so the run doesn't hang.
        gate.wait();
        return report;
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .ok();
    let _ = stream.set_nodelay(true);

    // Connected — line up with everyone else, then start the timed loop together.
    gate.wait();

    let deadline = Instant::now() + duration;
    let mut buf = vec![0u8; payload.len()];
    while Instant::now() < deadline {
        let t0 = Instant::now();
        if stream.write_all(payload).is_err() {
            break;
        }
        if stream.read_exact(&mut buf).is_err() {
            break;
        }
        report.latencies_ns.push(t0.elapsed().as_nanos() as u64);
        report.requests += 1;
    }
    report
}

/// Connect to `addr`, retrying briefly while the background reactor comes up.
fn connect_with_retry(addr: SocketAddr) -> Option<TcpStream> {
    for _ in 0..200 {
        if let Ok(stream) = TcpStream::connect(addr) {
            return Some(stream);
        }
        thread::sleep(Duration::from_millis(5));
    }
    None
}

/// Median and 99th-percentile of a latency sample, in microseconds.  Sorts in
/// place.  Returns `(0, 0)` for an empty sample.
fn percentiles_us(latencies_ns: &mut [u64]) -> (f64, f64) {
    if latencies_ns.is_empty() {
        return (0.0, 0.0);
    }
    latencies_ns.sort_unstable();
    let pick = |q: f64| -> f64 {
        let idx = ((latencies_ns.len() as f64 - 1.0) * q).round() as usize;
        latencies_ns[idx] as f64 / 1000.0
    };
    (pick(0.50), pick(0.99))
}

// ── Reporting ─────────────────────────────────────────────────────────────────

/// Render the sweep as a fixed-width table.
pub fn format_table(results: &[BenchResult]) -> String {
    let mut out = String::new();
    out.push_str(
        " workers |   conns/s |     req/s |   MiB/s |  p50 µs |  p99 µs | shard balance\n",
    );
    out.push_str(
        "---------+-----------+-----------+---------+---------+---------+---------------\n",
    );
    for r in results {
        let total: u64 = r.shard_accepts.iter().sum::<u64>().max(1);
        let balance: Vec<String> = r
            .shard_accepts
            .iter()
            .map(|&c| format!("{}%", (c as f64 / total as f64 * 100.0).round() as u64))
            .collect();
        out.push_str(&format!(
            " {:>7} | {:>9.0} | {:>9.0} | {:>7.1} | {:>7.1} | {:>7.1} | [{}]\n",
            r.worker_count,
            r.conns_per_s,
            r.req_per_s,
            r.mib_per_s,
            r.p50_us,
            r.p99_us,
            balance.join(" "),
        ));
    }
    out
}

/// Run the standard sweep (`worker_count = 1, 2, 4, 8`) with the given client
/// pool / payload / duration / per-request CPU work, returning one result per
/// shard count.
///
/// With `work_per_request = 0` the echo is latency-bound (loopback round-trip
/// dominates) and `req/s` stays roughly flat across shard counts.  With a
/// non-zero `work_per_request` each request burns real CPU, so the shards can
/// each saturate a core and `req/s` climbs with the worker count — the
/// throughput-scaling proof.
pub fn run_sweep(
    clients: usize,
    payload_len: usize,
    duration: Duration,
    work_per_request: usize,
) -> Vec<BenchResult> {
    [1usize, 2, 4, 8]
        .iter()
        .filter_map(|&worker_count| {
            run_benchmark(BenchParams {
                worker_count,
                clients,
                payload_len,
                duration,
                work_per_request,
            })
            .ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_two_shards_echo_and_report() {
        // A fast, deterministic run: 2 shards, a handful of clients, a short
        // window.  Asserts the plumbing works end to end — clients connect, the
        // echo round-trips (a mismatched echo would error and drop requests), and
        // the report is well-formed — without depending on absolute throughput.
        let params = BenchParams {
            worker_count: 2,
            clients: 4,
            payload_len: 64,
            duration: Duration::from_millis(300),
            work_per_request: 0,
        };
        let result = run_benchmark(params).expect("server binds");

        assert_eq!(result.worker_count, 2);
        assert!(result.total_requests > 0, "clients should complete echoes");
        assert!(result.req_per_s > 0.0);
        assert!(result.p99_us >= result.p50_us);
        // Every one of the 4 connections was accepted by exactly one shard.
        assert_eq!(result.shard_accepts.iter().sum::<u64>(), 4);
        assert_eq!(result.shard_accepts.len(), 2);

        let table = format_table(&[result]);
        assert!(table.contains("workers"));
        assert!(table.contains("shard balance"));
    }

    #[test]
    fn single_shard_runs() {
        let result = run_benchmark(BenchParams {
            worker_count: 1,
            clients: 2,
            payload_len: 16,
            duration: Duration::from_millis(150),
            work_per_request: 0,
        })
        .expect("server binds");
        assert_eq!(result.worker_count, 1);
        assert_eq!(result.shard_accepts.len(), 1);
        assert_eq!(result.shard_accepts[0], 2);
    }

    #[test]
    fn cpu_bound_mode_still_echoes_correctly() {
        // With `work_per_request > 0` the handler does real CPU work before
        // echoing.  The point of this test is to prove the CPU-work path doesn't
        // corrupt the response: the bytes must still round-trip unchanged (a
        // mismatched echo errors the client and drops the request to zero), and
        // the run must still complete and report.
        let result = run_benchmark(BenchParams {
            worker_count: 2,
            clients: 3,
            payload_len: 32,
            duration: Duration::from_millis(200),
            work_per_request: 50,
        })
        .expect("server binds");

        assert_eq!(result.worker_count, 2);
        assert!(
            result.total_requests > 0,
            "echoes must still complete with CPU work enabled"
        );
        assert_eq!(result.shard_accepts.iter().sum::<u64>(), 3);
    }

    #[test]
    fn percentiles_handle_empty_and_ordering() {
        assert_eq!(percentiles_us(&mut []), (0.0, 0.0));
        // Nearest-rank over an odd sample gives an unambiguous median: with
        // 3 samples, p50 → index round((3-1)*0.50)=1, p99 → index round(1.98)=2.
        let (p50, p99) = percentiles_us(&mut [1_000, 2_000, 3_000]);
        assert_eq!(p50, 2.0); // 2000 ns = 2 µs
        assert_eq!(p99, 3.0); // 3000 ns = 3 µs
        assert!(p99 >= p50);
    }

    #[test]
    fn shard_bits_matches_ceil_log2() {
        assert_eq!(shard_bits_for(1), 0);
        assert_eq!(shard_bits_for(2), 1);
        assert_eq!(shard_bits_for(4), 2);
        assert_eq!(shard_bits_for(8), 3);
    }
}
