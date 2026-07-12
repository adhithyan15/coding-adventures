# irc-proto (C)

Pure IRC message parsing and serialization (RFC 1459) in pure ISO C17. A
faithful port of the Rust `irc-proto` crate.

This is the foundation of an IRC stack: it knows nothing about sockets, threads,
or buffers — it only converts between the raw text lines of the IRC protocol and
structured `IrcMessage` values.

```text
message  = [ ":" prefix SPACE ] command [ params ] CRLF
params   = 0*14( SPACE middle ) [ SPACE ":" trailing ]
```

A message carries an optional **prefix**, a **command**, and up to 15
**parameters** — the last of which may contain spaces when introduced by `:`.

## API

```c
#include "irc_proto.h"

IrcMessage m;
if (irc_parse(":alice!alice@host PRIVMSG #chan :hello world", &m) == IRC_OK) {
    /* m.prefix == "alice!alice@host", m.command == "PRIVMSG",
       m.params[0] == "#chan", m.params[1] == "hello world" (spaces kept) */
    size_t len;
    unsigned char *wire = irc_serialize(&m, &len); /* ":alice… :hello world\r\n" */
    free(wire);
    irc_message_free(&m);
}
```

`irc_parse` fills a caller-provided `IrcMessage` with malloc'd strings (release
with `irc_message_free`, which frees the fields, not the struct) and returns a
typed `IrcStatus` where the Rust version returns a `Result`. `irc_serialize`
returns a malloc'd, CRLF-terminated byte buffer (also NUL-terminated past the
counted length); the trailing parameter is re-introduced with `:` exactly when
it contains a space, is empty, or begins with `:`.

Command upper-casing is ASCII-only (Rust's `to_uppercase` is Unicode-aware) —
IRC commands are ASCII, so the two agree byte-for-byte.

## Portability

Pure ISO C17 — no POSIX `strdup`/`strndup`, no extensions. Compiles clean under
GCC, Clang, and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors, via the shared [`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```
