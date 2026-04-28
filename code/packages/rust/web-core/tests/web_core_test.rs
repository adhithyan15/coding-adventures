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
use web_core::{LogLevel, RouteLookupResult, Router, WebApp, WebResponse, WebServer};

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
            content_length = trimmed
                .splitn(2, ':')
                .nth(1)
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
            content_length = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim().parse().unwrap_or(0);
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
