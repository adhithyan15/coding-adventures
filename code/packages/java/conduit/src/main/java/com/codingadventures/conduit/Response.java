package com.codingadventures.conduit;

import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * An immutable HTTP response: a status code, a header map, and a string body.
 *
 * <p>Handlers usually build responses through {@link Responses} rather than
 * constructing this directly:
 *
 * <pre>{@code
 * return Responses.json("{\"ok\":true}");
 * return Responses.html("<h1>Hi</h1>", 201);
 * }</pre>
 *
 * <p>The Rust dispatch layer reads this back through {@link #status()},
 * {@link #body()}, and {@link #headersEncoded()}.
 */
public final class Response {

    private final int status;
    private final Map<String, String> headers;
    private final String body;

    /**
     * @param status  HTTP status code (100–599; out-of-range collapses to 500
     *                on the Rust side)
     * @param headers response headers (names are lower-cased on the wire);
     *                may be null for none
     * @param body    response body as a UTF-8 string; may be null for empty
     */
    public Response(int status, Map<String, String> headers, String body) {
        this.status = status;
        this.headers = headers == null ? Collections.emptyMap() : new LinkedHashMap<>(headers);
        this.body = body == null ? "" : body;
    }

    /** The HTTP status code. */
    public int status() {
        return status;
    }

    /** The response body. */
    public String body() {
        return body;
    }

    /** The response headers (a defensive copy). */
    public Map<String, String> headers() {
        return new LinkedHashMap<>(headers);
    }

    /**
     * Encode the headers as the percent-encoded {@code k=v&k2=v2} wire format
     * read by the Rust side.
     *
     * <p>Header names are lower-cased. Any header whose name or value contains
     * a CR or LF is dropped here (and re-checked on the Rust side) to defend
     * against HTTP response splitting.
     */
    public String headersEncoded() {
        if (headers.isEmpty()) {
            return "";
        }
        StringBuilder sb = new StringBuilder();
        boolean first = true;
        for (Map.Entry<String, String> e : headers.entrySet()) {
            String name = e.getKey();
            String value = e.getValue();
            if (name == null || value == null) {
                continue;
            }
            if (containsControl(name) || containsControl(value)) {
                continue;
            }
            if (!first) {
                sb.append('&');
            }
            first = false;
            sb.append(encode(name.toLowerCase())).append('=').append(encode(value));
        }
        return sb.toString();
    }

    private static boolean containsControl(String s) {
        // Drop any header with a C0 control (< 0x20) or DEL (0x7F) — defense
        // against HTTP response splitting / header smuggling. The Rust side
        // re-checks with the same rule.
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c < 0x20 || c == 0x7f) {
                return true;
            }
        }
        return false;
    }

    private static String encode(String s) {
        // URLEncoder emits "+" for space and "%2B" for a literal "+"; the Rust
        // pct_decode accepts both, so the round-trip is lossless.
        return URLEncoder.encode(s, StandardCharsets.UTF_8);
    }
}
