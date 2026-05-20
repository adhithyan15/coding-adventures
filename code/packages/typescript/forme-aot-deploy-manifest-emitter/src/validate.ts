/**
 * validate.ts — output-path + content validation.
 *
 * Output paths are the security-critical piece — they land on
 * the deploy target's filesystem.  Reject every form of path
 * traversal or absolute escape.
 *
 * Output paths are RELATIVE (no leading `/`), unlike routes in
 * the page bundle which are URL paths (leading `/`).  The
 * page-bundle emitter already converts routes to relative
 * output paths; here we only need to validate the additional
 * extraFiles paths.
 *
 * @module validate
 */

// Segment charset: like page-bundle's ROUTE_SEGMENT_RE but
// stripped of URL-only chars and tightened for filesystem
// safety.  Specifically removed (vs. ROUTE_SEGMENT_RE):
//   - `%` — no percent-encoding decode at this layer.
//   - `?` `#` — URL syntax, not filesystem.
//   - `:` — Windows reserved (drive separator), macOS HFS+
//     historically used colon as path separator.  We forbid
//     it entirely rather than try to enforce
//     "not-in-first-segment" rules that bite cross-platform.
const PATH_SEGMENT_RE = /^[A-Za-z0-9._~!$&'()*+,;=@\-]+$/;

// Windows reserved device names — case-insensitive, with or
// without extension.  See Win32 `CreateFile` docs.
const WIN_RESERVED_RE = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;

/**
 * Validate a relative output path.
 *
 * Accepts: one or more segments joined by `/`.  Each segment
 * matches the segment charset above; no `..`, no `.`, no
 * empty segments.
 *
 * Rejects:
 *   - non-string / empty / over-cap (2048)
 *   - leading `/` (must be relative)
 *   - leading `~/` (home-dir expansion confuses some tools)
 *   - any `\` (Windows path separator)
 *   - `..` segment anywhere (path traversal)
 *   - `.` sole segment
 *   - empty mid-segment (`a//b`)
 *   - trailing `/` (would create a directory, not a file)
 *   - disallowed chars
 *   - Windows drive letter prefix (`C:`) — `:` is in the
 *     segment charset for cross-platform reasons (it's
 *     occasionally legal in URL routes), but a colon in the
 *     FIRST segment of an output path is suspicious; we
 *     reject `*:*` as the first segment.
 */
export function validateOutputPath(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${field} must be a string; got ${
        value === null ? "null" : typeof value
      }`,
    );
  }
  if (value.length === 0) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${field} must be non-empty`,
    );
  }
  if (value.length > 2048) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${field} must be ≤ 2048 chars`,
    );
  }
  if (value[0] === "/") {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${field} must be relative (no leading "/"); got ${JSON.stringify(shorten(value))}`,
    );
  }
  if (value[0] === "~") {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${field} must not start with "~" (home-dir expansion); got ${JSON.stringify(shorten(value))}`,
    );
  }
  if (value.indexOf("\\") !== -1) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${field} must not contain "\\"; got ${JSON.stringify(shorten(value))}`,
    );
  }

  const segments = value.split("/");
  for (const seg of segments) {
    if (seg.length === 0) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} must not contain empty segments (//); got ${JSON.stringify(shorten(value))}`,
      );
    }
    if (seg === "..") {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} must not contain ".." segments (path traversal); got ${JSON.stringify(shorten(value))}`,
      );
    }
    if (seg === ".") {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} must not contain "." segments; got ${JSON.stringify(shorten(value))}`,
      );
    }
    if (!PATH_SEGMENT_RE.test(seg)) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} segment ${JSON.stringify(seg)} contains disallowed characters; only [A-Za-z0-9._~!$&'()*+,;=@-] permitted (no ":" — Windows / HFS+ reserved)`,
      );
    }
    // Per-segment length cap.  ext4 / APFS / NTFS all cap
    // single filename components at 255 bytes; longer segments
    // would write-fail at deploy time.  We surface it here
    // instead of letting the deploy runner blow up.
    if (seg.length > 255) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} segment exceeds 255-byte filesystem limit; got ${seg.length} bytes`,
      );
    }
    // Windows reserved device names — CON, PRN, AUX, NUL,
    // COM1..9, LPT1..9 — with or without extension.  Win32
    // intercepts these and writes to the device instead of
    // creating a file.  Reject so cross-platform deploys are
    // safe.  Match is case-insensitive (Windows is too).
    if (WIN_RESERVED_RE.test(seg)) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} segment ${JSON.stringify(seg)} is a Windows reserved device name (CON/PRN/AUX/NUL/COM1-9/LPT1-9)`,
      );
    }
    // Trailing dot / space in a Windows filename gets silently
    // stripped by the Win32 layer, producing a different file
    // than the caller asked for.  Reject so the bug surfaces.
    const lastChar = seg[seg.length - 1];
    if (lastChar === "." || lastChar === " ") {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} segment ${JSON.stringify(seg)} must not end in "." or " " (Windows silently strips these)`,
      );
    }
    // Prototype-pollution defence (defense-in-depth — the
    // emitter also uses `Object.create(null)` for the output
    // table, but rejecting these names up-front gives a clear
    // error rather than a silent-write-into-prototype risk on
    // any downstream consumer that does the same).
    if (seg === "__proto__" || seg === "constructor" || seg === "prototype") {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: ${field} segment ${JSON.stringify(seg)} is a JS prototype-pollution sink name`,
      );
    }
  }
  return value;
}

/**
 * Validate a string field (used for content, contentType,
 * lastmod, the input JSON/XML/TXT strings).
 */
export function validateString(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${field} must be a string; got ${
        value === null ? "null" : typeof value
      }`,
    );
  }
  return value;
}

function shorten(s: string): string {
  return s.length > 100 ? `${s.slice(0, 100)}…` : s;
}
