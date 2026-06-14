// Unit tests for Response helpers + the native round-trip (no server needed).
#include "conduit/conduit.hpp"
#include "conduit_test.h"

using namespace conduit;

CONDUIT_TEST(html_defaults) {
    Response r = Response::html("<h1>Hi</h1>");
    CONDUIT_ASSERT_EQ(r.status, 200);
    CONDUIT_ASSERT_EQ(r.body, std::string("<h1>Hi</h1>"));
    bool found = false;
    for (auto& h : r.headers)
        if (h.first == "content-type" && h.second == "text/html; charset=utf-8") found = true;
    CONDUIT_ASSERT(found);
}

CONDUIT_TEST(html_explicit_status) {
    CONDUIT_ASSERT_EQ(Response::html("x", 201).status, 201);
}

CONDUIT_TEST(json_content_type) {
    Response r = Response::json("{\"ok\":1}");
    CONDUIT_ASSERT_EQ(r.headers.at(0).second, std::string("application/json"));
    CONDUIT_ASSERT_EQ(r.body, std::string("{\"ok\":1}"));
}

CONDUIT_TEST(text_content_type) {
    Response r = Response::text("pong");
    CONDUIT_ASSERT_EQ(r.headers.at(0).second, std::string("text/plain; charset=utf-8"));
}

CONDUIT_TEST(respond_custom) {
    Response r = Response::respond(204, "", {{"x-y", "z"}});
    CONDUIT_ASSERT_EQ(r.status, 204);
    CONDUIT_ASSERT_EQ(r.headers.at(0).first, std::string("x-y"));
}

CONDUIT_TEST(redirect_defaults) {
    Response r = Response::redirect("/new");
    CONDUIT_ASSERT_EQ(r.status, 302);
    CONDUIT_ASSERT_EQ(r.headers.at(0).first, std::string("location"));
    CONDUIT_ASSERT_EQ(r.headers.at(0).second, std::string("/new"));
}

CONDUIT_TEST(redirect_explicit_status) {
    CONDUIT_ASSERT_EQ(Response::redirect("/old", 301).status, 301);
}

CONDUIT_TEST(redirect_rejects_crlf) {
    bool threw = false;
    try {
        Response::redirect("/x\r\nSet-Cookie: evil=1");
    } catch (const std::invalid_argument&) {
        threw = true;
    }
    CONDUIT_ASSERT(threw);
}

// toC builds a native response; reading it back must round-trip everything.
CONDUIT_TEST(toC_round_trip) {
    Response r(200, "body", {{"x-a", "b"}, {"content-type", "text/plain"}});
    ConduitResponse* c = r.toC();
    CONDUIT_ASSERT(c != nullptr);
    Response back = Response::fromC(c);
    conduit_response_free(c);
    CONDUIT_ASSERT_EQ(back.status, 200);
    CONDUIT_ASSERT_EQ(back.body, std::string("body"));
    bool found = false;
    for (auto& h : back.headers)
        if (h.first == "x-a" && h.second == "b") found = true;
    CONDUIT_ASSERT(found);
}

CONDUIT_TEST(toC_clamps_status) {
    ConduitResponse* c = Response(999, "").toC();
    CONDUIT_ASSERT_EQ(static_cast<int>(conduit_response_status(c)), 599);
    conduit_response_free(c);
    ConduitResponse* c2 = Response(1, "").toC();
    CONDUIT_ASSERT_EQ(static_cast<int>(conduit_response_status(c2)), 100);
    conduit_response_free(c2);
}

CONDUIT_TEST(toC_drops_unsafe_headers) {
    Response r(200, "", {{"x-ok", "fine"}, {"x-bad", "a\r\nb"}});
    ConduitResponse* c = r.toC();
    Response back = Response::fromC(c);
    conduit_response_free(c);
    bool hasOk = false, hasBad = false;
    for (auto& h : back.headers) {
        if (h.first == "x-ok") hasOk = true;
        if (h.first == "x-bad") hasBad = true;
    }
    CONDUIT_ASSERT(hasOk);
    CONDUIT_ASSERT(!hasBad);
}

CONDUIT_MAIN()
