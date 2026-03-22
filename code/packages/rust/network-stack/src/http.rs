// ============================================================================
// Layer 7: HTTP — Hypertext Transfer Protocol
// ============================================================================
//
// HTTP is the protocol of the World Wide Web. It is text-based and
// request-response: the client sends a request, the server sends a response.
//
// Request format:
//   GET /index.html HTTP/1.1\r\n
//   Host: example.com\r\n
//   \r\n
//
// Response format:
//   HTTP/1.1 200 OK\r\n
//   Content-Type: text/html\r\n
//   \r\n
//   Hello, World!
//
// The blank line (\r\n\r\n) separates headers from body.
//
// ============================================================================

use std::collections::HashMap;
use crate::dns::DnsResolver;

// ============================================================================
// HttpRequest
// ============================================================================
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpRequest {
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    /// Serialize to HTTP/1.1 wire format.
    pub fn serialize(&self) -> String {
        let mut result = format!("{} {} HTTP/1.1\r\n", self.method, self.path);
        for (key, value) in &self.headers {
            result.push_str(&format!("{}: {}\r\n", key, value));
        }
        result.push_str("\r\n");
        if !self.body.is_empty() {
            result.push_str(&self.body);
        }
        result
    }

    /// Deserialize from HTTP/1.1 wire format.
    pub fn deserialize(text: &str) -> Option<Self> {
        if text.is_empty() {
            return None;
        }

        let parts: Vec<&str> = text.splitn(2, "\r\n\r\n").collect();
        let header_section = parts[0];
        let body = if parts.len() > 1 { parts[1] } else { "" };

        let lines: Vec<&str> = header_section.split("\r\n").collect();
        if lines.is_empty() {
            return None;
        }

        // Parse request line: "GET /path HTTP/1.1"
        let request_parts: Vec<&str> = lines[0].splitn(3, ' ').collect();
        if request_parts.len() < 2 {
            return None;
        }

        let method = request_parts[0].to_string();
        let path = request_parts[1].to_string();

        let mut headers = HashMap::new();
        for line in &lines[1..] {
            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        Some(Self {
            method,
            path,
            headers,
            body: body.to_string(),
        })
    }
}

// ============================================================================
// HttpResponse
// ============================================================================
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status_code: u16, status_text: &str) -> Self {
        Self {
            status_code,
            status_text: status_text.to_string(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    /// Serialize to HTTP/1.1 wire format.
    pub fn serialize(&self) -> String {
        let mut result = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text);
        for (key, value) in &self.headers {
            result.push_str(&format!("{}: {}\r\n", key, value));
        }
        result.push_str("\r\n");
        if !self.body.is_empty() {
            result.push_str(&self.body);
        }
        result
    }

    /// Deserialize from HTTP/1.1 wire format.
    pub fn deserialize(text: &str) -> Option<Self> {
        if text.is_empty() {
            return None;
        }

        let parts: Vec<&str> = text.splitn(2, "\r\n\r\n").collect();
        let header_section = parts[0];
        let body = if parts.len() > 1 { parts[1] } else { "" };

        let lines: Vec<&str> = header_section.split("\r\n").collect();
        if lines.is_empty() {
            return None;
        }

        // Parse status line: "HTTP/1.1 200 OK"
        let status_parts: Vec<&str> = lines[0].splitn(3, ' ').collect();
        if status_parts.len() < 3 {
            return None;
        }

        let status_code: u16 = status_parts[1].parse().ok()?;
        let status_text = status_parts[2].to_string();

        let mut headers = HashMap::new();
        for line in &lines[1..] {
            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        Some(Self {
            status_code,
            status_text,
            headers,
            body: body.to_string(),
        })
    }
}

// ============================================================================
// HttpClient — Build Requests and Parse Responses
// ============================================================================
pub struct HttpClient {
    pub dns_resolver: DnsResolver,
}

impl HttpClient {
    pub fn new() -> Self {
        Self { dns_resolver: DnsResolver::new() }
    }

    pub fn with_resolver(resolver: DnsResolver) -> Self {
        Self { dns_resolver: resolver }
    }

    /// Build an HTTP request for a URL.
    /// Returns (hostname, port, request) or None on parse failure.
    pub fn build_request(&self, url: &str, method: &str, body: &str, content_type: Option<&str>) -> Option<(String, u16, HttpRequest)> {
        let (hostname, port, path) = parse_url(url)?;

        let mut request = HttpRequest::new(method, &path);
        request.headers.insert("Host".to_string(), hostname.clone());

        if !body.is_empty() {
            request.body = body.to_string();
            request.headers.insert("Content-Length".to_string(), body.len().to_string());
            if let Some(ct) = content_type {
                request.headers.insert("Content-Type".to_string(), ct.to_string());
            }
        }

        Some((hostname, port, request))
    }

    /// Parse an HTTP response string.
    pub fn parse_response(&self, text: &str) -> Option<HttpResponse> {
        HttpResponse::deserialize(text)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a URL into (hostname, port, path).
///
/// Handles:
///   "http://example.com"           -> ("example.com", 80, "/")
///   "http://example.com:8080/api"  -> ("example.com", 8080, "/api")
fn parse_url(url: &str) -> Option<(String, u16, String)> {
    // Strip http:// or https://
    let rest = url.trim_start_matches("https://").trim_start_matches("http://");

    // Split host from path
    let (host_part, path) = if let Some(idx) = rest.find('/') {
        (&rest[..idx], &rest[idx..])
    } else {
        (rest, "/")
    };

    // Split host from port
    let (hostname, port) = if let Some(idx) = host_part.find(':') {
        let port_str = &host_part[idx + 1..];
        (&host_part[..idx], port_str.parse::<u16>().ok()?)
    } else {
        (host_part, 80u16)
    };

    Some((hostname.to_string(), port, path.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialize_get() {
        let mut req = HttpRequest::new("GET", "/index.html");
        req.headers.insert("Host".to_string(), "example.com".to_string());

        let text = req.serialize();
        assert!(text.contains("GET /index.html HTTP/1.1\r\n"));
        assert!(text.contains("Host: example.com\r\n"));
        assert!(text.ends_with("\r\n\r\n"));
    }

    #[test]
    fn test_request_serialize_post_with_body() {
        let mut req = HttpRequest::new("POST", "/api/data");
        req.body = r#"{"key":"val"}"#.to_string();
        req.headers.insert("Content-Type".to_string(), "application/json".to_string());

        let text = req.serialize();
        assert!(text.contains("POST /api/data HTTP/1.1"));
        assert!(text.contains(r#"{"key":"val"}"#));
    }

    #[test]
    fn test_request_deserialize() {
        let text = "GET /page HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let req = HttpRequest::deserialize(text).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/page");
        assert_eq!(req.headers.get("Host").unwrap(), "example.com");
        assert_eq!(req.body, "");
    }

    #[test]
    fn test_request_deserialize_with_body() {
        let text = "POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello";
        let req = HttpRequest::deserialize(text).unwrap();

        assert_eq!(req.method, "POST");
        assert_eq!(req.body, "hello");
    }

    #[test]
    fn test_request_round_trip() {
        let mut original = HttpRequest::new("GET", "/test");
        original.headers.insert("Host".to_string(), "localhost".to_string());

        let restored = HttpRequest::deserialize(&original.serialize()).unwrap();
        assert_eq!(restored.method, "GET");
        assert_eq!(restored.path, "/test");
        assert_eq!(restored.headers.get("Host").unwrap(), "localhost");
    }

    #[test]
    fn test_request_deserialize_empty() {
        assert!(HttpRequest::deserialize("").is_none());
    }

    #[test]
    fn test_response_serialize_200() {
        let mut resp = HttpResponse::new(200, "OK");
        resp.headers.insert("Content-Type".to_string(), "text/html".to_string());
        resp.body = "Hello, World!".to_string();

        let text = resp.serialize();
        assert!(text.contains("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Hello, World!"));
    }

    #[test]
    fn test_response_serialize_404() {
        let mut resp = HttpResponse::new(404, "Not Found");
        resp.body = "Page not found".to_string();

        let text = resp.serialize();
        assert!(text.contains("HTTP/1.1 404 Not Found"));
        assert!(text.contains("Page not found"));
    }

    #[test]
    fn test_response_deserialize() {
        let text = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        let resp = HttpResponse::deserialize(text).unwrap();

        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.status_text, "OK");
        assert_eq!(resp.headers.get("Content-Length").unwrap(), "5");
        assert_eq!(resp.body, "hello");
    }

    #[test]
    fn test_response_deserialize_404() {
        let text = "HTTP/1.1 404 Not Found\r\n\r\n";
        let resp = HttpResponse::deserialize(text).unwrap();
        assert_eq!(resp.status_code, 404);
        assert_eq!(resp.status_text, "Not Found");
    }

    #[test]
    fn test_response_round_trip() {
        let mut original = HttpResponse::new(200, "OK");
        original.headers.insert("Server".to_string(), "coding-adventures".to_string());
        original.body = "test body".to_string();

        let restored = HttpResponse::deserialize(&original.serialize()).unwrap();
        assert_eq!(restored.status_code, 200);
        assert_eq!(restored.body, "test body");
    }

    #[test]
    fn test_response_deserialize_empty() {
        assert!(HttpResponse::deserialize("").is_none());
    }

    #[test]
    fn test_parse_url_simple() {
        let (host, port, path) = parse_url("http://example.com/page").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        assert_eq!(path, "/page");
    }

    #[test]
    fn test_parse_url_with_port() {
        let (host, port, path) = parse_url("http://localhost:8080/api").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
        assert_eq!(path, "/api");
    }

    #[test]
    fn test_parse_url_no_path() {
        let (host, port, path) = parse_url("http://example.com").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        assert_eq!(path, "/");
    }

    #[test]
    fn test_client_build_get_request() {
        let client = HttpClient::new();
        let (hostname, port, request) = client.build_request("http://example.com/page", "GET", "", None).unwrap();

        assert_eq!(hostname, "example.com");
        assert_eq!(port, 80);
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/page");
    }

    #[test]
    fn test_client_build_post_request() {
        let client = HttpClient::new();
        let (_, _, request) = client.build_request(
            "http://example.com/api",
            "POST",
            r#"{"data":true}"#,
            Some("application/json"),
        ).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.body, r#"{"data":true}"#);
        assert_eq!(request.headers.get("Content-Type").unwrap(), "application/json");
        assert_eq!(request.headers.get("Content-Length").unwrap(), "13");
    }

    #[test]
    fn test_client_parse_response() {
        let client = HttpClient::new();
        let resp = client.parse_response("HTTP/1.1 200 OK\r\n\r\nhello").unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body, "hello");
    }

    #[test]
    fn test_client_dns_resolver() {
        let client = HttpClient::new();
        assert_eq!(client.dns_resolver.resolve("localhost"), Some([127, 0, 0, 1]));
    }

    #[test]
    fn test_client_custom_resolver() {
        let mut resolver = DnsResolver::new();
        resolver.add_static("test.com", [1, 2, 3, 4]);
        let client = HttpClient::with_resolver(resolver);
        assert_eq!(client.dns_resolver.resolve("test.com"), Some([1, 2, 3, 4]));
    }

    #[test]
    fn test_client_default() {
        let client = HttpClient::default();
        assert_eq!(client.dns_resolver.resolve("localhost"), Some([127, 0, 0, 1]));
    }
}
