# Changelog

## 0.1.0

- Initial implementation of the TypeScript static capability analyzer and VER15 CWE detector (VER-W0-C / VER08 / VER15).
- Zero runtime dependencies with AST analysis powered by TypeScript compiler API.
- Static ambient capability detection for `fs:read`, `fs:write`, `net:socket`, `process:spawn`, `env:read`, `ffi:unsafe`, `time:clock`, and `random:entropy`.
- Detection of VER15 CWE vulnerabilities:
  - CWE-1321: Prototype pollution (`__proto__`, `constructor`, `prototype`, dynamic bracket writes on plain objects)
  - CWE-1333: Inefficient regular expressions / ReDoS (catastrophic backtracking shapes)
  - CWE-674: Uncontrolled recursion (unbounded self-calls lacking depth guards)
  - CWE-94 / CWE-78: Code and command injection (`eval`, `new Function`, dynamic `child_process.exec`)
- Strict verification against `required_capabilities.json` manifests (`--strict` mode).
- Level 0-2 test suite with `@coding-adventures/test-bootstrap` including unit tests, PBT, hostile input suites, and CLI integration tests.
