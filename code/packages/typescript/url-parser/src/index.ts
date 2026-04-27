/**
 * @coding-adventures/url-parser
 *
 * RFC 1738 URL parser with relative resolution and percent-encoding.
 *
 * A URL (Uniform Resource Locator) tells you **where** something is on the
 * internet and **how** to get it. This module parses URLs into their component
 * parts, resolves relative URLs against a base, and handles percent-encoding.
 *
 * ## URL anatomy
 *
 * ```text
 *   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
 *   └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
 *  scheme  userinfo        host       port     path         query   fragment
 * ```
 *
 * - **scheme**: how to deliver (http, ftp, mailto)
 * - **host**: which server (www.example.com)
 * - **port**: which door (8080; defaults to 80 for http)
 * - **path**: which resource (/docs/page.html)
 * - **query**: parameters (?q=hello)
 * - **fragment**: client-side anchor (#section2) -- never sent to server
 * - **userinfo**: credentials (rare today, common in early web)
 *
 * ## Parsing algorithm
 *
 * The URL is parsed left-to-right in a single pass, no backtracking:
 *
 * 1. Find `://` -> extract scheme (lowercased)
 * 2. Find `#` from right -> extract fragment
 * 3. Find `?` -> extract query
 * 4. Find first `/` -> extract path
 * 5. Find `@` -> extract userinfo
 * 6. Find last `:` -> extract port
 * 7. Remainder -> host (lowercased)
 */

export const VERSION = "0.1.0";

// ============================================================================
// Error hierarchy
// ============================================================================
//
// Each error type is a distinct class so callers can use `instanceof` to
// catch specific failure modes. All inherit from UrlError, which itself
// extends the built-in Error.

/**
 * Base class for all URL parsing errors.
 */
export class UrlError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UrlError";
  }
}

/**
 * Thrown when the input has no scheme (e.g., "www.example.com" without "http://").
 */
export class MissingScheme extends UrlError {
  constructor() {
    super("missing scheme (expected '://')");
    this.name = "MissingScheme";
  }
}

/**
 * Thrown when the scheme contains invalid characters.
 * A valid scheme matches: `[a-z][a-z0-9+.-]*`
 */
export class InvalidScheme extends UrlError {
  constructor() {
    super("invalid scheme (must be [a-z][a-z0-9+.-]*)");
    this.name = "InvalidScheme";
  }
}

/**
 * Thrown when the port is not a valid number in range 0-65535.
 */
export class InvalidPort extends UrlError {
  constructor() {
    super("invalid port (must be 0-65535)");
    this.name = "InvalidPort";
  }
}

/**
 * Thrown when percent-encoding is malformed (e.g., "%GG", "%2" truncated).
 */
export class InvalidPercentEncoding extends UrlError {
  constructor() {
    super("malformed percent-encoding");
    this.name = "InvalidPercentEncoding";
  }
}

/**
 * Thrown when the host is empty in an authority-based URL ("http:///path").
 */
export class EmptyHost extends UrlError {
  constructor() {
    super("empty host in authority-based URL");
    this.name = "EmptyHost";
  }
}

/**
 * Thrown when a relative URL cannot be resolved without a base.
 */
export class RelativeWithoutBase extends UrlError {
  constructor() {
    super("relative URL requires a base URL");
    this.name = "RelativeWithoutBase";
  }
}

// ============================================================================
// Internal helpers
// ============================================================================

/**
 * Validate that a scheme matches `[a-z][a-z0-9+.-]*`.
 *
 * The scheme is the first thing in a URL and tells the client *how* to
 * retrieve the resource. "http" means use the HTTP protocol, "ftp" means
 * use FTP, "mailto" means compose an email.
 *
 * The rules are strict: must start with a letter, then letters, digits,
 * plus, hyphen, or dot. No spaces, no underscores, no special characters.
 */
function validateScheme(scheme: string): void {
  if (scheme.length === 0) {
    throw new InvalidScheme();
  }
  const first = scheme[0];
  if (first < "a" || first > "z") {
    throw new InvalidScheme();
  }
  for (let i = 1; i < scheme.length; i++) {
    const c = scheme[i];
    const isLower = c >= "a" && c <= "z";
    const isDigit = c >= "0" && c <= "9";
    const isSpecial = c === "+" || c === "-" || c === ".";
    if (!isLower && !isDigit && !isSpecial) {
      throw new InvalidScheme();
    }
  }
}

/**
 * Parse a port string to a number in the range 0-65535.
 *
 * Port numbers identify which "door" on the server to knock on. Well-known
 * ports: 80 (HTTP), 443 (HTTPS), 21 (FTP). The maximum is 65535 because
 * ports are stored as 16-bit unsigned integers in TCP/UDP headers.
 */
function parsePort(s: string): number {
  const n = parseInt(s, 10);
  if (isNaN(n) || n < 0 || n > 65535 || s !== String(n)) {
    throw new InvalidPort();
  }
  return n;
}

/**
 * Return the default port for well-known schemes.
 *
 * | Scheme | Default Port | Why                              |
 * |--------|-------------|----------------------------------|
 * | http   | 80          | Tim Berners-Lee chose it in 1991 |
 * | https  | 443         | Assigned by IANA for TLS-HTTP    |
 * | ftp    | 21          | One of the oldest protocols      |
 */
function defaultPort(scheme: string): number | null {
  switch (scheme) {
    case "http":
      return 80;
    case "https":
      return 443;
    case "ftp":
      return 21;
    default:
      return null;
  }
}

/**
 * Split a string at the first `#`, returning [before, after] or [input, null].
 *
 * The fragment (everything after `#`) is a client-side locator. It is never
 * sent to the server -- the browser uses it to scroll to an anchor on the page.
 */
function splitFragment(input: string): [string, string | null] {
  const pos = input.indexOf("#");
  if (pos !== -1) {
    return [input.slice(0, pos), input.slice(pos + 1)];
  }
  return [input, null];
}

/**
 * Split a string at the first `?`, returning [before, after] or [input, null].
 *
 * The query string carries key=value pairs that parameterize the request.
 * For example, `?q=hello&lang=en` asks the server to search for "hello"
 * in English.
 */
function splitQuery(input: string): [string, string | null] {
  const pos = input.indexOf("?");
  if (pos !== -1) {
    return [input.slice(0, pos), input.slice(pos + 1)];
  }
  return [input, null];
}

/**
 * Merge a base path and a relative path.
 *
 * Takes everything in `basePath` up to and including the last `/`,
 * then appends `relativePath`.
 *
 * ```text
 * merge("/a/b/c", "d")   -> "/a/b/d"
 * merge("/a/b/",  "d")   -> "/a/b/d"
 * merge("/a",     "d")   -> "/d"
 * ```
 *
 * Think of it like a filesystem: if you're viewing `/a/b/c.html` and click
 * a link to `d.html`, the browser navigates to `/a/b/d.html` -- same
 * directory, different file.
 */
function mergePaths(basePath: string, relativePath: string): string {
  const lastSlash = basePath.lastIndexOf("/");
  if (lastSlash !== -1) {
    return basePath.slice(0, lastSlash + 1) + relativePath;
  }
  return "/" + relativePath;
}

/**
 * Remove `.` and `..` segments from a path.
 *
 * Implements the "remove dot segments" algorithm from RFC 3986 S5.2.4:
 *
 * ```text
 * /a/b/../c     -> /a/c       (.. goes up one level)
 * /a/./b        -> /a/b       (. means "current directory")
 * /a/b/../../c  -> /c         (two levels up)
 * /a/../../../c -> /c         (can't go above root)
 * ```
 *
 * This is analogous to `cd` in a terminal: `cd /a/b/../c` ends up at `/a/c`
 * because `..` cancels out the `b` directory.
 */
function removeDotSegments(path: string): string {
  const segments = path.split("/");
  const output: string[] = [];

  for (const segment of segments) {
    if (segment === ".") {
      // Skip -- "current directory" is a no-op
      continue;
    } else if (segment === "..") {
      // Go up one level -- remove the last segment (if any)
      output.pop();
    } else {
      output.push(segment);
    }
  }

  const result = output.join("/");
  // Ensure the path starts with "/" if the input did
  if (path.startsWith("/") && !result.startsWith("/")) {
    return "/" + result;
  }
  return result;
}

/**
 * Convert a single hex character to its numeric value (0-15).
 *
 * Hex digits extend decimal (0-9) with letters (A-F) to represent values
 * 10-15. This gives us 16 values per digit -- perfect for encoding a
 * nibble (half a byte). Two hex digits encode one full byte (0-255).
 */
function hexDigit(charCode: number): number {
  // 0-9
  if (charCode >= 0x30 && charCode <= 0x39) return charCode - 0x30;
  // a-f
  if (charCode >= 0x61 && charCode <= 0x66) return charCode - 0x61 + 10;
  // A-F
  if (charCode >= 0x41 && charCode <= 0x46) return charCode - 0x41 + 10;
  throw new InvalidPercentEncoding();
}

/**
 * Check if a byte value is an "unreserved" character that does NOT need
 * percent-encoding.
 *
 * RFC 1738 unreserved characters: `A-Z a-z 0-9 - _ . ~`
 * Plus path-safe character: `/`
 *
 * These characters have no special meaning in URLs and can appear literally.
 * Everything else (spaces, non-ASCII, special chars) must be encoded as
 * `%XX` where XX is the hex value of each byte.
 */
function isUnreserved(byte: number): boolean {
  // A-Z
  if (byte >= 0x41 && byte <= 0x5a) return true;
  // a-z
  if (byte >= 0x61 && byte <= 0x7a) return true;
  // 0-9
  if (byte >= 0x30 && byte <= 0x39) return true;
  // - _ . ~ /
  if (
    byte === 0x2d ||
    byte === 0x5f ||
    byte === 0x2e ||
    byte === 0x7e ||
    byte === 0x2f
  ) {
    return true;
  }
  return false;
}

// ============================================================================
// Percent-encoding / decoding
// ============================================================================

/**
 * Percent-encode a string for use in a URL path or query.
 *
 * Encodes all characters except unreserved ones (`A-Z a-z 0-9 - _ . ~ /`).
 * Each byte of a non-ASCII character (like a Japanese kanji) is encoded
 * separately, producing multiple `%XX` sequences.
 *
 * ## How it works
 *
 * 1. Convert the string to UTF-8 bytes
 * 2. For each byte:
 *    - If it's unreserved (safe), emit it as-is
 *    - Otherwise, emit `%` followed by two uppercase hex digits
 *
 * ## Examples
 *
 * ```
 * percentEncode("hello world")  // "hello%20world"  (space = byte 0x20)
 * percentEncode("/path/to/file") // "/path/to/file"  (slashes are unreserved)
 * ```
 */
export function percentEncode(input: string): string {
  // Use TextEncoder to get UTF-8 bytes, since JavaScript strings are UTF-16
  const encoder = new TextEncoder();
  const bytes = encoder.encode(input);
  let result = "";

  for (const byte of bytes) {
    if (isUnreserved(byte)) {
      result += String.fromCharCode(byte);
    } else {
      // Format as %XX with uppercase hex
      result += "%" + byte.toString(16).toUpperCase().padStart(2, "0");
    }
  }

  return result;
}

/**
 * Percent-decode a string: `"%20"` -> `" "`, `"%E6%97%A5"` -> `"日"`.
 *
 * Each `%XX` sequence is replaced by the byte with that hex value. The
 * resulting bytes are interpreted as UTF-8.
 *
 * ## How it works
 *
 * 1. Scan through the string character by character
 * 2. When we see `%`, read the next two characters as hex digits
 * 3. Convert to a byte value and accumulate
 * 4. At the end, decode the byte array as UTF-8
 *
 * ## Examples
 *
 * ```
 * percentDecode("hello%20world")  // "hello world"
 * percentDecode("%E6%97%A5")      // "日" (3-byte UTF-8 sequence)
 * ```
 */
export function percentDecode(input: string): string {
  const bytes: number[] = [];
  let i = 0;

  while (i < input.length) {
    if (input[i] === "%") {
      // Need at least 2 more hex digits after the %
      if (i + 2 >= input.length) {
        throw new InvalidPercentEncoding();
      }
      const hi = hexDigit(input.charCodeAt(i + 1));
      const lo = hexDigit(input.charCodeAt(i + 2));
      bytes.push((hi << 4) | lo);
      i += 3;
    } else {
      bytes.push(input.charCodeAt(i));
      i += 1;
    }
  }

  // Decode the accumulated bytes as UTF-8
  const decoder = new TextDecoder("utf-8", { fatal: true });
  try {
    return decoder.decode(new Uint8Array(bytes));
  } catch {
    throw new InvalidPercentEncoding();
  }
}

// ============================================================================
// Url class
// ============================================================================

/**
 * A parsed URL with all components separated.
 *
 * All string fields store the values as they appear in the URL. The `raw`
 * field preserves the original input for debugging.
 *
 * ## Invariants
 *
 * - `scheme` is always lowercased
 * - `host` is always lowercased (when present)
 * - `path` starts with `/` for authority-based URLs (http, ftp)
 * - `query` does NOT include the leading `?`
 * - `fragment` does NOT include the leading `#`
 *
 * ## Example
 *
 * ```typescript
 * const url = Url.parse("http://www.example.com:8080/docs/page.html?q=hello#s2");
 * // url.scheme   === "http"
 * // url.host     === "www.example.com"
 * // url.port     === 8080
 * // url.path     === "/docs/page.html"
 * // url.query    === "q=hello"
 * // url.fragment === "s2"
 * ```
 */
export class Url {
  /** The scheme (protocol), lowercased. Examples: "http", "ftp", "mailto". */
  scheme: string;
  /** Optional userinfo before the `@` in the authority. Example: "alice:secret". */
  userinfo: string | null;
  /** Optional host, lowercased. Example: "www.example.com". */
  host: string | null;
  /** Optional explicit port number. `null` means use the scheme default. */
  port: number | null;
  /** The path component. Always starts with `/` for HTTP URLs. */
  path: string;
  /** Optional query string, without the leading `?`. */
  query: string | null;
  /** Optional fragment identifier, without the leading `#`. */
  fragment: string | null;
  /** The original input string, preserved verbatim. */
  private raw: string;

  private constructor(
    scheme: string,
    userinfo: string | null,
    host: string | null,
    port: number | null,
    path: string,
    query: string | null,
    fragment: string | null,
    raw: string,
  ) {
    this.scheme = scheme;
    this.userinfo = userinfo;
    this.host = host;
    this.port = port;
    this.path = path;
    this.query = query;
    this.fragment = fragment;
    this.raw = raw;
  }

  /**
   * Parse an absolute URL string.
   *
   * The input must contain a scheme (e.g., "http://..."). For relative URLs,
   * first parse the base URL, then call `resolve()`.
   *
   * ## Algorithm
   *
   * Single-pass, left-to-right:
   *
   * ```text
   * "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2"
   *  ^^^^                                                              ^^^^
   *  Step 1: scheme = "http"                            Step 2: fragment = "sec2"
   *                                                   ^^^^^^^^
   *                                           Step 3: query = "q=hello"
   *                                    ^^^^^^^^^^^^^^^
   *                            Step 4: path = "/docs/page.html"
   *        ^^^^^^^^^^^^
   *    Step 5: userinfo = "alice:secret"
   *                                ^^^^
   *                    Step 6: port = 8080
   *                       ^^^^^^^^^^^^^^^
   *               Step 7: host = "www.example.com"
   * ```
   */
  static parse(input: string): Url {
    const raw = input;
    const trimmed = input.trim();

    // Step 1: Extract scheme by finding "://"
    //
    // The "://" delimiter separates the scheme from the authority. This is
    // one of Tim Berners-Lee's design decisions from 1991 that he later
    // said he regretted -- the double slash was borrowed from Apollo Domain
    // filesystem paths and is technically redundant.
    const schemeDelim = trimmed.indexOf("://");

    if (schemeDelim !== -1) {
      const scheme = trimmed.slice(0, schemeDelim).toLowerCase();
      validateScheme(scheme);
      let afterScheme = trimmed.slice(schemeDelim + 3);

      // Step 2: Extract fragment (find "#")
      //
      // We extract fragment first because `#` can appear in query strings
      // in some edge cases, but the spec says the first `#` wins.
      let fragment: string | null;
      [afterScheme, fragment] = splitFragment(afterScheme);

      // Step 3: Extract query (find "?")
      let query: string | null;
      [afterScheme, query] = splitQuery(afterScheme);

      // Step 4: Split authority from path (find first "/")
      //
      // Everything before the first "/" is the authority (host, port, userinfo).
      // Everything from the first "/" onward is the path.
      // If there's no "/", the path defaults to "/" (the root resource).
      let authorityStr: string;
      let path: string;
      const slashPos = afterScheme.indexOf("/");
      if (slashPos !== -1) {
        authorityStr = afterScheme.slice(0, slashPos);
        path = afterScheme.slice(slashPos);
      } else {
        authorityStr = afterScheme;
        path = "/";
      }

      // Step 5: Extract userinfo (find "@" in authority)
      //
      // Userinfo was common in the early web: `ftp://anonymous@ftp.example.com`.
      // Today it's mostly used for database connection strings and is considered
      // a security risk in HTTP URLs (browsers warn about it).
      let userinfo: string | null = null;
      let hostPort: string;
      const atPos = authorityStr.lastIndexOf("@");
      if (atPos !== -1) {
        userinfo = authorityStr.slice(0, atPos);
        hostPort = authorityStr.slice(atPos + 1);
      } else {
        hostPort = authorityStr;
      }

      // Step 6 & 7: Extract port and host
      //
      // IPv6 addresses are enclosed in brackets: [::1]:8080
      // This is necessary because IPv6 addresses contain colons, which
      // would otherwise be confused with the port delimiter.
      //
      // For IPv4/hostname, the LAST colon separates host from port, but
      // only if everything after it is digits (to avoid treating IPv6
      // literal colons as port delimiters).
      let host: string;
      let port: number | null = null;

      if (hostPort.startsWith("[")) {
        // IPv6: find closing bracket
        const bracketPos = hostPort.indexOf("]");
        if (bracketPos !== -1) {
          host = hostPort.slice(0, bracketPos + 1);
          const afterBracket = hostPort.slice(bracketPos + 1);
          if (afterBracket.startsWith(":")) {
            port = parsePort(afterBracket.slice(1));
          }
        } else {
          // Malformed IPv6, treat whole thing as host
          host = hostPort;
        }
      } else {
        // IPv4 or hostname: last ":" separates host from port
        const colonPos = hostPort.lastIndexOf(":");
        if (colonPos !== -1) {
          const maybePort = hostPort.slice(colonPos + 1);
          // Only treat as port if it's non-empty and all digits
          if (maybePort.length > 0 && /^\d+$/.test(maybePort)) {
            host = hostPort.slice(0, colonPos);
            port = parsePort(maybePort);
          } else {
            host = hostPort;
          }
        } else {
          host = hostPort;
        }
      }

      // Host is lowercased for case-insensitive comparison.
      // Empty host becomes null (no host component).
      const normalizedHost = host.length === 0 ? null : host.toLowerCase();

      return new Url(
        scheme,
        userinfo,
        normalizedHost,
        port,
        path,
        query,
        fragment,
        raw,
      );
    }

    // Also handle "scheme:path" form (e.g., "mailto:alice@example.com")
    //
    // Some URL schemes don't use the "//" authority syntax. The mailto:
    // scheme is the most common example. In this case, everything after
    // the colon is the "path" (though semantically it's an email address).
    const colonPos = trimmed.indexOf(":");
    if (colonPos > 0 && !trimmed.slice(0, colonPos).includes("/")) {
      const scheme = trimmed.slice(0, colonPos).toLowerCase();
      validateScheme(scheme);
      let rest = trimmed.slice(colonPos + 1);

      // Still split fragment and query from the path
      let fragment: string | null;
      [rest, fragment] = splitFragment(rest);
      let query: string | null;
      let pathPart: string;
      [pathPart, query] = splitQuery(rest);

      return new Url(scheme, null, null, null, pathPart, query, fragment, raw);
    }

    // No scheme found at all -- this is an error for absolute URL parsing
    throw new MissingScheme();
  }

  /**
   * Resolve a relative URL against this URL as the base.
   *
   * Implements the RFC 1808 relative resolution algorithm. The idea is that
   * a relative URL is like giving directions from where you already are,
   * rather than from the city center (absolute URL).
   *
   * ## Decision tree
   *
   * ```text
   * if R is empty         -> base without fragment
   * if R starts with #    -> update fragment only
   * if R has scheme       -> R is absolute, return as-is
   * if R starts with //   -> inherit scheme only
   * if R starts with /    -> inherit scheme + authority, replace path
   * otherwise             -> merge paths, resolve . and ..
   * ```
   *
   * ## Examples
   *
   * Given base = `http://host/a/b/c.html`:
   *
   * | Relative     | Resolved               | Why                    |
   * |-------------|------------------------|------------------------|
   * | `d.html`    | `http://host/a/b/d.html` | Same directory         |
   * | `../d.html` | `http://host/a/d.html`   | Parent directory       |
   * | `/x/y.html` | `http://host/x/y.html`   | Absolute path          |
   * | `#sec`      | `http://host/a/b/c.html#sec` | Fragment only     |
   */
  resolve(relative: string): Url {
    const trimmed = relative.trim();

    // Empty relative -> return base without fragment
    //
    // This is the "refresh" case: clicking a link to "" reloads the
    // current page without any fragment anchor.
    if (trimmed.length === 0) {
      const result = this.clone();
      result.fragment = null;
      result.raw = this.toUrlString();
      return result;
    }

    // Fragment-only: "#section"
    //
    // Only the fragment changes. This is how in-page navigation works:
    // clicking a link to "#section2" scrolls to that anchor without
    // reloading the page.
    if (trimmed.startsWith("#")) {
      const result = this.clone();
      result.fragment = trimmed.slice(1);
      result.raw = result.toUrlString();
      return result;
    }

    // If R has a scheme, it's already absolute
    //
    // Check if the relative URL contains "://" or has a scheme-like prefix
    // before any "/". If so, parse it independently -- it's a full URL.
    if (
      trimmed.includes("://") ||
      (trimmed.includes(":") && !trimmed.startsWith("/"))
    ) {
      const colonIdx = trimmed.indexOf(":");
      if (colonIdx !== -1) {
        const maybeScheme = trimmed.slice(0, colonIdx);
        if (
          maybeScheme.length > 0 &&
          /^[a-zA-Z][a-zA-Z0-9+\-.]*$/.test(maybeScheme)
        ) {
          return Url.parse(trimmed);
        }
      }
    }

    // Scheme-relative: "//host/path"
    //
    // Inherits only the scheme from the base. This pattern is used on
    // websites to serve resources over whatever protocol (HTTP or HTTPS)
    // the page was loaded with.
    if (trimmed.startsWith("//")) {
      const full = this.scheme + ":" + trimmed;
      return Url.parse(full);
    }

    // Absolute path: "/path"
    //
    // Replaces the entire path while keeping the scheme, host, and port.
    // Like navigating to a different page on the same website.
    if (trimmed.startsWith("/")) {
      let rest: string;
      let fragment: string | null;
      [rest, fragment] = splitFragment(trimmed);
      let pathPart: string;
      let query: string | null;
      [pathPart, query] = splitQuery(rest);
      const result = this.clone();
      result.path = removeDotSegments(pathPart);
      result.query = query;
      result.fragment = fragment;
      result.raw = result.toUrlString();
      return result;
    }

    // Relative path: merge with base
    //
    // This is the most common case for web navigation. The relative URL
    // is resolved against the base URL's directory (everything up to the
    // last "/"). Then dot segments (. and ..) are resolved.
    let rest: string;
    let fragment: string | null;
    [rest, fragment] = splitFragment(trimmed);
    let relativePath: string;
    let query: string | null;
    [relativePath, query] = splitQuery(rest);

    const merged = mergePaths(this.path, relativePath);
    const resolvedPath = removeDotSegments(merged);

    const result = this.clone();
    result.path = resolvedPath;
    result.query = query;
    result.fragment = fragment;
    result.raw = result.toUrlString();
    return result;
  }

  /**
   * The effective port -- explicit port if set, otherwise the scheme default.
   *
   * When a URL like `http://example.com` has no explicit port, HTTP clients
   * connect to port 80 by default. This method returns that implicit port.
   *
   * | Scheme | Default Port |
   * |--------|-------------|
   * | http   | 80          |
   * | https  | 443         |
   * | ftp    | 21          |
   */
  effectivePort(): number | null {
    if (this.port !== null) return this.port;
    return defaultPort(this.scheme);
  }

  /**
   * The authority string: `[userinfo@]host[:port]`
   *
   * The authority identifies who or what you're connecting to. It's the
   * part between `://` and the first `/` in a URL.
   */
  authority(): string {
    let auth = "";
    if (this.userinfo !== null) {
      auth += this.userinfo + "@";
    }
    if (this.host !== null) {
      auth += this.host;
    }
    if (this.port !== null) {
      auth += ":" + String(this.port);
    }
    return auth;
  }

  /**
   * Serialize back to a URL string.
   *
   * Reconstructs the URL from its parsed components. The output follows
   * the canonical form:
   *
   * - Host present: `scheme://[userinfo@]host[:port]path[?query][#fragment]`
   * - No host: `scheme:path[?query][#fragment]`
   */
  toUrlString(): string {
    let s = this.scheme;

    if (this.host !== null) {
      s += "://" + this.authority();
    } else {
      s += ":";
    }

    s += this.path;

    if (this.query !== null) {
      s += "?" + this.query;
    }
    if (this.fragment !== null) {
      s += "#" + this.fragment;
    }
    return s;
  }

  /**
   * Alias for toUrlString() -- implements JavaScript's toString convention.
   */
  toString(): string {
    return this.toUrlString();
  }

  /**
   * Create a shallow clone of this URL.
   *
   * Used internally by resolve() to produce a new Url with modified fields
   * without mutating the original.
   */
  private clone(): Url {
    return new Url(
      this.scheme,
      this.userinfo,
      this.host,
      this.port,
      this.path,
      this.query,
      this.fragment,
      this.raw,
    );
  }
}
