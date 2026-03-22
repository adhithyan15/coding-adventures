# frozen_string_literal: true

# ============================================================================
# Layer 7: HTTP — Hypertext Transfer Protocol
# ============================================================================
#
# HTTP is the protocol of the World Wide Web. It is a text-based,
# request-response protocol built on top of TCP. Every time you load a
# web page, your browser sends an HTTP request and the server sends back
# an HTTP response.
#
# HTTP is "stateless" — each request-response pair is independent. The
# server does not remember previous requests (that's what cookies and
# sessions are for).
#
# Request format (what the client sends):
#
#   GET /index.html HTTP/1.1\r\n
#   Host: example.com\r\n
#   Content-Type: text/plain\r\n
#   \r\n
#   (optional body)
#
# Response format (what the server sends):
#
#   HTTP/1.1 200 OK\r\n
#   Content-Type: text/html\r\n
#   Content-Length: 13\r\n
#   \r\n
#   Hello, World!
#
# The blank line (\r\n\r\n) separates the headers from the body. This is
# how both sides know where the headers end and the body begins.
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    # ========================================================================
    # HTTPRequest
    # ========================================================================
    #
    # Represents an HTTP request from client to server. The three essential
    # parts are:
    #   - method: what action to perform (GET, POST, PUT, DELETE, etc.)
    #   - path: which resource to act on ("/", "/api/users", etc.)
    #   - headers: metadata about the request (Host, Content-Type, etc.)
    #
    # The optional body carries data for methods like POST and PUT.
    #
    # ========================================================================
    class HTTPRequest
      attr_accessor :method, :path, :headers, :body

      def initialize(method:, path:, headers: {}, body: "")
        @method  = method
        @path    = path
        @headers = headers.dup
        @body    = body
      end

      # Serialize the request to HTTP/1.1 wire format.
      #
      # Example output:
      #   "GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n"
      #
      def serialize
        lines = []
        lines.push("#{@method} #{@path} HTTP/1.1")
        @headers.each { |key, value| lines.push("#{key}: #{value}") }
        result = lines.join("\r\n") + "\r\n\r\n"
        result += @body unless @body.empty?
        result
      end

      # Deserialize an HTTP request from wire format.
      #
      # Parses the request line, headers, and body.
      #
      def self.deserialize(text)
        return nil if text.nil? || text.empty?

        # Split headers from body at the blank line
        parts = text.split("\r\n\r\n", 2)
        header_section = parts[0]
        body = parts[1] || ""

        lines = header_section.split("\r\n")
        return nil if lines.empty?

        # Parse request line: "GET /path HTTP/1.1"
        request_line = lines[0].split(" ", 3)
        return nil if request_line.length < 2

        method = request_line[0]
        path   = request_line[1]

        # Parse headers
        headers = {}
        lines[1..].each do |line|
          key, value = line.split(": ", 2)
          headers[key] = value if key && value
        end

        new(method: method, path: path, headers: headers, body: body)
      end
    end

    # ========================================================================
    # HTTPResponse
    # ========================================================================
    #
    # Represents an HTTP response from server to client. The key fields are:
    #   - status_code: numeric result (200, 404, 500, etc.)
    #   - status_text: human-readable result ("OK", "Not Found", etc.)
    #   - headers: metadata (Content-Type, Content-Length, etc.)
    #   - body: the response payload (HTML, JSON, etc.)
    #
    # Common status codes:
    #   200 OK              — request succeeded
    #   201 Created         — resource was created (POST)
    #   301 Moved Permanently — resource moved to a new URL
    #   400 Bad Request     — client sent invalid request
    #   404 Not Found       — resource does not exist
    #   500 Internal Error  — server crashed or had a bug
    #
    # ========================================================================
    class HTTPResponse
      attr_accessor :status_code, :status_text, :headers, :body

      def initialize(status_code:, status_text:, headers: {}, body: "")
        @status_code = status_code
        @status_text = status_text
        @headers     = headers.dup
        @body        = body
      end

      # Serialize the response to HTTP/1.1 wire format.
      #
      # Example output:
      #   "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!"
      #
      def serialize
        lines = []
        lines.push("HTTP/1.1 #{@status_code} #{@status_text}")
        @headers.each { |key, value| lines.push("#{key}: #{value}") }
        result = lines.join("\r\n") + "\r\n\r\n"
        result += @body unless @body.empty?
        result
      end

      # Deserialize an HTTP response from wire format.
      def self.deserialize(text)
        return nil if text.nil? || text.empty?

        parts = text.split("\r\n\r\n", 2)
        header_section = parts[0]
        body = parts[1] || ""

        lines = header_section.split("\r\n")
        return nil if lines.empty?

        # Parse status line: "HTTP/1.1 200 OK"
        status_parts = lines[0].split(" ", 3)
        return nil if status_parts.length < 3

        status_code = status_parts[1].to_i
        status_text = status_parts[2]

        headers = {}
        lines[1..].each do |line|
          key, value = line.split(": ", 2)
          headers[key] = value if key && value
        end

        new(status_code: status_code, status_text: status_text, headers: headers, body: body)
      end
    end

    # ========================================================================
    # HTTPClient — Build and Parse HTTP Messages
    # ========================================================================
    #
    # A simplified HTTP client that can build requests and parse responses.
    # In a real system, the client would use the Socket API to send the
    # request over TCP and receive the response. Here we focus on the
    # message formatting and parsing.
    #
    # ========================================================================
    class HTTPClient
      attr_reader :dns_resolver

      def initialize(dns_resolver: nil)
        @dns_resolver = dns_resolver || DNSResolver.new
      end

      # Build an HTTP GET request for a URL.
      #
      # URL format: "http://hostname:port/path" or "http://hostname/path"
      # Default port is 80.
      #
      # Returns [hostname, port, HTTPRequest] or nil on parse failure.
      #
      def build_request(url, method: "GET", body: "", content_type: nil)
        parsed = parse_url(url)
        return nil unless parsed

        hostname, port, path = parsed

        headers = {"Host" => hostname}
        unless body.empty?
          headers["Content-Length"] = body.length.to_s
          headers["Content-Type"] = content_type if content_type
        end

        request = HTTPRequest.new(method: method, path: path, headers: headers, body: body)
        [hostname, port, request]
      end

      # Parse an HTTP response string.
      def parse_response(text)
        HTTPResponse.deserialize(text)
      end

      private

      # Parse a URL into [hostname, port, path].
      #
      # Handles:
      #   "http://example.com"           -> ["example.com", 80, "/"]
      #   "http://example.com:8080/api"  -> ["example.com", 8080, "/api"]
      #   "http://example.com/page"      -> ["example.com", 80, "/page"]
      #
      def parse_url(url)
        # Strip "http://" prefix
        rest = url.sub(%r{^https?://}, "")

        # Split host from path
        slash_idx = rest.index("/")
        if slash_idx
          host_part = rest[0...slash_idx]
          path = rest[slash_idx..]
        else
          host_part = rest
          path = "/"
        end

        # Split host from port
        colon_idx = host_part.index(":")
        if colon_idx
          hostname = host_part[0...colon_idx]
          port = host_part[(colon_idx + 1)..].to_i
        else
          hostname = host_part
          port = 80
        end

        [hostname, port, path]
      end
    end
  end
end
