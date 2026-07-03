package com.codingadventures.conduit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

/**
 * Tests for {@link Application} registration and lifecycle. These exercise the
 * native library (each builder call crosses into Rust) but never start a
 * server, so they are fast.
 */
class ApplicationTest {

    @Test
    void buildersChainAndReturnSameInstance() {
        try (Application app = new Application()) {
            Application a2 = app
                    .get("/", req -> Responses.text("ok"))
                    .post("/u", req -> Responses.text("u"))
                    .put("/p", req -> Responses.text("p"))
                    .delete("/d", req -> Responses.text("d"))
                    .patch("/x", req -> Responses.text("x"))
                    .before(req -> null)
                    .after(req -> null)
                    .notFound(req -> Responses.html("404", 404))
                    .onError(req -> Responses.text("err", 500));
            assertSame(app, a2);
        }
    }

    @Test
    void settingsRoundTrip() {
        try (Application app = new Application()) {
            app.set("app_name", "Conduit");
            assertEquals("Conduit", app.getSetting("app_name"));
            assertNull(app.getSetting("missing"));
        }
    }

    @Test
    void nullHandlerRejected() {
        try (Application app = new Application()) {
            assertThrows(NullPointerException.class, () -> app.get("/", null));
        }
    }

    @Test
    void useAfterCloseThrows() {
        Application app = new Application();
        app.close();
        assertThrows(IllegalStateException.class, () -> app.get("/", req -> Responses.text("x")));
    }

    @Test
    void bindingConsumesTheApp() {
        Application app = new Application();
        app.get("/", req -> Responses.text("ok"));
        Server server = Server.bind(app, "127.0.0.1", 0);
        try {
            // After a Server consumes it, the app can no longer be used.
            assertThrows(IllegalStateException.class, () -> app.get("/late", req -> Responses.text("x")));
        } finally {
            server.close();
            app.close(); // no-op (consumed) — must not double-free
        }
    }
}
