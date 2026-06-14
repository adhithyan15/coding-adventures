package com.codingadventures.conduit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Map;
import org.junit.jupiter.api.Test;

/** Pure-JVM tests for the {@link Responses} factory helpers (no native lib). */
class ResponsesTest {

    @Test
    void htmlDefaultsTo200AndSetsContentType() {
        Response r = Responses.html("<h1>OK</h1>");
        assertEquals(200, r.status());
        assertEquals("<h1>OK</h1>", r.body());
        assertEquals("text/html; charset=utf-8", r.headers().get("content-type"));
    }

    @Test
    void htmlWithExplicitStatus() {
        Response r = Responses.html("<h1>Created</h1>", 201);
        assertEquals(201, r.status());
    }

    @Test
    void jsonSetsApplicationJson() {
        Response r = Responses.json("{\"ok\":true}");
        assertEquals(200, r.status());
        assertEquals("application/json", r.headers().get("content-type"));
        assertEquals("{\"ok\":true}", r.body());
    }

    @Test
    void jsonWithStatus() {
        assertEquals(500, Responses.json("{\"error\":\"x\"}", 500).status());
    }

    @Test
    void textSetsTextPlain() {
        Response r = Responses.text("pong");
        assertEquals("text/plain; charset=utf-8", r.headers().get("content-type"));
        assertEquals("pong", r.body());
    }

    @Test
    void respondPassesEverythingThrough() {
        Response r = Responses.respond(204, "", Map.of("x-custom", "v"));
        assertEquals(204, r.status());
        assertEquals("v", r.headers().get("x-custom"));
        assertEquals("", r.body());
    }

    @Test
    void respondWithoutHeaders() {
        Response r = Responses.respond(200, "hi");
        assertEquals(200, r.status());
        assertTrue(r.headers().isEmpty());
    }

    @Test
    void haltSetsStatusAndBody() {
        Response r = Responses.halt(403, "Forbidden");
        assertEquals(403, r.status());
        assertEquals("Forbidden", r.body());
    }

    @Test
    void redirectDefaults302WithLocation() {
        Response r = Responses.redirect("/login");
        assertEquals(302, r.status());
        assertEquals("/login", r.headers().get("location"));
        assertEquals("", r.body());
    }

    @Test
    void redirectWithExplicitStatus() {
        assertEquals(301, Responses.redirect("/new", 301).status());
    }

    @Test
    void redirectRejectsCrlf() {
        assertThrows(IllegalArgumentException.class,
                () -> Responses.redirect("/x\r\nSet-Cookie: evil=1"));
    }

    @Test
    void headersEncodedIsPercentEncodedPairs() {
        Response r = Responses.respond(200, "b", Map.of("content-type", "text/html"));
        // single pair → "content-type=text%2Fhtml"
        String enc = r.headersEncoded();
        assertTrue(enc.startsWith("content-type="), enc);
        assertTrue(enc.contains("%2F"), enc); // '/' is percent-encoded
    }

    @Test
    void headersEncodedDropsCrlfBearingHeaders() {
        Response r = Responses.respond(200, "b", Map.of("x-bad", "line1\r\nline2"));
        assertEquals("", r.headersEncoded());
    }

    @Test
    void headersEncodedLowercasesNames() {
        Response r = Responses.respond(200, "b", Map.of("X-Upper", "v"));
        assertTrue(r.headersEncoded().startsWith("x-upper="));
    }
}
