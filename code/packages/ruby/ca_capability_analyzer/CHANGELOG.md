# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial release of the Ruby capability analyzer.
- **Capability detection** via Prism AST walking:
  - Import detection: require "socket", "net/http", "open3", "fileutils", "tempfile", "pathname", "etc", "fiddle", etc.
  - Class method detection: File.read/write/delete, Dir.glob/mkdir, IO.read/write, ENV.fetch, FileUtils.rm/cp/mkdir_p, TCPSocket.new, Process.spawn, etc.
  - Bare method detection: system(), exec(), spawn(), fork(), open().
  - ENV subscript detection: ENV["KEY"].
  - Backtick execution detection: `cmd` and `cmd #{expr}`.
  - Net::HTTP detection: Net::HTTP.get/post/start/new.
- **Banned construct detection**:
  - eval() and eval-family: instance_eval, class_eval, module_eval with string args.
  - Dynamic dispatch: send/public_send/__send__ with non-literal first argument.
  - Dynamic require: require(variable).
  - Dynamic const_get: Object.const_get(variable).
  - Dynamic define_method: define_method(variable).
  - method_missing definition.
  - Backtick execution.
  - System/exec with string interpolation.
  - Banned class methods: Binding.eval, Kernel.eval, Kernel.exec, Kernel.system.
- **Manifest loading and comparison**:
  - Load required_capabilities.json manifests.
  - Default deny policy (no manifest = zero capabilities).
  - Glob-style target matching via File.fnmatch.
  - Asymmetric comparison: undeclared = error, unused = warning.
- **CLI** with three commands: detect, check, banned.
- Comprehensive test suite (50+ tests).
