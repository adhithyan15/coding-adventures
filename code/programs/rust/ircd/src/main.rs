//! # ircd — IRC server daemon
//!
//! This program is the wiring layer — the topmost layer of the IRC stack.
//! It connects the pure IRC logic (`irc-server`) to the TCP transport layer
//! (`irc-net-stdlib`) via a single adapter type, `DriverHandler`.
//!
//! ## Wiring diagram
//!
//! ```text
//! TCP socket
//!    ↓ raw bytes
//! EventLoop.worker thread            ← irc-net-stdlib
//!    ↓ on_data(conn_id, raw_bytes)
//! DriverHandler.on_data()            ← THIS PROGRAM
//!    ↓ feeds bytes into per-connection Framer
//! Framer.frames()                    ← irc-framing
//!    ↓ b"NICK alice"
//! irc_proto::parse()                 ← irc-proto
//!    ↓ Message { command: "NICK", ... }
//! IRCServer.on_message()             ← irc-server
//!    ↓ Vec<Response>
//! irc_proto::serialize()             ← irc-proto
//!    ↓ b":irc.local 001 alice :Welcome\r\n"
//! EventLoop.send_to()                ← irc-net-stdlib
//!    ↓ bytes on the wire
//! ```
//!
//! None of the four dependency packages know about each other — only this
//! program imports all four and wires them together.  This is the Dependency
//! Inversion Principle at work: higher-level modules (`irc-server`) know nothing
//! about lower-level infrastructure (sockets), because both talk through a
//! common message interface.
//!
//! ## Usage
//!
//! ```text
//! ircd --port 6667
//! ircd --host 127.0.0.1 --port 6668 --server-name irc.local --oper-password secret
//! ```

use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

use irc_framing::Framer;
use irc_net_stdlib::{ConnId, EventLoop, Handler};
use irc_proto::{parse, serialize, ParseError};
use irc_server::IRCServer;

// ──────────────────────────────────────────────────────────────────────────────
// Config — command-line configuration
// ──────────────────────────────────────────────────────────────────────────────

/// All runtime configuration for `ircd`.
///
/// Values are populated by [`parse_args`] from `std::env::args()`.
/// Default values match the conventional IRC server setup.
#[derive(Debug, Clone)]
pub struct Config {
    /// IP address to bind to.  `"0.0.0.0"` means all interfaces.
    pub host: String,

    /// TCP port to listen on.  6667 is the standard unencrypted IRC port.
    pub port: u16,

    /// Server hostname advertised to clients in the 001 welcome message
    /// and as the prefix of all server-generated messages.
    pub server_name: String,

    /// Lines of the Message of the Day, shown during connection.
    pub motd: Vec<String>,

    /// Password for the OPER command.  Empty string disables oper promotion.
    pub oper_password: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: "0.0.0.0".to_string(),
            port: 6667,
            server_name: "irc.local".to_string(),
            motd: vec!["Welcome.".to_string()],
            oper_password: String::new(),
        }
    }
}

/// Parse command-line arguments into a [`Config`].
///
/// Supported flags:
/// - `--host <addr>`            — bind address (default: 0.0.0.0)
/// - `--port <number>`          — TCP port (default: 6667)
/// - `--server-name <name>`     — server hostname (default: irc.local)
/// - `--motd <text>`            — MOTD line (may be repeated)
/// - `--oper-password <secret>` — oper password (default: empty)
/// - `--help`                   — print usage and exit
///
/// # Errors
///
/// Prints an error message and calls `std::process::exit(1)` on bad input.
/// This is acceptable for a top-level program entry point.
///
/// # Example
///
/// ```
/// use ircd::parse_args;
/// let cfg = parse_args(&["--port", "6668"]);
/// assert_eq!(cfg.port, 6668);
/// ```
pub fn parse_args(args: &[&str]) -> Config {
    let mut config = Config::default();
    let mut i = 0;
    let mut motd_lines: Vec<String> = Vec::new();

    while i < args.len() {
        match args[i] {
            "--help" | "-h" => {
                eprintln!("Usage: ircd [OPTIONS]");
                eprintln!("  --host <addr>           Bind address (default: 0.0.0.0)");
                eprintln!("  --port <number>         TCP port (default: 6667)");
                eprintln!("  --server-name <name>    Server hostname (default: irc.local)");
                eprintln!("  --motd <text>           MOTD line (may be repeated)");
                eprintln!("  --oper-password <pass>  OPER password (default: empty)");
                std::process::exit(0);
            }
            "--host" => {
                i += 1;
                if i < args.len() {
                    config.host = args[i].to_string();
                }
            }
            "--port" => {
                i += 1;
                if i < args.len() {
                    config.port = args[i].parse().unwrap_or_else(|_| {
                        eprintln!("error: --port must be a number");
                        std::process::exit(1);
                    });
                }
            }
            "--server-name" => {
                i += 1;
                if i < args.len() {
                    config.server_name = args[i].to_string();
                }
            }
            "--motd" => {
                i += 1;
                if i < args.len() {
                    motd_lines.push(args[i].to_string());
                }
            }
            "--oper-password" => {
                i += 1;
                if i < args.len() {
                    config.oper_password = args[i].to_string();
                }
            }
            other => {
                eprintln!("warning: unknown argument: {}", other);
            }
        }
        i += 1;
    }

    if !motd_lines.is_empty() {
        config.motd = motd_lines;
    }

    config
}

// ──────────────────────────────────────────────────────────────────────────────
// DriverHandler — bridges irc-net-stdlib and irc-server
// ──────────────────────────────────────────────────────────────────────────────

/// Adapts `IRCServer` to the `Handler` interface expected by `irc-net-stdlib`.
///
/// The `irc-net-stdlib` event loop calls three lifecycle callbacks on a `Handler`:
///
/// * `on_connect(conn_id, host)` — a new TCP connection arrived.
/// * `on_data(conn_id, data)`    — raw bytes from an established connection.
/// * `on_disconnect(conn_id)`    — the TCP connection has closed.
///
/// `DriverHandler` translates these raw-bytes events into structured `Message`
/// objects that `IRCServer` can process, and sends the resulting `Response`
/// values back over the wire via `loop.send_to()`.
///
/// ## Per-connection framing
///
/// IRC uses CRLF-terminated text lines.  TCP delivers an arbitrary byte stream
/// — a single `read()` call may return half a message, one complete message,
/// or five messages concatenated together.  To reassemble byte chunks into
/// complete lines, each connection gets its own `Framer` instance (from
/// `irc-framing`).  The `Framer` is stored in a `HashMap` keyed by `ConnId`,
/// created in `on_connect` and removed in `on_disconnect`.
///
/// ## Concurrency
///
/// The `irc-net-stdlib` event loop already holds its `handler_lock` before
/// calling any `Handler` method.  This means all three callbacks here run
/// serially — we never have two threads in `on_data` simultaneously.
/// `IRCServer` is therefore safe without an additional lock inside this type.
///
/// The `Mutex<IRCServer>` and `Mutex<HashMap<ConnId, Framer>>` in this struct
/// are for the `Arc<DriverHandler>` wrapper used by `irc-net-stdlib` (`Handler`
/// requires `&self`, not `&mut self`), not for concurrent access protection.
pub struct DriverHandler {
    /// The IRC state machine — pure, no I/O.
    ///
    /// `Mutex` is needed because `Handler` trait methods take `&self`, not
    /// `&mut self`, but `IRCServer` methods take `&mut self`.
    server: Mutex<IRCServer>,

    /// The event loop — used only for `loop.send_to(conn_id, bytes)`.
    event_loop: Arc<EventLoop>,

    /// One `Framer` per live connection.  Framers accumulate partial IRC lines
    /// across multiple `on_data` calls until a full CRLF-terminated line is available.
    ///
    /// Protected by a `Mutex` for the same reason as `server`.
    framers: Mutex<HashMap<u64, Framer>>,
}

impl DriverHandler {
    /// Create a new `DriverHandler` wrapping an `IRCServer` and an `EventLoop`.
    ///
    /// The `DriverHandler` borrows the event loop's `send_to` capability but
    /// never calls `run()` or `stop()` — those are the caller's responsibility.
    pub fn new(server: IRCServer, event_loop: Arc<EventLoop>) -> Self {
        DriverHandler {
            server: Mutex::new(server),
            event_loop,
            framers: Mutex::new(HashMap::new()),
        }
    }
}

impl Handler for DriverHandler {
    fn on_connect(&self, conn_id: ConnId, host: &str) {
        // Create a per-connection framer before registering with the server.
        {
            let mut framers = self.framers.lock().unwrap();
            framers.insert(conn_id.0, Framer::new());
        }

        // Notify the server.  Returns [] (no initial responses).
        let responses = {
            let mut server = self.server.lock().unwrap();
            server.on_connect(irc_server::ConnId(conn_id.0), host)
        };
        self.send_responses(responses);
    }

    fn on_data(&self, conn_id: ConnId, data: &[u8]) {
        // Feed raw bytes into the per-connection framer and dispatch messages.
        //
        // Sequence:
        // 1. Feed raw bytes into the Framer.
        // 2. Extract all complete lines (Framer.frames()).
        // 3. Decode each line from UTF-8 (errors="replace" avoids crashes).
        // 4. Parse each line with irc_proto::parse(); skip unparseable lines.
        // 5. Pass the parsed Message to IRCServer::on_message().
        // 6. Send any resulting Response values.

        // Get (or create) the framer for this connection.
        let frames: Vec<Vec<u8>> = {
            let mut framers = self.framers.lock().unwrap();
            if let Some(framer) = framers.get_mut(&conn_id.0) {
                framer.feed(data);
                framer.frames()
            } else {
                // Defensive: data arrived for a connection we have no framer for.
                // This should be impossible but we handle it gracefully.
                return;
            }
        };

        for raw_line in frames {
            // IRC is specified as ASCII but UTF-8 is universally accepted.
            // `errors="replace"` is Rust's lossy conversion — we never want
            // a single bad byte to crash the connection.
            let line = String::from_utf8_lossy(&raw_line).into_owned();

            let msg = match parse(&line) {
                Ok(m) => m,
                Err(ParseError(_)) => {
                    // Malformed or empty line — skip silently.  IRC servers
                    // traditionally ignore unparseable input rather than
                    // disconnecting the client.
                    continue;
                }
            };

            let responses = {
                let mut server = self.server.lock().unwrap();
                server.on_message(irc_server::ConnId(conn_id.0), &msg)
            };
            self.send_responses(responses);
        }
    }

    fn on_disconnect(&self, conn_id: ConnId) {
        // Notify IRCServer (which broadcasts a QUIT to all channels the client
        // was in), dispatch those responses, then discard the framer.
        let responses = {
            let mut server = self.server.lock().unwrap();
            server.on_disconnect(irc_server::ConnId(conn_id.0))
        };
        self.send_responses(responses);

        let mut framers = self.framers.lock().unwrap();
        framers.remove(&conn_id.0);
    }
}

impl DriverHandler {
    /// Serialize and deliver a list of `Response` values.
    ///
    /// `IRCServer` returns `Vec<irc_server::Response>`.  We serialize each
    /// `Message` to bytes using `irc_proto::serialize()` and forward it to the
    /// event loop's `send_to()`.
    ///
    /// This indirection (serialize here, not in irc-server) keeps `irc-server`
    /// free of any dependency on `irc-proto`'s serialization side — the server
    /// only needs to *construct* `Message` objects, not wire-encode them.
    fn send_responses(&self, responses: Vec<irc_server::Response>) {
        for response in responses {
            let wire = serialize(&response.msg);
            self.event_loop.send_to(ConnId(response.conn_id.0), &wire);
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// main — entry point
// ──────────────────────────────────────────────────────────────────────────────

fn main() {
    // 1. Parse command-line arguments.
    //
    //    We collect from std::env::args() (skipping argv[0]) into a Vec<String>,
    //    then pass string slices to parse_args().
    let args_owned: Vec<String> = env::args().skip(1).collect();
    let args_refs: Vec<&str> = args_owned.iter().map(String::as_str).collect();
    let config = parse_args(&args_refs);

    // 2. Print the startup banner.
    println!(
        "ircd starting on {}:{} (server name: {})",
        config.host, config.port, config.server_name
    );

    // 3. Create the IRC state machine.
    //    This is pure Rust — no I/O, no threads.
    let server = IRCServer::new(&config.server_name, config.motd.clone(), &config.oper_password);

    // 4. Create the event loop.
    //    The event loop owns the TCP listener and the thread pool.
    let event_loop = Arc::new(EventLoop::new());

    // 5. Create the driver handler.
    //    This bridges the event loop and the IRC server.
    let handler = Arc::new(DriverHandler::new(server, Arc::clone(&event_loop)));

    // 6. Install a Ctrl-C handler for graceful shutdown.
    //    On Ctrl-C, we call event_loop.stop() which causes run() to exit.
    //
    //    Note: ctrlc is not added as a dependency to keep this zero-dependency.
    //    Instead, we run the server in a background thread and join it.
    //    (A full production server would use ctrlc or signal-hook crate here.)
    let el_for_shutdown = Arc::clone(&event_loop);

    // Run the server loop in the current thread.
    // The loop blocks until stop() is called.
    let bind_addr = format!("{}:{}", config.host, config.port);

    // Register a Ctrl-C signal handler on Unix (SIGINT).
    // On Windows, Ctrl-C handling is done via SetConsoleCtrlHandler, but
    // a simple thread that reads from stdin can work as a fallback.
    #[cfg(unix)]
    {
        let el_sig = Arc::clone(&el_for_shutdown);
        unsafe {
            libc_signal_setup(el_sig);
        }
    }

    // Run in a thread so we can potentially join it.
    let run_el = Arc::clone(&event_loop);
    let run_handler = Arc::clone(&handler);
    let run_addr = bind_addr.clone();

    // Spawn the server thread.
    let server_thread = std::thread::spawn(move || {
        if let Err(e) = run_el.run(&run_addr, run_handler) {
            eprintln!("ircd: error: {}", e);
            std::process::exit(1);
        }
    });

    println!("ircd listening on {} (press Ctrl-C to stop)", bind_addr);

    // On non-Unix (Windows), stop after a Ctrl-C by blocking on stdin.
    // We can also call el_for_shutdown.stop() from another thread.
    // For now, just wait for the server thread to finish.
    //
    // In a real deployment, a process supervisor (systemd, Windows Service)
    // would send SIGTERM (Unix) or SERVICE_CONTROL_STOP (Windows), which
    // the operator would handle appropriately.
    server_thread.join().expect("server thread panicked");
}

// Signal handling helper (Unix only) — set up SIGINT/SIGTERM to call stop().
// This uses raw libc; a production server would use the `signal-hook` crate.
#[cfg(unix)]
mod signal_handling {
    use std::sync::Arc;
    use crate::EventLoop;

    // We use a global pointer to the event loop so the signal handler
    // (which is a C function with no user data) can reach it.
    static mut EVENT_LOOP_PTR: Option<Arc<EventLoop>> = None;

    extern "C" fn sigint_handler(_: libc::c_int) {
        unsafe {
            if let Some(ref el) = EVENT_LOOP_PTR {
                el.stop();
            }
        }
    }

    pub unsafe fn setup(el: Arc<EventLoop>) {
        EVENT_LOOP_PTR = Some(el);
        libc::signal(libc::SIGINT, sigint_handler as libc::sighandler_t);
        libc::signal(libc::SIGTERM, sigint_handler as libc::sighandler_t);
    }
}

#[cfg(unix)]
unsafe fn libc_signal_setup(el: Arc<EventLoop>) {
    // Safety: we set the global before installing the signal handler, and
    // the signal handler only reads it (stops the event loop once).
    signal_handling::setup(el);
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    /// Helper: start a test server on a random port, return the port.
    fn start_test_server() -> (Arc<EventLoop>, u16) {
        use std::net::TcpListener;

        // Find a free port.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let server = IRCServer::new("irc.test", vec!["Test MOTD".to_string()], "testoper");
        let el = Arc::new(EventLoop::new());
        let handler = Arc::new(DriverHandler::new(server, Arc::clone(&el)));

        let el_clone = Arc::clone(&el);
        let addr = format!("127.0.0.1:{}", port);

        std::thread::spawn(move || {
            el_clone.run(&addr, handler).unwrap();
        });

        // Give the server a moment to start.
        std::thread::sleep(Duration::from_millis(50));

        (el, port)
    }

    /// Helper: perform the IRC registration handshake (NICK + USER).
    /// Returns a BufReader wrapping the stream for line-by-line reading.
    fn register(stream: &mut TcpStream, nick: &str) -> Vec<String> {
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        write!(stream, "NICK {}\r\n", nick).unwrap();
        write!(stream, "USER {} 0 * :Test User\r\n", nick).unwrap();

        // Read until we see 376 (End of MOTD), which marks the end of welcome.
        let mut lines = Vec::new();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 { break; }
            let trimmed = line.trim_end_matches(|c| c == '\r' || c == '\n').to_string();
            let is_end = trimmed.contains("376");
            lines.push(trimmed);
            if is_end { break; }
        }
        lines
    }

    #[test]
    fn test_registration_welcome_sequence() {
        let (el, port) = start_test_server();
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        let lines = register(&mut stream, "alice");

        // Should contain 001 RPL_WELCOME.
        let has_001 = lines.iter().any(|l| l.contains(" 001 "));
        assert!(has_001, "expected 001 welcome, got: {:?}", lines);

        // Should contain 376 RPL_ENDOFMOTD.
        let has_376 = lines.iter().any(|l| l.contains(" 376 "));
        assert!(has_376, "expected 376 end of MOTD, got: {:?}", lines);

        el.stop();
    }

    #[test]
    fn test_nick_in_use() {
        let (el, port) = start_test_server();

        let mut stream1 = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        register(&mut stream1, "alice");

        let mut stream2 = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        stream2.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        write!(stream2, "NICK alice\r\n").unwrap();

        let mut reader = BufReader::new(stream2.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        assert!(line.contains("433"), "expected 433 ERR_NICKNAMEINUSE, got: {}", line.trim());

        el.stop();
    }

    #[test]
    fn test_join_and_privmsg() {
        let (el, port) = start_test_server();

        let mut stream1 = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        let mut stream2 = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        stream2.set_read_timeout(Some(Duration::from_secs(2))).unwrap();

        register(&mut stream1, "alice");
        register(&mut stream2, "bob");

        write!(stream1, "JOIN #test\r\n").unwrap();
        write!(stream2, "JOIN #test\r\n").unwrap();
        std::thread::sleep(Duration::from_millis(100));

        write!(stream1, "PRIVMSG #test :Hello from Alice!\r\n").unwrap();
        std::thread::sleep(Duration::from_millis(100));

        // Bob should receive Alice's message.
        let mut reader2 = BufReader::new(stream2.try_clone().unwrap());
        let mut all_lines = String::new();
        // Drain available data with a short read timeout.
        loop {
            let mut line = String::new();
            match reader2.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => all_lines.push_str(&line),
            }
        }

        // We check for PRIVMSG or the channel messages in what Bob received.
        // (Bob may have received JOIN, NAMES, NOTOPIC, and PRIVMSG)
        assert!(
            all_lines.contains("PRIVMSG") || all_lines.contains("Hello"),
            "Bob should have received Alice's PRIVMSG; got: {}",
            all_lines
        );

        el.stop();
    }

    #[test]
    fn test_parse_args_defaults() {
        let config = parse_args(&[]);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 6667);
        assert_eq!(config.server_name, "irc.local");
        assert!(!config.motd.is_empty());
        assert!(config.oper_password.is_empty());
    }

    #[test]
    fn test_parse_args_custom() {
        let config = parse_args(&[
            "--host", "127.0.0.1",
            "--port", "6668",
            "--server-name", "irc.example.com",
            "--motd", "Hello World",
            "--oper-password", "supersecret",
        ]);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 6668);
        assert_eq!(config.server_name, "irc.example.com");
        assert_eq!(config.motd, vec!["Hello World"]);
        assert_eq!(config.oper_password, "supersecret");
    }

    #[test]
    fn test_ping_pong() {
        let (el, port) = start_test_server();
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        register(&mut stream, "alice");

        write!(stream, "PING :irc.test\r\n").unwrap();

        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut found_pong = false;
        for _ in 0..20 {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 { break; }
            if line.contains("PONG") {
                found_pong = true;
                break;
            }
        }
        assert!(found_pong, "expected PONG response");

        el.stop();
    }
}
