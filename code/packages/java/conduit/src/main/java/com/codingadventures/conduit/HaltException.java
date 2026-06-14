package com.codingadventures.conduit;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Thrown to short-circuit a handler with an immediate response — the
 * Sinatra-style {@code halt}.
 *
 * <p>Two ways to halt:
 *
 * <ol>
 *   <li><b>Return a halt response</b> with {@link Responses#halt} /
 *       {@link Responses#redirect}. Cleanest for lambdas.</li>
 *   <li><b>Throw a HaltException</b> for a non-local halt from deep inside
 *       helper code. The Rust dispatch layer detects it via
 *       {@code IsInstanceOf} and converts it to a response.</li>
 * </ol>
 *
 * <p>Unlike an ordinary error, a HaltException is intentional control flow and
 * does <em>not</em> route to the error handler.
 */
public final class HaltException extends RuntimeException {

    private static final long serialVersionUID = 1L;

    private final int status;
    private final String body;
    private final Map<String, String> headers;

    /** Halt with a status and body and no extra headers. */
    public HaltException(int status, String body) {
        this(status, body, Collections.emptyMap());
    }

    /** Halt with a status, body, and headers. */
    public HaltException(int status, String body, Map<String, String> headers) {
        super("halt(" + status + ")");
        this.status = status;
        this.body = body == null ? "" : body;
        this.headers = headers == null ? Collections.emptyMap() : new LinkedHashMap<>(headers);
    }

    /** The HTTP status code to send. */
    public int status() {
        return status;
    }

    /** The response body. */
    public String body() {
        return body;
    }

    /**
     * The headers in the percent-encoded {@code k=v&k2=v2} wire format read by
     * the Rust side. Delegates to {@link Response#headersEncoded()} semantics.
     */
    public String headersEncoded() {
        return new Response(status, headers, body).headersEncoded();
    }
}
