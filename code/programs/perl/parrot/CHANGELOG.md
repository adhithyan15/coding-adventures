# Changelog

All notable changes to this program will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- `parrot.pl` — Parrot REPL script demonstrating the `CodingAdventures::Repl`
  framework with a custom `Parrot::Prompt` and parrot-themed prompts
- `lib/Parrot/Prompt.pm` — custom Prompt implementation with parrot emoji and
  banner text; implements the two-method Prompt interface (`global_prompt`,
  `line_prompt`)
- `t/test_parrot.t` — 16 Test2::V0 tests covering `Parrot::Prompt` unit
  behaviour and full REPL loop integration with injected I/O
- `Makefile.PL` — ExtUtils::MakeMaker build configuration
- `cpanfile` — dependency declaration (Test2::V0 for tests)
- `BUILD` — CI build command (`cpanm --installdeps` + `prove`)
- `BUILD_windows` — skip marker for Windows CI
- `README.md` — usage, architecture, and file layout
