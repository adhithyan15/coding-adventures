"""
HTTP — Layer 7 (Application)
=============================

HTTP (HyperText Transfer Protocol) is the language of the web. When your
browser loads a webpage, it sends an HTTP request to a server, and the
server responds with HTML, images, JSON, or whatever the page contains.

HTTP is a **request-response** protocol: the client sends a request, the
server sends a response, and (in HTTP/1.0) the connection closes. HTTP/1.1
added persistent connections, and HTTP/2 added multiplexing, but the
fundamental request-response pattern remains.

HTTP Message Format
-------------------

Both requests and responses are plain text (at least in HTTP/1.1). This
makes HTTP easy to debug — you can literally read the bytes on the wire.

Request::

    GET /index.html HTTP/1.1\\r\\n
    Host: example.com\\r\\n
    User-Agent: Mozilla/5.0\\r\\n
    \\r\\n
    (optional body)

    ^^^^^^^^^^^^^^^^^^^^^^^^^^^
    First line: METHOD PATH VERSION
    Then: headers (key: value), one per line
    Then: blank line (\\r\\n)
    Then: body (optional, for POST/PUT)

Response::

    HTTP/1.1 200 OK\\r\\n
    Content-Type: text/html\\r\\n
    Content-Length: 13\\r\\n
    \\r\\n
    Hello, World!

    ^^^^^^^^^^^^^^^^^^
    First line: VERSION STATUS_CODE STATUS_TEXT
    Then: headers
    Then: blank line
    Then: body

The \\r\\n (CRLF) line endings are required by the HTTP spec. A blank line
(just \\r\\n) separates the headers from the body.

HTTP Methods
------------

- **GET**: Retrieve a resource. No body. Idempotent (safe to retry).
- **POST**: Submit data to the server. Has a body. Not idempotent.
- **PUT**: Replace a resource. Has a body. Idempotent.
- **DELETE**: Remove a resource. Idempotent.
- **HEAD**: Like GET but only returns headers, no body.

Status Codes
------------

- **1xx**: Informational (100 Continue)
- **2xx**: Success (200 OK, 201 Created, 204 No Content)
- **3xx**: Redirection (301 Moved, 302 Found, 304 Not Modified)
- **4xx**: Client error (400 Bad Request, 403 Forbidden, 404 Not Found)
- **5xx**: Server error (500 Internal Server Error, 503 Service Unavailable)
"""

from __future__ import annotations

from dataclasses import dataclass, field

from network_stack.dns import DNSResolver


@dataclass
class HTTPRequest:
    """
    An HTTP request — what the client sends to the server.

    Attributes
    ----------
    method : str
        The HTTP method (GET, POST, PUT, DELETE, etc.).
    path : str
        The path on the server (e.g., "/index.html", "/api/users").
    headers : dict[str, str]
        HTTP headers (e.g., {"Host": "example.com"}).
    body : bytes
        The request body (empty for GET requests).

    Example
    -------
    >>> req = HTTPRequest(method="GET", path="/",
    ...                   headers={"Host": "example.com"})
    >>> print(req.serialize().decode())
    GET / HTTP/1.1\\r\\nHost: example.com\\r\\n\\r\\n
    """

    method: str = "GET"
    path: str = "/"
    headers: dict[str, str] = field(default_factory=dict)
    body: bytes = b""

    def serialize(self) -> bytes:
        """
        Convert this request to raw HTTP bytes.

        Format::

            METHOD PATH HTTP/1.1\\r\\n
            Header-Name: Header-Value\\r\\n
            ...\\r\\n
            \\r\\n
            body

        If the request has a body, we automatically add a Content-Length
        header so the server knows how many bytes to expect.
        """
        # Request line: "GET / HTTP/1.1"
        lines = [f"{self.method} {self.path} HTTP/1.1"]

        # Headers
        headers = dict(self.headers)
        if self.body and "Content-Length" not in headers:
            headers["Content-Length"] = str(len(self.body))

        for name, value in headers.items():
            lines.append(f"{name}: {value}")

        # Join with CRLF, add the blank line separator, then body
        header_bytes = "\r\n".join(lines).encode() + b"\r\n\r\n"
        return header_bytes + self.body


@dataclass
class HTTPResponse:
    """
    An HTTP response — what the server sends back to the client.

    Attributes
    ----------
    status_code : int
        The HTTP status code (200, 404, 500, etc.).
    status_text : str
        The human-readable status message ("OK", "Not Found", etc.).
    headers : dict[str, str]
        Response headers.
    body : bytes
        The response body.
    """

    status_code: int = 200
    status_text: str = "OK"
    headers: dict[str, str] = field(default_factory=dict)
    body: bytes = b""

    def serialize(self) -> bytes:
        """
        Convert this response to raw HTTP bytes.

        Format::

            HTTP/1.1 200 OK\\r\\n
            Content-Type: text/html\\r\\n
            Content-Length: 13\\r\\n
            \\r\\n
            Hello, World!
        """
        lines = [f"HTTP/1.1 {self.status_code} {self.status_text}"]

        headers = dict(self.headers)
        if self.body and "Content-Length" not in headers:
            headers["Content-Length"] = str(len(self.body))

        for name, value in headers.items():
            lines.append(f"{name}: {value}")

        header_bytes = "\r\n".join(lines).encode() + b"\r\n\r\n"
        return header_bytes + self.body

    @classmethod
    def deserialize(cls, data: bytes) -> HTTPResponse:
        """
        Parse raw HTTP bytes into an HTTPResponse.

        The parser splits on the blank line (\\r\\n\\r\\n) to separate
        headers from body, then parses the status line and headers.

        Parameters
        ----------
        data : bytes
            Raw HTTP response bytes.

        Returns
        -------
        HTTPResponse
            The parsed response.

        Raises
        ------
        ValueError
            If the data doesn't look like a valid HTTP response.
        """
        # Split headers and body at the blank line
        text = data.decode("utf-8", errors="replace")

        if "\r\n\r\n" in text:
            header_section, body_text = text.split("\r\n\r\n", 1)
        else:
            header_section = text
            body_text = ""

        lines = header_section.split("\r\n")

        # Parse the status line: "HTTP/1.1 200 OK"
        if not lines:
            msg = "Empty HTTP response"
            raise ValueError(msg)

        status_parts = lines[0].split(" ", 2)
        if len(status_parts) < 2:
            msg = f"Invalid HTTP status line: {lines[0]}"
            raise ValueError(msg)

        status_code = int(status_parts[1])
        status_text = status_parts[2] if len(status_parts) > 2 else ""

        # Parse headers: "Key: Value"
        headers: dict[str, str] = {}
        for line in lines[1:]:
            if ": " in line:
                key, value = line.split(": ", 1)
                headers[key] = value

        return cls(
            status_code=status_code,
            status_text=status_text,
            headers=headers,
            body=body_text.encode(),
        )


class HTTPClient:
    """
    A simple HTTP client that builds requests and parses responses.

    This client handles the application-level HTTP logic: building properly
    formatted requests, parsing URLs, and interpreting responses. It relies
    on a DNS resolver to convert hostnames to IP addresses.

    In a real HTTP client (like ``curl`` or Python's ``requests``), this
    would also manage TCP connections, handle redirects, parse cookies,
    support HTTPS (TLS), etc. Our version focuses on the HTTP message
    format.

    Parameters
    ----------
    dns_resolver : DNSResolver
        Used to resolve hostnames in URLs to IP addresses.

    Example
    -------
    >>> resolver = DNSResolver()
    >>> resolver.add_static("example.com", 0x5DB8D822)
    >>> client = HTTPClient(resolver)
    >>> host, port, req = client.build_request("GET", "http://example.com/page")
    >>> host
    'example.com'
    >>> req.path
    '/page'
    """

    def __init__(self, dns_resolver: DNSResolver) -> None:
        self.dns = dns_resolver

    def build_request(
        self, method: str, url: str, body: bytes = b""
    ) -> tuple[str, int, HTTPRequest]:
        """
        Build an HTTP request from a URL.

        Parses the URL to extract the hostname, port, and path, then
        creates an HTTPRequest with the appropriate Host header.

        URL format: ``http://hostname[:port]/path``

        Parameters
        ----------
        method : str
            HTTP method (GET, POST, etc.).
        url : str
            The full URL.
        body : bytes
            Optional request body.

        Returns
        -------
        tuple[str, int, HTTPRequest]
            (hostname, port, request)

        Raises
        ------
        ValueError
            If the URL format is invalid.
        """
        # Strip the scheme (http://)
        if url.startswith("http://"):
            url = url[7:]
        elif url.startswith("https://"):
            url = url[8:]

        # Split host and path
        if "/" in url:
            host_part, path = url.split("/", 1)
            path = "/" + path
        else:
            host_part = url
            path = "/"

        # Split host and port
        if ":" in host_part:
            hostname, port_str = host_part.split(":", 1)
            port = int(port_str)
        else:
            hostname = host_part
            port = 80  # Default HTTP port

        headers = {"Host": hostname}
        request = HTTPRequest(
            method=method,
            path=path,
            headers=headers,
            body=body,
        )

        return hostname, port, request

    def parse_response(self, data: bytes) -> HTTPResponse:
        """
        Parse raw bytes into an HTTPResponse.

        This is a convenience wrapper around HTTPResponse.deserialize().
        """
        return HTTPResponse.deserialize(data)
