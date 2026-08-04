# Changelog

All notable changes to the `in-memory-data-store-protocol` (C) package are
documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — RESP protocol intermediate representation** (CCPP02 port
  campaign, bucket A / pure-ISO, port #1). The C port of the Rust
  `in-memory-data-store-protocol` crate: the little IR a Redis-style in-memory
  data store uses between the wire and its engine. The first bucket-A port after
  the thread slice — a crate that needs no OS, so it rides the `iso-harness`
  (links nothing, `-pedantic-errors` / `/permissive-`).
  - **Command frame.** `imds_command_frame` (`char *command` + `imds_arg`
    {bytes,len} array). `imds_command_frame_new` (copy a name + args) and
    `imds_command_frame_from_parts` (first wire part → uppercased command, rest →
    args; `IMDS_NONE` on an empty list, mirroring Rust's `Option::None` from
    `split_first`). `imds_command_frame_free`.
  - **Engine response.** Recursive tagged union `imds_engine_response`
    (`SIMPLE_STRING` / `ERROR` / `INTEGER` / `BULK_STRING` / `ARRAY`) with
    constructors `imds_resp_simple_string` / `error` / `integer` / `bulk_string`
    / `bulk_null` / `array` / `array_null`, plus the `ok` / `null` / `zero` /
    `one` shortcuts. `imds_engine_response_free` recursively frees a nested array
    tree.
  - **Faithfulness.** `Option<T>` → an `is_null` flag beside the payload (so
    `$-1`/`*-1` stay distinct from an empty-but-present blob/array) and the
    `IMDS_NONE` status. `ascii_upper` is byte-exact: `a`..=`z` shift only, and a
    byte ≥ `0x80` expands to its two-byte UTF-8 encoding (`byte as char` +
    `collect::<String>()`), so `command` matches the Rust `String`'s bytes for
    any input. Rust `String`/`Vec` → owned `malloc` buffers; every value owns its
    heap and frees cleanly; `imds_resp_array` takes ownership of the `items`
    buffer. Allocating constructors return `IMDS_ERR_NOMEM` and unwind cleanly
    (the Rust aborts on OOM).
  - **Build.** Pure ISO, no OS, no link libraries. `run.sh` builds under every
    available C compiler via the iso-harness; `run.ps1` under MSVC.
  - **Test (`tests/imds_protocol_test.c`).** Frame new/from_parts (empty →
    `IMDS_NONE`, uppercasing, input-independence, the Latin-1→UTF-8 `ascii_upper`
    edge), every response variant, the convenience shortcuts, a nested array tree
    freed recursively, zeroed/NULL free-safety, and invalid-parameter paths. 89
    checks, verified under gcc + clang with `-pedantic-errors`, clean under
    ASan+UBSan, 0 leaks.
