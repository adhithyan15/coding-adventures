# vault-revisions (C)

**CCPP02 port campaign — bucket B, thread slice, port #2.** The revision-history
layer of a password/secrets ("Vault") stack: every `put` archives the *prior*
ciphertext to a per-`(namespace, key)` history list, so a user can list past
revisions and restore one. The C port of the Rust `vault-revisions` crate (VLT12),
and the second proof that a real *Mutex-guarded* crate runs on os-platform's
concurrency primitive (`osp_mutex`) with no per-OS code of its own.

```c
vr_store *s; vr_store_create(&s);

vr_revision r;
vr_archive(s, "kv", "login/github", ct, ct_len, now_ms, &r);   /* → id 1, 2, 3… */
vr_revision_free(&r);

vr_revision_meta *metas; size_t n;
vr_list(s, "kv", "login/github", &metas, &n);                  /* metadata only */
vr_meta_list_free(metas, n);

vr_restore(s, "kv", "login/github", 1, now_ms, &r);            /* re-archive rev 1 as NEW */
vr_revision_free(&r);
```

| Function | Purpose |
|----------|---------|
| `vr_store_create` / `vr_store_destroy` | make / free a thread-safe store |
| `vr_archive` | append a ciphertext; assigns a fresh monotonic id; enforces retention |
| `vr_list` | metadata for every revision at `(ns,key)`, ascending by id (no bytes) |
| `vr_get_revision` | one revision *with* its ciphertext |
| `vr_restore` | re-archive an old revision's bytes as a new revision (append-only) |
| `vr_purge_due` | drop revisions older than `now − max_age` (host-driven) |
| `vr_policy_for` / `vr_set_policy` | per-namespace retention policy |
| `vr_summary_of` | payload-free counts & byte totals |

**`restore` is not a rollback** — the history list is append-only; old revisions
disappear only via the per-namespace **retention policy**: a `max_revisions_per_key`
count cap enforced oldest-first on every archive, and a `max_age_ms` cap the host
drives via `vr_purge_due` (so the crate stays clock-pure). `Option<T>` becomes a
`has_*` flag + value.

**Ciphertext-only.** This layer sees opaque bytes; sealing happens above it. Every
variable-length field is length-bounded (`VR_MAX_*`) and validated (non-empty, no
control / whitespace / Unicode bidi-override / zero-width characters — the same
defence as the Rust crate, with a minimal UTF-8 decoder for the codepoint checks).

## The thread story (why this is a *thread*-bucket port)

The store is guarded by a single **`osp_mutex`**: every operation takes the lock
for its whole read/mutate, so the store is safe to share across threads (the Rust
original's `Mutex<InMemoryInner>` + `Send + Sync`). The test spawns **four worker
threads** (`osp_thread_spawn`) that archive concurrently and asserts all 100
revisions land — no lost updates. The store is a pure consumer of os-platform's
thread primitive; there are no changes to os-platform.

## Build & test

`tests/vault_revisions_test.c` mirrors the Rust crate's tests (archive/list/get/
restore, retention eviction, age purge, policy set/replace, validation, summary)
and adds the concurrent-archive proof.

```sh
cd code/packages/c/vault-revisions
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 109 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks; and — with four concurrent writers — ThreadSanitizer reports no data race.

## Layout

```
vault-revisions/
├── include/vault_revisions/vault_revisions.h   # public API
├── src/vault_revisions.c                        # the store — one OS-agnostic file
├── tests/vault_revisions_test.c                 # tests, incl. concurrent archives
├── tools/run.sh  · run.ps1                       # build with os-platform thread
├── BUILD  · BUILD_windows                        # per-OS build drivers
└── required_capabilities.json                    # CI needs gcc, clang, cl
```

The store composes os-platform's `thread` backend (`osp_mutex`), so the build
compiles that backend and links the OS thread library (`-pthread` on POSIX; the
CRT on Windows). No changes to os-platform itself — a pure consumer.
