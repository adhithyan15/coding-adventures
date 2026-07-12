// Tests for the C++ http-core helpers, using the header-only iso_test.h harness
// (pure ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "http_core.hpp"

namespace http = ca::http;
using Params = std::vector<std::pair<std::string, std::string>>;

int main() {
    // ── HTTP version parse / display ─────────────────────────────────────
    {
        auto v = http::HttpVersion::parse("HTTP/1.1");
        ISO_CHECK(v.has_value());
        ISO_CHECK_EQ_UINT(v->major, 1u);
        ISO_CHECK_EQ_UINT(v->minor, 1u);
        ISO_CHECK_STR_EQ(v->to_string().c_str(), "HTTP/1.1");
        ISO_CHECK(!http::HttpVersion::parse("1.1").has_value());
        ISO_CHECK(!http::HttpVersion::parse("HTTP/x.1").has_value());
        ISO_CHECK(!http::HttpVersion::parse("HTTP/1").has_value());
    }

    // ── case-insensitive header lookup ───────────────────────────────────
    {
        std::vector<http::Header> headers = {{"Content-Type", "text/plain"}};
        const std::string* v = http::find_header(headers, "content-type");
        ISO_CHECK(v != nullptr);
        if (v) ISO_CHECK_STR_EQ(v->c_str(), "text/plain");
        ISO_CHECK(http::find_header(headers, "missing") == nullptr);
    }

    // ── content-length / content-type helpers ────────────────────────────
    {
        std::vector<http::Header> headers = {
            {"Content-Length", "42"},
            {"Content-Type", "text/html; charset=utf-8"}};
        auto len = http::parse_content_length(headers);
        ISO_CHECK(len.has_value() && *len == 42u);
        auto ct = http::parse_content_type(headers);
        ISO_CHECK(ct.has_value());
        if (ct) {
            ISO_CHECK_STR_EQ(ct->first.c_str(), "text/html");
            ISO_CHECK(ct->second.has_value());
            if (ct->second) ISO_CHECK_STR_EQ(ct->second->c_str(), "utf-8");
        }
    }
    {
        // An oversized Content-Length must be rejected, not silently wrapped.
        std::vector<http::Header> headers = {
            {"Content-Length", "18446744073709551616"}}; // 2^64
        ISO_CHECK(!http::parse_content_length(headers).has_value());
    }
    {
        // A media type with no charset parameter.
        std::vector<http::Header> headers = {{"Content-Type", "application/json"}};
        auto ct = http::parse_content_type(headers);
        ISO_CHECK(ct.has_value());
        if (ct) {
            ISO_CHECK_STR_EQ(ct->first.c_str(), "application/json");
            ISO_CHECK(!ct->second.has_value());
        }
    }

    // ── request-target parsing (query not decoded) ───────────────────────
    {
        auto t = http::parse_request_target(
            "/clip/v2/resource/light?id=abc%20123&limit=10#ignored");
        ISO_CHECK_STR_EQ(t.path.c_str(), "/clip/v2/resource/light");
        ISO_CHECK(t.query.has_value());
        if (t.query) ISO_CHECK_STR_EQ(t.query->c_str(), "id=abc%20123&limit=10");
        ISO_CHECK(t.fragment.has_value());
        if (t.fragment) ISO_CHECK_STR_EQ(t.fragment->c_str(), "ignored");

        Params want = {{"id", "abc%20123"}, {"limit", "10"}};
        ISO_CHECK(t.query_pairs() == want);
        auto lim = t.query_value("limit");
        ISO_CHECK(lim.has_value() && *lim == "10");
        ISO_CHECK(!t.query_value("missing").has_value());
    }

    // ── request head path/query helpers ──────────────────────────────────
    {
        http::RequestHead req;
        req.method = "GET";
        req.target = "/api/devices?room=kitchen&verbose";
        req.version = {1, 1};
        ISO_CHECK_STR_EQ(req.path().c_str(), "/api/devices");
        auto room = req.query_value("room");
        ISO_CHECK(room.has_value() && *room == "kitchen");
        auto verbose = req.query_value("verbose");
        ISO_CHECK(verbose.has_value() && *verbose == ""); // present, empty value
    }

    // ── route matching by path only (query ignored) ──────────────────────
    {
        auto pat = http::RoutePattern::parse("/clip/v2/resource/:kind/:id");
        auto m = pat.match_target("/clip/v2/resource/light/abc?limit=10");
        ISO_CHECK(m.has_value());
        if (m) {
            Params want = {{"kind", "light"}, {"id", "abc"}};
            ISO_CHECK(*m == want);
        }
        ISO_CHECK(!pat.match_target("/clip/v2/resource/light").has_value());
    }

    // ── heads delegate to the content helpers ────────────────────────────
    {
        http::RequestHead req;
        req.method = "POST";
        req.target = "/submit";
        req.version = {1, 1};
        req.headers = {{"Content-Length", "5"}};
        auto len = req.content_length();
        ISO_CHECK(len.has_value() && *len == 5u);

        http::ResponseHead resp;
        resp.version = {1, 0};
        resp.status = 200;
        resp.reason = "OK";
        resp.headers = {{"Content-Type", "application/json"}};
        auto ct = resp.content_type();
        ISO_CHECK(ct.has_value());
        if (ct) {
            ISO_CHECK_STR_EQ(ct->first.c_str(), "application/json");
            ISO_CHECK(!ct->second.has_value());
        }
    }

    // ── named-parameter matching ─────────────────────────────────────────
    {
        auto pat = http::RoutePattern::parse("/hello/:name");
        auto m = pat.match_path("/hello/Adhithya");
        ISO_CHECK(m.has_value());
        if (m) {
            Params want = {{"name", "Adhithya"}};
            ISO_CHECK(*m == want);
        }
        ISO_CHECK(!pat.match_path("/hello").has_value());
        ISO_CHECK(!pat.match_path("/goodbye/Adhithya").has_value());
    }

    // ── root-path handling ───────────────────────────────────────────────
    {
        auto pat = http::RoutePattern::parse("/");
        auto m = pat.match_path("/");
        ISO_CHECK(m.has_value());
        if (m) ISO_CHECK(m->empty());
        ISO_CHECK(!pat.match_path("/extra").has_value());
    }

    return ISO_TEST_RESULT();
}
