/*
 * Tests for the C read-write-separation library, using the header-only
 * iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "read_write_separation.h"

/* Assert a capability's identifier equals `expected`. */
static void check_id(const RwsCapability *c, const char *expected) {
    char *id = rws_capability_identifier(c);
    ISO_CHECK(id != NULL);
    if (id) {
        ISO_CHECK_STR_EQ(id, expected);
    }
    free(id);
}

int main(void) {
    /* ── enum names ──────────────────────────────────────────────────────── */
    ISO_CHECK_STR_EQ(rws_flavor_str(RWS_FLAVOR_INGESTION), "ingestion");
    ISO_CHECK_STR_EQ(rws_flavor_str(RWS_FLAVOR_ACTUATION), "actuation");
    ISO_CHECK_STR_EQ(rws_flavor_str(RWS_FLAVOR_INTERNAL), "internal");
    ISO_CHECK_STR_EQ(rws_trust_str(RWS_TRUST_TRUSTED), "trusted");
    ISO_CHECK_STR_EQ(rws_trust_str(RWS_TRUST_UNTRUSTED), "untrusted");

    /* ── pure ingestion manifest is accepted ─────────────────────────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "net", "connect",
                                      "api.weather.gov:443") == 0);
        rws_capability_set_flavor(&c[0], RWS_FLAVOR_INGESTION);
        ISO_CHECK(rws_capability_init(&c[1], "channel", "write",
                                      "weather-snapshots") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_OK);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── pure actuation manifest is accepted ─────────────────────────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "channel", "read",
                                      "email-drafts") == 0);
        rws_capability_set_trust(&c[0], RWS_TRUST_TRUSTED);
        ISO_CHECK(rws_capability_init(&c[1], "net", "connect",
                                      "smtp.gmail.com:465") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_OK);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── mixed manifest rejected, with both lists populated ──────────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "net", "connect",
                                      "imap.gmail.com:993") == 0);
        rws_capability_set_flavor(&c[0], RWS_FLAVOR_INGESTION);
        ISO_CHECK(rws_capability_init(&c[1], "fs", "write",
                                      "/tmp/outbox/message.txt") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_VIOLATION);
        ISO_CHECK_EQ_UINT(v.n_untrusted_inputs, 1u);
        ISO_CHECK_EQ_UINT(v.n_actuations, 1u);
        check_id(v.untrusted_inputs[0], "net:connect:imap.gmail.com:993");
        check_id(v.actuations[0], "fs:write:/tmp/outbox/message.txt");
        rws_violation_release(&v);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── fs read/write overlap on same path rejected ─────────────────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "fs", "read",
                                      "package:/state/cache.json") == 0);
        ISO_CHECK(rws_capability_init(&c[1], "fs", "write",
                                      "package:/state/cache.json") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_VIOLATION);
        check_id(v.untrusted_inputs[0], "fs:read:package:/state/cache.json");
        check_id(v.actuations[0], "fs:write:package:/state/cache.json");
        rws_violation_release(&v);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── fs read/write overlap on a glob rejected ────────────────────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(
            rws_capability_init(&c[0], "fs", "read", "package:/state/*") == 0);
        ISO_CHECK(rws_capability_init(&c[1], "fs", "write",
                                      "package:/state/cache.json") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_VIOLATION);
        rws_violation_release(&v);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── fs read/write on disjoint paths accepted ────────────────────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "fs", "read",
                                      "package:/templates/weather.txt") == 0);
        ISO_CHECK(rws_capability_init(&c[1], "fs", "write",
                                      "/tmp/weather-email.txt") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_OK);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── vault read/write same secret rejected; disjoint accepted ────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "vault", "read",
                                      "gmail-app-password") == 0);
        ISO_CHECK(rws_capability_init(&c[1], "vault", "write",
                                      "gmail-app-password") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_VIOLATION);
        check_id(v.untrusted_inputs[0], "vault:read:gmail-app-password");
        check_id(v.actuations[0], "vault:write:gmail-app-password");
        rws_violation_release(&v);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);

        ISO_CHECK(rws_capability_init(&c[0], "vault", "read",
                                      "imap-credentials") == 0);
        ISO_CHECK(rws_capability_init(&c[1], "vault", "write",
                                      "smtp-credentials") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_OK);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── channel read/write same channel rejected ────────────────────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "channel", "read",
                                      "weather-snapshots") == 0);
        ISO_CHECK(rws_capability_init(&c[1], "channel", "write",
                                      "weather-snapshots") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_VIOLATION);
        check_id(v.untrusted_inputs[0], "channel:read:weather-snapshots");
        check_id(v.actuations[0], "channel:write:weather-snapshots");
        rws_violation_release(&v);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── manifest summary counts capability shape and risk ───────────────── */
    {
        RwsCapability c[4];
        ISO_CHECK(rws_capability_init(&c[0], "net", "connect",
                                      "api.weather.gov:443") == 0);
        rws_capability_set_flavor(&c[0], RWS_FLAVOR_INGESTION);
        ISO_CHECK(rws_capability_set_justification(
                      &c[0], "fetch weather alerts") == 0);
        ISO_CHECK(rws_capability_init(&c[1], "channel", "write",
                                      "weather-snapshots") == 0);
        ISO_CHECK(
            rws_capability_init(&c[2], "fs", "read", "package:/state/*") == 0);
        ISO_CHECK(rws_capability_init(&c[3], "fs", "write",
                                      "package:/state/cache.json") == 0);

        RwsSummary s = rws_summarize(c, 4);
        ISO_CHECK_EQ_UINT(s.total_capabilities, 4u);
        ISO_CHECK_EQ_UINT(s.ingestion_capabilities, 1u);
        ISO_CHECK_EQ_UINT(s.actuation_capabilities, 1u);
        ISO_CHECK_EQ_UINT(s.internal_capabilities, 2u);
        ISO_CHECK_EQ_UINT(s.trusted_capabilities, 3u);
        ISO_CHECK_EQ_UINT(s.untrusted_capabilities, 1u);
        ISO_CHECK_EQ_UINT(s.input_capabilities, 2u);
        ISO_CHECK_EQ_UINT(s.untrusted_inputs, 1u);
        ISO_CHECK_EQ_UINT(s.external_actuations, 1u);
        ISO_CHECK_EQ_UINT(s.read_side_capabilities, 1u);
        ISO_CHECK_EQ_UINT(s.write_side_capabilities, 2u);
        ISO_CHECK_EQ_UINT(s.overlapping_read_write_pairs, 1u);
        ISO_CHECK_EQ_UINT(s.justified_capabilities, 1u);
        ISO_CHECK(rws_summary_has_rws_risk(&s));
        ISO_CHECK(rws_summary_has_same_resource_overlap(&s));
        ISO_CHECK(!rws_summary_is_empty(&s));

        int i;
        for (i = 0; i < 4; i++) {
            rws_capability_release(&c[i]);
        }
    }

    /* ── empty manifest summary is empty ─────────────────────────────────── */
    {
        RwsSummary s = rws_summarize(NULL, 0);
        ISO_CHECK(rws_summary_is_empty(&s));
        ISO_CHECK(!rws_summary_has_rws_risk(&s));
        ISO_CHECK(!rws_summary_has_same_resource_overlap(&s));
    }

    /* ── explicit ingestion flavor allows ingestion-only networks ────────── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "net", "connect",
                                      "api.weather.gov:443") == 0);
        rws_capability_set_flavor(&c[0], RWS_FLAVOR_INGESTION);
        ISO_CHECK(rws_capability_init(&c[1], "net", "connect",
                                      "forecast.weather.gov:443") == 0);
        rws_capability_set_flavor(&c[1], RWS_FLAVOR_INGESTION);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_OK);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    /* ── default net connect is untrusted actuation; listen is the input ── */
    {
        RwsCapability c[2];
        RwsViolation v;
        ISO_CHECK(rws_capability_init(&c[0], "net", "connect",
                                      "smtp.gmail.com:465") == 0);
        RwsClassification cl = rws_classify(&c[0]);
        ISO_CHECK(cl.flavor == RWS_FLAVOR_ACTUATION);
        ISO_CHECK(cl.trust == RWS_TRUST_UNTRUSTED);
        ISO_CHECK(!cl.is_untrusted_input); /* an actuation, not an input */
        ISO_CHECK(cl.is_external_actuation);

        ISO_CHECK(
            rws_capability_init(&c[1], "net", "listen", "0.0.0.0:8080") == 0);
        ISO_CHECK(rws_validate(c, 2, &v) == RWS_VIOLATION);
        check_id(v.untrusted_inputs[0], "net:listen:0.0.0.0:8080");
        check_id(v.actuations[0], "net:connect:smtp.gmail.com:465");
        rws_violation_release(&v);
        rws_capability_release(&c[0]);
        rws_capability_release(&c[1]);
    }

    return ISO_TEST_RESULT();
}
