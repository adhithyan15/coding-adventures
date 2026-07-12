# irc-proto (C++)

Pure IRC message parsing and serialization (RFC 1459) in pure ISO C++17,
header-only, in namespace `ca::irc`. A faithful port of the Rust `irc-proto`
crate.

This is the foundation of an IRC stack: it knows nothing about sockets, threads,
or buffers — it only converts between the raw text lines of the IRC protocol and
structured `Message` values.

```text
message  = [ ":" prefix SPACE ] command [ params ] CRLF
params   = 0*14( SPACE middle ) [ SPACE ":" trailing ]
```

## API

```cpp
#include "irc_proto.hpp"
namespace irc = ca::irc;

irc::Message m = irc::parse(":alice!alice@host PRIVMSG #chan :hello world");
// m.prefix == "alice!alice@host", m.command == "PRIVMSG",
// m.params == {"#chan", "hello world"}  (trailing spaces preserved)

std::string wire = irc::serialize(m);  // ":alice… :hello world\r\n"
```

`parse` throws `ca::irc::ParseError` on an empty/whitespace-only line or a prefix
with no command; `try_parse` returns `std::optional<Message>` instead. `Message`
is a value type (`std::optional<std::string> prefix`, `std::string command`,
`std::vector<std::string> params`). `serialize` returns a CRLF-terminated
`std::string`, reintroducing the trailing parameter's `:` when it contains a
space, is empty, or begins with `:`.

Command upper-casing is ASCII-only (Rust's `to_uppercase` is Unicode-aware) —
IRC commands are ASCII, so the two agree byte-for-byte.

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and
MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../../c/iso-harness).

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```
