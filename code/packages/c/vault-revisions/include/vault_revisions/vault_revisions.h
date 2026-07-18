/*
 * vault_revisions/vault_revisions.h — revision history for the Vault stack.
 * ===========================================================================
 *
 * The C port of the Rust `vault-revisions` crate (VLT12), and the second consumer
 * of the CCPP02 **thread** slice: a thread-safe in-memory store proving a real
 * Mutex-guarded crate runs on os-platform's `osp_mutex`.
 *
 * WHAT IT IS. Every `put` in a password/secrets manager archives the PRIOR
 * ciphertext to a per-`(namespace, key)` history list, so a user can list past
 * revisions and restore one:
 *
 *      archive(ns, key, ciphertext, at_ms)  -> a Revision with a fresh monotonic id
 *      list(ns, key)                        -> metadata for every revision (no bytes)
 *      get_revision(ns, key, id)            -> one revision WITH its ciphertext
 *      restore(ns, key, id, at_ms)          -> re-archive an old revision's bytes as NEW
 *      purge_due(ns, policy, now_ms)        -> drop revisions older than the age bound
 *
 * `restore` is NOT a rollback — the history list is append-only; old revisions
 * disappear only via the per-namespace retention policy (a max-count cap enforced
 * on every archive, and an age cap the host drives via purge_due).
 *
 * CIPHERTEXT-ONLY. This layer sees only opaque bytes; sealing happens above it.
 * Every variable-length field is length-bounded and validated on the way in.
 *
 * THREAD STORY (why this is a *thread*-bucket port). The store is guarded by a
 * single `osp_mutex`, so it is safe to share across threads — every operation
 * takes the lock for the duration of its read/mutate. The test drives concurrent
 * archives from multiple worker threads (osp_thread_spawn) and asserts no lost
 * updates. The store is a pure consumer of os-platform's thread primitive; there
 * are no changes to os-platform.
 *
 * OWNERSHIP. Functions that return ciphertext (vr_archive, vr_get_revision,
 * vr_restore) fill a caller-owned `vr_revision` whose `ciphertext` is a fresh
 * copy — free it with vr_revision_free. vr_list returns a malloc'd array of
 * `vr_revision_meta` — free it with vr_meta_list_free.
 */
#ifndef VAULT_REVISIONS_VAULT_REVISIONS_H
#define VAULT_REVISIONS_VAULT_REVISIONS_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Bounds (bytes) ───────────────────────────────────────────────────────── */
#define VR_MAX_CIPHERTEXT_LEN (1024u * 1024u) /* one ciphertext payload */
#define VR_MAX_NAMESPACE_LEN 128u
#define VR_MAX_KEY_LEN 512u

/* Every result the revision layer can produce. Mirrors the Rust RevisionError. */
typedef enum {
    VR_OK = 0,
    VR_ERR_INVALID_PARAMETER, /* oversize / empty / forbidden char */
    VR_ERR_UNKNOWN_REVISION,  /* the (ns,key) exists but not that revision id */
    VR_ERR_NOT_FOUND,         /* the (ns,key) has no archived revisions */
    VR_ERR_OVERFLOW,          /* revision id counter overflowed (unreachable) */
    VR_ERR_NOMEM              /* allocation failure (C-specific) */
} vr_status;

/*
 * One archived revision, with its opaque ciphertext. `ciphertext` is owned by the
 * struct after vr_archive / vr_get_revision / vr_restore fill it; release with
 * vr_revision_free.
 */
typedef struct {
    uint64_t id;             /* monotonic per-(ns,key) id, starts at 1 */
    uint64_t archived_at_ms; /* caller-supplied wall-clock, ms since epoch */
    unsigned char *ciphertext;
    size_t ciphertext_len;
} vr_revision;

/* A payload-free metadata view of a revision (what vr_list returns). */
typedef struct {
    uint64_t id;
    uint64_t archived_at_ms;
    size_t ciphertext_len;
} vr_revision_meta;

/*
 * Per-namespace retention policy. Rust's Option<T> becomes a has_* flag + value:
 * has_max_revisions == 0 → unbounded by count; has_max_age == 0 → unbounded by age.
 */
typedef struct {
    int has_max_revisions;
    size_t max_revisions_per_key; /* >= 1 when has_max_revisions */
    int has_max_age;
    uint64_t max_age_ms;
} vr_retention_policy;

/* A reasonable default: 32 revisions per key, 90 days. */
vr_retention_policy vr_retention_default_password_manager(void);
/* "Never evict" — both bounds disabled. */
vr_retention_policy vr_retention_unbounded(void);

/*
 * A payload-free snapshot of the store — counts and byte totals only, no
 * namespaces / keys / ids / ciphertext, so hosts can report storage pressure
 * without widening their read surface.
 */
typedef struct {
    size_t history_count;           /* (ns,key) rows retained */
    size_t non_empty_history_count; /* rows with >= 1 revision */
    size_t empty_history_count;     /* rows with 0 revisions */
    size_t namespace_count;         /* distinct namespaces with rows */
    size_t configured_policy_count; /* namespaces with an explicit policy */
    size_t revision_count;          /* total revisions across all rows */
    uint64_t total_ciphertext_bytes;
    size_t largest_history_len;
    int has_oldest;                 /* 0 → no revisions retained */
    uint64_t oldest_archived_at_ms;
    int has_newest;
    uint64_t newest_archived_at_ms;
} vr_summary;

/* Opaque thread-safe in-memory store. */
typedef struct vr_store vr_store;

/* Create / destroy a store. create writes NULL-checked *out. */
vr_status vr_store_create(vr_store **out);
void vr_store_destroy(vr_store *s);

/*
 * Archive a ciphertext under (namespace, key) at time archived_at_ms. Assigns the
 * next monotonic id (1 for a key's first archive), enforces the namespace's
 * max-count retention, and fills *out_rev (whose ciphertext is a fresh copy —
 * free with vr_revision_free). VR_ERR_INVALID_PARAMETER / VR_ERR_OVERFLOW /
 * VR_ERR_NOMEM.
 */
vr_status vr_archive(vr_store *s, const char *namespace, const char *key,
                     const unsigned char *ciphertext, size_t ciphertext_len,
                     uint64_t archived_at_ms, vr_revision *out_rev);

/*
 * List metadata for every revision at (namespace, key), ascending by id. Writes a
 * malloc'd array through *out_metas (NULL when *out_count == 0) — free with
 * vr_meta_list_free. An unknown path is not an error: it yields an empty list.
 */
vr_status vr_list(vr_store *s, const char *namespace, const char *key,
                  vr_revision_meta **out_metas, size_t *out_count);

/*
 * Fetch one revision by id, with its ciphertext, into *out_rev (free with
 * vr_revision_free). VR_ERR_NOT_FOUND (no history) / VR_ERR_UNKNOWN_REVISION.
 */
vr_status vr_get_revision(vr_store *s, const char *namespace, const char *key,
                          uint64_t id, vr_revision *out_rev);

/*
 * Restore: fetch revision `id`, then archive its ciphertext as a NEW revision.
 * Fills *out_rev with the new revision. Append-only — not a rollback.
 */
vr_status vr_restore(vr_store *s, const char *namespace, const char *key,
                     uint64_t id, uint64_t archived_at_ms, vr_revision *out_rev);

/*
 * Evict revisions older than now_ms - retention.max_age_ms across a namespace.
 * Writes the count evicted through *out_evicted. If retention has no age bound,
 * evicts nothing. now_ms is trusted host input.
 */
vr_status vr_purge_due(vr_store *s, const char *namespace,
                       const vr_retention_policy *retention, uint64_t now_ms,
                       size_t *out_evicted);

/*
 * The effective policy for a namespace (infallible): the configured policy, or the
 * default when none/invalid. Written through *out_policy.
 */
void vr_policy_for(vr_store *s, const char *namespace,
                   vr_retention_policy *out_policy);

/*
 * Set (replace) a namespace's policy. Validates the namespace and rejects
 * max_revisions_per_key == 0 with has_max_revisions set (use has_max_revisions=0
 * to disable). VR_ERR_INVALID_PARAMETER / VR_ERR_NOMEM.
 */
vr_status vr_set_policy(vr_store *s, const char *namespace,
                        const vr_retention_policy *policy);

/* A payload-free summary of the store. */
void vr_summary_of(vr_store *s, vr_summary *out_summary);

/* Release the ciphertext a vr_revision owns (safe on a zeroed struct). */
void vr_revision_free(vr_revision *rev);
/* Release a vr_list result. */
void vr_meta_list_free(vr_revision_meta *metas, size_t count);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* VAULT_REVISIONS_VAULT_REVISIONS_H */
