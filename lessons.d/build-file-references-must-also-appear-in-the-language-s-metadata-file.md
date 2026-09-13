---
category: BUILD files & dependency management
---

# BUILD-file references must also appear in the language's metadata file

so the build-tool validator can see the dep edge: Python `[project] dependencies` (and `[tool.uv.sources]` for local paths), Ruby `.gemspec` `spec.add_dependency` (block var must be `spec`, not `s`), Perl `cpanfile`, Go `go.mod`, Swift `Package.swift`, Rust `Cargo.toml`, TypeScript `package.json`. Missing this raises `undeclared local package refs:` in the detect job.
