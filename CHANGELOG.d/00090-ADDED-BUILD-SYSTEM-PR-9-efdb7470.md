### Added — Build System (PR #9)
- **Directed graph library** in Python (73 tests, 98%), Ruby (77 tests, 100%), and Go (39 tests, 94%)
- **Build tool** in Go (primary), Python (reference), and Ruby (educational) — incremental, parallel, git-diff-based change detection
- **BUILD files** for all packages — declarative build commands per package
- **GitHub Actions CI** — compiles Go build tool, runs affected packages in parallel
- Go 1.26 added to mise.toml

