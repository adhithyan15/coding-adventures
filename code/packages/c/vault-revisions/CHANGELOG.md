# Changelog

All notable changes to the `vault-revisions` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — thread-safe in-memory revision history** (CCPP02 port
  campaign, bucket B / thread slice, port #2). The C port of the Rust
  `vault-revisions` crate (VLT12): a per-`(namespace, key)` append-only history of
  opaque ciphertext blobs with a per-namespace retention policy — and the second
  proof that a real Mutex-guarded crate runs on os-platform's `osp_mutex`.
  - `vr_store_create`/`vr_store_destroy`; `vr_archive` (append a ciphertext with a
    fresh monotonic id, enforce the count cap), `vr_list` (metadata only,
    ascending by id), `vr_get_revision` (bytes included), `vr_restore` (re-archive
    an old revision as new — append-only, not a rollback), `vr_purge_due` (drop
    revisions older than `now − max_age`, host-driven), `vr_policy_for` /
    `vr_set_policy`, `vr_summary_of` (payload-free counts). Domain error enum
    `vr_status`; `Option<T>` → `has_*` flag + value; output ciphertext is a fresh
    copy (`vr_revision_free` / `vr_meta_list_free`).
  - **Validation.** Namespace/key non-empty and length-bounded (`VR_MAX_*`), with
    a minimal UTF-8 decoder rejecting control / whitespace / Unicode bidi-override
    / zero-width characters — the Rust crate's `is_safe_id_string` defence.
  - **Thread story.** The store is guarded by one `osp_mutex`; every op takes the
    lock for its whole read/mutate, so it is safe to share across threads (the
    Rust `Mutex<InMemoryInner>` + `Send + Sync`). Pure consumer of os-platform's
    thread backend — no changes to os-platform. (The Rust `BTreeMap` ordering is
    not observable through this API, so the C store uses plain growable arrays.)
  - **Build.** OS-agnostic single source; `run.sh` compiles it with
    `os-platform/src/thread_posix.c` and links `-pthread`
    (`_POSIX_C_SOURCE=200809L`); `run.ps1` uses `thread_windows.c` (CRT + kernel32).
  - **Test (`tests/vault_revisions_test.c`).** Mirrors the Rust tests
    (archive/list/get/restore, retention eviction, age purge, policy set/replace,
    validation, summary) and adds the payoff: four worker threads
    (`osp_thread_spawn`) archive concurrently and no updates are lost. 109 checks,
    verified under ASan+UBSan, 0 leaks, and ThreadSanitizer clean under four
    concurrent writers.
