### Added — Publish Workflow + Package Completeness (PR #13)
- `.github/workflows/publish.yml` — release publishing for PyPI and RubyGems
- PyPI publishing via OIDC Trusted Publishers (no API tokens)
- Native extension support via maturin: builds wheels on Linux, macOS (arm64 + x86_64), Windows
- Ruby gem publishing via `RUBYGEMS_API_KEY` secret
- Fixed 8 incomplete packages:
  - Go: README + CHANGELOG for assembler, python-lexer, ruby-lexer
  - Ruby: test suites for assembler, html_renderer, jit_compiler shell gems
  - Python: README/CHANGELOG for hello-world and pipeline-visualizer programs

