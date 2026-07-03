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

CONDUIT_MAIN()
