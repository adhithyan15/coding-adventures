# frozen_string_literal: true

require_relative "coding_adventures/url_parser/version"

# = CodingAdventures::UrlParser
#
# RFC 1738 URL parser with relative resolution and percent-encoding.
#
# A URL (Uniform Resource Locator) tells you *where* something is on the
# internet and *how* to get it. This module parses URLs into their component
# parts, resolves relative URLs against a base, and handles percent-encoding.
#
# == URL anatomy
#
#   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
#   └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
#  scheme  userinfo        host       port     path         query   fragment
#
# - *scheme*: how to deliver (http, ftp, mailto)
# - *host*: which server (www.example.com)
# - *port*: which door (8080; defaults to 80 for http)
# - *path*: which resource (/docs/page.html)
# - *query*: parameters (?q=hello)
# - *fragment*: client-side anchor (#section2) -- never sent to server
# - *userinfo*: credentials (rare today, common in early web)
module CodingAdventures
  module UrlParser
    # ========================================================================
    # Error hierarchy
    # ========================================================================

    # Base error class for all URL parsing errors.
    class UrlError < StandardError; end

    # Raised when the input has no scheme (e.g., "www.example.com" without "http://").
    class MissingScheme < UrlError; end

    # Raised when the scheme contains invalid characters (must be [a-z][a-z0-9+.-]*).
    class InvalidScheme < UrlError; end

    # Raised when the port is not a valid integer in the range 0-65535.
    class InvalidPort < UrlError; end

    # Raised when percent-encoding is malformed (e.g., "%GG", "%2" truncated).
    class InvalidPercentEncoding < UrlError; end

    # Raised when the host is empty in an authority-based URL ("http:///path").
    class EmptyHost < UrlError; end

    # Raised when a relative URL is resolved without a base.
    class RelativeWithoutBase < UrlError; end

    # ========================================================================
    # Default ports for well-known schemes
    # ========================================================================
    #
    # When a URL omits the port, we can infer the default from the scheme.
    # These are the three most common schemes with well-known defaults:
    #
    #   | Scheme | Default Port |
    #   |--------|-------------|
    #   | http   | 80          |
    #   | https  | 443         |
    #   | ftp    | 21          |
    DEFAULT_PORTS = {
      "http" => 80,
      "https" => 443,
      "ftp" => 21
    }.freeze

    # ========================================================================
    # Scheme validation pattern
    # ========================================================================
    #
    # A valid scheme must match [a-z][a-z0-9+.-]* per RFC 1738.
    # The first character must be a letter; subsequent characters can include
    # digits, plus, hyphen, and period. We validate after lowercasing.
    SCHEME_PATTERN = /\A[a-z][a-z0-9+.\-]*\z/

    # ========================================================================
    # Unreserved characters for percent-encoding
    # ========================================================================
    #
    # RFC 1738 defines "unreserved" characters that do NOT need encoding:
    #   A-Z a-z 0-9 - _ . ~
    # We also preserve forward slashes since they're path delimiters.
    UNRESERVED = /[A-Za-z0-9\-_.~\/]/

    # ========================================================================
    # Module-level percent-encoding functions
    # ========================================================================

    # Percent-encode a string for use in a URL path or query.
    #
    # Encodes all characters except unreserved ones (A-Z a-z 0-9 - _ . ~ /).
    # Each byte that needs encoding becomes %XX with uppercase hex digits.
    #
    # == Examples
    #
    #   CodingAdventures::UrlParser.percent_encode("hello world")
    #   # => "hello%20world"
    #
    #   CodingAdventures::UrlParser.percent_encode("/path/to/file")
    #   # => "/path/to/file"   (slashes are unreserved)
    #
    # == How it works
    #
    # We iterate over each *byte* of the UTF-8 string (not characters), because
    # percent-encoding operates at the byte level. A multi-byte character like
    # "日" (3 bytes: E6 97 A5) becomes "%E6%97%A5".
    def self.percent_encode(input)
      result = +""
      input.each_byte do |byte|
        if byte.chr.match?(UNRESERVED)
          result << byte.chr
        else
          result << format("%%%02X", byte)
        end
      end
      result
    end

    # Percent-decode a string: "%20" becomes " ", "%E6%97%A5" becomes "日".
    #
    # Each %XX sequence is replaced by the byte with that hex value. The
    # resulting bytes are interpreted as UTF-8.
    #
    # == Examples
    #
    #   CodingAdventures::UrlParser.percent_decode("hello%20world")
    #   # => "hello world"
    #
    #   CodingAdventures::UrlParser.percent_decode("%E6%97%A5")
    #   # => "日"
    #
    # == Algorithm
    #
    # Walk through the input byte by byte:
    #   - If we see '%', read the next two characters as hex digits
    #   - Otherwise, copy the byte through unchanged
    # Finally, force the result to UTF-8 encoding.
    def self.percent_decode(input)
      bytes = input.bytes.to_a
      result = []
      i = 0

      while i < bytes.length
        if bytes[i] == 0x25 # '%' character
          # Need at least 2 more hex digits after the '%'
          raise InvalidPercentEncoding, "truncated percent-encoding" if i + 2 >= bytes.length

          hi = hex_digit(bytes[i + 1])
          lo = hex_digit(bytes[i + 2])
          result << ((hi << 4) | lo)
          i += 3
        else
          result << bytes[i]
          i += 1
        end
      end

      # Pack bytes and interpret as UTF-8
      decoded = result.pack("C*")
      decoded.force_encoding("UTF-8")

      raise InvalidPercentEncoding, "invalid UTF-8 in decoded output" unless decoded.valid_encoding?

      decoded
    end

    # ========================================================================
    # Url class
    # ========================================================================

    # A parsed URL with all components separated.
    #
    # All string fields store the decoded values. The original input is
    # preserved for round-tripping via +to_s+.
    #
    # == Invariants
    #
    # - +scheme+ is always lowercased
    # - +host+ is always lowercased (when present)
    # - +path+ starts with "/" for authority-based URLs (http, ftp)
    # - +query+ does NOT include the leading "?"
    # - +fragment+ does NOT include the leading "#"
    class Url
      attr_reader :scheme, :userinfo, :host, :port, :path, :query, :fragment

      def initialize(scheme:, path:, userinfo: nil, host: nil, port: nil, query: nil, fragment: nil)
        @scheme = scheme
        @userinfo = userinfo
        @host = host
        @port = port
        @path = path
        @query = query
        @fragment = fragment
      end

      # Parse an absolute URL string.
      #
      # The input must contain a scheme (e.g., "http://..."). For relative URLs,
      # first parse the base URL, then call Url#resolve.
      #
      # == Algorithm
      #
      # Single-pass, left-to-right:
      #
      #   "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2"
      #    ^^^^                                                              ^^^^
      #    Step 1: scheme = "http"                            Step 2: fragment = "sec2"
      #                                                     ^^^^^^^^
      #                                             Step 3: query = "q=hello"
      #                                      ^^^^^^^^^^^^^^^
      #                              Step 4: path = "/docs/page.html"
      #          ^^^^^^^^^^^^
      #      Step 5: userinfo = "alice:secret"
      #                                  ^^^^
      #                      Step 6: port = 8080
      #                         ^^^^^^^^^^^^^^^
      #                 Step 7: host = "www.example.com"
      #
      # == Error conditions
      #
      # - MissingScheme: no "://" and no "scheme:" prefix found
      # - InvalidScheme: scheme doesn't match [a-z][a-z0-9+.-]*
      # - InvalidPort: port is not a valid integer 0-65535
      def self.parse(input)
        raw = input.to_s
        input = raw.strip

        # Step 1: Extract scheme by finding "://"
        #
        # Two forms of URLs:
        #   1. "scheme://authority/path" (most URLs)
        #   2. "scheme:path" (mailto:, data:, etc.)
        sep_index = input.index("://")

        if sep_index
          # Standard authority-based URL
          scheme = input[0...sep_index].downcase
          validate_scheme!(scheme)
          after_scheme = input[(sep_index + 3)..]
        else
          # Try "scheme:path" form (e.g., "mailto:alice@example.com")
          colon_index = input.index(":")
          if colon_index && colon_index > 0 && !input[0...colon_index].include?("/")
            scheme = input[0...colon_index].downcase
            validate_scheme!(scheme)

            # No authority -- the rest is the path
            rest = input[(colon_index + 1)..]

            # Still split fragment and query from path
            rest, fragment = UrlParser.send(:split_fragment, rest)
            rest, query = UrlParser.send(:split_query, rest)

            return new(scheme: scheme, path: rest, query: query, fragment: fragment)
          else
            raise MissingScheme, "missing scheme (expected '://')"
          end
        end

        # Step 2: Extract fragment (find "#" from the right)
        #
        # The fragment is everything after the first '#'. It's a client-side
        # anchor and is never sent to the server.
        after_scheme, fragment = UrlParser.send(:split_fragment, after_scheme)

        # Step 3: Extract query (find "?")
        #
        # The query is everything between "?" and the end (or "#" which we
        # already removed).
        after_scheme, query = UrlParser.send(:split_query, after_scheme)

        # Step 4: Split authority from path (find first "/")
        #
        # In "host:port/path/to/resource", the first "/" begins the path.
        # If there's no "/", the path defaults to "/".
        slash_index = after_scheme.index("/")
        if slash_index
          authority_str = after_scheme[0...slash_index]
          path = after_scheme[slash_index..]
        else
          authority_str = after_scheme
          path = "/"
        end

        # Step 5: Extract userinfo (find "@" in authority)
        #
        # The userinfo is everything before the last "@" in the authority.
        # Example: "alice:secret@host.com" -> userinfo="alice:secret", host_port="host.com"
        at_index = authority_str.rindex("@")
        if at_index
          userinfo = authority_str[0...at_index]
          host_port = authority_str[(at_index + 1)..]
        else
          userinfo = nil
          host_port = authority_str
        end

        # Step 6 & 7: Extract port and host
        #
        # IPv6 addresses are enclosed in brackets: [::1]:8080
        # For IPv6, the port delimiter is the ":" AFTER the closing "]"
        # For IPv4/hostname, it's the last ":" where everything after is digits
        host, port = parse_host_port(host_port)

        # Normalize: empty host becomes nil, host is lowercased
        host = if host.nil? || host.empty?
                 nil
               else
                 host.downcase
               end

        new(
          scheme: scheme,
          userinfo: userinfo,
          host: host,
          port: port,
          path: path,
          query: query,
          fragment: fragment
        )
      end

      # Resolve a relative URL against this URL as the base.
      #
      # Implements the RFC 1808 relative resolution algorithm:
      #
      #   if R has scheme     -> R is absolute, return as-is
      #   if R starts with // -> inherit scheme only
      #   if R starts with /  -> inherit scheme + authority, replace path
      #   otherwise           -> merge paths, resolve . and ..
      #
      # == Examples
      #
      #   base = CodingAdventures::UrlParser::Url.parse("http://host/a/b/c.html")
      #
      #   # Same directory
      #   base.resolve("d.html").path  # => "/a/b/d.html"
      #
      #   # Parent directory
      #   base.resolve("../d.html").path  # => "/a/d.html"
      #
      #   # Absolute path
      #   base.resolve("/x/y.html").path  # => "/x/y.html"
      def resolve(relative)
        relative = relative.strip

        # Empty relative -> return base without fragment
        #
        # Per RFC 1808, an empty reference inherits everything from the base
        # except the fragment (which is stripped).
        if relative.empty?
          return self.class.new(
            scheme: @scheme, userinfo: @userinfo, host: @host,
            port: @port, path: @path, query: @query, fragment: nil
          )
        end

        # Fragment-only: "#section"
        #
        # Only the fragment changes; everything else comes from the base.
        if relative.start_with?("#")
          return self.class.new(
            scheme: @scheme, userinfo: @userinfo, host: @host,
            port: @port, path: @path, query: @query,
            fragment: relative[1..]
          )
        end

        # If R has a scheme, it's already absolute
        #
        # We check for "://" or a "scheme:" prefix where the part before ":"
        # looks like a valid scheme (starts with letter, no slashes).
        if relative.include?("://") || (relative.include?(":") && !relative.start_with?("/"))
          colon = relative.index(":")
          if colon
            maybe_scheme = relative[0...colon]
            if !maybe_scheme.empty? &&
               maybe_scheme.match?(/\A[a-zA-Z][a-zA-Z0-9+.\-]*\z/)
              return self.class.parse(relative)
            end
          end
        end

        # Scheme-relative: "//host/path"
        #
        # Inherit only the scheme from the base; everything else comes from
        # the relative reference.
        if relative.start_with?("//")
          full = "#{@scheme}:#{relative}"
          return self.class.parse(full)
        end

        # Absolute path: "/path"
        #
        # Inherit scheme + authority from base; replace the path entirely.
        if relative.start_with?("/")
          rel_path, frag = UrlParser.send(:split_fragment, relative)
          rel_path, qry = UrlParser.send(:split_query, rel_path)

          return self.class.new(
            scheme: @scheme, userinfo: @userinfo, host: @host,
            port: @port, path: UrlParser.send(:remove_dot_segments, rel_path),
            query: qry, fragment: frag
          )
        end

        # Relative path: merge with base
        #
        # Take the base path up to and including the last "/", then append
        # the relative path. Finally, resolve any "." and ".." segments.
        rel_path, frag = UrlParser.send(:split_fragment, relative)
        rel_path, qry = UrlParser.send(:split_query, rel_path)

        merged = UrlParser.send(:merge_paths, @path, rel_path)
        resolved_path = UrlParser.send(:remove_dot_segments, merged)

        self.class.new(
          scheme: @scheme, userinfo: @userinfo, host: @host,
          port: @port, path: resolved_path, query: qry, fragment: frag
        )
      end

      # The effective port -- explicit port if set, otherwise the scheme default.
      #
      # Returns the explicit port when present, or looks up the default port
      # for the scheme. Returns nil if neither is available (e.g., mailto: URLs).
      #
      # == Truth table
      #
      #   | port  | scheme | effective_port |
      #   |-------|--------|----------------|
      #   | 8080  | http   | 8080           |
      #   | nil   | http   | 80             |
      #   | nil   | https  | 443            |
      #   | nil   | ftp    | 21             |
      #   | nil   | mailto | nil            |
      def effective_port
        @port || DEFAULT_PORTS[@scheme]
      end

      # The authority string: [userinfo@]host[:port]
      #
      # Reconstructs the authority component from its pieces. This is the
      # part between "://" and the path.
      #
      # == Examples
      #
      #   "user:pass@host.com:8080"  (all parts)
      #   "host.com"                 (host only)
      #   ""                         (no authority, e.g., mailto:)
      def authority
        auth = +""
        if @userinfo
          auth << @userinfo
          auth << "@"
        end
        auth << @host if @host
        if @port
          auth << ":"
          auth << @port.to_s
        end
        auth
      end

      # Serialize back to a URL string.
      #
      # Reconstructs the full URL from its components. For authority-based URLs
      # (those with a host), uses "scheme://authority/path". For scheme-only
      # URLs (like mailto:), uses "scheme:path".
      def to_url_string
        s = +""
        s << @scheme

        if @host
          s << "://"
          s << authority
        else
          s << ":"
        end

        s << @path

        if @query
          s << "?"
          s << @query
        end

        if @fragment
          s << "#"
          s << @fragment
        end

        s
      end

      # Display uses to_url_string for a human-readable representation.
      def to_s
        to_url_string
      end

      private

      # Parse the host:port portion of the authority.
      #
      # Handles two cases:
      #   1. IPv6: [::1]:8080 -- port comes after the closing bracket
      #   2. IPv4/hostname: host.com:8080 -- port after the last colon
      #
      # For IPv4, we only treat the part after the last colon as a port if
      # it consists entirely of digits. This prevents "host:name" from being
      # misinterpreted.
      def self.parse_host_port(host_port)
        if host_port.start_with?("[")
          # IPv6: find closing bracket
          bracket_pos = host_port.index("]")
          if bracket_pos
            host = host_port[0..bracket_pos]
            after_bracket = host_port[(bracket_pos + 1)..]
            if after_bracket.start_with?(":")
              port = parse_port!(after_bracket[1..])
              [host, port]
            else
              [host, nil]
            end
          else
            # Malformed IPv6 -- treat whole thing as host
            [host_port, nil]
          end
        else
          # IPv4 or hostname: last ":" separates host from port
          colon_pos = host_port.rindex(":")
          if colon_pos
            maybe_port = host_port[(colon_pos + 1)..]
            # Only treat as port if it's all digits and non-empty
            if !maybe_port.empty? && maybe_port.match?(/\A\d+\z/)
              host = host_port[0...colon_pos]
              port = parse_port!(maybe_port)
              [host, port]
            else
              [host_port, nil]
            end
          else
            [host_port, nil]
          end
        end
      end

      # Parse a port string to an integer (0-65535).
      #
      # Port numbers are unsigned 16-bit integers. Values outside 0-65535
      # raise InvalidPort.
      def self.parse_port!(port_str)
        port = Integer(port_str, 10)
        raise InvalidPort, "port #{port} out of range (must be 0-65535)" unless port.between?(0, 65_535)

        port
      rescue ArgumentError
        raise InvalidPort, "invalid port: #{port_str}"
      end

      # Validate that a scheme matches [a-z][a-z0-9+.-]*.
      #
      # The scheme has already been lowercased before this check. We verify:
      #   1. It's not empty
      #   2. First character is a lowercase letter
      #   3. Remaining characters are lowercase letters, digits, +, -, or .
      def self.validate_scheme!(scheme)
        raise InvalidScheme, "empty scheme" if scheme.empty?
        raise InvalidScheme, "invalid scheme: #{scheme}" unless scheme.match?(SCHEME_PATTERN)
      end
    end

    # ========================================================================
    # Private module helpers
    # ========================================================================

    # Split a string at the first "#", returning [before, after] or [input, nil].
    #
    # The fragment is everything after the first "#" character. This is always
    # the outermost split because fragments can contain "?" characters.
    def self.split_fragment(input)
      pos = input.index("#")
      if pos
        [input[0...pos], input[(pos + 1)..]]
      else
        [input, nil]
      end
    end

    # Split a string at the first "?", returning [before, after] or [input, nil].
    #
    # The query is everything between "?" and the end of the remaining string
    # (we've already stripped the fragment).
    def self.split_query(input)
      pos = input.index("?")
      if pos
        [input[0...pos], input[(pos + 1)..]]
      else
        [input, nil]
      end
    end

    # Merge a base path and a relative path.
    #
    # Takes everything in base_path up to and including the last "/",
    # then appends relative_path.
    #
    # == Examples
    #
    #   merge_paths("/a/b/c", "d")   # => "/a/b/d"
    #   merge_paths("/a/b/",  "d")   # => "/a/b/d"
    #   merge_paths("/a",     "d")   # => "/d"
    def self.merge_paths(base_path, relative_path)
      last_slash = base_path.rindex("/")
      if last_slash
        base_path[0..last_slash] + relative_path
      else
        "/#{relative_path}"
      end
    end

    # Remove "." and ".." segments from a path.
    #
    # Implements the "remove dot segments" algorithm from RFC 3986 section 5.2.4:
    #
    #   /a/b/../c    => /a/c       (go up one level)
    #   /a/./b       => /a/b       (current directory is a no-op)
    #   /a/b/../../c => /c         (go up two levels)
    #   /a/../../../c => /c        (can't go above root)
    #
    # == Algorithm
    #
    # Split the path on "/". For each segment:
    #   - "."  -> skip (current directory)
    #   - ".." -> pop the last segment (go up one level)
    #   - anything else -> push onto the output stack
    #
    # Finally, rejoin with "/" and ensure the result starts with "/" if the
    # input did.
    def self.remove_dot_segments(path)
      output = []

      path.split("/", -1).each do |segment|
        case segment
        when "."
          # Skip -- "current directory" is a no-op
        when ".."
          # Go up one level -- remove the last segment (if any)
          output.pop
        else
          output.push(segment)
        end
      end

      result = output.join("/")

      # Ensure the path starts with "/" if the input did
      if path.start_with?("/") && !result.start_with?("/")
        "/#{result}"
      else
        result
      end
    end

    # Convert a hex ASCII digit to its numeric value (0-15).
    #
    # Accepts 0-9, a-f, A-F. Raises InvalidPercentEncoding for anything else.
    def self.hex_digit(byte)
      case byte
      when 0x30..0x39 then byte - 0x30       # '0'-'9'
      when 0x61..0x66 then byte - 0x61 + 10  # 'a'-'f'
      when 0x41..0x46 then byte - 0x41 + 10  # 'A'-'F'
      else
        raise InvalidPercentEncoding, "invalid hex digit: #{byte.chr}"
      end
    end

    # Make helpers private at the module level
    private_class_method :split_fragment, :split_query, :merge_paths,
                         :remove_dot_segments, :hex_digit
  end
end
