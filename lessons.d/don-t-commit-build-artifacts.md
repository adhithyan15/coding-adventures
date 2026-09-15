---
category: BUILD files & dependency management
---

# Don't commit build artifacts

After agents run tests, always `git status` for `.build/`, `.swiftpm/`, `cover/`, `_build/`, `deps/`, `node_modules/`, `.venv/`, `__pycache__/`, `blib/`, `MYMETA.*`, `pm_to_blib`, Perl-generated `Makefile`, `target/`, copied `.so`/`.pyd` files, etc. Stage by explicit path, never `git add .`. Every Swift package needs `.gitignore` with `.build/` and `.swiftpm/` BEFORE the first `swift test`.
