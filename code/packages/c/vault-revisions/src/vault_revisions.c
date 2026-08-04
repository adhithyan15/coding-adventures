/*
 * vault_revisions.c — thread-safe in-memory revision history, on os-platform.
 * ===========================================================================
 *
 * A namespace/key → revision-history store guarded by one osp_mutex. The Rust
 * reference uses a BTreeMap<(ns,key), History> + HashMap<ns, Policy>; the map's
 * iteration ORDER is not observable through this API (list() is per-(ns,key) and
 * returns by ascending id = push order; summary() aggregates order-independently;
 * purge_due() evicts order-independently), so a plain growable array with linear
 * lookup is a faithful and simpler realization.
 *
 * Every operation takes the store lock for its whole read/mutate, so the store is
 * safe to share across threads — the concurrency the *thread*-bucket port proves.
 */
#include "vault_revisions/vault_revisions.h"

#include "os_platform/thread.h" /* osp_mutex — guards the store */

#include <stdlib.h>
#include <string.h>

/* One key's append-only revision list. */
struct vr_history {
    char *ns;
    char *key;
    vr_revision *revs; /* each owns a ciphertext copy */
    size_t nrev, crev;
    uint64_t next_id;
};

/* A namespace's configured policy. */
struct vr_policy_entry {
    char *ns;
    vr_retention_policy pol;
};

struct vr_store {
    osp_mutex *lock;
    struct vr_history *hist;
    size_t nhist, chist;
    struct vr_policy_entry *pols;
    size_t npol, cpol;
};

/* ── small helpers ────────────────────────────────────────────────────────── */

static char *vr__strdup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) {
        memcpy(p, s, n);
    }
    return p;
}

static unsigned char *vr__memdup(const unsigned char *p, size_t n) {
    unsigned char *q = (unsigned char *)malloc(n == 0 ? 1 : n);
    if (q != NULL && n != 0) {
        memcpy(q, p, n);
    }
    return q;
}

/*
 * Reject identifiers with control characters, whitespace, or Unicode
 * bidi-override / zero-width codepoints — the same defence as the Rust crate.
 * Input is UTF-8; a minimal decoder recovers codepoints for the multibyte checks,
 * and invalid UTF-8 is rejected (safe default).
 */
static int vr__cp_forbidden(unsigned long cp) {
    /* ASCII control + DEL */
    if (cp < 0x20 || cp == 0x7F) {
        return 1;
    }
    /* ASCII whitespace */
    if (cp == ' ' || cp == '\t' || cp == '\n' || cp == '\r' || cp == '\f' ||
        cp == '\v') {
        return 1;
    }
    /* Common Unicode whitespace */
    if (cp == 0x0085 || cp == 0x00A0 || cp == 0x1680 ||
        (cp >= 0x2000 && cp <= 0x200A) || cp == 0x2028 || cp == 0x2029 ||
        cp == 0x202F || cp == 0x205F || cp == 0x3000) {
        return 1;
    }
    /* Bidi overrides / isolates + zero-width + BOM */
    if ((cp >= 0x202A && cp <= 0x202E) || (cp >= 0x2066 && cp <= 0x2069) ||
        (cp >= 0x200B && cp <= 0x200D) || cp == 0xFEFF) {
        return 1;
    }
    return 0;
}

static int vr__is_safe_id(const char *s, size_t len) {
    size_t i = 0;
    while (i < len) {
        unsigned char b0 = (unsigned char)s[i];
        unsigned long cp;
        size_t need;
        if (b0 < 0x80) {
            cp = b0;
            need = 0;
        } else if ((b0 & 0xE0) == 0xC0) {
            cp = b0 & 0x1F;
            need = 1;
        } else if ((b0 & 0xF0) == 0xE0) {
            cp = b0 & 0x0F;
            need = 2;
        } else if ((b0 & 0xF8) == 0xF0) {
            cp = b0 & 0x07;
            need = 3;
        } else {
            return 0; /* invalid lead byte */
        }
        if (i + need + 1 > len) {
            return 0; /* truncated multibyte sequence */
        }
        {
            size_t k;
            for (k = 1; k <= need; k++) {
                unsigned char bc = (unsigned char)s[i + k];
                if ((bc & 0xC0) != 0x80) {
                    return 0; /* bad continuation byte */
                }
                cp = (cp << 6) | (bc & 0x3F);
            }
        }
        /* Reject invalid UTF-8 the way the Rust reference does: overlong
         * encodings (a codepoint below the minimal length for its byte count),
         * UTF-16 surrogates, and out-of-range codepoints. */
        {
            static const unsigned long min_cp[4] = {0, 0x80, 0x800, 0x10000};
            if (cp < min_cp[need] || (cp >= 0xD800 && cp <= 0xDFFF) ||
                cp > 0x10FFFF) {
                return 0;
            }
        }
        if (vr__cp_forbidden(cp)) {
            return 0;
        }
        i += need + 1;
    }
    return 1;
}

static vr_status vr__validate_namespace(const char *ns) {
    size_t n;
    if (ns == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    n = strlen(ns);
    if (n == 0 || n > VR_MAX_NAMESPACE_LEN || !vr__is_safe_id(ns, n)) {
        return VR_ERR_INVALID_PARAMETER;
    }
    return VR_OK;
}

static vr_status vr__validate_ns_key(const char *ns, const char *key) {
    size_t n;
    vr_status st = vr__validate_namespace(ns);
    if (st != VR_OK) {
        return st;
    }
    if (key == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    n = strlen(key);
    if (n == 0 || n > VR_MAX_KEY_LEN || !vr__is_safe_id(key, n)) {
        return VR_ERR_INVALID_PARAMETER;
    }
    return VR_OK;
}

static vr_status vr__validate_ciphertext(const unsigned char *ct, size_t len) {
    if (ct == NULL || len == 0 || len > VR_MAX_CIPHERTEXT_LEN) {
        return VR_ERR_INVALID_PARAMETER;
    }
    return VR_OK;
}

static vr_status vr__validate_retention(const vr_retention_policy *p) {
    if (p->has_max_revisions && p->max_revisions_per_key == 0) {
        return VR_ERR_INVALID_PARAMETER;
    }
    return VR_OK;
}

/* ── presets ──────────────────────────────────────────────────────────────── */

vr_retention_policy vr_retention_default_password_manager(void) {
    vr_retention_policy p;
    p.has_max_revisions = 1;
    p.max_revisions_per_key = 32;
    p.has_max_age = 1;
    p.max_age_ms = (uint64_t)90 * 24 * 60 * 60 * 1000;
    return p;
}

vr_retention_policy vr_retention_unbounded(void) {
    vr_retention_policy p;
    p.has_max_revisions = 0;
    p.max_revisions_per_key = 0;
    p.has_max_age = 0;
    p.max_age_ms = 0;
    return p;
}

/* ── store internals (all called with the lock held) ──────────────────────── */

static struct vr_history *vr__find_history(vr_store *s, const char *ns,
                                           const char *key) {
    size_t i;
    for (i = 0; i < s->nhist; i++) {
        if (strcmp(s->hist[i].ns, ns) == 0 && strcmp(s->hist[i].key, key) == 0) {
            return &s->hist[i];
        }
    }
    return NULL;
}

static struct vr_history *vr__find_or_create_history(vr_store *s, const char *ns,
                                                     const char *key) {
    struct vr_history *h = vr__find_history(s, ns, key);
    if (h != NULL) {
        return h;
    }
    if (s->nhist == s->chist) {
        size_t ncap = s->chist ? s->chist * 2 : 8;
        struct vr_history *na =
            (struct vr_history *)realloc(s->hist, ncap * sizeof(*na));
        if (na == NULL) {
            return NULL;
        }
        s->hist = na;
        s->chist = ncap;
    }
    h = &s->hist[s->nhist];
    h->ns = vr__strdup(ns);
    h->key = vr__strdup(key);
    if (h->ns == NULL || h->key == NULL) {
        free(h->ns);
        free(h->key);
        return NULL;
    }
    h->revs = NULL;
    h->nrev = 0;
    h->crev = 0;
    h->next_id = 1;
    s->nhist++;
    return h;
}

static vr_retention_policy vr__policy_for_locked(vr_store *s, const char *ns) {
    size_t i;
    for (i = 0; i < s->npol; i++) {
        if (strcmp(s->pols[i].ns, ns) == 0) {
            return s->pols[i].pol;
        }
    }
    return vr_retention_default_password_manager();
}

/* Evict oldest-first while the history exceeds the count cap. */
static void vr__enforce_max(struct vr_history *h,
                            const vr_retention_policy *pol) {
    if (!pol->has_max_revisions) {
        return;
    }
    while (h->nrev > pol->max_revisions_per_key) {
        free(h->revs[0].ciphertext);
        memmove(&h->revs[0], &h->revs[1],
                (h->nrev - 1) * sizeof(h->revs[0]));
        h->nrev--;
    }
}

/* Fill out_rev with a fresh copy of an in-store revision. */
static vr_status vr__copy_out(const vr_revision *src, vr_revision *out) {
    out->id = src->id;
    out->archived_at_ms = src->archived_at_ms;
    out->ciphertext_len = src->ciphertext_len;
    out->ciphertext = vr__memdup(src->ciphertext, src->ciphertext_len);
    if (out->ciphertext == NULL) {
        return VR_ERR_NOMEM;
    }
    return VR_OK;
}

/* ── public API ───────────────────────────────────────────────────────────── */

vr_status vr_store_create(vr_store **out) {
    vr_store *s;
    if (out == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    s = (vr_store *)calloc(1, sizeof(*s));
    if (s == NULL) {
        return VR_ERR_NOMEM;
    }
    if (osp_mutex_init(&s->lock) != OSP_OK) {
        free(s);
        return VR_ERR_NOMEM;
    }
    *out = s;
    return VR_OK;
}

void vr_store_destroy(vr_store *s) {
    size_t i, j;
    if (s == NULL) {
        return;
    }
    for (i = 0; i < s->nhist; i++) {
        for (j = 0; j < s->hist[i].nrev; j++) {
            free(s->hist[i].revs[j].ciphertext);
        }
        free(s->hist[i].revs);
        free(s->hist[i].ns);
        free(s->hist[i].key);
    }
    free(s->hist);
    for (i = 0; i < s->npol; i++) {
        free(s->pols[i].ns);
    }
    free(s->pols);
    if (s->lock != NULL) {
        osp_mutex_destroy(s->lock);
    }
    free(s);
}

vr_status vr_archive(vr_store *s, const char *namespace, const char *key,
                     const unsigned char *ciphertext, size_t ciphertext_len,
                     uint64_t archived_at_ms, vr_revision *out_rev) {
    vr_status st;
    struct vr_history *h;
    vr_retention_policy pol;
    unsigned char *ct_copy;
    vr_revision *slot;

    if (s == NULL || out_rev == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    st = vr__validate_ns_key(namespace, key);
    if (st != VR_OK) {
        return st;
    }
    st = vr__validate_ciphertext(ciphertext, ciphertext_len);
    if (st != VR_OK) {
        return st;
    }

    osp_mutex_lock(s->lock);
    h = vr__find_or_create_history(s, namespace, key);
    if (h == NULL) {
        osp_mutex_unlock(s->lock);
        return VR_ERR_NOMEM;
    }
    if (h->next_id == UINT64_MAX) {
        /* next allocation would wrap: overflow (unreachable in practice). */
        osp_mutex_unlock(s->lock);
        return VR_ERR_OVERFLOW;
    }
    if (h->nrev == h->crev) {
        size_t ncap = h->crev ? h->crev * 2 : 4;
        vr_revision *na = (vr_revision *)realloc(h->revs, ncap * sizeof(*na));
        if (na == NULL) {
            osp_mutex_unlock(s->lock);
            return VR_ERR_NOMEM;
        }
        h->revs = na;
        h->crev = ncap;
    }
    ct_copy = vr__memdup(ciphertext, ciphertext_len);
    if (ct_copy == NULL) {
        osp_mutex_unlock(s->lock);
        return VR_ERR_NOMEM;
    }
    slot = &h->revs[h->nrev];
    slot->id = h->next_id;
    slot->archived_at_ms = archived_at_ms;
    slot->ciphertext = ct_copy;
    slot->ciphertext_len = ciphertext_len;
    h->next_id++;
    h->nrev++;

    /* Copy out BEFORE enforce_max, so a cap of exactly the new count still
     * returns the just-archived revision (enforce never evicts the newest). */
    st = vr__copy_out(slot, out_rev);
    if (st != VR_OK) {
        /* undo the push on OOM so the store stays consistent */
        free(slot->ciphertext);
        h->nrev--;
        h->next_id--;
        osp_mutex_unlock(s->lock);
        return st;
    }
    pol = vr__policy_for_locked(s, namespace);
    vr__enforce_max(h, &pol);
    osp_mutex_unlock(s->lock);
    return VR_OK;
}

vr_status vr_list(vr_store *s, const char *namespace, const char *key,
                  vr_revision_meta **out_metas, size_t *out_count) {
    vr_status st;
    struct vr_history *h;
    vr_revision_meta *metas = NULL;
    size_t i;

    if (s == NULL || out_metas == NULL || out_count == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    *out_metas = NULL;
    *out_count = 0;
    st = vr__validate_ns_key(namespace, key);
    if (st != VR_OK) {
        return st;
    }
    osp_mutex_lock(s->lock);
    h = vr__find_history(s, namespace, key);
    if (h != NULL && h->nrev > 0) {
        metas = (vr_revision_meta *)malloc(h->nrev * sizeof(*metas));
        if (metas == NULL) {
            osp_mutex_unlock(s->lock);
            return VR_ERR_NOMEM;
        }
        for (i = 0; i < h->nrev; i++) {
            metas[i].id = h->revs[i].id;
            metas[i].archived_at_ms = h->revs[i].archived_at_ms;
            metas[i].ciphertext_len = h->revs[i].ciphertext_len;
        }
        *out_metas = metas;
        *out_count = h->nrev;
    }
    osp_mutex_unlock(s->lock);
    return VR_OK;
}

vr_status vr_get_revision(vr_store *s, const char *namespace, const char *key,
                          uint64_t id, vr_revision *out_rev) {
    vr_status st;
    struct vr_history *h;
    size_t i;

    if (s == NULL || out_rev == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    st = vr__validate_ns_key(namespace, key);
    if (st != VR_OK) {
        return st;
    }
    osp_mutex_lock(s->lock);
    h = vr__find_history(s, namespace, key);
    if (h == NULL) {
        osp_mutex_unlock(s->lock);
        return VR_ERR_NOT_FOUND;
    }
    for (i = 0; i < h->nrev; i++) {
        if (h->revs[i].id == id) {
            st = vr__copy_out(&h->revs[i], out_rev);
            osp_mutex_unlock(s->lock);
            return st;
        }
    }
    osp_mutex_unlock(s->lock);
    return VR_ERR_UNKNOWN_REVISION;
}

vr_status vr_restore(vr_store *s, const char *namespace, const char *key,
                     uint64_t id, uint64_t archived_at_ms, vr_revision *out_rev) {
    vr_revision old;
    vr_status st;

    if (s == NULL || out_rev == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    memset(&old, 0, sizeof(old));
    st = vr_get_revision(s, namespace, key, id, &old);
    if (st != VR_OK) {
        return st;
    }
    st = vr_archive(s, namespace, key, old.ciphertext, old.ciphertext_len,
                    archived_at_ms, out_rev);
    vr_revision_free(&old);
    return st;
}

vr_status vr_purge_due(vr_store *s, const char *namespace,
                       const vr_retention_policy *retention, uint64_t now_ms,
                       size_t *out_evicted) {
    vr_status st;
    uint64_t cutoff;
    size_t i, total = 0;

    if (s == NULL || retention == NULL || out_evicted == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    *out_evicted = 0;
    st = vr__validate_namespace(namespace);
    if (st != VR_OK) {
        return st;
    }
    if (!retention->has_max_age) {
        return VR_OK; /* no age bound → nothing to purge */
    }
    /* saturating subtraction: anything strictly older than the cut-off is dropped */
    cutoff = (now_ms >= retention->max_age_ms) ? now_ms - retention->max_age_ms
                                               : 0;
    osp_mutex_lock(s->lock);
    for (i = 0; i < s->nhist; i++) {
        struct vr_history *h = &s->hist[i];
        size_t r, w = 0;
        if (strcmp(h->ns, namespace) != 0) {
            continue;
        }
        for (r = 0; r < h->nrev; r++) {
            if (h->revs[r].archived_at_ms >= cutoff) {
                if (w != r) {
                    h->revs[w] = h->revs[r];
                }
                w++;
            } else {
                free(h->revs[r].ciphertext);
                total++;
            }
        }
        h->nrev = w;
    }
    osp_mutex_unlock(s->lock);
    *out_evicted = total;
    return VR_OK;
}

void vr_policy_for(vr_store *s, const char *namespace,
                   vr_retention_policy *out_policy) {
    if (s == NULL || out_policy == NULL) {
        return;
    }
    if (namespace == NULL) {
        *out_policy = vr_retention_default_password_manager();
        return;
    }
    osp_mutex_lock(s->lock);
    *out_policy = vr__policy_for_locked(s, namespace);
    osp_mutex_unlock(s->lock);
}

vr_status vr_set_policy(vr_store *s, const char *namespace,
                        const vr_retention_policy *policy) {
    vr_status st;
    size_t i;

    if (s == NULL || policy == NULL) {
        return VR_ERR_INVALID_PARAMETER;
    }
    st = vr__validate_namespace(namespace);
    if (st != VR_OK) {
        return st;
    }
    st = vr__validate_retention(policy);
    if (st != VR_OK) {
        return st;
    }
    osp_mutex_lock(s->lock);
    for (i = 0; i < s->npol; i++) {
        if (strcmp(s->pols[i].ns, namespace) == 0) {
            s->pols[i].pol = *policy; /* replace */
            osp_mutex_unlock(s->lock);
            return VR_OK;
        }
    }
    if (s->npol == s->cpol) {
        size_t ncap = s->cpol ? s->cpol * 2 : 8;
        struct vr_policy_entry *na =
            (struct vr_policy_entry *)realloc(s->pols, ncap * sizeof(*na));
        if (na == NULL) {
            osp_mutex_unlock(s->lock);
            return VR_ERR_NOMEM;
        }
        s->pols = na;
        s->cpol = ncap;
    }
    s->pols[s->npol].ns = vr__strdup(namespace);
    if (s->pols[s->npol].ns == NULL) {
        osp_mutex_unlock(s->lock);
        return VR_ERR_NOMEM;
    }
    s->pols[s->npol].pol = *policy;
    s->npol++;
    osp_mutex_unlock(s->lock);
    return VR_OK;
}

void vr_summary_of(vr_store *s, vr_summary *out) {
    size_t i, j;
    const char **seen_ns;
    size_t nseen = 0;

    if (s == NULL || out == NULL) {
        return;
    }
    memset(out, 0, sizeof(*out));
    osp_mutex_lock(s->lock);
    out->history_count = s->nhist;
    out->configured_policy_count = s->npol;
    /* Count distinct namespaces among histories via a scratch pointer set. */
    seen_ns = (const char **)malloc((s->nhist ? s->nhist : 1) * sizeof(*seen_ns));
    for (i = 0; i < s->nhist; i++) {
        struct vr_history *h = &s->hist[i];
        if (h->nrev == 0) {
            out->empty_history_count++;
        } else {
            out->non_empty_history_count++;
        }
        if (h->nrev > out->largest_history_len) {
            out->largest_history_len = h->nrev;
        }
        if (seen_ns != NULL) {
            int found = 0;
            for (j = 0; j < nseen; j++) {
                if (strcmp(seen_ns[j], h->ns) == 0) {
                    found = 1;
                    break;
                }
            }
            if (!found) {
                seen_ns[nseen++] = h->ns;
            }
        }
        for (j = 0; j < h->nrev; j++) {
            uint64_t at = h->revs[j].archived_at_ms;
            out->revision_count++;
            out->total_ciphertext_bytes += (uint64_t)h->revs[j].ciphertext_len;
            if (!out->has_oldest || at < out->oldest_archived_at_ms) {
                out->has_oldest = 1;
                out->oldest_archived_at_ms = at;
            }
            if (!out->has_newest || at > out->newest_archived_at_ms) {
                out->has_newest = 1;
                out->newest_archived_at_ms = at;
            }
        }
    }
    out->namespace_count = (seen_ns != NULL) ? nseen : 0;
    osp_mutex_unlock(s->lock);
    free(seen_ns);
}

void vr_revision_free(vr_revision *rev) {
    if (rev != NULL) {
        free(rev->ciphertext);
        rev->ciphertext = NULL;
        rev->ciphertext_len = 0;
    }
}

void vr_meta_list_free(vr_revision_meta *metas, size_t count) {
    (void)count;
    free(metas);
}
