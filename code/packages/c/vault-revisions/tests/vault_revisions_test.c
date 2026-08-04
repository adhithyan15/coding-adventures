/*
 * vault_revisions_test.c — the revision store, including concurrent archives.
 * ===========================================================================
 *
 * Mirrors the Rust crate's tests (archive/list/get/restore, retention eviction,
 * age purge, policy, validation, summary) and adds the *thread*-bucket payoff:
 * several worker threads (os-platform osp_thread) archive concurrently and the
 * store — guarded by one osp_mutex — loses no updates.
 */
#include "iso_test.h"

#include "os_platform/thread.h" /* osp_thread_spawn / join */
#include "vault_revisions/vault_revisions.h"

#include <stddef.h>
#include <string.h>

/* Helper: archive a NUL-terminated string as ciphertext, discard the returned
 * revision (freeing its ciphertext). Returns the vr_status. */
static vr_status arch(vr_store *s, const char *ns, const char *key,
                      const char *ct, uint64_t at) {
    vr_revision r;
    vr_status st;
    memset(&r, 0, sizeof(r));
    st = vr_archive(s, ns, key, (const unsigned char *)ct, strlen(ct), at, &r);
    if (st == VR_OK) {
        vr_revision_free(&r);
    }
    return st;
}

/* Cross-thread worker context + body (file scope so the worker's address and the
 * struct type are shared cleanly). Each worker archives 25 revisions to a
 * per-index key "k<idx>". */
struct vr_wk {
    vr_store *store;
    int idx;
};

static void *vr_test_worker(void *arg) {
    struct vr_wk *w = (struct vr_wk *)arg;
    char key[4];
    int j;
    key[0] = 'k';
    key[1] = (char)('0' + w->idx);
    key[2] = '\0';
    for (j = 0; j < 25; j++) {
        vr_revision r;
        memset(&r, 0, sizeof(r));
        if (vr_archive(w->store, "kv", key, (const unsigned char *)"payload", 7,
                       (uint64_t)j, &r) == VR_OK) {
            vr_revision_free(&r);
        }
    }
    return NULL;
}

int main(void) {
    /* ── archive returns id starting at 1; second increments ────────────────── */
    {
        vr_store *s = NULL;
        vr_revision r;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        memset(&r, 0, sizeof(r));
        ISO_CHECK(vr_archive(s, "kv", "k", (const unsigned char *)"v1", 2, 100, &r) == VR_OK);
        ISO_CHECK_EQ_UINT((unsigned)r.id, 1u);
        ISO_CHECK_EQ_UINT(r.ciphertext_len, 2u);
        ISO_CHECK_MEM_EQ(r.ciphertext, "v1", 2);
        vr_revision_free(&r);
        memset(&r, 0, sizeof(r));
        ISO_CHECK(vr_archive(s, "kv", "k", (const unsigned char *)"v2", 2, 200, &r) == VR_OK);
        ISO_CHECK_EQ_UINT((unsigned)r.id, 2u);
        vr_revision_free(&r);
        vr_store_destroy(s);
    }

    /* ── list returns metadata only, ascending by id ────────────────────────── */
    {
        vr_store *s = NULL;
        vr_revision_meta *m = NULL;
        size_t n = 0;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "v1", 100) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "v2-longer", 200) == VR_OK);
        ISO_CHECK(vr_list(s, "kv", "k", &m, &n) == VR_OK);
        ISO_CHECK_EQ_UINT(n, 2u);
        ISO_CHECK_EQ_UINT((unsigned)m[0].id, 1u);
        ISO_CHECK_EQ_UINT((unsigned)m[0].archived_at_ms, 100u);
        ISO_CHECK_EQ_UINT(m[0].ciphertext_len, 2u);
        ISO_CHECK_EQ_UINT((unsigned)m[1].id, 2u);
        ISO_CHECK_EQ_UINT(m[1].ciphertext_len, 9u);
        vr_meta_list_free(m, n);
        vr_store_destroy(s);
    }

    /* ── unknown path: empty list, empty summary ────────────────────────────── */
    {
        vr_store *s = NULL;
        vr_revision_meta *m = (vr_revision_meta *)0x1; /* must be overwritten to NULL */
        size_t n = 99;
        vr_summary sum;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(vr_list(s, "kv", "missing", &m, &n) == VR_OK);
        ISO_CHECK(m == NULL);
        ISO_CHECK_EQ_UINT(n, 0u);
        vr_summary_of(s, &sum);
        ISO_CHECK_EQ_UINT(sum.revision_count, 0u);
        ISO_CHECK_EQ_UINT(sum.history_count, 0u);
        ISO_CHECK_EQ_UINT(sum.namespace_count, 0u);
        ISO_CHECK(sum.has_oldest == 0 && sum.has_newest == 0);
        vr_store_destroy(s);
    }

    /* ── get_revision returns ciphertext; error cases ───────────────────────── */
    {
        vr_store *s = NULL;
        vr_revision r;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "secret", 100) == VR_OK);
        memset(&r, 0, sizeof(r));
        ISO_CHECK(vr_get_revision(s, "kv", "k", 1, &r) == VR_OK);
        ISO_CHECK_EQ_UINT(r.ciphertext_len, 6u);
        ISO_CHECK_MEM_EQ(r.ciphertext, "secret", 6);
        vr_revision_free(&r);
        ISO_CHECK(vr_get_revision(s, "kv", "k", 99, &r) == VR_ERR_UNKNOWN_REVISION);
        ISO_CHECK(vr_get_revision(s, "kv", "nokey", 1, &r) == VR_ERR_NOT_FOUND);
        vr_store_destroy(s);
    }

    /* ── restore re-archives an old revision as a NEW one (append-only) ─────── */
    {
        vr_store *s = NULL;
        vr_revision r;
        vr_revision_meta *m = NULL;
        size_t n = 0;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "AAA", 100) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "BBB", 200) == VR_OK);
        /* restore rev 1 ("AAA") → new rev id 3 with AAA's bytes */
        memset(&r, 0, sizeof(r));
        ISO_CHECK(vr_restore(s, "kv", "k", 1, 300, &r) == VR_OK);
        ISO_CHECK_EQ_UINT((unsigned)r.id, 3u);
        ISO_CHECK_MEM_EQ(r.ciphertext, "AAA", 3);
        vr_revision_free(&r);
        ISO_CHECK(vr_list(s, "kv", "k", &m, &n) == VR_OK);
        ISO_CHECK_EQ_UINT(n, 3u); /* append-only: still 3 revisions */
        vr_meta_list_free(m, n);
        vr_store_destroy(s);
    }

    /* ── retention: max_revisions_per_key evicts oldest-first ───────────────── */
    {
        vr_store *s = NULL;
        vr_retention_policy pol = vr_retention_unbounded();
        vr_revision_meta *m = NULL;
        size_t n = 0;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        pol.has_max_revisions = 1;
        pol.max_revisions_per_key = 2;
        ISO_CHECK(vr_set_policy(s, "kv", &pol) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "r1", 100) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "r2", 200) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "r3", 300) == VR_OK); /* evicts r1 */
        ISO_CHECK(vr_list(s, "kv", "k", &m, &n) == VR_OK);
        ISO_CHECK_EQ_UINT(n, 2u);
        ISO_CHECK_EQ_UINT((unsigned)m[0].id, 2u); /* r1 (id 1) evicted */
        ISO_CHECK_EQ_UINT((unsigned)m[1].id, 3u);
        vr_meta_list_free(m, n);
        /* rev 1 is gone */
        {
            vr_revision r;
            ISO_CHECK(vr_get_revision(s, "kv", "k", 1, &r) == VR_ERR_UNKNOWN_REVISION);
        }
        vr_store_destroy(s);
    }

    /* ── purge_due drops revisions older than now - max_age ─────────────────── */
    {
        vr_store *s = NULL;
        vr_retention_policy pol = vr_retention_unbounded();
        size_t evicted = 99;
        vr_revision_meta *m = NULL;
        size_t n = 0;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "old", 1000) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "mid", 5000) == VR_OK);
        ISO_CHECK(arch(s, "kv", "k", "new", 9000) == VR_OK);
        pol.has_max_age = 1;
        pol.max_age_ms = 3000; /* now=9000 → cutoff 6000; keep >= 6000 (only "new") */
        ISO_CHECK(vr_purge_due(s, "kv", &pol, 9000, &evicted) == VR_OK);
        ISO_CHECK_EQ_UINT(evicted, 2u);
        ISO_CHECK(vr_list(s, "kv", "k", &m, &n) == VR_OK);
        ISO_CHECK_EQ_UINT(n, 1u);
        ISO_CHECK_EQ_UINT((unsigned)m[0].archived_at_ms, 9000u);
        vr_meta_list_free(m, n);
        /* no age bound → purge is a no-op */
        {
            vr_retention_policy none = vr_retention_unbounded();
            size_t e2 = 99;
            ISO_CHECK(vr_purge_due(s, "kv", &none, 100000, &e2) == VR_OK);
            ISO_CHECK_EQ_UINT(e2, 0u);
        }
        vr_store_destroy(s);
    }

    /* ── policy_for default when unset; set/replace; reject max=0 ───────────── */
    {
        vr_store *s = NULL;
        vr_retention_policy got;
        vr_retention_policy p = vr_retention_unbounded();
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        vr_policy_for(s, "kv", &got);
        /* default password-manager policy: 32 revisions, 90 days */
        ISO_CHECK(got.has_max_revisions == 1 && got.max_revisions_per_key == 32u);
        ISO_CHECK(got.has_max_age == 1);
        p.has_max_revisions = 1;
        p.max_revisions_per_key = 5;
        ISO_CHECK(vr_set_policy(s, "kv", &p) == VR_OK);
        vr_policy_for(s, "kv", &got);
        ISO_CHECK(got.has_max_revisions == 1 && got.max_revisions_per_key == 5u);
        /* replace */
        p.max_revisions_per_key = 7;
        ISO_CHECK(vr_set_policy(s, "kv", &p) == VR_OK);
        vr_policy_for(s, "kv", &got);
        ISO_CHECK_EQ_UINT(got.max_revisions_per_key, 7u);
        /* reject max_revisions == 0 with the flag set */
        p.has_max_revisions = 1;
        p.max_revisions_per_key = 0;
        ISO_CHECK(vr_set_policy(s, "kv", &p) == VR_ERR_INVALID_PARAMETER);
        vr_store_destroy(s);
    }

    /* ── validation: empty / oversize / forbidden chars ─────────────────────── */
    {
        vr_store *s = NULL;
        vr_revision r;
        char big_key[VR_MAX_KEY_LEN + 8];
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(arch(s, "", "k", "v", 1) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(arch(s, "kv", "", "v", 1) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(arch(s, "kv", "k", "", 1) == VR_ERR_INVALID_PARAMETER); /* empty ct */
        ISO_CHECK(arch(s, "kv", "has space", "v", 1) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(arch(s, "kv", "tab\ttab", "v", 1) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(arch(s, "kv", "ctl\x01", "v", 1) == VR_ERR_INVALID_PARAMETER);
        /* a zero-width space (U+200B = e2 80 8b) is rejected */
        ISO_CHECK(arch(s, "kv", "zw\xe2\x80\x8bkey", "v", 1) == VR_ERR_INVALID_PARAMETER);
        memset(big_key, 'a', sizeof(big_key));
        big_key[sizeof(big_key) - 1] = '\0';
        ISO_CHECK(arch(s, "kv", big_key, "v", 1) == VR_ERR_INVALID_PARAMETER);
        /* a plain ASCII key is fine */
        memset(&r, 0, sizeof(r));
        ISO_CHECK(vr_archive(s, "kv", "ok-key", (const unsigned char *)"v", 1, 1, &r) == VR_OK);
        vr_revision_free(&r);
        vr_store_destroy(s);
    }

    /* ── summary reports counts + totals across namespaces ──────────────────── */
    {
        vr_store *s = NULL;
        vr_summary sum;
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(arch(s, "ns1", "a", "xx", 100) == VR_OK);   /* 2 bytes */
        ISO_CHECK(arch(s, "ns1", "a", "yyy", 200) == VR_OK);  /* 3 bytes */
        ISO_CHECK(arch(s, "ns2", "b", "z", 50) == VR_OK);     /* 1 byte, oldest */
        vr_summary_of(s, &sum);
        ISO_CHECK_EQ_UINT(sum.history_count, 2u);      /* (ns1,a) and (ns2,b) */
        ISO_CHECK_EQ_UINT(sum.namespace_count, 2u);
        ISO_CHECK_EQ_UINT(sum.revision_count, 3u);
        ISO_CHECK_EQ_UINT((unsigned)sum.total_ciphertext_bytes, 6u);
        ISO_CHECK_EQ_UINT(sum.largest_history_len, 2u);
        ISO_CHECK(sum.has_oldest && sum.oldest_archived_at_ms == 50u);
        ISO_CHECK(sum.has_newest && sum.newest_archived_at_ms == 200u);
        vr_store_destroy(s);
    }

    /* ── CROSS-THREAD: concurrent archives lose no updates ──────────────────── */
    {
        vr_store *s = NULL;
        vr_summary sum;
        osp_thread *workers[4];
        struct vr_wk ctx[4];
        int i;

        /* Each worker archives 25 revisions to its own key; with 4 workers that
         * is 100 revisions across 4 histories — none lost if the mutex holds. */
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        for (i = 0; i < 4; i++) {
            ctx[i].store = s;
            ctx[i].idx = i;
            ISO_CHECK(osp_thread_spawn(&workers[i], vr_test_worker, &ctx[i]) == OSP_OK);
        }
        for (i = 0; i < 4; i++) {
            ISO_CHECK(osp_thread_join(workers[i], NULL) == OSP_OK);
        }
        vr_summary_of(s, &sum);
        ISO_CHECK_EQ_UINT(sum.history_count, 4u);
        ISO_CHECK_EQ_UINT(sum.revision_count, 100u); /* 4 workers × 25, none lost */
        vr_store_destroy(s);
    }

    /* ── NULL / argument validation ─────────────────────────────────────────── */
    {
        vr_store *s = NULL;
        vr_revision r;
        vr_revision_meta *m = NULL;
        size_t n = 0;
        vr_retention_policy p = vr_retention_unbounded();
        size_t e = 0;
        ISO_CHECK(vr_store_create(NULL) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(vr_store_create(&s) == VR_OK);
        ISO_CHECK(vr_archive(NULL, "kv", "k", (const unsigned char *)"v", 1, 1, &r) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(vr_archive(s, "kv", "k", (const unsigned char *)"v", 1, 1, NULL) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(vr_list(NULL, "kv", "k", &m, &n) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(vr_get_revision(NULL, "kv", "k", 1, &r) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(vr_purge_due(NULL, "kv", &p, 1, &e) == VR_ERR_INVALID_PARAMETER);
        ISO_CHECK(vr_set_policy(NULL, "kv", &p) == VR_ERR_INVALID_PARAMETER);
        vr_store_destroy(NULL); /* no-op, must not crash */
        vr_store_destroy(s);
    }

    return ISO_TEST_RESULT();
}
