// Tests for the C++ read-write-separation library, using the header-only
// iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <optional>
#include <string>
#include <vector>

#include "read_write_separation.hpp"

namespace rws = ca::read_write_separation;
using rws::Capability;
using rws::Flavor;
using rws::Trust;

static Capability cap(const std::string& category, const std::string& action,
                      const std::string& target) {
    return Capability(category, action, target);
}

int main() {
    // ── enum names ────────────────────────────────────────────────────────
    ISO_CHECK_STR_EQ(rws::to_string(Flavor::Ingestion), "ingestion");
    ISO_CHECK_STR_EQ(rws::to_string(Flavor::Actuation), "actuation");
    ISO_CHECK_STR_EQ(rws::to_string(Flavor::Internal), "internal");
    ISO_CHECK_STR_EQ(rws::to_string(Trust::Trusted), "trusted");
    ISO_CHECK_STR_EQ(rws::to_string(Trust::Untrusted), "untrusted");

    // ── pure ingestion manifest is accepted ───────────────────────────────
    {
        std::vector<Capability> caps = {
            cap("net", "connect", "api.weather.gov:443")
                .with_flavor(Flavor::Ingestion),
            cap("channel", "write", "weather-snapshots")};
        ISO_CHECK(!rws::validate_manifest(caps).has_value());
    }

    // ── pure actuation manifest is accepted ────────────────────────────────
    {
        std::vector<Capability> caps = {
            cap("channel", "read", "email-drafts").with_trust(Trust::Trusted),
            cap("net", "connect", "smtp.gmail.com:465")};
        ISO_CHECK(!rws::validate_manifest(caps).has_value());
    }

    // ── mixed manifest rejected, with both lists populated ─────────────────
    {
        std::vector<Capability> caps = {
            cap("net", "connect", "imap.gmail.com:993")
                .with_flavor(Flavor::Ingestion),
            cap("fs", "write", "/tmp/outbox/message.txt")};
        auto v = rws::validate_manifest(caps);
        ISO_CHECK(v.has_value());
        ISO_CHECK_EQ_UINT(v->untrusted_inputs.size(), 1u);
        ISO_CHECK_EQ_UINT(v->actuations.size(), 1u);
        ISO_CHECK_STR_EQ(v->untrusted_inputs[0].identifier().c_str(),
                         "net:connect:imap.gmail.com:993");
        ISO_CHECK_STR_EQ(v->actuations[0].identifier().c_str(),
                         "fs:write:/tmp/outbox/message.txt");
    }

    // ── fs read/write overlap on same path rejected ───────────────────────
    {
        std::vector<Capability> caps = {
            cap("fs", "read", "package:/state/cache.json"),
            cap("fs", "write", "package:/state/cache.json")};
        auto v = rws::validate_manifest(caps);
        ISO_CHECK(v.has_value());
        ISO_CHECK_STR_EQ(v->untrusted_inputs[0].identifier().c_str(),
                         "fs:read:package:/state/cache.json");
        ISO_CHECK_STR_EQ(v->actuations[0].identifier().c_str(),
                         "fs:write:package:/state/cache.json");
    }

    // ── fs overlap on a glob rejected; disjoint accepted ──────────────────
    {
        std::vector<Capability> glob = {
            cap("fs", "read", "package:/state/*"),
            cap("fs", "write", "package:/state/cache.json")};
        ISO_CHECK(rws::validate_manifest(glob).has_value());

        std::vector<Capability> disjoint = {
            cap("fs", "read", "package:/templates/weather.txt"),
            cap("fs", "write", "/tmp/weather-email.txt")};
        ISO_CHECK(!rws::validate_manifest(disjoint).has_value());
    }

    // ── vault same secret rejected; disjoint accepted ─────────────────────
    {
        std::vector<Capability> same = {
            cap("vault", "read", "gmail-app-password"),
            cap("vault", "write", "gmail-app-password")};
        auto v = rws::validate_manifest(same);
        ISO_CHECK(v.has_value());
        ISO_CHECK_STR_EQ(v->untrusted_inputs[0].identifier().c_str(),
                         "vault:read:gmail-app-password");
        ISO_CHECK_STR_EQ(v->actuations[0].identifier().c_str(),
                         "vault:write:gmail-app-password");

        std::vector<Capability> disjoint = {
            cap("vault", "read", "imap-credentials"),
            cap("vault", "write", "smtp-credentials")};
        ISO_CHECK(!rws::validate_manifest(disjoint).has_value());
    }

    // ── channel same channel rejected ─────────────────────────────────────
    {
        std::vector<Capability> caps = {
            cap("channel", "read", "weather-snapshots"),
            cap("channel", "write", "weather-snapshots")};
        auto v = rws::validate_manifest(caps);
        ISO_CHECK(v.has_value());
        ISO_CHECK_STR_EQ(v->untrusted_inputs[0].identifier().c_str(),
                         "channel:read:weather-snapshots");
        ISO_CHECK_STR_EQ(v->actuations[0].identifier().c_str(),
                         "channel:write:weather-snapshots");
    }

    // ── manifest summary counts shape and risk ────────────────────────────
    {
        std::vector<Capability> caps = {
            cap("net", "connect", "api.weather.gov:443")
                .with_flavor(Flavor::Ingestion)
                .with_justification("fetch weather alerts"),
            cap("channel", "write", "weather-snapshots"),
            cap("fs", "read", "package:/state/*"),
            cap("fs", "write", "package:/state/cache.json")};
        rws::Summary s = rws::summarize_manifest(caps);
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
        ISO_CHECK(s.has_rws_risk());
        ISO_CHECK(s.has_same_resource_overlap());
        ISO_CHECK(!s.is_empty());
    }

    // ── empty manifest summary is empty ───────────────────────────────────
    {
        rws::Summary s = rws::summarize_manifest({});
        ISO_CHECK(s.is_empty());
        ISO_CHECK(!s.has_rws_risk());
        ISO_CHECK(!s.has_same_resource_overlap());
    }

    // ── explicit ingestion flavor allows ingestion-only networks ──────────
    {
        std::vector<Capability> caps = {
            cap("net", "connect", "api.weather.gov:443")
                .with_flavor(Flavor::Ingestion),
            cap("net", "connect", "forecast.weather.gov:443")
                .with_flavor(Flavor::Ingestion)};
        ISO_CHECK(!rws::validate_manifest(caps).has_value());
    }

    // ── default net connect is an untrusted actuation; listen is the input ─
    {
        Capability def = cap("net", "connect", "smtp.gmail.com:465");
        rws::Classification cl = rws::classify_capability(def);
        ISO_CHECK(cl.flavor == Flavor::Actuation);
        ISO_CHECK(cl.trust == Trust::Untrusted);
        ISO_CHECK(!cl.is_untrusted_input);
        ISO_CHECK(cl.is_external_actuation);

        std::vector<Capability> caps = {def,
                                        cap("net", "listen", "0.0.0.0:8080")};
        auto v = rws::validate_manifest(caps);
        ISO_CHECK(v.has_value());
        ISO_CHECK_STR_EQ(v->untrusted_inputs[0].identifier().c_str(),
                         "net:listen:0.0.0.0:8080");
        ISO_CHECK_STR_EQ(v->actuations[0].identifier().c_str(),
                         "net:connect:smtp.gmail.com:465");
    }

    return ISO_TEST_RESULT();
}
