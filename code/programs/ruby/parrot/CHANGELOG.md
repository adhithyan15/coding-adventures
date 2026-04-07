# Changelog — parrot (Ruby)

All notable changes to this program are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [1.0.0] — 2026-04-06

### Added

- Initial release of the Parrot REPL program in Ruby.
- `lib/parrot/prompt.rb` — `Parrot::Prompt` class that includes
  `CodingAdventures::Repl::Prompt` and implements `global_prompt` and
  `line_prompt` with parrot-themed strings and emoji.
- `lib/parrot.rb` — top-level `Parrot` module with a `Parrot.run` class
  method that wires `EchoLanguage`, `Parrot::Prompt`, and `SilentWaiting`
  together with real `$stdin`/`$stdout` I/O.
- `bin/parrot` — executable entry point that prepends `lib/` to `$LOAD_PATH`
  and calls `Parrot.run`.
- `test/test_helper.rb` — shared Minitest setup; adds `lib/` to load path.
- `test/test_parrot.rb` — 15 Minitest tests covering echo behaviour, quit
  handling, sync/async modes, prompt content, EOF handling, empty input, output
  call counting, and loop termination on `:quit`.
- `Gemfile` — depends on `coding_adventures_repl` via local `path:` and
  `minitest`/`rake` in the test group.
- `Rakefile` — defines `rake test` (default task) using `Rake::TestTask`.
- `BUILD` and `BUILD_windows` — build scripts that bundle install then run
  the test suite.
