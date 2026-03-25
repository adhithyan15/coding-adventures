# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-25

### Added

- `ControlFlow` module with `CONTINUE` (`:continue`) and `EXIT` (`:exit`) symbol constants
- `Loop` class with duck-typed source registration and block-based handler registration
- Three-phase loop (collect → dispatch → idle) with `Thread.pass` CPU yield
- `add_source`, `on_event`, `run`, `stop` methods; `add_source` and `on_event` return `self` for chaining
- 10 minitest tests, 19 assertions covering delivery, exit, stop, multiple handlers/sources, chaining
