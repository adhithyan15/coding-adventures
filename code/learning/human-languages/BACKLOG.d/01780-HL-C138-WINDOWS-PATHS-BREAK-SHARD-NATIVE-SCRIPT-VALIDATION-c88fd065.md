## HL-C138 — Windows paths break shard-native script validation

The shard-native script inventory plugin tightened its realpath boundary, but
its Vite config passed raw `file:` URL pathnames into `node:path`. On Windows,
`/C:/...` resolved as `C:\C:\...`, so script-ductus validation stopped before
the test runner loaded. This deterministic origin/main regression outranks the
next measured glyph closure because it blocks that closure's local evidence.

Convert both config roots with Node's platform-aware `fileURLToPath`. The
plugin keeps its fail-closed realpath checks, while Windows and POSIX now hand
it native absolute paths. Its regression tests likewise compare native path
suffixes and use a directory junction on Windows, where ordinary symlink
creation requires elevated privileges. After this repair, the measured queue
returns to Tamil **ஒ** at **2 affected realizations**.
