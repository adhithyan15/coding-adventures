# Windows Ruby oracle stdout needs binary mode for LF byte comparisons
category: Testing & coverage

The native LANG80 runner emitted the specified LF bytes, but a direct Windows
Ruby oracle invocation emitted CRLF because stdout used text mode. An initial
byte comparison therefore failed even for `puts(1, 2, 3)`. Invoke the independent
oracle with `STDOUT.binmode; load ARGV.fetch(0)` before executing the fixture.
Compare the resulting raw bytes without post-hoc newline normalization; this
keeps the native runner's LF contract explicit and still checks exact output.

The later arithmetic-token audit repeated this mistake on its first oracle
fixture: native output began `3\n4\n6\n`, while Windows Ruby emitted CRLF.
Reusing the binary-mode invocation made all six independent fixtures match
without changing product behavior or normalizing captured output. Read this
transport contract before building another Windows Ruby oracle harness.
