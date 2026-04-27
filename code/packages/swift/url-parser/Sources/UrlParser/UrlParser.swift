// UrlParser.swift — RFC 1738 URL parser with relative resolution and percent-encoding
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Overview
// ============================================================================
//
// A URL (Uniform Resource Locator) tells you **where** something is on the
// internet and **how** to get it. This module parses URLs into their component
// parts, resolves relative URLs against a base, and handles percent-encoding.
//
// ## URL anatomy
//
//   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
//   └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
//  scheme  userinfo        host       port     path         query   fragment
//
// - **scheme**: how to deliver (http, ftp, mailto)
// - **host**: which server (www.example.com)
// - **port**: which door (8080; defaults to 80 for http)
// - **path**: which resource (/docs/page.html)
// - **query**: parameters (?q=hello)
// - **fragment**: client-side anchor (#section2) — never sent to server
// - **userinfo**: credentials (rare today, common in early web)
//
// ## Parsing algorithm
//
// The URL is parsed left-to-right in a single pass, no backtracking:
//
// 1. Find `://` → extract scheme (lowercased)
// 2. Find `#` → extract fragment
// 3. Find `?` → extract query
// 4. Find first `/` → extract path (default "/")
// 5. Find `@` → extract userinfo
// 6. Find last `:` → extract port (digits only)
// 7. Remainder → host (lowercased)

import Foundation

// ============================================================================
// Error type
// ============================================================================

/// Errors that can occur when parsing or resolving a URL.
///
/// Each case corresponds to a specific structural problem in the input:
///
/// | Error                    | Meaning                                          |
/// |--------------------------|--------------------------------------------------|
/// | missingScheme            | No `://` or `scheme:` found                      |
/// | invalidScheme            | Scheme doesn't match `[a-z][a-z0-9+.-]*`         |
/// | invalidPort              | Port is not a valid UInt16 (0–65535)              |
/// | invalidPercentEncoding   | `%XX` is malformed or not valid UTF-8             |
/// | emptyHost                | Empty host in an authority-based URL              |
/// | relativeWithoutBase      | Relative URL needs a base to resolve against      |
public enum UrlError: Error, Equatable {
    case missingScheme
    case invalidScheme
    case invalidPort
    case invalidPercentEncoding
    case emptyHost
    case relativeWithoutBase
}

// ============================================================================
// Url struct
// ============================================================================

/// A parsed URL with all components separated.
///
/// All string fields store the values as they appear in the URL (not decoded).
/// The `raw` field preserves the original input for round-tripping.
///
/// ## Invariants
///
/// - `scheme` is always lowercased
/// - `host` is always lowercased (when present)
/// - `path` starts with `/` for authority-based URLs (http, ftp)
/// - `query` does NOT include the leading `?`
/// - `fragment` does NOT include the leading `#`
///
/// ## Example
///
/// ```swift
/// let url = try Url.parse("http://www.example.com:8080/docs?q=1#s2")
/// // url.scheme   == "http"
/// // url.host     == "www.example.com"
/// // url.port     == 8080
/// // url.path     == "/docs"
/// // url.query    == "q=1"
/// // url.fragment == "s2"
/// ```
public struct Url: Equatable, CustomStringConvertible {
    /// The scheme (protocol), lowercased. Examples: "http", "ftp", "mailto".
    public let scheme: String

    /// Optional userinfo before the `@` in the authority. Example: "alice:secret".
    public let userinfo: String?

    /// Optional host, lowercased. Example: "www.example.com".
    /// `nil` for schemes like `mailto:` that have no authority.
    public let host: String?

    /// Optional explicit port number. `nil` means use the scheme default.
    public let port: UInt16?

    /// The path component. Always starts with `/` for HTTP URLs.
    public let path: String

    /// Optional query string, without the leading `?`.
    public let query: String?

    /// Optional fragment identifier, without the leading `#`.
    public let fragment: String?

    /// The original input string, preserved verbatim.
    private let raw: String

    // ── Internal initializer ───────────────────────────────────────────────
    // We keep `init` internal so that users go through `parse()`.

    init(
        scheme: String,
        userinfo: String?,
        host: String?,
        port: UInt16?,
        path: String,
        query: String?,
        fragment: String?,
        raw: String
    ) {
        self.scheme = scheme
        self.userinfo = userinfo
        self.host = host
        self.port = port
        self.path = path
        self.query = query
        self.fragment = fragment
        self.raw = raw
    }

    // ════════════════════════════════════════════════════════════════════════
    // Parsing
    // ════════════════════════════════════════════════════════════════════════

    /// Parse an absolute URL string.
    ///
    /// The input must contain a scheme (e.g., "http://..."). For relative URLs,
    /// first parse the base URL, then call `resolve(_:)`.
    ///
    /// ## Algorithm — single-pass, left-to-right
    ///
    /// ```text
    /// "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2"
    ///  ^^^^                                                              ^^^^
    ///  Step 1: scheme = "http"                            Step 2: fragment = "sec2"
    ///                                                   ^^^^^^^^
    ///                                           Step 3: query = "q=hello"
    ///                                    ^^^^^^^^^^^^^^^
    ///                            Step 4: path = "/docs/page.html"
    ///        ^^^^^^^^^^^^
    ///    Step 5: userinfo = "alice:secret"
    ///                                ^^^^
    ///                    Step 6: port = 8080
    ///                       ^^^^^^^^^^^^^^^
    ///               Step 7: host = "www.example.com"
    /// ```
    ///
    /// - Throws: `UrlError` if the URL is malformed.
    public static func parse(_ input: String) throws -> Url {
        let raw = input
        let trimmed = input.trimmingCharacters(in: .whitespaces)

        // Step 1: Extract scheme by finding "://"
        //
        // Two forms are supported:
        //   - "scheme://authority/path" — full URL with authority
        //   - "scheme:path" — opaque URL like "mailto:alice@example.com"
        if let schemeEnd = trimmed.range(of: "://") {
            let scheme = String(trimmed[trimmed.startIndex..<schemeEnd.lowerBound]).lowercased()
            try validateScheme(scheme)
            let afterScheme = String(trimmed[schemeEnd.upperBound...])

            // Step 2: Extract fragment (split at first "#")
            let (afterFrag, fragment) = splitFragment(afterScheme)

            // Step 3: Extract query (split at first "?")
            let (afterQuery, query) = splitQuery(afterFrag)

            // Step 4: Split authority from path (find first "/")
            let (authorityStr, path): (String, String)
            if let slashIdx = afterQuery.firstIndex(of: "/") {
                authorityStr = String(afterQuery[afterQuery.startIndex..<slashIdx])
                path = String(afterQuery[slashIdx...])
            } else {
                authorityStr = afterQuery
                path = "/"
            }

            // Step 5: Extract userinfo (find "@" in authority)
            //
            // The `@` delimiter separates "user:password" from "host:port".
            // We search from the right so that userinfo can itself contain `@`.
            let (userinfo, hostPort): (String?, String)
            if let atIdx = authorityStr.lastIndex(of: "@") {
                userinfo = String(authorityStr[authorityStr.startIndex..<atIdx])
                hostPort = String(authorityStr[authorityStr.index(after: atIdx)...])
            } else {
                userinfo = nil
                hostPort = authorityStr
            }

            // Steps 6 & 7: Extract port and host
            //
            // IPv6 addresses are enclosed in brackets: [::1]:8080
            // For IPv6, the port delimiter is the ":" AFTER the closing "]"
            let (hostStr, portVal) = try parseHostPort(hostPort)

            let host: String? = hostStr.isEmpty ? nil : hostStr.lowercased()

            return Url(
                scheme: scheme,
                userinfo: userinfo,
                host: host,
                port: portVal,
                path: path,
                query: query,
                fragment: fragment,
                raw: raw
            )
        } else {
            // Try "scheme:path" form (e.g., "mailto:alice@example.com")
            //
            // We look for the first ":" that isn't preceded by a "/", which
            // would indicate a path rather than a scheme separator.
            if let colonIdx = trimmed.firstIndex(of: ":") {
                let beforeColon = String(trimmed[trimmed.startIndex..<colonIdx])
                if !beforeColon.isEmpty && !beforeColon.contains("/") {
                    let scheme = beforeColon.lowercased()
                    try validateScheme(scheme)
                    let afterColon = String(trimmed[trimmed.index(after: colonIdx)...])

                    // Still split fragment and query from the path portion
                    let (pathAfterFrag, fragment) = splitFragment(afterColon)
                    let (pathFinal, query) = splitQuery(pathAfterFrag)

                    return Url(
                        scheme: scheme,
                        userinfo: nil,
                        host: nil,
                        port: nil,
                        path: pathFinal,
                        query: query,
                        fragment: fragment,
                        raw: raw
                    )
                }
            }

            throw UrlError.missingScheme
        }
    }

    // ════════════════════════════════════════════════════════════════════════
    // Relative resolution
    // ════════════════════════════════════════════════════════════════════════

    /// Resolve a relative URL against this URL as the base.
    ///
    /// Implements the RFC 1808 relative resolution algorithm:
    ///
    /// ```text
    /// if R has scheme     → R is absolute, return as-is
    /// if R starts with // → inherit scheme only
    /// if R starts with /  → inherit scheme + authority, replace path
    /// otherwise           → merge paths, resolve . and ..
    /// ```
    ///
    /// ## Examples
    ///
    /// ```swift
    /// let base = try Url.parse("http://www.example.com/a/b/c.html")
    ///
    /// // Same directory
    /// let r1 = try base.resolve("d.html")       // path: "/a/b/d.html"
    ///
    /// // Parent directory
    /// let r2 = try base.resolve("../d.html")     // path: "/a/d.html"
    ///
    /// // Absolute path
    /// let r3 = try base.resolve("/x/y.html")     // path: "/x/y.html"
    /// ```
    public func resolve(_ relative: String) throws -> Url {
        let rel = relative.trimmingCharacters(in: .whitespaces)

        // Empty relative → return base without fragment
        if rel.isEmpty {
            return Url(
                scheme: scheme,
                userinfo: userinfo,
                host: host,
                port: port,
                path: path,
                query: query,
                fragment: nil,
                raw: toUrlString()
            )
        }

        // Fragment-only: "#section"
        if rel.hasPrefix("#") {
            let frag = String(rel.dropFirst())
            let result = Url(
                scheme: scheme,
                userinfo: userinfo,
                host: host,
                port: port,
                path: path,
                query: query,
                fragment: frag,
                raw: ""
            )
            return Url(
                scheme: result.scheme,
                userinfo: result.userinfo,
                host: result.host,
                port: result.port,
                path: result.path,
                query: result.query,
                fragment: result.fragment,
                raw: result.toUrlString()
            )
        }

        // If R has a scheme, it's already absolute
        //
        // We check for "://" first, then check for a "scheme:" pattern
        // where the part before ":" looks like a valid scheme identifier.
        if rel.contains("://") || (rel.contains(":") && !rel.hasPrefix("/")) {
            if let colonIdx = rel.firstIndex(of: ":") {
                let maybeScheme = String(rel[rel.startIndex..<colonIdx])
                if !maybeScheme.isEmpty
                    && maybeScheme.allSatisfy({ $0.isASCIIAlphanumeric || $0 == "+" || $0 == "-" || $0 == "." })
                    && maybeScheme.first!.isASCIIAlpha
                {
                    return try Url.parse(rel)
                }
            }
        }

        // Scheme-relative: "//host/path"
        if rel.hasPrefix("//") {
            let full = "\(scheme):\(rel)"
            return try Url.parse(full)
        }

        // Absolute path: "/path"
        if rel.hasPrefix("/") {
            let (pathPart, fragment) = splitFragment(rel)
            let (pathFinal, query) = splitQuery(pathPart)
            let result = Url(
                scheme: scheme,
                userinfo: userinfo,
                host: host,
                port: port,
                path: removeDotSegments(pathFinal),
                query: query,
                fragment: fragment,
                raw: ""
            )
            return Url(
                scheme: result.scheme,
                userinfo: result.userinfo,
                host: result.host,
                port: result.port,
                path: result.path,
                query: result.query,
                fragment: result.fragment,
                raw: result.toUrlString()
            )
        }

        // Relative path: merge with base
        //
        // Take the base path up to the last "/", then append the relative path.
        // After merging, remove "." and ".." segments.
        let (relativePath, fragment) = splitFragment(rel)
        let (relativePathFinal, queryPart) = splitQuery(relativePath)

        let merged = mergePaths(basePath: path, relativePath: relativePathFinal)
        let resolvedPath = removeDotSegments(merged)

        let result = Url(
            scheme: scheme,
            userinfo: userinfo,
            host: host,
            port: port,
            path: resolvedPath,
            query: queryPart,
            fragment: fragment,
            raw: ""
        )
        return Url(
            scheme: result.scheme,
            userinfo: result.userinfo,
            host: result.host,
            port: result.port,
            path: result.path,
            query: result.query,
            fragment: result.fragment,
            raw: result.toUrlString()
        )
    }

    // ════════════════════════════════════════════════════════════════════════
    // Accessors
    // ════════════════════════════════════════════════════════════════════════

    /// The effective port — explicit port if set, otherwise the scheme default.
    ///
    /// | Scheme | Default Port |
    /// |--------|-------------|
    /// | http   | 80          |
    /// | https  | 443         |
    /// | ftp    | 21          |
    ///
    /// Returns `nil` for unknown schemes without an explicit port.
    public func effectivePort() -> UInt16? {
        return port ?? defaultPort(for: scheme)
    }

    /// The authority string: `[userinfo@]host[:port]`
    ///
    /// This reconstructs the authority portion of the URL from its components.
    /// For example, `"user:pass@host.com:8080"`.
    public func authority() -> String {
        var auth = ""
        if let ui = userinfo {
            auth += ui
            auth += "@"
        }
        if let h = host {
            auth += h
        }
        if let p = port {
            auth += ":\(p)"
        }
        return auth
    }

    /// Serialize back to a URL string.
    ///
    /// Reconstructs the URL from its parsed components. For authority-based
    /// URLs (those with a host), uses the `://` separator. For opaque URLs
    /// like `mailto:`, uses just `:`.
    public func toUrlString() -> String {
        var s = scheme

        if host != nil {
            s += "://"
            s += authority()
        } else {
            s += ":"
        }

        s += path

        if let q = query {
            s += "?\(q)"
        }
        if let f = fragment {
            s += "#\(f)"
        }
        return s
    }

    /// CustomStringConvertible conformance — delegates to `toUrlString()`.
    public var description: String {
        return toUrlString()
    }
}

// ============================================================================
// Percent-encoding / decoding
// ============================================================================

/// Characters that do NOT need percent-encoding in a URL path.
///
/// RFC 1738 unreserved characters: `A-Z a-z 0-9 - _ . ~`
/// Plus the path separator: `/`
///
/// Everything else gets encoded as `%XX` where XX is the uppercase hex value
/// of each UTF-8 byte.
private func isUnreserved(_ byte: UInt8) -> Bool {
    // ASCII alphanumeric
    if byte >= 0x41 && byte <= 0x5A { return true } // A-Z
    if byte >= 0x61 && byte <= 0x7A { return true } // a-z
    if byte >= 0x30 && byte <= 0x39 { return true } // 0-9
    // Special unreserved characters
    switch byte {
    case 0x2D, 0x5F, 0x2E, 0x7E, 0x2F: // - _ . ~ /
        return true
    default:
        return false
    }
}

/// Percent-encode a string for use in a URL path or query.
///
/// Encodes all characters except unreserved ones (`A-Z a-z 0-9 - _ . ~ /`).
/// Each non-safe byte is replaced with `%XX` using uppercase hex.
///
/// ## How it works
///
/// The string is converted to its UTF-8 byte representation. Each byte is
/// checked: if it's "unreserved" (safe), it passes through as-is. Otherwise,
/// it becomes a percent-encoded triplet. Multi-byte UTF-8 characters like
/// `日` (3 bytes: E6 97 A5) produce multiple triplets: `%E6%97%A5`.
///
/// ## Example
///
/// ```swift
/// percentEncode("hello world")     // "hello%20world"
/// percentEncode("/path/to/file")   // "/path/to/file"  (slashes preserved)
/// ```
public func percentEncode(_ input: String) -> String {
    var result = ""
    result.reserveCapacity(input.count)

    for byte in input.utf8 {
        if isUnreserved(byte) {
            result.append(Character(UnicodeScalar(byte)))
        } else {
            // Format as %XX with uppercase hex digits
            result.append("%")
            result.append(hexChar(byte >> 4))
            result.append(hexChar(byte & 0x0F))
        }
    }
    return result
}

/// Percent-decode a string: `"%20"` → `" "`, `"%E6%97%A5"` → `"日"`.
///
/// Each `%XX` sequence is replaced by the byte with that hex value. The
/// resulting bytes are interpreted as UTF-8. If the percent-encoding is
/// malformed (truncated, bad hex digit, or invalid UTF-8), an error is thrown.
///
/// ## Example
///
/// ```swift
/// try percentDecode("hello%20world")  // "hello world"
/// try percentDecode("%E6%97%A5")      // "日"
/// ```
public func percentDecode(_ input: String) throws -> String {
    let bytes = Array(input.utf8)
    var result: [UInt8] = []
    result.reserveCapacity(bytes.count)
    var i = 0

    while i < bytes.count {
        if bytes[i] == 0x25 { // '%'
            // Need at least 2 more hex digits after the '%'
            guard i + 2 < bytes.count else {
                throw UrlError.invalidPercentEncoding
            }
            let hi = try hexDigitValue(bytes[i + 1])
            let lo = try hexDigitValue(bytes[i + 2])
            result.append((hi << 4) | lo)
            i += 3
        } else {
            result.append(bytes[i])
            i += 1
        }
    }

    // Interpret the decoded bytes as UTF-8
    guard let decoded = String(bytes: result, encoding: .utf8) else {
        throw UrlError.invalidPercentEncoding
    }
    return decoded
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Convert a hex ASCII byte to its numeric value (0–15).
///
/// ```text
/// '0'–'9' → 0–9
/// 'a'–'f' → 10–15
/// 'A'–'F' → 10–15
/// anything else → error
/// ```
private func hexDigitValue(_ b: UInt8) throws -> UInt8 {
    switch b {
    case 0x30...0x39: return b - 0x30       // '0'–'9'
    case 0x61...0x66: return b - 0x61 + 10  // 'a'–'f'
    case 0x41...0x46: return b - 0x41 + 10  // 'A'–'F'
    default: throw UrlError.invalidPercentEncoding
    }
}

/// Convert a nibble (0–15) to its uppercase hex character.
private func hexChar(_ nibble: UInt8) -> Character {
    let chars: [Character] = [
        "0", "1", "2", "3", "4", "5", "6", "7",
        "8", "9", "A", "B", "C", "D", "E", "F"
    ]
    return chars[Int(nibble & 0x0F)]
}

/// Validate that a scheme matches `[a-z][a-z0-9+.-]*`.
///
/// The scheme is the "protocol" part of the URL. It must:
/// - Start with a lowercase ASCII letter
/// - Contain only lowercase letters, digits, `+`, `-`, `.`
///
/// Examples of valid schemes: `http`, `https`, `ftp`, `mailto`, `svn+ssh`
private func validateScheme(_ scheme: String) throws {
    guard !scheme.isEmpty else {
        throw UrlError.invalidScheme
    }

    guard let first = scheme.first, first.isASCIIAlpha else {
        throw UrlError.invalidScheme
    }

    for ch in scheme {
        if !ch.isASCIIAlpha && !ch.isASCIIDigit && ch != "+" && ch != "-" && ch != "." {
            throw UrlError.invalidScheme
        }
    }
}

/// Parse a port string to UInt16. Returns `UrlError.invalidPort` if out of range.
private func parsePort(_ s: String) throws -> UInt16 {
    guard let port = UInt16(s) else {
        throw UrlError.invalidPort
    }
    return port
}

/// Return the default port for a scheme, if known.
///
/// | Scheme | Default Port |
/// |--------|-------------|
/// | http   | 80          |
/// | https  | 443         |
/// | ftp    | 21          |
private func defaultPort(for scheme: String) -> UInt16? {
    switch scheme {
    case "http":  return 80
    case "https": return 443
    case "ftp":   return 21
    default:      return nil
    }
}

/// Split a string at the first `#`, returning (before, after) or (input, nil).
///
/// The fragment is the part after `#` — it's a client-side anchor that is
/// never sent to the server.
private func splitFragment(_ input: String) -> (String, String?) {
    if let idx = input.firstIndex(of: "#") {
        return (String(input[input.startIndex..<idx]), String(input[input.index(after: idx)...]))
    }
    return (input, nil)
}

/// Split a string at the first `?`, returning (before, after) or (input, nil).
///
/// The query is the part after `?` — it carries key=value parameters.
private func splitQuery(_ input: String) -> (String, String?) {
    if let idx = input.firstIndex(of: "?") {
        return (String(input[input.startIndex..<idx]), String(input[input.index(after: idx)...]))
    }
    return (input, nil)
}

/// Parse the host:port portion of the authority.
///
/// Handles two cases:
///
/// 1. **IPv6**: `[::1]:8080` — the host is everything inside the brackets
///    (inclusive), and the port follows the closing bracket with a `:`.
///
/// 2. **IPv4 / hostname**: `example.com:8080` — the last `:` separates
///    the host from the port, but only if the part after `:` is all digits.
///    This prevents treating `host:name` as having a port.
private func parseHostPort(_ input: String) throws -> (String, UInt16?) {
    if input.hasPrefix("[") {
        // IPv6: find closing bracket
        if let bracketIdx = input.firstIndex(of: "]") {
            let host = String(input[input.startIndex...bracketIdx])
            let afterBracket = String(input[input.index(after: bracketIdx)...])
            if afterBracket.hasPrefix(":") {
                let portStr = String(afterBracket.dropFirst())
                let port = try parsePort(portStr)
                return (host, port)
            }
            return (host, nil)
        }
        // Malformed IPv6: treat the whole thing as host
        return (input, nil)
    }

    // IPv4 or hostname: last ":" separates host from port
    if let colonIdx = input.lastIndex(of: ":") {
        let maybePort = String(input[input.index(after: colonIdx)...])
        // Only treat as port if non-empty and all digits
        if !maybePort.isEmpty && maybePort.allSatisfy({ $0.isASCIIDigit }) {
            let host = String(input[input.startIndex..<colonIdx])
            let port = try parsePort(maybePort)
            return (host, port)
        }
    }

    return (input, nil)
}

/// Merge a base path and a relative path.
///
/// Takes everything in `basePath` up to and including the last `/`,
/// then appends `relativePath`.
///
/// ```text
/// merge("/a/b/c", "d")   → "/a/b/d"
/// merge("/a/b/",  "d")   → "/a/b/d"
/// merge("/a",     "d")   → "/d"
/// ```
private func mergePaths(basePath: String, relativePath: String) -> String {
    if let slashIdx = basePath.lastIndex(of: "/") {
        return String(basePath[basePath.startIndex...slashIdx]) + relativePath
    }
    return "/" + relativePath
}

/// Remove `.` and `..` segments from a path.
///
/// Implements the "remove dot segments" algorithm from RFC 3986 section 5.2.4:
///
/// ```text
/// /a/b/../c      → /a/c        (go up one level)
/// /a/./b         → /a/b        (current directory = no-op)
/// /a/b/../../c   → /c          (go up two levels)
/// /a/../../../c  → /c          (can't go above root)
/// ```
///
/// The algorithm uses a stack: for each segment, `.` is skipped, `..` pops
/// the stack, and everything else is pushed. At the end, the stack is joined
/// with `/` to produce the cleaned path.
private func removeDotSegments(_ path: String) -> String {
    var outputSegments: [String] = []

    for segment in path.split(separator: "/", omittingEmptySubsequences: false) {
        let seg = String(segment)
        switch seg {
        case ".":
            // Skip — "current directory" is a no-op
            break
        case "..":
            // Go up one level — remove the last segment (if any)
            if !outputSegments.isEmpty {
                outputSegments.removeLast()
            }
        default:
            outputSegments.append(seg)
        }
    }

    let result = outputSegments.joined(separator: "/")
    // Ensure the path starts with "/" if the input did
    if path.hasPrefix("/") && !result.hasPrefix("/") {
        return "/" + result
    }
    return result
}

// ============================================================================
// Character classification helpers
// ============================================================================
//
// Swift's Character type doesn't have built-in ASCII classification methods,
// so we add them as extensions. These are intentionally narrow — they only
// match the ASCII range, not Unicode letters/digits.

extension Character {
    /// Is this an ASCII letter (a-z, A-Z)?
    var isASCIIAlpha: Bool {
        return ("a"..."z").contains(self) || ("A"..."Z").contains(self)
    }

    /// Is this an ASCII digit (0-9)?
    var isASCIIDigit: Bool {
        return ("0"..."9").contains(self)
    }

    /// Is this an ASCII alphanumeric character?
    var isASCIIAlphanumeric: Bool {
        return isASCIIAlpha || isASCIIDigit
    }
}
