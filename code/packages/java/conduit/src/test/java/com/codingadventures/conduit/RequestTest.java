package com.codingadventures.conduit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * Pure-JVM tests for {@link Request} parsing (no native lib). The constructor
 * is package-private; we feed it the same percent-encoded wire format the Rust
 * side produces.
 */
class RequestTest {

    private Request req(String routeParamsEnc, String queryString, String headersEnc) {
        return new Request("GET", "/p", queryString, "body", "application/json",
                "127.0.0.1", routeParamsEnc, headersEnc, "");
    }

    @Test
    void scalarAccessors() {
        Request r = new Request("POST", "/echo", "a=1", "hello", "text/plain",
                "10.0.0.1", "", "", "");
        assertEquals("POST", r.method());
        assertEquals("/echo", r.path());
        assertEquals("a=1", r.queryString());
        assertEquals("hello", r.body());
        assertEquals("text/plain", r.contentType());
        assertEquals("10.0.0.1", r.remoteAddr());
        assertEquals("", r.error());
    }

    @Test
    void nullsBecomeDefaults() {
        Request r = new Request(null, null, null, null, null, null, null, null, null);
        assertEquals("", r.method());
        assertEquals("/", r.path());
        assertEquals("", r.body());
        assertTrue(r.params().isEmpty());
    }

    @Test
    void routeParamsParseAndDecode() {
        Request r = req("name=world&id=42", "", "");
        assertEquals("world", r.param("name"));
        assertEquals("42", r.param("id"));
        assertNull(r.param("missing"));
    }

    @Test
    void percentEncodedValuesDecode() {
        // "%20" → space, "%2F" → '/', "%2B" → '+'
        Request r = req("greeting=hello%20world&path=a%2Fb&op=1%2B1", "", "");
        assertEquals("hello world", r.param("greeting"));
        assertEquals("a/b", r.param("path"));
        assertEquals("1+1", r.param("op"));
    }

    @Test
    void queryParamsParseFromQueryString() {
        Request r = req("", "q=hello&page=2", "");
        assertEquals("hello", r.queryParam("q"));
        assertEquals("2", r.queryParam("page"));
        assertNull(r.queryParam("nope"));
    }

    @Test
    void queryStringPlusIsSpace() {
        // Raw query strings use '+' for space (form convention); URLDecoder honors it.
        Request r = req("", "q=hello+world", "");
        assertEquals("hello world", r.queryParam("q"));
    }

    @Test
    void headersParseLowercasedAndCaseInsensitiveLookup() {
        Request r = req("", "", "content-type=application%2Fjson&host=localhost");
        assertEquals("application/json", r.header("content-type"));
        assertEquals("application/json", r.header("Content-Type")); // case-insensitive
        assertEquals("localhost", r.header("host"));
        assertNull(r.header("x-absent"));
    }

    @Test
    void emptyEncodedMapsAreEmpty() {
        Request r = req("", "", "");
        assertTrue(r.params().isEmpty());
        assertTrue(r.queryParams().isEmpty());
        assertTrue(r.headers().isEmpty());
    }

    @Test
    void mapsAreCachedAcrossCalls() {
        Request r = req("k=v", "", "");
        assertEquals(r.params(), r.params()); // stable
        assertEquals("v", r.param("k"));
    }

    @Test
    void errorFieldSurfacedForErrorHandler() {
        Request r = new Request("GET", "/boom", "", "", "", "127.0.0.1", "", "",
                "RuntimeException: kaboom");
        assertEquals("RuntimeException: kaboom", r.error());
    }
}
