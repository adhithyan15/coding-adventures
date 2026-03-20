# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] — 2026-03-19

### Added

- Initial release of the Rust capability analyzer.
- **Use statement detection**: Maps `use std::fs`, `use std::net`, `use std::process`, `use std::env`, `use std::os`, `use std::io`, and `use libc` to capability categories.
- **Function call detection**: Detects `File::open`, `File::create`, `fs::read_to_string`, `fs::write`, `fs::remove_file`, `fs::create_dir`, `fs::read_dir`, `TcpStream::connect`, `TcpListener::bind`, `UdpSocket::bind`, `Command::new`, `env::var`, `env::set_var`, `env::remove_var`, and `mem::transmute`.
- **Banned construct detection**: Flags `unsafe` blocks, `extern "C"` declarations, and `include_bytes!`/`include_str!` macros.
- **Manifest comparison**: Loads `required_capabilities.json`, compares detected vs declared capabilities with fnmatch-style glob matching.
- **Default deny**: Packages without a manifest are treated as declaring zero capabilities.
- **CLI**: Three subcommands — `detect`, `check`, `banned` — with `--json`, `--exclude-tests`, and `--manifest` options.
- **50+ unit tests** across all modules.
