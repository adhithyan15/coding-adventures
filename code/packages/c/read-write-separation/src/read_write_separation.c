/*
 * read_write_separation.c — capability classification + manifest validation.
 * ===========================================================================
 * All the analysis is string comparison over (category, action, target) triples
 * and small enum logic. See read_write_separation.h for the model.
 */
#include "read_write_separation.h"

#include <stdlib.h>
#include <string.h>

/* ---------------------------------------------------------------------------
 *  String helpers
 * ------------------------------------------------------------------------- */

static int str_eq(const char *a, const char *b) { return strcmp(a, b) == 0; }

static int starts_with(const char *s, const char *prefix) {
    size_t lp = strlen(prefix);
    return strncmp(s, prefix, lp) == 0;
}

static char *dup_cstr(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (out) {
        memcpy(out, s, n + 1);
    }
    return out;
}

/* ---------------------------------------------------------------------------
 *  Enum names
 * ------------------------------------------------------------------------- */

const char *rws_flavor_str(RwsFlavor f) {
    switch (f) {
        case RWS_FLAVOR_INGESTION: return "ingestion";
        case RWS_FLAVOR_ACTUATION: return "actuation";
        case RWS_FLAVOR_INTERNAL: return "internal";
    }
    return "internal";
}

const char *rws_trust_str(RwsTrust t) {
    switch (t) {
        case RWS_TRUST_TRUSTED: return "trusted";
        case RWS_TRUST_UNTRUSTED: return "untrusted";
    }
    return "trusted";
}

/* ---------------------------------------------------------------------------
 *  Capability lifecycle
 * ------------------------------------------------------------------------- */

int rws_capability_init(RwsCapability *c, const char *category,
                        const char *action, const char *target) {
    c->category = dup_cstr(category);
    c->action = dup_cstr(action);
    c->target = dup_cstr(target);
    c->has_flavor = 0;
    c->flavor = RWS_FLAVOR_INTERNAL;
    c->has_trust = 0;
    c->trust = RWS_TRUST_TRUSTED;
    c->justification = NULL;
    if (!c->category || !c->action || !c->target) {
        rws_capability_release(c);
        return -1;
    }
    return 0;
}

void rws_capability_set_flavor(RwsCapability *c, RwsFlavor f) {
    c->has_flavor = 1;
    c->flavor = f;
}

void rws_capability_set_trust(RwsCapability *c, RwsTrust t) {
    c->has_trust = 1;
    c->trust = t;
}

int rws_capability_set_justification(RwsCapability *c,
                                     const char *justification) {
    char *dup = dup_cstr(justification);
    if (!dup) {
        return -1;
    }
    free(c->justification);
    c->justification = dup;
    return 0;
}

void rws_capability_release(RwsCapability *c) {
    if (!c) {
        return;
    }
    free(c->category);
    free(c->action);
    free(c->target);
    free(c->justification);
    c->category = NULL;
    c->action = NULL;
    c->target = NULL;
    c->justification = NULL;
}

char *rws_capability_identifier(const RwsCapability *c) {
    size_t n = strlen(c->category) + strlen(c->action) + strlen(c->target) + 3;
    char *out = malloc(n);
    if (!out) {
        return NULL;
    }
    /* "category:action:target" */
    size_t pos = 0;
    size_t la = strlen(c->category);
    memcpy(out + pos, c->category, la);
    pos += la;
    out[pos++] = ':';
    size_t lb = strlen(c->action);
    memcpy(out + pos, c->action, lb);
    pos += lb;
    out[pos++] = ':';
    size_t lc = strlen(c->target);
    memcpy(out + pos, c->target, lc);
    pos += lc;
    out[pos] = '\0';
    return out;
}

/* ---------------------------------------------------------------------------
 *  Classification rules
 * ------------------------------------------------------------------------- */

static RwsFlavor default_flavor(const RwsCapability *c) {
    const char *cat = c->category, *act = c->action;
    if ((str_eq(cat, "net") && str_eq(act, "connect")) ||
        (str_eq(cat, "fs") &&
         (str_eq(act, "write") || str_eq(act, "create") ||
          str_eq(act, "delete"))) ||
        (str_eq(cat, "vault") &&
         (str_eq(act, "write") || str_eq(act, "request_lease")))) {
        return RWS_FLAVOR_ACTUATION;
    }
    if (str_eq(cat, "proc")) {
        return RWS_FLAVOR_ACTUATION;
    }
    return RWS_FLAVOR_INTERNAL;
}

static int is_loopback_target(const char *t) {
    return str_eq(t, "localhost") || starts_with(t, "localhost:") ||
           str_eq(t, "127.0.0.1") || starts_with(t, "127.0.0.1:") ||
           str_eq(t, "::1") || starts_with(t, "[::1]:");
}

static int is_package_internal_target(const char *t) {
    return starts_with(t, "package:") || starts_with(t, "pkg:") ||
           starts_with(t, "./package/") || starts_with(t, "package/");
}

static RwsTrust default_trust(const RwsCapability *c) {
    const char *cat = c->category, *act = c->action;
    if (str_eq(cat, "net") && str_eq(act, "connect")) {
        return RWS_TRUST_UNTRUSTED;
    }
    if (str_eq(cat, "net") && str_eq(act, "listen")) {
        return is_loopback_target(c->target) ? RWS_TRUST_TRUSTED
                                             : RWS_TRUST_UNTRUSTED;
    }
    if (str_eq(cat, "fs") && str_eq(act, "read")) {
        return is_package_internal_target(c->target) ? RWS_TRUST_TRUSTED
                                                     : RWS_TRUST_UNTRUSTED;
    }
    return RWS_TRUST_TRUSTED;
}

static int is_input_capability(const RwsCapability *c, RwsFlavor flavor) {
    const char *cat = c->category, *act = c->action;
    if (str_eq(cat, "net") && str_eq(act, "connect")) {
        return flavor == RWS_FLAVOR_INGESTION;
    }
    if ((str_eq(cat, "net") && str_eq(act, "listen")) ||
        (str_eq(cat, "fs") && str_eq(act, "read")) ||
        (str_eq(cat, "channel") && str_eq(act, "read"))) {
        return 1;
    }
    return flavor == RWS_FLAVOR_INGESTION;
}

RwsClassification rws_classify(const RwsCapability *c) {
    RwsFlavor flavor = c->has_flavor ? c->flavor : default_flavor(c);
    RwsTrust trust = c->has_trust ? c->trust : default_trust(c);
    RwsClassification cl;
    cl.flavor = flavor;
    cl.trust = trust;
    cl.is_input = is_input_capability(c, flavor);
    cl.is_untrusted_input = cl.is_input && trust == RWS_TRUST_UNTRUSTED;
    cl.is_external_actuation = flavor == RWS_FLAVOR_ACTUATION;
    return cl;
}

static int is_read_side(const RwsCapability *c) {
    const char *cat = c->category, *act = c->action;
    return (str_eq(cat, "fs") && str_eq(act, "read")) ||
           (str_eq(cat, "vault") && str_eq(act, "read")) ||
           (str_eq(cat, "channel") && str_eq(act, "read"));
}

static int is_write_side(const RwsCapability *c) {
    const char *cat = c->category, *act = c->action;
    return (str_eq(cat, "fs") &&
            (str_eq(act, "write") || str_eq(act, "create") ||
             str_eq(act, "delete"))) ||
           (str_eq(cat, "vault") &&
            (str_eq(act, "write") || str_eq(act, "request_lease"))) ||
           (str_eq(cat, "channel") && str_eq(act, "write"));
}

/* A glob "prefix*" matches value if value starts with "prefix". */
static int glob_prefix_matches(const char *pattern, const char *value) {
    size_t lp = strlen(pattern);
    if (lp == 0 || pattern[lp - 1] != '*') {
        return 0;
    }
    return strncmp(value, pattern, lp - 1) == 0;
}

static int resources_overlap(const char *left, const char *right) {
    return str_eq(left, right) || glob_prefix_matches(left, right) ||
           glob_prefix_matches(right, left);
}

/* ---------------------------------------------------------------------------
 *  Capability value equality (mirrors the Rust PartialEq)
 * ------------------------------------------------------------------------- */

static int opt_str_eq(const char *a, const char *b) {
    if ((a == NULL) != (b == NULL)) {
        return 0;
    }
    if (a == NULL) {
        return 1;
    }
    return str_eq(a, b);
}

static int capability_equals(const RwsCapability *a, const RwsCapability *b) {
    if (!str_eq(a->category, b->category) || !str_eq(a->action, b->action) ||
        !str_eq(a->target, b->target)) {
        return 0;
    }
    if (a->has_flavor != b->has_flavor ||
        (a->has_flavor && a->flavor != b->flavor)) {
        return 0;
    }
    if (a->has_trust != b->has_trust ||
        (a->has_trust && a->trust != b->trust)) {
        return 0;
    }
    return opt_str_eq(a->justification, b->justification);
}

/* ---------------------------------------------------------------------------
 *  Growable borrowed-pointer list with value-based dedup
 * ------------------------------------------------------------------------- */

typedef struct {
    const RwsCapability **data;
    size_t len, cap;
    int ok;
} PtrVec;

static void ptrvec_init(PtrVec *v) {
    v->data = NULL;
    v->len = 0;
    v->cap = 0;
    v->ok = 1;
}

static void ptrvec_free(PtrVec *v) {
    free(v->data);
    v->data = NULL;
    v->len = 0;
    v->cap = 0;
}

/* Append `c` unless an equal-by-value capability is already present. */
static void ptrvec_push_unique(PtrVec *v, const RwsCapability *c) {
    if (!v->ok) {
        return;
    }
    size_t i;
    for (i = 0; i < v->len; i++) {
        if (capability_equals(v->data[i], c)) {
            return; /* already present */
        }
    }
    if (v->len == v->cap) {
        if (v->cap > ((size_t)-1) / 2 / sizeof(const RwsCapability *)) {
            v->ok = 0;
            return;
        }
        size_t nc = v->cap ? v->cap * 2 : 4;
        const RwsCapability **nd =
            realloc(v->data, nc * sizeof(const RwsCapability *));
        if (!nd) {
            v->ok = 0;
            return;
        }
        v->data = nd;
        v->cap = nc;
    }
    v->data[v->len++] = c;
}

/* ---------------------------------------------------------------------------
 *  Summary
 * ------------------------------------------------------------------------- */

static size_t count_overlap_pairs(const RwsCapability *caps, size_t n) {
    size_t count = 0, i, j;
    for (i = 0; i < n; i++) {
        if (!is_read_side(&caps[i])) {
            continue;
        }
        for (j = 0; j < n; j++) {
            if (!is_write_side(&caps[j]) ||
                !str_eq(caps[i].category, caps[j].category)) {
                continue;
            }
            if (resources_overlap(caps[i].target, caps[j].target)) {
                count++;
            }
        }
    }
    return count;
}

RwsSummary rws_summarize(const RwsCapability *caps, size_t n) {
    RwsSummary s;
    memset(&s, 0, sizeof s);
    s.overlapping_read_write_pairs = count_overlap_pairs(caps, n);
    size_t i;
    for (i = 0; i < n; i++) {
        const RwsCapability *c = &caps[i];
        RwsClassification cl = rws_classify(c);
        s.total_capabilities++;
        switch (cl.flavor) {
            case RWS_FLAVOR_INGESTION: s.ingestion_capabilities++; break;
            case RWS_FLAVOR_ACTUATION: s.actuation_capabilities++; break;
            case RWS_FLAVOR_INTERNAL: s.internal_capabilities++; break;
        }
        switch (cl.trust) {
            case RWS_TRUST_TRUSTED: s.trusted_capabilities++; break;
            case RWS_TRUST_UNTRUSTED: s.untrusted_capabilities++; break;
        }
        if (cl.is_input) {
            s.input_capabilities++;
        }
        if (cl.is_untrusted_input) {
            s.untrusted_inputs++;
        }
        if (cl.is_external_actuation) {
            s.external_actuations++;
        }
        if (is_read_side(c)) {
            s.read_side_capabilities++;
        }
        if (is_write_side(c)) {
            s.write_side_capabilities++;
        }
        if (c->justification != NULL) {
            s.justified_capabilities++;
        }
    }
    return s;
}

int rws_summary_is_empty(const RwsSummary *s) {
    return s->total_capabilities == 0;
}
int rws_summary_has_rws_risk(const RwsSummary *s) {
    return s->untrusted_inputs > 0 && s->external_actuations > 0;
}
int rws_summary_has_same_resource_overlap(const RwsSummary *s) {
    return s->overlapping_read_write_pairs > 0;
}

/* ---------------------------------------------------------------------------
 *  Validation
 * ------------------------------------------------------------------------- */

/* Add overlapping read/write pairs to the two lists; returns 1 if any found. */
static int collect_overlap_violations(const RwsCapability *caps, size_t n,
                                      PtrVec *reads, PtrVec *writes) {
    int found = 0;
    size_t i, j;
    for (i = 0; i < n; i++) {
        if (!is_read_side(&caps[i])) {
            continue;
        }
        for (j = 0; j < n; j++) {
            if (!is_write_side(&caps[j]) ||
                !str_eq(caps[i].category, caps[j].category)) {
                continue;
            }
            if (resources_overlap(caps[i].target, caps[j].target)) {
                ptrvec_push_unique(reads, &caps[i]);
                ptrvec_push_unique(writes, &caps[j]);
                found = 1;
            }
        }
    }
    return found;
}

RwsStatus rws_validate(const RwsCapability *caps, size_t n, RwsViolation *out) {
    PtrVec untrusted_inputs, actuations;
    ptrvec_init(&untrusted_inputs);
    ptrvec_init(&actuations);

    size_t i;
    for (i = 0; i < n; i++) {
        RwsClassification cl = rws_classify(&caps[i]);
        if (cl.is_untrusted_input) {
            ptrvec_push_unique(&untrusted_inputs, &caps[i]);
        }
        if (cl.is_external_actuation) {
            ptrvec_push_unique(&actuations, &caps[i]);
        }
    }

    /* Computed BEFORE the overlap pass may extend the lists. */
    int has_untrusted_and_actuation =
        untrusted_inputs.len > 0 && actuations.len > 0;
    int has_overlap = collect_overlap_violations(caps, n, &untrusted_inputs,
                                                 &actuations);

    if (!untrusted_inputs.ok || !actuations.ok) {
        ptrvec_free(&untrusted_inputs);
        ptrvec_free(&actuations);
        return RWS_ERR_NOMEM;
    }

    if (has_untrusted_and_actuation || has_overlap) {
        const char *msg =
            has_overlap
                ? "read/write separation violation: manifest contains "
                  "overlapping read/write capabilities"
                : "read/write separation violation: manifest contains "
                  "untrusted inputs and external actuations; split the agent "
                  "or insert a trusted channel boundary";
        char *message = dup_cstr(msg);
        if (!message) {
            ptrvec_free(&untrusted_inputs);
            ptrvec_free(&actuations);
            return RWS_ERR_NOMEM;
        }
        out->untrusted_inputs = untrusted_inputs.data;
        out->n_untrusted_inputs = untrusted_inputs.len;
        out->actuations = actuations.data;
        out->n_actuations = actuations.len;
        out->message = message;
        return RWS_VIOLATION;
    }

    ptrvec_free(&untrusted_inputs);
    ptrvec_free(&actuations);
    return RWS_OK;
}

void rws_violation_release(RwsViolation *v) {
    if (!v) {
        return;
    }
    free(v->untrusted_inputs);
    free(v->actuations);
    free(v->message);
    v->untrusted_inputs = NULL;
    v->actuations = NULL;
    v->message = NULL;
    v->n_untrusted_inputs = 0;
    v->n_actuations = 0;
}
