// Unit tests for the Application DSL: construction, chaining, settings, bind.
#include "conduit/conduit.hpp"
#include "conduit_test.h"

using namespace conduit;

CONDUIT_TEST(settings_round_trip) {
    Application app;
    app.set("views", "tmpl");
    auto v = app.getSetting("views");
    CONDUIT_ASSERT(v.has_value());
    CONDUIT_ASSERT_EQ(v.value(), std::string("tmpl"));
}

CONDUIT_TEST(missing_setting_is_nullopt) {
    Application app;
    CONDUIT_ASSERT(!app.getSetting("nope").has_value());
}

CONDUIT_TEST(registrations_chain) {
    Application app;
    CONDUIT_ASSERT(&app.set("a", "1") == &app);
    CONDUIT_ASSERT(&app.get("/", [](const Request&) { return Response::text("x"); }) == &app);
    CONDUIT_ASSERT(&app.post("/x", [](const Request&) { return Response::text("x"); }) == &app);
    CONDUIT_ASSERT(&app.put("/x", [](const Request&) { return Response::text("x"); }) == &app);
    CONDUIT_ASSERT(&app.del("/x", [](const Request&) { return Response::text("x"); }) == &app);
    CONDUIT_ASSERT(&app.patch("/x", [](const Request&) { return Response::text("x"); }) == &app);
    CONDUIT_ASSERT(&app.before([](const Request&) { return std::nullopt; }) == &app);
    CONDUIT_ASSERT(&app.after([](const Request&, Response r) { return r; }) == &app);
    CONDUIT_ASSERT(&app.notFound([](const Request&) { return Response::text("nf", 404); }) == &app);
    CONDUIT_ASSERT(&app.onError([](const Request&) { return Response::text("err", 500); }) == &app);
}

CONDUIT_TEST(bind_returns_server_with_port) {
    Application app;
    app.get("/", [](const Request&) { return Response::text("x"); });
    Server server = app.bind("127.0.0.1", 0);
    CONDUIT_ASSERT(server.localPort() > 0);
    server.stop();
}

// bind() hands the native app to the server (and frees it on a failed bind),
// so the Application must refuse further use instead of writing through a
// dangling pointer. Each call below used to be a use-after-free.
static void expect_unusable(Application& app) {
    auto throws = [](auto&& fn) {
        try {
            fn();
        } catch (const std::logic_error&) {
            return true;
        }
        return false;
    };
    CONDUIT_ASSERT(throws([&] { app.get("/", [](const Request&) { return Response::text("x"); }); }));
    CONDUIT_ASSERT(throws([&] { app.before([](const Request&) { return std::nullopt; }); }));
    CONDUIT_ASSERT(throws([&] { app.after([](const Request&, Response r) { return r; }); }));
    CONDUIT_ASSERT(throws([&] { app.notFound([](const Request&) { return Response::text("nf", 404); }); }));
    CONDUIT_ASSERT(throws([&] { app.onError([](const Request&) { return Response::text("e", 500); }); }));
    CONDUIT_ASSERT(throws([&] { app.set("k", "v"); }));
    CONDUIT_ASSERT(throws([&] { app.getSetting("k"); }));
    CONDUIT_ASSERT(throws([&] { app.bind("127.0.0.1", 0); }));
}

CONDUIT_TEST(application_is_unusable_after_bind) {
    Application app;
    Server server = app.bind("127.0.0.1", 0);
    expect_unusable(app);
    server.stop();
}

CONDUIT_TEST(application_is_unusable_after_failed_bind) {
    // Fail the bind deterministically by asking for a port another server
    // already holds (an unparsable host would hit a DNS lookup first).
    Application holder;
    Server held = holder.bind("127.0.0.1", 0);
    Application app;
    bool threw = false;
    try {
        app.bind("127.0.0.1", held.localPort());
    } catch (const std::runtime_error&) {
        threw = true;
    }
    CONDUIT_ASSERT(threw);
    expect_unusable(app);
    held.stop();
}

CONDUIT_TEST(moved_from_application_is_unusable) {
    Application a;
    Application b(std::move(a));
    expect_unusable(a);
    CONDUIT_ASSERT(&b.set("k", "v") == &b);
}

CONDUIT_MAIN()
