//! Bounded synchronous HTTP/1.0 GET transport.
//!
//! This crate is intentionally an orchestrator. URL parsing, TCP I/O, HTTP
//! syntax, and semantic message types remain in their existing packages.

use http1::{parse_response_head, Http1ParseError};
use http_core::{BodyKind, ResponseHead};
use std::fmt;
use tcp_client::{connect, ConnectOptions, TcpConnection, TcpError};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_USER_AGENT: &str = "Venture/0.1";
pub const DEFAULT_MAX_REDIRECTS: usize = 5;
pub const DEFAULT_MAX_HEAD_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// A complete HTTP response plus the final URL after redirects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub final_url: String,
    pub head: ResponseHead,
    pub body: Vec<u8>,
}

/// Configuration for one-request-per-connection HTTP/1.0 GETs.
#[derive(Debug, Clone)]
pub struct HttpClient {
    pub connect_options: ConnectOptions,
    pub max_redirects: usize,
    pub max_head_bytes: usize,
    pub max_body_bytes: usize,
    pub user_agent: String,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self {
            connect_options: ConnectOptions::default(),
            max_redirects: DEFAULT_MAX_REDIRECTS,
            max_head_bytes: DEFAULT_MAX_HEAD_BYTES,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Perform an HTTP/1.0 GET, following relative or absolute 301/302
    /// redirects up to `max_redirects`.
    pub fn get(&self, url: &str) -> Result<HttpResponse, HttpClientError> {
        let mut current = Url::parse(url).map_err(HttpClientError::Url)?;
        let mut redirects_followed = 0;

        loop {
            let response = self.get_once(&current)?;
            let is_redirect = matches!(response.head.status, 301 | 302);
            let Some(location) = response.head.header("Location") else {
                return Ok(response);
            };
            if !is_redirect {
                return Ok(response);
            }
            if redirects_followed >= self.max_redirects {
                return Err(HttpClientError::TooManyRedirects {
                    limit: self.max_redirects,
                });
            }

            current = current.resolve(location).map_err(HttpClientError::Url)?;
            redirects_followed += 1;
        }
    }

    fn get_once(&self, url: &Url) -> Result<HttpResponse, HttpClientError> {
        validate_url(url)?;
        validate_header_value("User-Agent", &self.user_agent)?;
        let host = url.host.as_deref().ok_or(HttpClientError::MissingHost)?;
        let port = url.effective_port().ok_or(HttpClientError::MissingPort)?;
        let target = request_target(url);
        validate_request_target(&target)?;
        let host_header = host_header(url, host, port);
        validate_header_value("Host", &host_header)?;
        let request = format!(
            "GET {target} HTTP/1.0\r\n\
             Host: {host_header}\r\n\
             User-Agent: {}\r\n\
             Accept: */*\r\n\
             Connection: close\r\n\
             \r\n",
            self.user_agent
        );

        let mut connection =
            connect(host, port, self.connect_options.clone()).map_err(HttpClientError::Tcp)?;
        connection
            .write_all(request.as_bytes())
            .map_err(HttpClientError::Tcp)?;
        connection.shutdown_write().map_err(HttpClientError::Tcp)?;

        let parsed = read_response_head(&mut connection, self.max_head_bytes)?;
        let body = read_response_body(&mut connection, &parsed.body_kind, self.max_body_bytes)?;

        Ok(HttpResponse {
            final_url: url.to_url_string(),
            head: parsed.head,
            body,
        })
    }
}

/// Perform a GET with [`HttpClient::default`].
pub fn get(url: &str) -> Result<HttpResponse, HttpClientError> {
    HttpClient::new().get(url)
}

#[derive(Debug)]
pub enum HttpClientError {
    Url(UrlError),
    Tcp(TcpError),
    Http(Http1ParseError),
    UnsupportedScheme(String),
    UnsupportedCredentials,
    MissingHost,
    MissingPort,
    InvalidRequestTarget,
    InvalidHeaderValue { name: &'static str },
    IncompleteResponseHead,
    ResponseHeadTooLarge { limit: usize },
    ResponseBodyTooLarge { limit: usize },
    UnsupportedBodyFraming(BodyKind),
    TooManyRedirects { limit: usize },
}

impl fmt::Display for HttpClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(error) => write!(formatter, "URL error: {error}"),
            Self::Tcp(error) => write!(formatter, "TCP error: {error}"),
            Self::Http(error) => write!(formatter, "HTTP response error: {error}"),
            Self::UnsupportedScheme(scheme) => {
                write!(formatter, "unsupported URL scheme: {scheme}")
            }
            Self::UnsupportedCredentials => {
                formatter.write_str("userinfo credentials are not supported")
            }
            Self::MissingHost => formatter.write_str("HTTP URL is missing a host"),
            Self::MissingPort => formatter.write_str("HTTP URL has no effective port"),
            Self::InvalidRequestTarget => {
                formatter.write_str("HTTP request target contains unsafe characters")
            }
            Self::InvalidHeaderValue { name } => {
                write!(formatter, "{name} contains unsafe control characters")
            }
            Self::IncompleteResponseHead => formatter.write_str("incomplete HTTP response head"),
            Self::ResponseHeadTooLarge { limit } => {
                write!(formatter, "HTTP response head exceeds {limit} bytes")
            }
            Self::ResponseBodyTooLarge { limit } => {
                write!(formatter, "HTTP response body exceeds {limit} bytes")
            }
            Self::UnsupportedBodyFraming(kind) => {
                write!(
                    formatter,
                    "unsupported HTTP response body framing: {kind:?}"
                )
            }
            Self::TooManyRedirects { limit } => {
                write!(formatter, "HTTP redirect limit exceeded ({limit})")
            }
        }
    }
}

impl std::error::Error for HttpClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tcp(error) => Some(error),
            Self::Http(error) => Some(error),
            _ => None,
        }
    }
}

fn validate_url(url: &Url) -> Result<(), HttpClientError> {
    if url.scheme != "http" {
        return Err(HttpClientError::UnsupportedScheme(url.scheme.clone()));
    }
    if url.userinfo.is_some() {
        return Err(HttpClientError::UnsupportedCredentials);
    }
    Ok(())
}

fn validate_header_value(name: &'static str, value: &str) -> Result<(), HttpClientError> {
    if value.bytes().any(|byte| !(b' '..=b'~').contains(&byte)) {
        return Err(HttpClientError::InvalidHeaderValue { name });
    }
    Ok(())
}

fn validate_request_target(target: &str) -> Result<(), HttpClientError> {
    if target.bytes().any(|byte| !byte.is_ascii_graphic()) {
        return Err(HttpClientError::InvalidRequestTarget);
    }
    Ok(())
}

fn request_target(url: &Url) -> String {
    let mut target = if url.path.is_empty() {
        "/".to_string()
    } else {
        url.path.clone()
    };
    if let Some(query) = &url.query {
        target.push('?');
        target.push_str(query);
    }
    target
}

fn host_header(url: &Url, host: &str, port: u16) -> String {
    if url.port.is_some() && port != 80 {
        format!("{host}:{port}")
    } else {
        host.to_string()
    }
}

fn read_response_head(
    connection: &mut TcpConnection,
    max_head_bytes: usize,
) -> Result<http1::ParsedResponseHead, HttpClientError> {
    let mut bytes = Vec::new();
    loop {
        let remaining = max_head_bytes.saturating_sub(bytes.len());
        let line = connection
            .read_until_limit(b'\n', remaining)
            .map_err(|error| match error {
                TcpError::ReadLimitExceeded { .. } => HttpClientError::ResponseHeadTooLarge {
                    limit: max_head_bytes,
                },
                other => HttpClientError::Tcp(other),
            })?;
        if line.is_empty() {
            return Err(HttpClientError::IncompleteResponseHead);
        }
        let is_blank = line == b"\n" || line == b"\r\n";
        bytes.extend_from_slice(&line);
        if is_blank {
            break;
        }
    }

    parse_response_head(&bytes).map_err(HttpClientError::Http)
}

fn read_response_body(
    connection: &mut TcpConnection,
    kind: &BodyKind,
    max_body_bytes: usize,
) -> Result<Vec<u8>, HttpClientError> {
    match kind {
        BodyKind::None => Ok(Vec::new()),
        BodyKind::ContentLength(length) => {
            if *length > max_body_bytes {
                return Err(HttpClientError::ResponseBodyTooLarge {
                    limit: max_body_bytes,
                });
            }
            connection.read_exact(*length).map_err(HttpClientError::Tcp)
        }
        BodyKind::UntilEof => read_until_eof(connection, max_body_bytes),
        BodyKind::Chunked => Err(HttpClientError::UnsupportedBodyFraming(BodyKind::Chunked)),
    }
}

fn read_until_eof(
    connection: &mut TcpConnection,
    max_body_bytes: usize,
) -> Result<Vec<u8>, HttpClientError> {
    let mut body = Vec::new();
    loop {
        let remaining = max_body_bytes.saturating_sub(body.len());
        let read_size = if remaining == 0 {
            1
        } else {
            remaining.min(8 * 1024)
        };
        let chunk = connection
            .read_chunk(read_size)
            .map_err(HttpClientError::Tcp)?;
        if chunk.is_empty() {
            return Ok(body);
        }
        if chunk.len() > remaining {
            return Err(HttpClientError::ResponseBodyTooLarge {
                limit: max_body_bytes,
            });
        }
        body.extend_from_slice(&chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    fn serve(responses: Vec<Vec<u8>>) -> (String, Receiver<Vec<u8>>, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request_head(&mut stream);
                let _ = request_tx.send(request);
                stream.write_all(&response).unwrap();
                stream.flush().unwrap();
            }
        });
        (format!("http://{address}"), request_rx, handle)
    }

    fn read_request_head(stream: &mut TcpStream) -> Vec<u8> {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0_u8; 1];
        while !request.ends_with(b"\r\n\r\n") {
            let count = stream.read(&mut byte).unwrap();
            if count == 0 {
                break;
            }
            request.push(byte[0]);
        }
        request
    }

    fn test_client() -> HttpClient {
        HttpClient {
            connect_options: ConnectOptions {
                connect_timeout: Duration::from_secs(5),
                read_timeout: Some(Duration::from_secs(5)),
                write_timeout: Some(Duration::from_secs(5)),
                buffer_size: 1024,
            },
            ..HttpClient::default()
        }
    }

    #[test]
    fn gets_content_length_response() {
        let response = b"HTTP/1.0 200 OK\r\n\
                         Content-Length: 5\r\n\
                         Content-Type: text/html; charset=utf-8\r\n\
                         \r\n\
                         hello"
            .to_vec();
        let (origin, requests, server) = serve(vec![response]);
        let url = format!("{origin}/docs/index.html?q=venture#top");

        let result = test_client().get(&url).unwrap();

        assert_eq!(result.head.status, 200);
        assert_eq!(result.body, b"hello");
        assert_eq!(result.final_url, url);
        assert_eq!(
            result.head.content_type(),
            Some(("text/html".to_string(), Some("utf-8".to_string())))
        );
        let request = String::from_utf8(requests.recv().unwrap()).unwrap();
        assert!(request.starts_with("GET /docs/index.html?q=venture HTTP/1.0\r\n"));
        assert!(request.contains("\r\nUser-Agent: Venture/0.1\r\n"));
        assert!(request.contains("\r\nConnection: close\r\n"));
        server.join().unwrap();
    }

    #[test]
    fn reads_binary_body_until_eof() {
        let mut response = b"HTTP/1.0 200 OK\r\nContent-Type: image/gif\r\n\r\n".to_vec();
        response.extend_from_slice(b"a\0b");
        let (origin, _, server) = serve(vec![response]);

        let result = test_client().get(&format!("{origin}/image.gif")).unwrap();

        assert_eq!(result.body, b"a\0b");
        server.join().unwrap();
    }

    #[test]
    fn follows_relative_redirect() {
        let first =
            b"HTTP/1.0 302 Found\r\nLocation: /target\r\nContent-Length: 0\r\n\r\n".to_vec();
        let second = b"HTTP/1.0 200 OK\r\nContent-Length: 4\r\n\r\ndone".to_vec();
        let (origin, requests, server) = serve(vec![first, second]);

        let result = test_client().get(&format!("{origin}/start")).unwrap();

        assert_eq!(result.head.status, 200);
        assert_eq!(result.body, b"done");
        assert_eq!(result.final_url, format!("{origin}/target"));
        assert!(String::from_utf8(requests.recv().unwrap())
            .unwrap()
            .starts_with("GET /start HTTP/1.0\r\n"));
        assert!(String::from_utf8(requests.recv().unwrap())
            .unwrap()
            .starts_with("GET /target HTTP/1.0\r\n"));
        server.join().unwrap();
    }

    #[test]
    fn enforces_redirect_limit() {
        let redirect =
            b"HTTP/1.0 302 Found\r\nLocation: /again\r\nContent-Length: 0\r\n\r\n".to_vec();
        let (origin, _, server) = serve(vec![redirect.clone(), redirect.clone(), redirect]);
        let mut client = test_client();
        client.max_redirects = 2;

        let error = client.get(&format!("{origin}/start")).unwrap_err();

        assert!(matches!(
            error,
            HttpClientError::TooManyRedirects { limit: 2 }
        ));
        server.join().unwrap();
    }

    #[test]
    fn rejects_oversized_declared_body_before_allocating() {
        let response = b"HTTP/1.0 200 OK\r\nContent-Length: 6\r\n\r\nabcdef".to_vec();
        let (origin, _, server) = serve(vec![response]);
        let mut client = test_client();
        client.max_body_bytes = 5;

        let error = client.get(&origin).unwrap_err();

        assert!(matches!(
            error,
            HttpClientError::ResponseBodyTooLarge { limit: 5 }
        ));
        server.join().unwrap();
    }

    #[test]
    fn rejects_oversized_eof_body() {
        let response = b"HTTP/1.0 200 OK\r\n\r\nabcdef".to_vec();
        let (origin, _, server) = serve(vec![response]);
        let mut client = test_client();
        client.max_body_bytes = 5;

        let error = client.get(&origin).unwrap_err();

        assert!(matches!(
            error,
            HttpClientError::ResponseBodyTooLarge { limit: 5 }
        ));
        server.join().unwrap();
    }

    #[test]
    fn rejects_chunked_http11_response() {
        let response =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\ntest\r\n0\r\n\r\n".to_vec();
        let (origin, _, server) = serve(vec![response]);

        let error = test_client().get(&origin).unwrap_err();

        assert!(matches!(
            error,
            HttpClientError::UnsupportedBodyFraming(BodyKind::Chunked)
        ));
        server.join().unwrap();
    }

    #[test]
    fn rejects_incomplete_response_head() {
        let response = b"HTTP/1.0 200 OK\r\nContent-Length: 0\r\n".to_vec();
        let (origin, _, server) = serve(vec![response]);

        let error = test_client().get(&origin).unwrap_err();

        assert!(matches!(error, HttpClientError::IncompleteResponseHead));
        server.join().unwrap();
    }

    #[test]
    fn rejects_response_head_line_over_limit() {
        let response = b"HTTP/1.0 200 This-reason-is-too-long\r\n\r\n".to_vec();
        let (origin, _, server) = serve(vec![response]);
        let mut client = test_client();
        client.max_head_bytes = 16;

        let error = client.get(&origin).unwrap_err();

        assert!(matches!(
            error,
            HttpClientError::ResponseHeadTooLarge { limit: 16 }
        ));
        server.join().unwrap();
    }

    #[test]
    fn validates_scheme_credentials_and_request_metadata_before_connecting() {
        assert!(matches!(
            test_client().get("https://example.test/"),
            Err(HttpClientError::UnsupportedScheme(scheme)) if scheme == "https"
        ));
        assert!(matches!(
            test_client().get("http://user@example.test/"),
            Err(HttpClientError::UnsupportedCredentials)
        ));
        assert!(matches!(
            test_client().get("http:///missing"),
            Err(HttpClientError::MissingHost)
        ));

        let mut client = test_client();
        client.user_agent = "Venture\r\nInjected: yes".to_string();
        assert!(matches!(
            client.get("http://example.test/"),
            Err(HttpClientError::InvalidHeaderValue { name: "User-Agent" })
        ));
        assert!(matches!(
            test_client().get("http://example.test/bad path"),
            Err(HttpClientError::InvalidRequestTarget)
        ));
        assert!(matches!(
            test_client().get("http://bad\nhost/"),
            Err(HttpClientError::InvalidHeaderValue { name: "Host" })
        ));
    }
}
