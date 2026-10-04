//! Random-loopback, last-good-only preview server.

use crate::worker::{ValidatedWorkerOutput, WorkerError, WorkerRun};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const MAX_REQUEST_BYTES: usize = 8 * 1024;

#[derive(Debug)]
struct Snapshot {
    files: BTreeMap<String, Vec<u8>>,
    root: Option<String>,
}

#[derive(Debug)]
pub struct PreviewServer {
    address: SocketAddr,
    token: String,
    snapshot: Arc<RwLock<Option<Arc<Snapshot>>>>,
    shutdown: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl PreviewServer {
    pub fn bind() -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let token = format!(
            "{}{}",
            uuid::Uuid::now_v7().simple(),
            uuid::Uuid::now_v7().simple()
        );
        let snapshot = Arc::new(RwLock::new(None));
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_snapshot = Arc::clone(&snapshot);
        let server_shutdown = Arc::clone(&shutdown);
        let server_token = token.clone();
        let handle = thread::Builder::new()
            .name("forme-preview-loopback".to_owned())
            .spawn(move || serve(listener, server_snapshot, server_shutdown, server_token))?;
        Ok(Self {
            address,
            token,
            snapshot,
            shutdown,
            thread: Mutex::new(Some(handle)),
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn url(&self) -> String {
        format!("http://{}/{}/", self.address, self.token)
    }

    pub fn publish_validated(
        &self,
        output: &ValidatedWorkerOutput,
        output_root: &Path,
    ) -> Result<(), WorkerError> {
        let next = prepare_snapshot(output, output_root)?;
        *self.snapshot.write().map_err(|_| WorkerError::Io)? = Some(next);
        Ok(())
    }

    pub fn publish_run(&self, run: WorkerRun) -> Result<(), WorkerError> {
        let next = match prepare_snapshot(run.output(), &run.output_root()) {
            Ok(next) => next,
            Err(error) => {
                return match run.retire() {
                    Ok(()) => Err(error),
                    Err(retirement) => Err(retirement),
                };
            }
        };
        run.retire()?;
        *self.snapshot.write().map_err(|_| WorkerError::Io)? = Some(next);
        Ok(())
    }

    pub fn clear(&self) -> Result<(), WorkerError> {
        *self.snapshot.write().map_err(|_| WorkerError::Io)? = None;
        Ok(())
    }
}

fn prepare_snapshot(
    output: &ValidatedWorkerOutput,
    output_root: &Path,
) -> Result<Arc<Snapshot>, WorkerError> {
    let mut files = BTreeMap::new();
    let mut root = None;
    for record in &output.files {
        let path = output_root.join(&record.path);
        let metadata = fs::symlink_metadata(&path).map_err(|_| WorkerError::Output)?;
        if !metadata.file_type().is_file() || metadata.len() != record.size {
            return Err(WorkerError::Output);
        }
        let bytes = fs::read(path).map_err(|_| WorkerError::Output)?;
        if bytes.len() as u64 != record.size
            || format!("{:x}", Sha256::digest(&bytes)) != record.sha256
        {
            return Err(WorkerError::Output);
        }
        if root.is_none() && record.path.ends_with(".html") {
            root = Some(record.path.clone());
        }
        files.insert(record.path.clone(), bytes);
    }
    Ok(Arc::new(Snapshot { files, root }))
}

impl Drop for PreviewServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Ok(slot) = self.thread.get_mut() {
            if let Some(handle) = slot.take() {
                let _ = handle.join();
            }
        }
    }
}

fn serve(
    listener: TcpListener,
    snapshot: Arc<RwLock<Option<Arc<Snapshot>>>>,
    shutdown: Arc<AtomicBool>,
    token: String,
) {
    while !shutdown.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, peer)) if peer.ip().is_loopback() => {
                let current = snapshot.read().ok().and_then(|value| value.clone());
                let _ = serve_connection(stream, current, listener.local_addr().ok(), &token);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }
}

fn serve_connection(
    mut stream: TcpStream,
    snapshot: Option<Arc<Snapshot>>,
    expected_address: Option<SocketAddr>,
    token: &str,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while request.len() <= MAX_REQUEST_BYTES {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    if request.len() > MAX_REQUEST_BYTES {
        return write_response(&mut stream, 400, "Bad Request", "text/plain", b"", false);
    }
    let first_line = request
        .split(|byte| *byte == b'\n')
        .next()
        .and_then(|line| std::str::from_utf8(line).ok())
        .map(str::trim_end)
        .unwrap_or("");
    let expected_host = expected_address.map(|address| address.to_string());
    let hosts = request
        .split(|byte| *byte == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
        .filter_map(|line| line.trim_end().strip_prefix("Host: "))
        .collect::<Vec<_>>();
    let mut fields = first_line.split(' ');
    let method = fields.next().unwrap_or("");
    let target = fields.next().unwrap_or("");
    let version = fields.next().unwrap_or("");
    if fields.next().is_some()
        || version != "HTTP/1.1"
        || hosts.len() != 1
        || hosts.first().copied() != expected_host.as_deref()
    {
        return write_response(&mut stream, 400, "Bad Request", "text/plain", b"", false);
    }
    if method != "GET" && method != "HEAD" {
        return write_response(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain",
            b"",
            false,
        );
    }
    let Some(snapshot) = snapshot else {
        return write_response(
            &mut stream,
            503,
            "Service Unavailable",
            "text/plain; charset=utf-8",
            b"Preview is not ready.",
            method == "HEAD",
        );
    };
    let Some((candidate, capability_path)) = target
        .strip_prefix('/')
        .and_then(|value| value.split_once('/'))
    else {
        return write_response(
            &mut stream,
            404,
            "Not Found",
            "text/plain",
            b"",
            method == "HEAD",
        );
    };
    if !constant_time_equal(candidate.as_bytes(), token.as_bytes()) {
        return write_response(
            &mut stream,
            404,
            "Not Found",
            "text/plain",
            b"",
            method == "HEAD",
        );
    }
    let path = if capability_path.is_empty() {
        snapshot.root.as_deref().unwrap_or("")
    } else if !capability_path.contains(['?', '#', '%', '\\'])
        && !capability_path
            .split('/')
            .any(|part| part == ".." || part == ".")
    {
        capability_path
    } else {
        ""
    };
    let Some(body) = snapshot.files.get(path) else {
        return write_response(
            &mut stream,
            404,
            "Not Found",
            "text/plain",
            b"",
            method == "HEAD",
        );
    };
    write_response(
        &mut stream,
        200,
        "OK",
        content_type(path),
        body,
        method == "HEAD",
    )
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    head: bool,
) -> io::Result<()> {
    write!(
        stream,
        concat!(
            "HTTP/1.1 {} {}\r\n",
            "Content-Length: {}\r\n",
            "Content-Type: {}\r\n",
            "Cache-Control: no-store\r\n",
            "X-Content-Type-Options: nosniff\r\n",
            "Content-Security-Policy: default-src 'none'; base-uri 'none'; form-action 'none'; ",
            "img-src data:; style-src 'unsafe-inline'\r\n",
            "Connection: close\r\n\r\n"
        ),
        status,
        reason,
        body.len(),
        content_type,
    )?;
    if !head {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    }
}
