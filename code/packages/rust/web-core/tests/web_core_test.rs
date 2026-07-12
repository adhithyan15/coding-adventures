//! Integration tests for web-core.
//!
//! Hook and pipeline tests exercise `WebApp::handle` with synthesised
//! `HttpRequest` values. End-to-end tests spin up a real server on port 0.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use embeddable_http_server::{HttpRequest, HttpServerOptions};
use http_core::{Header, HttpVersion, RequestHead};
use tcp_runtime::{ConnectionId, TcpConnectionInfo};
use web_core::{
    LogLevel, MailboxWebServer, RouteLookupResult, Router, WebApp, WebResponse, WebServer,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_http_request(method: &str, target: &str) -> HttpRequest {
    HttpRequest {
        connection: TcpConnectionInfo {
            id: ConnectionId(0),
            peer_addr: SocketAddr::from(([127, 0, 0, 1], 1024)),
            local_addr: SocketAddr::from(([127, 0, 0, 1], 3000)),
        },
        head: RequestHead {
            method: method.to_string(),
            target: target.to_string(),
            version: HttpVersion { major: 1, minor: 1 },
            headers: vec![Header {
                name: "Host".into(),
                value: "localhost".into(),
            }],
        },
        body: Vec::new(),
    }
}

fn http_request(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

    let req_str = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req_str.as_bytes()).expect("write request");

    let mut reader = BufReader::new(&stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).expect("read status line");

    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .expect("status code field")
        .parse()
        .expect("parse status code");

    let mut content_length = 0usize;
    let mut response_headers: Vec<String> = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            break;
        }
        if trimmed.to_ascii_lowercase().starts_with("content-length:") {
            content_length = trimmed.split_once(':').map(|x| x.1)
                .unwrap_or("")
                .trim()
                .parse()
                .unwrap_or(0);
        }
        response_headers.push(trimmed);
    }

    let mut body_buf = vec![0u8; content_length];
    std::io::Read::read_exact(&mut reader, &mut body_buf).unwrap_or(());
    (status, String::from_utf8_lossy(&body_buf).into_owned())
}

fn http_get(port: u16, path: &str) -> (u16, String) {
    http_request(port, "GET", path, "")
}

/// Bind and start a `WebServer` on port 0, returning the port and stop handle.
fn start_server(app: WebApp) -> (u16, tcp_runtime::StopHandle) {
    let app = Arc::new(app);

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    let mut server =
        WebServer::bind_kqueue("127.0.0.1:0", HttpServerOptions::default(), Arc::clone(&app))
            .expect("bind kqueue");

    #[cfg(target_os = "linux")]
    let mut server =
        WebServer::bind_epoll("127.0.0.1:0", HttpServerOptions::default(), Arc::clone(&app))
            .expect("bind epoll");

    #[cfg(target_os = "windows")]
    let mut server =
        WebServer::bind_windows("127.0.0.1:0", HttpServerOptions::default(), Arc::clone(&app))
            .expect("bind windows");

    let port = server.local_addr().port();
    let stop = server.stop_handle();
    thread::spawn(move || {
        let _ = server.serve();
    });
    thread::sleep(Duration::from_millis(20));
    (port, stop)
}

// ---------------------------------------------------------------------------
// Router unit tests
// ---------------------------------------------------------------------------

#[test]
fn router_matches_static_path() {
    let mut router = Router::new();
    router.get("/hello", |_| WebResponse::text("hi"));
    assert!(matches!(router.lookup("GET", "/hello"), RouteLookupResult::Matched(_)));
}

#[test]
fn router_extracts_named_params() {
    let mut router = Router::new();
    router.get("/hello/:name", |_| WebResponse::text("hi"));
    match router.lookup("GET", "/hello/Adhithya") {
        RouteLookupResult::Matched(m) => {
            assert_eq!(m.params, vec![("name".into(), "Adhithya".into())]);
        }
        _ => panic!("expected Matched"),
    }
}

#[test]
fn router_returns_not_found_for_unknown_path() {
    let mut router = Router::new();
    router.get("/hello/:name", |_| WebResponse::text("hi"));
    assert!(matches!(router.lookup("GET", "/goodbye"), RouteLookupResult::NotFound));
}

#[test]
fn router_returns_method_not_allowed_when_path_matches_wrong_method() {
    let mut router = Router::new();
    router.get("/hello/:name", |_| WebResponse::text("hi"));
    assert!(matches!(
        router.lookup("POST", "/hello/Adhithya"),
        RouteLookupResult::MethodNotAllowed
    ));
}

#[test]
fn router_first_registered_route_wins() {
    let mut router = Router::new();
    // `by-id` registered first, `special` second — `:id` should win.
    router.get("/items/:id", |_| WebResponse::text("by-id"));
    router.get("/items/special", |_| WebResponse::text("special"));
    match router.lookup("GET", "/items/special") {
        RouteLookupResult::Matched(m) => {
            // `m.params` has `id = "special"` because the first route won.
            assert_eq!(m.params, vec![("id".into(), "special".into())]);
        }
        _ => panic!("expected Matched"),
    }
}

#[test]
fn router_method_is_case_insensitive() {
    let mut router = Router::new();
    router.get("/ping", |_| WebResponse::text("pong"));
    assert!(matches!(router.lookup("get", "/ping"), RouteLookupResult::Matched(_)));
    assert!(matches!(router.lookup("Get", "/ping"), RouteLookupResult::Matched(_)));
}

// ---------------------------------------------------------------------------
// Hook pipeline tests (via WebApp::handle)
// ---------------------------------------------------------------------------

#[test]
fn before_routing_can_short_circuit() {
    let mut app = WebApp::new();
    app.get("/secret", |_| WebResponse::text("secret content"));
    app.before_routing(|_| Some(WebResponse::new(401, b"Unauthorized".to_vec())));

    let resp = app.handle(make_http_request("GET", "/secret"));
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body, b"Unauthorized");
}

#[test]
fn before_routing_passes_through_when_none() {
    let mut app = WebApp::new();
    app.get("/hello", |_| WebResponse::text("hello"));
    app.before_routing(|_| None);

    let resp = app.handle(make_http_request("GET", "/hello"));
    assert_eq!(resp.status, 200);
}

#[test]
fn on_not_found_overrides_default_404() {
    let mut app = WebApp::new();
    app.on_not_found(|_| WebResponse::new(404, b"custom not found".to_vec()));

    let resp = app.handle(make_http_request("GET", "/missing"));
    assert_eq!(resp.status, 404);
    assert_eq!(resp.body, b"custom not found");
}

#[test]
fn default_404_when_no_hook_registered() {
    let mut app = WebApp::new();
    app.get("/exists", |_| WebResponse::text("exists"));

    let resp = app.handle(make_http_request("GET", "/missing"));
    assert_eq!(resp.status, 404);
}

#[test]
fn on_method_not_allowed_overrides_default_405() {
    let mut app = WebApp::new();
    app.get("/items", |_| WebResponse::text("items"));
    app.on_method_not_allowed(|_| WebResponse::new(405, b"custom 405".to_vec()));

    let resp = app.handle(make_http_request("DELETE", "/items"));
    assert_eq!(resp.status, 405);
    assert_eq!(resp.body, b"custom 405");
}

#[test]
fn default_405_when_no_hook_registered() {
    let mut app = WebApp::new();
    app.get("/items", |_| WebResponse::text("items"));

    let resp = app.handle(make_http_request("DELETE", "/items"));
    assert_eq!(resp.status, 405);
}

#[test]
fn panicking_handler_triggers_on_handler_error() {
    let mut app = WebApp::new();
    app.get("/boom", |_| panic!("intentional panic"));
    app.on_handler_error(|_, _| WebResponse::new(500, b"caught panic".to_vec()));

    let resp = app.handle(make_http_request("GET", "/boom"));
    assert_eq!(resp.status, 500);
    assert_eq!(resp.body, b"caught panic");
}

#[test]
fn default_500_on_panic_when_no_error_hook() {
    let mut app = WebApp::new();
    app.get("/boom", |_| panic!("intentional panic"));

    let resp = app.handle(make_http_request("GET", "/boom"));
    assert_eq!(resp.status, 500);
}

#[test]
fn after_handler_hooks_chain_in_registration_order() {
    let mut app = WebApp::new();
    app.get("/chain", |_| WebResponse::text("base"));
    app.after_handler(|_, mut r| {
        r.headers.push(("x-step".into(), "one".into()));
        r
    });
    app.after_handler(|_, mut r| {
        r.headers.push(("x-step".into(), "two".into()));
        r
    });

    let resp = app.handle(make_http_request("GET", "/chain"));
    let steps: Vec<_> = resp
        .headers
        .iter()
        .filter(|h| h.name.eq_ignore_ascii_case("x-step"))
        .map(|h| h.value.as_str())
        .collect();
    assert_eq!(steps, ["one", "two"]);
}

#[test]
fn route_params_are_injected_into_request() {
    let mut app = WebApp::new();
    app.get("/hello/:name", |req| {
        let name = req.route_params.get("name").cloned().unwrap_or_default();
        WebResponse::text(name)
    });

    let resp = app.handle(make_http_request("GET", "/hello/Adhithya"));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"Adhithya");
}

#[test]
fn query_params_are_parsed_from_target() {
    let mut app = WebApp::new();
    app.get("/search", |req| {
        let q = req.query_params.get("q").cloned().unwrap_or_default();
        WebResponse::text(q)
    });

    let resp = app.handle(make_http_request("GET", "/search?q=rust"));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"rust");
}

#[test]
fn after_send_fires_after_response() {
    let fired = Arc::new(AtomicUsize::new(0));
    let fired_clone = Arc::clone(&fired);

    let mut app = WebApp::new();
    app.get("/ping", |_| WebResponse::text("pong"));
    app.after_send(move |_, _, _| {
        fired_clone.fetch_add(1, Ordering::SeqCst);
    });

    app.handle(make_http_request("GET", "/ping"));
    assert_eq!(fired.load(Ordering::SeqCst), 1);
}

#[test]
fn on_log_hook_receives_application_events() {
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let log_clone = Arc::clone(&log);

    let mut app = WebApp::new();
    app.on_log(move |level, msg, _| {
        log_clone.lock().unwrap().push(format!("{level:?}: {msg}"));
    });
    app.log(LogLevel::Info, "hello from app", &HashMap::new());

    let entries = log.lock().unwrap().clone();
    assert_eq!(entries, ["Info: hello from app"]);
}

#[test]
fn before_handler_can_short_circuit() {
    let handler_called = Arc::new(AtomicUsize::new(0));
    let handler_clone = Arc::clone(&handler_called);

    let mut app = WebApp::new();
    app.get("/gated", move |_| {
        handler_clone.fetch_add(1, Ordering::SeqCst);
        WebResponse::text("handler ran")
    });
    app.before_handler(|_| Some(WebResponse::new(403, b"Forbidden".to_vec())));

    let resp = app.handle(make_http_request("GET", "/gated"));
    assert_eq!(resp.status, 403);
    assert_eq!(handler_called.load(Ordering::SeqCst), 0, "handler should not have run");
}

// ---------------------------------------------------------------------------
// End-to-end tests
// ---------------------------------------------------------------------------

#[test]
fn e2e_hello_route_with_name_param() {
    let mut app = WebApp::new();
    app.get("/hello/:name", |req| {
        let name = req.route_params.get("name").cloned().unwrap_or_default();
        WebResponse::text(format!("Hello {name}"))
    });
    let (port, stop) = start_server(app);
    let (status, body) = http_get(port, "/hello/Adhithya");
    stop.stop();
    assert_eq!(status, 200);
    assert_eq!(body, "Hello Adhithya");
}

#[test]
fn e2e_missing_path_returns_404() {
    let mut app = WebApp::new();
    app.get("/hello/:name", |_| WebResponse::text("hi"));
    let (port, stop) = start_server(app);
    let (status, _) = http_get(port, "/missing");
    stop.stop();
    assert_eq!(status, 404);
}

#[test]
fn e2e_wrong_method_returns_405() {
    let mut app = WebApp::new();
    app.get("/hello/:name", |_| WebResponse::text("hi"));
    let (port, stop) = start_server(app);
    let (status, _) = http_request(port, "DELETE", "/hello/Adhithya", "");
    stop.stop();
    assert_eq!(status, 405);
}

#[test]
fn e2e_query_string_accessible_in_handler() {
    let mut app = WebApp::new();
    app.get("/search", |req| {
        let q = req.query_params.get("q").cloned().unwrap_or_default();
        WebResponse::text(format!("query={q}"))
    });
    let (port, stop) = start_server(app);
    let (status, body) = http_get(port, "/search?q=rust");
    stop.stop();
    assert_eq!(status, 200);
    assert_eq!(body, "query=rust");
}

#[test]
fn e2e_before_routing_rejects_request() {
    let mut app = WebApp::new();
    app.get("/secret", |_| WebResponse::text("secret"));
    app.before_routing(|_| Some(WebResponse::new(401, b"Unauthorized".to_vec())));
    let (port, stop) = start_server(app);
    let (status, body) = http_get(port, "/secret");
    stop.stop();
    assert_eq!(status, 401);
    assert_eq!(body, "Unauthorized");
}

#[test]
fn e2e_after_handler_adds_header() {
    let mut app = WebApp::new();
    app.get("/ping", |_| WebResponse::text("pong"));
    app.after_handler(|_, mut r| {
        r.headers.push(("x-powered-by".into(), "web-core".into()));
        r
    });
    let (port, stop) = start_server(app);

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(stream, "GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();

    let mut reader = BufReader::new(&stream);
    let mut all_headers: Vec<String> = Vec::new();
    let mut content_length = 0usize;
    let mut first = true;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let trimmed = line.trim().to_string();
        if first { first = false; continue; } // skip status line
        if trimmed.is_empty() { break; }
        if trimmed.to_ascii_lowercase().starts_with("content-length:") {
            content_length = trimmed.split_once(':').map(|x| x.1).unwrap_or("").trim().parse().unwrap_or(0);
        }
        all_headers.push(trimmed);
    }
    let mut body_buf = vec![0u8; content_length];
    std::io::Read::read_exact(&mut reader, &mut body_buf).unwrap_or(());
    stop.stop();

    assert!(
        all_headers.iter().any(|h| h.to_ascii_lowercase().starts_with("x-powered-by:")),
        "x-powered-by header missing; got: {all_headers:?}"
    );
    assert_eq!(String::from_utf8_lossy(&body_buf), "pong");
}

#[test]
fn e2e_on_server_start_fires() {
    let started = Arc::new(AtomicUsize::new(0));
    let started_clone = Arc::clone(&started);

    let mut app = WebApp::new();
    app.get("/ping", |_| WebResponse::text("pong"));
    app.on_server_start(move |_| {
        started_clone.fetch_add(1, Ordering::SeqCst);
    });

    let (port, stop) = start_server(app);
    assert_eq!(started.load(Ordering::SeqCst), 1, "on_server_start should have fired once");
    http_get(port, "/ping");
    stop.stop();
}

#[test]
fn e2e_on_server_stop_fires_after_serve_exits() {
    let stopped = Arc::new(AtomicUsize::new(0));
    let stopped_clone = Arc::clone(&stopped);

    let mut app = WebApp::new();
    app.get("/ping", |_| WebResponse::text("pong"));
    app.on_server_stop(move || {
        stopped_clone.fetch_add(1, Ordering::SeqCst);
    });

    let (port, stop) = start_server(app);
    http_get(port, "/ping");
    stop.stop();
    thread::sleep(Duration::from_millis(100));
    assert_eq!(stopped.load(Ordering::SeqCst), 1, "on_server_stop should have fired once");
}

// ---------------------------------------------------------------------------
// WEB01a-2 — ShardedWebServer: parallel request handling across reactor shards
// ---------------------------------------------------------------------------

/// Bind and start a `ShardedWebServer` on port 0 with `worker_count` shards,
/// returning the port and a stop handle. Mirrors `start_server` but parallel.
fn start_sharded_server(
    app: WebApp,
    worker_count: usize,
) -> (u16, usize, tcp_runtime::ShardedStopHandle) {
    use web_core::ShardedWebServer;
    let app = Arc::new(app);

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    let mut server = ShardedWebServer::bind_kqueue_sharded(
        "127.0.0.1:0",
        HttpServerOptions::default(),
        worker_count,
        Arc::clone(&app),
    )
    .expect("bind kqueue sharded");

    #[cfg(target_os = "linux")]
    let mut server = ShardedWebServer::bind_epoll_sharded(
        "127.0.0.1:0",
        HttpServerOptions::default(),
        worker_count,
        Arc::clone(&app),
    )
    .expect("bind epoll sharded");

    #[cfg(target_os = "windows")]
    let mut server = ShardedWebServer::bind_windows_sharded(
        "127.0.0.1:0",
        HttpServerOptions::default(),
        worker_count,
        Arc::clone(&app),
    )
    .expect("bind windows sharded");

    let port = server.local_addr().port();
    let shards = server.worker_count();
    let stop = server.stop_handle();
    thread::spawn(move || {
        let _ = server.serve();
    });
    thread::sleep(Duration::from_millis(20));
    (port, shards, stop)
}

/// A `ShardedWebServer` actually handles requests **concurrently** across its
/// shards — proven deterministically (not by wall-clock thresholds, which flake
/// under CI load). The handler bumps an in-flight gauge, holds it briefly, and
/// records the maximum number of handlers running at once. If dispatch were
/// serial (single reactor) the gauge would never exceed 1; observing ≥ 2
/// concurrent handlers proves real cross-shard parallelism. Every client also
/// gets a correct 200, so parallelism does not corrupt the request/response
/// contract.
#[cfg(not(target_os = "windows"))]
#[test]
fn sharded_web_server_handles_requests_concurrently() {
    let worker_count = 4;
    let client_count = 8;

    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let handler_in_flight = Arc::clone(&in_flight);
    let handler_max = Arc::clone(&max_in_flight);

    let mut app = WebApp::new();
    app.get("/work", move |_| {
        // Enter: bump the in-flight count and raise the observed maximum.
        let now = handler_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        handler_max.fetch_max(now, Ordering::SeqCst);
        // Hold the handler open long enough for sibling requests on other shards
        // to overlap (loopback round-trips are sub-millisecond).
        thread::sleep(Duration::from_millis(40));
        handler_in_flight.fetch_sub(1, Ordering::SeqCst);
        WebResponse::text("ok")
    });

    let (port, shards, stop) = start_sharded_server(app, worker_count);
    assert_eq!(shards, worker_count, "all requested shards spawned");

    // Release all clients together so they spread across shards and overlap.
    let barrier = Arc::new(std::sync::Barrier::new(client_count));
    let clients: Vec<_> = (0..client_count)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                http_get(port, "/work")
            })
        })
        .collect();

    for client in clients {
        let (status, body) = client.join().expect("client thread");
        assert_eq!(status, 200, "every concurrent request gets a 200");
        assert_eq!(body, "ok");
    }

    let observed_max = max_in_flight.load(Ordering::SeqCst);
    assert!(
        observed_max >= 2,
        "expected concurrent handler execution across shards, but the max observed \
         in-flight handlers was {observed_max} (serial dispatch would never exceed 1)",
    );

    stop.stop();
}

/// CPU-bound throughput benchmark for `ShardedWebServer` (WEB01a-2).
///
/// `#[ignore]`d — like the embeddable-http-server stress test — because
/// wall-clock scaling is sensitive to the host's core count and load, so it is a
/// runnable *measurement*, not a CI pass/fail gate (the deterministic
/// `sharded_web_server_handles_requests_concurrently` test is the CI proof of
/// parallelism). Run manually on a multi-core machine:
///
/// ```sh
/// cargo test -p web-core --test web_core_test -- --ignored --nocapture \
///     sharded_web_server_cpu_bound_throughput_scales
/// ```
///
/// Each request burns a fixed CPU budget (a busy hash loop — NOT an echo, which
/// is latency-bound and would not scale). It serves the same concurrent load on
/// a 1-shard and an N-shard server and prints both wall-clock times; with real
/// cores the N-shard run finishes meaningfully faster.
#[cfg(not(target_os = "windows"))]
#[test]
#[ignore]
fn sharded_web_server_cpu_bound_throughput_scales() {
    use std::time::Instant;

    // A deterministic, optimiser-resistant CPU burn (~a few ms per request).
    fn cpu_burn() -> u64 {
        let mut acc: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
        for i in 0..2_000_000u64 {
            acc = (acc ^ i).wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
        }
        acc
    }

    fn run_load(worker_count: usize, client_count: usize) -> std::time::Duration {
        let mut app = WebApp::new();
        app.get("/compute", move |_| WebResponse::text(format!("{}", cpu_burn())));
        let (port, _shards, stop) = start_sharded_server(app, worker_count);

        let barrier = Arc::new(std::sync::Barrier::new(client_count));
        let start = Instant::now();
        let clients: Vec<_> = (0..client_count)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    let (status, _) = http_get(port, "/compute");
                    assert_eq!(status, 200);
                })
            })
            .collect();
        for c in clients {
            c.join().expect("compute client");
        }
        let elapsed = start.elapsed();
        stop.stop();
        thread::sleep(Duration::from_millis(50));
        elapsed
    }

    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let client_count = 16;

    let serial = run_load(1, client_count);
    let parallel = run_load(cores.max(2), client_count);

    println!(
        "CPU-bound throughput: {client_count} requests — 1 shard: {serial:?}, \
         {} shards: {parallel:?} (speedup {:.2}x, {cores} cores)",
        cores.max(2),
        serial.as_secs_f64() / parallel.as_secs_f64(),
    );

    if cores >= 2 {
        assert!(
            parallel < serial,
            "expected the sharded server to finish CPU-bound load faster than a \
             single reactor on {cores} cores (1 shard: {serial:?}, parallel: {parallel:?})",
        );
    }
}

// ---------------------------------------------------------------------------
// WEB01b-3 — comparative benchmark: single-reactor vs sharded vs mailbox
// ---------------------------------------------------------------------------

/// Bind and start a `MailboxWebServer` on port 0 with a `worker_count`-thread
/// pool, returning the port and the (cloneable) server so the caller can `stop`
/// it. Mirrors `start_server` / `start_sharded_server`, but the mailbox stack is
/// cross-platform (one `bind`) and `serve` takes `&self`, so we serve a clone.
fn start_mailbox_server(app: WebApp, worker_count: usize) -> (u16, MailboxWebServer) {
    let app = Arc::new(app);
    let server = MailboxWebServer::bind(
        "127.0.0.1",
        0,
        HttpServerOptions::default(),
        worker_count,
        Arc::clone(&app),
    )
    .expect("bind mailbox");
    let port = server.local_addr().port();
    let serve = server.clone();
    thread::spawn(move || {
        let _ = serve.serve();
    });
    thread::sleep(Duration::from_millis(20));
    (port, server)
}

/// Comparative CPU-bound throughput across the three WEB01 serving modes
/// (WEB01b-3): single-reactor [`WebServer`], `ShardedWebServer` (parallel *by
/// connection*), and `MailboxWebServer` (parallel *by request*).
///
/// `#[ignore]`d for the same reason as the sharded benchmark above: wall-clock
/// scaling depends on the host's core count and load, so this is a runnable
/// *measurement* that documents **when to pick which mode**, not a CI pass/fail
/// gate (the deterministic concurrency tests are the CI proofs). Run manually on
/// a multi-core machine:
///
/// ```sh
/// cargo test -p web-core --test web_core_test -- --ignored --nocapture \
///     web_serving_modes_cpu_bound_comparison
/// ```
///
/// Each request burns a fixed CPU budget (a busy hash loop — NOT an echo, which
/// is latency-bound and would not scale). All three modes serve the same
/// concurrent load; the table shows that both parallel modes beat the single
/// reactor on real cores. Sharded and mailbox scale comparably for one-shot
/// (`Connection: close`) clients — the spread shows where their dispatch models
/// differ (sharded parallelises across connections; mailbox across requests, so
/// it also overlaps sequential keep-alive on one connection — see WEB01b-1a/2).
#[cfg(not(target_os = "windows"))]
#[test]
#[ignore]
fn web_serving_modes_cpu_bound_comparison() {
    use std::time::Instant;

    // A deterministic, optimiser-resistant CPU burn (~a few ms per request) —
    // identical to the sharded benchmark's, so the numbers are comparable.
    fn cpu_burn() -> u64 {
        let mut acc: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
        for i in 0..2_000_000u64 {
            acc = (acc ^ i).wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
        }
        acc
    }

    fn build_app() -> WebApp {
        let mut app = WebApp::new();
        app.get("/compute", move |_| WebResponse::text(format!("{}", cpu_burn())));
        app
    }

    // Fire `client_count` concurrent one-shot clients at `port`, all released
    // together via the barrier, and return the wall-clock to drain them all.
    fn drive_load(port: u16, client_count: usize) -> std::time::Duration {
        let barrier = Arc::new(std::sync::Barrier::new(client_count));
        let start = Instant::now();
        let clients: Vec<_> = (0..client_count)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    let (status, _) = http_get(port, "/compute");
                    assert_eq!(status, 200);
                })
            })
            .collect();
        for c in clients {
            c.join().expect("compute client");
        }
        start.elapsed()
    }

    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let workers = cores.max(2);
    let client_count = 16;

    // 1) Single reactor (WebServer) — the baseline.
    let (port, stop) = start_server(build_app());
    let single = drive_load(port, client_count);
    stop.stop();
    thread::sleep(Duration::from_millis(50));

    // 2) ShardedWebServer — parallel by connection.
    let (port, _shards, stop) = start_sharded_server(build_app(), workers);
    let sharded = drive_load(port, client_count);
    stop.stop();
    thread::sleep(Duration::from_millis(50));

    // 3) MailboxWebServer — parallel by request.
    let (port, server) = start_mailbox_server(build_app(), workers);
    let mailbox = drive_load(port, client_count);
    server.stop();
    thread::sleep(Duration::from_millis(50));

    println!(
        "WEB01 serving modes — {client_count} CPU-bound requests on {cores} cores ({workers} workers):\n  \
         single-reactor : {single:?}\n  \
         sharded ({workers}x)   : {sharded:?}  (speedup {:.2}x)\n  \
         mailbox ({workers}x)   : {mailbox:?}  (speedup {:.2}x)",
        single.as_secs_f64() / sharded.as_secs_f64(),
        single.as_secs_f64() / mailbox.as_secs_f64(),
    );

    if cores >= 2 {
        assert!(
            sharded < single,
            "expected sharded to beat single-reactor on {cores} cores \
             (single: {single:?}, sharded: {sharded:?})",
        );
        assert!(
            mailbox < single,
            "expected mailbox to beat single-reactor on {cores} cores \
             (single: {single:?}, mailbox: {mailbox:?})",
        );
    }
}
