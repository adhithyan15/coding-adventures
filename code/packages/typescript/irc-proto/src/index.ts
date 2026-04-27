/**
 * irc-proto — Pure IRC message parsing and serialization (RFC 1459).
 *
 * This package is the foundation of the IRC stack.  It knows nothing about
 * sockets, threads, or buffers — it only converts between the raw text lines
 * of the IRC protocol and structured {@link Message} values.
 *
 * Every other IRC package depends on irc-proto's `Message` type, but irc-proto
 * itself depends on nothing.  This is intentional: a pure parsing library is
 * easy to test exhaustively and easy to port to new languages.
 *
 * The IRC message grammar (RFC 1459) in informal BNF:
 *
 * ```
 * message    = [ ":" prefix SPACE ] command [ params ] CRLF
 * prefix     = servername / ( nick [ "!" user ] [ "@" host ] )
 * command    = 1*letter / 3digit
 * params     = 0*14( SPACE middle ) [ SPACE ":" trailing ]
 *            / 14( SPACE middle ) [ SPACE [ ":" ] trailing ]
 * middle     = nospcrlfcl *( ":" / nospcrlfcl )
 * trailing   = *( ":" / " " / nospcrlfcl )
 * SPACE      = 0x20
 * ```
 *
 * In practice: a message is at most 512 bytes including the final CRLF, and
 * carries a prefix, a command, and up to 15 parameters (the last of which may
 * contain spaces when prefixed by `:`).
 */

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/**
 * A single parsed IRC protocol message.
 *
 * Think of this as a plain envelope with three slots:
 *
 * - `prefix`  — *Who sent it?*  `null` for client-originated messages.
 *   For server messages this is a server name (`"irc.example.com"`).
 *   For relayed client messages it is a full nick-mask
 *   (`"alice!alice@127.0.0.1"`).
 * - `command` — *What kind of message is it?*  Always uppercase, e.g.
 *   `"PRIVMSG"`, `"JOIN"`, or the 3-digit numeric string `"001"`.
 * - `params`  — *The arguments.* A plain array of strings.  The
 *   "trailing" param (the one that may contain spaces) is already stripped
 *   of its leading `:` and lives as the last element of the array — no
 *   special treatment needed by callers.
 *
 * @example
 * ```ts
 * { prefix: null, command: 'NICK', params: ['alice'] }
 * { prefix: 'irc.local', command: '001', params: ['alice', 'Welcome!'] }
 * { prefix: 'alice!alice@host', command: 'PRIVMSG', params: ['#general', 'hello world'] }
 * ```
 */
export interface Message {
  prefix: string | null;
  command: string;
  params: string[];
}

/**
 * Raised when a raw line cannot be understood as an IRC message.
 *
 * Callers should catch this when reading from untrusted sources and either
 * skip the offending line or close the connection, depending on policy.
 */
export class ParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ParseError";
  }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

// RFC 1459 allows at most 15 parameters in a single message.  A 16th token
// (or any beyond) is silently discarded.  This constant documents that limit.
const MAX_PARAMS = 15;

/**
 * Parse a single IRC message line into a {@link Message}.
 *
 * The `line` argument must already have its trailing `\r\n` stripped
 * (that is the responsibility of the framing layer, which hands us clean
 * lines to interpret).
 *
 * Throws {@link ParseError} when:
 * - the line is empty, or
 * - the line contains only whitespace (nothing to parse), or
 * - there is no command token after the (optional) prefix.
 *
 * Parsing proceeds in three stages:
 *
 * 1. **Optional prefix** — if the line starts with `:`, consume everything
 *    up to the first space as the prefix (dropping the leading `:`).
 * 2. **Command** — the next whitespace-delimited token, normalised to
 *    uppercase.
 * 3. **Params** — each remaining space-delimited token is a param.  When a
 *    token begins with `:`, that token *and everything that follows it*
 *    (spaces included) forms the last param, with the `:` stripped.
 *
 * @example
 * ```ts
 * parse("NICK alice")
 * // → { prefix: null, command: 'NICK', params: ['alice'] }
 *
 * parse(":irc.local 001 alice :Welcome!")
 * // → { prefix: 'irc.local', command: '001', params: ['alice', 'Welcome!'] }
 *
 * parse(":alice!alice@host PRIVMSG #chan :hello world")
 * // → { prefix: 'alice!alice@host', command: 'PRIVMSG', params: ['#chan', 'hello world'] }
 *
 * parse("join #foo")
 * // → { prefix: null, command: 'JOIN', params: ['#foo'] }
 * ```
 */
export function parse(line: string): Message {
  // ── Stage 0: reject empty / whitespace-only input ─────────────────────────
  // An empty line carries no information and RFC 1459 does not permit them.
  // A whitespace-only line likewise has no command and cannot be parsed.
  if (!line || !line.trim()) {
    throw new ParseError(`empty or whitespace-only line: ${JSON.stringify(line)}`);
  }

  // We work with a mutable "rest" view of the input, consuming tokens from
  // the left as we identify each field.
  let rest = line;

  // ── Stage 1: optional prefix ───────────────────────────────────────────────
  // The presence of a leading colon is the unambiguous signal that a prefix
  // follows.  The prefix ends at the first space character.
  //
  //   ":irc.local 001 alice :Welcome!\r\n"
  //    ↑                                   ← leading colon triggers prefix parsing
  //       ↑↑↑↑↑↑↑↑                        ← prefix value (colon stripped)
  let prefix: string | null = null;
  if (rest.startsWith(":")) {
    // Split on the first space only so the prefix can contain no spaces.
    const spacePos = rest.indexOf(" ");
    if (spacePos === -1) {
      // A line that is *only* a prefix with no command is malformed.
      throw new ParseError(`line has prefix but no command: ${JSON.stringify(line)}`);
    }
    // Strip the leading colon when storing the prefix value.
    prefix = rest.slice(1, spacePos);
    // Advance past the prefix and the separating space.
    rest = rest.slice(spacePos + 1);
  }

  // ── Stage 2: command ───────────────────────────────────────────────────────
  // The command is the first whitespace-delimited token remaining.
  // RFC 1459 says commands are case-insensitive; we normalise to uppercase
  // so the rest of the stack never has to deal with mixed-case commands.
  //
  //   "001 alice :Welcome!"  →  command="001", rest="alice :Welcome!"
  //   "PRIVMSG #c :hi"       →  command="PRIVMSG", rest="#c :hi"
  const spaceIdx = rest.indexOf(" ");
  let command: string;
  if (spaceIdx === -1) {
    // No space: the rest of the line is just the command, no params.
    command = rest.toUpperCase();
    rest = "";
  } else {
    command = rest.slice(0, spaceIdx).toUpperCase();
    rest = rest.slice(spaceIdx + 1);
  }

  if (!command) {
    throw new ParseError(`could not extract command from line: ${JSON.stringify(line)}`);
  }

  // ── Stage 3: parameters ────────────────────────────────────────────────────
  // Parameters are collected one token at a time.  When we encounter a token
  // that begins with `:`, it signals the start of the *trailing* param:
  // everything from that `:`, through to the end of the line (spaces and
  // all), belongs to this single parameter.  The leading `:` is stripped.
  //
  // Example:
  //   "#c :hello world"
  //   → first token: "#c"   (regular param)
  //   → next token starts with ":": trailing = "hello world"
  //
  // We also enforce the RFC 1459 limit of 15 params; extras are discarded.
  const params: string[] = [];

  while (rest) {
    if (rest.startsWith(":")) {
      // Trailing param — absorbs the rest of the line.
      // Strip the leading colon; spaces are preserved as-is.
      params.push(rest.slice(1));
      break; // nothing can follow the trailing param
    }

    // Split off the next space-delimited token.
    const nextSpace = rest.indexOf(" ");
    if (nextSpace === -1) {
      // No more spaces: the remainder is a single final token.
      params.push(rest);
      break;
    } else {
      const token = rest.slice(0, nextSpace);
      params.push(token);
      rest = rest.slice(nextSpace + 1);
    }

    // Enforce the maximum parameter count.  If we have already collected
    // 15 params, stop — any trailing content is silently dropped.
    if (params.length === MAX_PARAMS) {
      break;
    }
  }

  return { prefix, command, params };
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/**
 * Serialize a {@link Message} back to IRC wire format.
 *
 * Returns a CRLF-terminated `Buffer` ready to be written to a socket or
 * compared against expected protocol output in tests.
 *
 * Serialization rules:
 *
 * 1. If `msg.prefix` is set, the output begins with `:<prefix> `.
 * 2. The command follows, already normalised to uppercase by the caller or
 *    the `parse()` function.
 * 3. Each param is appended with a leading space.  If the last param contains
 *    a space character, it **must** be written with a leading `:` so the
 *    receiver knows it is the trailing param.
 * 4. The message always ends with `\r\n` (CRLF), the IRC line terminator.
 *
 * @example
 * ```ts
 * serialize({ prefix: null, command: 'NICK', params: ['alice'] })
 * // → Buffer('NICK alice\r\n')
 *
 * serialize({ prefix: 'irc.local', command: '001', params: ['alice', 'Welcome to the server'] })
 * // → Buffer(':irc.local 001 alice :Welcome to the server\r\n')
 *
 * serialize({ prefix: null, command: 'PRIVMSG', params: ['#chan', 'hello world'] })
 * // → Buffer('PRIVMSG #chan :hello world\r\n')
 * ```
 */
export function serialize(msg: Message): Buffer {
  // We build the message as an array of string fragments, then join and encode.
  const parts: string[] = [];

  // ── Prefix ─────────────────────────────────────────────────────────────────
  // Prefix, if present, is always wrapped in a leading colon and followed by
  // a single space so the receiver can find the boundary between prefix and
  // command.
  if (msg.prefix !== null) {
    parts.push(`:${msg.prefix}`);
  }

  // ── Command ────────────────────────────────────────────────────────────────
  parts.push(msg.command);

  // ── Parameters ─────────────────────────────────────────────────────────────
  // Walk through every param.  For all but the last we emit the value as-is
  // (preceded by a space via join).  For the last param, we check whether it
  // contains a space; if it does, it must be serialized as a trailing param
  // with a leading colon so the receiver knows to absorb the rest of the line.
  for (let i = 0; i < msg.params.length; i++) {
    const param = msg.params[i];
    const isLast = i === msg.params.length - 1;

    if (isLast && param.includes(" ")) {
      // Trailing param: the colon signals "everything from here to CRLF
      // belongs to this single parameter, spaces and all".
      parts.push(`:${param}`);
    } else {
      parts.push(param);
    }
  }

  // Join with spaces and append the mandatory CRLF line terminator.
  // IRC uses CRLF (0x0D 0x0A), not just LF.
  const line = parts.join(" ") + "\r\n";

  // Encode to Buffer for direct socket/buffer use.
  // IRC is specified in ASCII, but UTF-8 is widely accepted in practice.
  return Buffer.from(line, "utf-8");
}
