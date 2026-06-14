package com.codingadventures.conduit;

import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Static factory helpers for building {@link Response} objects — the Conduit
 * equivalent of Sinatra's {@code html}/{@code json}/{@code halt}/{@code redirect}.
 *
 * <p>Designed for static import:
 *
 * <pre>{@code
 * import static com.codingadventures.conduit.Responses.*;
 *
 * app.get("/",      req -> html("<h1>Hello</h1>"));
 * app.get("/data",  req -> json("{\"ok\":true}"));
 * app.get("/old",   req -> redirect("/new"));
 * app.get("/secret",req -> halt(403, "Forbidden"));
 * }</pre>
 */
public final class Responses {

    private Responses() {
    }

    private static Map<String, String> contentType(String value) {
        Map<String, String> h = new LinkedHashMap<>();
        h.put("content-type", value);
        return h;
    }

    /** {@code 200 text/html} response. */
    public static Response html(String body) {
        return html(body, 200);
    }

    /** {@code text/html} response with an explicit status. */
    public static Response html(String body, int status) {
        return new Response(status, contentType("text/html; charset=utf-8"), body);
    }

    /** {@code 200 application/json} response from pre-serialized JSON text. */
    public static Response json(String body) {
        return json(body, 200);
    }

    /** {@code application/json} response with an explicit status. */
    public static Response json(String body, int status) {
        return new Response(status, contentType("application/json"), body);
    }

    /** {@code 200 text/plain} response. */
    public static Response text(String body) {
        return text(body, 200);
    }

    /** {@code text/plain} response with an explicit status. */
    public static Response text(String body, int status) {
        return new Response(status, contentType("text/plain; charset=utf-8"), body);
    }

    /** Arbitrary response: status, body, and explicit headers. */
    public static Response respond(int status, String body, Map<String, String> headers) {
        return new Response(status, headers, body);
    }

    /** Arbitrary response with no extra headers. */
    public static Response respond(int status, String body) {
        return new Response(status, null, body);
    }

    /**
     * A halt response — short-circuit with this status and body. Returning it
     * from a before filter or handler responds immediately.
     *
     * <p>For a non-local halt from deep in helper code, {@code throw new
     * HaltException(status, body)} instead.
     */
    public static Response halt(int status, String body) {
        return new Response(status, contentType("text/plain; charset=utf-8"), body);
    }

    /** A {@code 302} redirect to {@code location}. */
    public static Response redirect(String location) {
        return redirect(location, 302);
    }

    /**
     * A redirect to {@code location} with an explicit status (e.g. 301).
     *
     * @throws IllegalArgumentException if {@code location} contains CR or LF
     *     (defense against HTTP response splitting). Open-redirect validation
     *     is the caller's responsibility — do not pass unvalidated user input.
     */
    public static Response redirect(String location, int status) {
        if (location != null && (location.indexOf('\r') >= 0 || location.indexOf('\n') >= 0)) {
            throw new IllegalArgumentException("redirect location must not contain CR or LF");
        }
        Map<String, String> h = new LinkedHashMap<>();
        h.put("location", location);
        return new Response(status, h, "");
    }
}
