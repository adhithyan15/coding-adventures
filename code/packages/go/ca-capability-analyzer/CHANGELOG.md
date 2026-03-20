# Changelog

All notable changes to the ca-capability-analyzer Go package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-19

### Added
- Core AST-based capability detection using `go/ast`, `go/parser`, `go/token`
- Import detection for 12 packages: os, io, io/ioutil, net, net/http, os/exec, syscall, unsafe, plugin, C, reflect
- Function call detection for 19 patterns: os.Open, os.Create, os.Remove, os.Mkdir, os.ReadDir, os.ReadFile, os.WriteFile, os.RemoveAll, os.MkdirAll, os.Getenv, os.Setenv, os.Exit, exec.Command, net.Dial, net.Listen, http.Get, http.Post, http.Head
- Banned construct detection: reflect.Value.Call, reflect.MethodByName, plugin.Open, //go:linkname, unsafe.Pointer, import "C" (cgo)
- Manifest loading and comparison with glob-based target matching
- CLI with three subcommands: detect, check, banned
- JSON output support for all subcommands
- Directory walking with test file exclusion
- Import alias resolution for accurate call detection
- 50+ tests across analyzer, banned, and manifest modules
