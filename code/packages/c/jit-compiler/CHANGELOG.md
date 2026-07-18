# Changelog

All notable changes to the `jit-compiler` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — hot-path profiler + shell native-block registry** (CCPP02
  port campaign, bucket A / pure-ISO, port #2). The C port of the Rust
  `jit-compiler` crate: the *management* layer of a JIT (profiling + block
  registry) with no code generation — honest shell scaffolding. A pure-ISO crate
  (no OS), so it rides the `iso-harness` (links nothing, `-pedantic-errors` /
  `/permissive-`). Distinct from os-platform's `jit` executor primitive.
  - **Config.** `jit_isa` {RISCV, ARM, X86}; `jit_config` {hot_threshold, target}
    built via `jit_config_new` (rejects `hot_threshold == 0`, the Rust `assert!`).
  - **Profiling.** `jit_compiler_observe_execution` counts one run per bytecode
    offset and reports, via `*became_hot`, the exact call on which the path
    transitions to hot (`count == threshold`, true once). `jit_compiler_profile`
    returns the snapshot (`execution_count`, `is_hot = count >= threshold`) via a
    `*found` flag (the Rust `Option`).
  - **Registry.** `jit_compiler_install_shell_block` (empty machine code, target
    = configured ISA, a copy of the assumptions; replaces any block at the offset)
    returns a **borrowed** pointer into the store. `jit_compiler_has_native_block`
    / `jit_compiler_native_block` test / borrow. `jit_compiler_deoptimize`
    **moves** the block out to the caller (freed with `jit_native_block_free`) and
    drops the slot without freeing the moved pointers — no double-free.
  - **Faithfulness.** `Option<T>` → status + `*found`; `&NativeBlock` → borrowed
    pointer (valid until the next registry mutation, since install may `realloc`);
    `NativeBlock` by value → owned move-out. Both `BTreeMap`s are only
    point-accessed by key (ordering not observable) → plain growable arrays +
    linear scan (as `vault-revisions`). Rust aborts on OOM; the C returns
    `JIT_ERR_NOMEM` and unwinds (a failed install undoes its assumptions copy).
    Growth multiplies are guarded against `size_t` wrap.
  - **Build.** Pure ISO, no OS, no link libraries. `run.sh` builds under every
    available C compiler via the iso-harness; `run.ps1` under MSVC.
  - **Test (`tests/jit_compiler_test.c`).** The four Rust tests (threshold
    transition, profile snapshot, shell-block install, deoptimize) plus
    install-replaces-existing, no-assumptions, many-offset growth, config
    accessors, the owned-move / borrow contract, and the invalid-parameter paths.
    343 checks, verified under gcc + clang with `-pedantic-errors`, clean under
    ASan+UBSan, 0 leaks.
