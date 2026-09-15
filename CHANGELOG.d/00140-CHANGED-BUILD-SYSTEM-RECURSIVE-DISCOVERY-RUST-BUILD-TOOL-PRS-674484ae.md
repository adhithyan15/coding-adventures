### Changed — Build System: Recursive Discovery + Rust Build Tool (PRs #16, #17)
- **Recursive BUILD file discovery** replaces DIRS-based routing in all build tools (Go, Python, Ruby)
- Build tools now walk the directory tree automatically — no DIRS files needed
- Added skip list for non-source directories (`.git`, `.venv`, `node_modules`, `target`, `.claude`, etc.)
- **Rust added as recognized language** — 6 Rust packages now properly discovered (were "unknown")
- **New: Rust build tool** — complete port with rayon parallelism, SHA256 hashing, git-diff detection
- **All 18 DIRS files removed** from the repository
- Total discovered packages increased from 77 (DIRS-routed) to 126+ (recursive)

