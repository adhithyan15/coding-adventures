/*
 * read_write_separation.h — capability read/write-separation analysis, in pure
 * ISO C17. A faithful port of the Rust `read-write-separation` crate.
 * ===========================================================================
 *
 * An agent declares a *manifest* of capabilities — each a (category, action,
 * target) triple, e.g. ("net", "connect", "smtp.gmail.com:465"). The read/write
 * separation (RWS) principle says a single agent must not simultaneously hold an
 * UNTRUSTED INPUT and an EXTERNAL ACTUATION (nor overlapping read/write access
 * to the same resource) — that combination lets untrusted data drive a
 * side-effecting action. This library classifies capabilities and validates a
 * manifest against that rule.
 *
 * Each capability has a *flavor* (Ingestion / Actuation / Internal) and a *trust*
 * (Trusted / Untrusted); if not set explicitly they are inferred from the
 * category/action (e.g. "net connect" defaults to an untrusted actuation, while
 * "fs read" of a "package:"-internal target is trusted).
 *
 * OWNERSHIP. A `RwsCapability` owns its three strings (and an optional
 * justification). Build with `rws_capability_init`, release with
 * `rws_capability_release`. Analysis functions take a borrowed array of
 * capabilities; an `RwsViolation` borrows pointers INTO that array (so keep it
 * alive) plus an owned message — release with `rws_violation_release`.
 *
 * DIVERGENCE FROM RUST. Rust returns `Result<(), RwsViolation>` and owns cloned
 * capabilities in the violation; this port returns an `RwsStatus` and the
 * violation holds borrowed pointers. Errors (OOM) surface as status codes.
 *
 * PORTABILITY. Pure ISO C17 — no compiler extensions. Builds clean under GCC,
 * Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_READ_WRITE_SEPARATION_H
#define CA_READ_WRITE_SEPARATION_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Whether a capability ingests data, actuates externally, or is internal. */
typedef enum {
    RWS_FLAVOR_INGESTION,
    RWS_FLAVOR_ACTUATION,
    RWS_FLAVOR_INTERNAL
} RwsFlavor;

/* Whether a capability's data is trusted or untrusted. */
typedef enum { RWS_TRUST_TRUSTED, RWS_TRUST_UNTRUSTED } RwsTrust;

/* Lowercase name of a flavor / trust (e.g. "ingestion", "untrusted"). */
const char *rws_flavor_str(RwsFlavor f);
const char *rws_trust_str(RwsTrust t);

/* A declared capability. The three strings and the optional justification are
 * owned; flavor/trust are optional (has_* == 0 means "infer from the triple"). */
typedef struct {
    char *category;
    char *action;
    char *target;
    int has_flavor;
    RwsFlavor flavor;
    int has_trust;
    RwsTrust trust;
    char *justification; /* NULL if none */
} RwsCapability;

/* Initialize a capability from its triple (flavor/trust unset, no
 * justification). Returns 0, or -1 on allocation failure (nothing allocated). */
int rws_capability_init(RwsCapability *c, const char *category,
                        const char *action, const char *target);
/* Optional-field setters. set_justification returns 0 or -1 (OOM). */
void rws_capability_set_flavor(RwsCapability *c, RwsFlavor f);
void rws_capability_set_trust(RwsCapability *c, RwsTrust t);
int rws_capability_set_justification(RwsCapability *c, const char *justification);
/* Free the owned strings (safe to call on a zeroed struct). */
void rws_capability_release(RwsCapability *c);
/* "category:action:target" as a malloc'd string (caller frees); NULL on OOM. */
char *rws_capability_identifier(const RwsCapability *c);

/* The resolved classification of a single capability. */
typedef struct {
    RwsFlavor flavor;
    RwsTrust trust;
    int is_input;
    int is_untrusted_input;
    int is_external_actuation;
} RwsClassification;

RwsClassification rws_classify(const RwsCapability *c);

/* Aggregate counts over a manifest. */
typedef struct {
    size_t total_capabilities;
    size_t ingestion_capabilities;
    size_t actuation_capabilities;
    size_t internal_capabilities;
    size_t trusted_capabilities;
    size_t untrusted_capabilities;
    size_t input_capabilities;
    size_t untrusted_inputs;
    size_t external_actuations;
    size_t read_side_capabilities;
    size_t write_side_capabilities;
    size_t overlapping_read_write_pairs;
    size_t justified_capabilities;
} RwsSummary;

/* Summarize a borrowed array of `n` capabilities. */
RwsSummary rws_summarize(const RwsCapability *caps, size_t n);
int rws_summary_is_empty(const RwsSummary *s);
int rws_summary_has_rws_risk(const RwsSummary *s);
int rws_summary_has_same_resource_overlap(const RwsSummary *s);

/* Result of validating a manifest. */
typedef enum {
    RWS_OK = 0,      /* manifest satisfies read/write separation */
    RWS_VIOLATION,   /* manifest violates it (see the filled RwsViolation) */
    RWS_ERR_NOMEM
} RwsStatus;

/* A violation report: borrowed pointers into the analyzed array plus an owned
 * message. Release with rws_violation_release. */
typedef struct {
    const RwsCapability **untrusted_inputs;
    size_t n_untrusted_inputs;
    const RwsCapability **actuations;
    size_t n_actuations;
    char *message; /* owned */
} RwsViolation;

/* Validate a manifest. Returns RWS_OK (satisfies RWS), RWS_VIOLATION (fills
 * *out — release it), or RWS_ERR_NOMEM. */
RwsStatus rws_validate(const RwsCapability *caps, size_t n, RwsViolation *out);
void rws_violation_release(RwsViolation *v);

#ifdef __cplusplus
}
#endif

#endif /* CA_READ_WRITE_SEPARATION_H */
