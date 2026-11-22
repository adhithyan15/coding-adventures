# Changelog

## Unreleased

### Fixed & Hardened (VER-W9-D Cross-Stack Audit)
- Eliminated `CWE-1321` Prototype Pollution false positives on numeric array and `TypedArray` bracket writes (`buf[i] = ...`, `buf[2 * k + 1] = ...`, `this._data[2 * idx] = ...`, `target[targetIndex] = ...`, `out[outIdx++] = ...`) across `dsp-*` and `music-machine-*` via scope-aware numeric variable/type tracking and bounded `isNumericExpression(..., depth)` evaluation while preserving detection for string/unknown/reassigned/`for-of` keys.
- Added CommonJS `require(...)` and dynamic `import(...)` capability/CWE-94 detection, `globalThis`/`window`/`global` `eval`/`Function` checks, `MethodDeclaration` (`this.<method>()`) `CWE-674` recursion tracking, static template literal handling for `RegExp`/`exec`, and `fs`-receiver gating for `AMBIGUOUS_FS_METHODS` (`stat`, `lstat`, `access`, `watch`, `rm`, `rename`, `truncate`, `chmod`, `chown`).
- Added cross-stack regression tests in `tests/analyzer.test.ts` (including `dsp-complex`, `dsp-filters`, and `music-machine-node` `--strict` checks).

### Added
- Wired `@coding-adventures/test-bootstrap` into `devDependencies` (`package.json`, `package-lock.json`, `BUILD`, `BUILD_windows`) and added `.gitignore` for `dist/`, `dist-test/`, `node_modules/`, `coverage/`.
- Migrated tests from `--experimental-strip-types` to standalone `tsconfig.test.json` with `--experimental-test-coverage --test-coverage-lines=95`.
- Added package-local `ca-capability-analyzer.warrant` matching `code/specs/ca-capability-analyzer.warrant`.
- Wired `SplitMix64` from `@coding-adventures/test-bootstrap` into `tests/pbt.test.ts` and `tests/hostile.test.ts`.
- Hardened input boundaries (CWE-20 / CWE-400 / CWE-1321): bounded `sourceText` to `10,485,760` bytes (`MAX_SOURCE_BYTES`), bounded regex pattern inspection length (`MAX_REGEX_PATTERN_LEN`), rejected `null`/non-object options, NUL-byte paths, symlinks/oversized/malformed `required_capabilities.json` manifests, and excluded `dist-test` in `scanner.ts`.

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
