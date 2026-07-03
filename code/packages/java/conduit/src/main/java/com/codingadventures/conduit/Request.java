package com.codingadventures.conduit;

import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * An immutable view of an incoming HTTP request, built by the Rust side and
 * handed to a {@link ConduitHandler}.
 *
 * <p>The route params, query params, and headers cross the JNI boundary as
 * percent-encoded {@code k=v&k2=v2} strings (see the WEB09 spec). They are
 * parsed lazily on first access and cached, so handlers that never touch a map
 * pay nothing.
 *
 * <p>All accessors are null-safe: a missing param/header returns {@code null}
 * from {@link #param}/{@link #queryParam}/{@link #header}; the body and
 * content-type default to empty strings.
 */
public final class Request {

    private final String method;
    private final String path;
    private final String queryString;
    private final String body;
    private final String contentType;
    private final String remoteAddr;
    private final String routeParamsEnc;
    private final String headersEnc;
    private final String error;

    // Lazily-parsed maps (null until first access).
    private Map<String, String> params;
    private Map<String, String> queryParams;
    private Map<String, String> headers;

    /**
     * Constructed from the Rust dispatch layer via JNI {@code NewObjectA}. The
     * argument order is part of the ABI contract with
     * {@code conduit-jni::build_request} — do not reorder.
     */
    Request(
            String method,
            String path,
            String queryString,
            String body,
            String contentType,
            String remoteAddr,
            String routeParamsEnc,
            String headersEnc,
            String error) {
        this.method = method == null ? "" : method;
        this.path = path == null ? "/" : path;
        this.queryString = queryString == null ? "" : queryString;
        this.body = body == null ? "" : body;
        this.contentType = contentType == null ? "" : contentType;
        this.remoteAddr = remoteAddr == null ? "" : remoteAddr;
        this.routeParamsEnc = routeParamsEnc == null ? "" : routeParamsEnc;
        this.headersEnc = headersEnc == null ? "" : headersEnc;
        this.error = error == null ? "" : error;
    }

    /** HTTP method, e.g. {@code "GET"}. */
    public String method() {
        return method;
    }

    /** Request path without the query string, e.g. {@code "/hello/world"}. */
    public String path() {
        return path;
    }

    /** Raw query string without the leading {@code "?"}, or {@code ""}. */
    public String queryString() {
        return queryString;
    }

    /** Raw request body as a UTF-8 string, or {@code ""}. */
    public String body() {
        return body;
    }

    /** Value of the {@code Content-Type} header, or {@code ""}. */
    public String contentType() {
        return contentType;
    }

    /** Remote peer IP address. */
    public String remoteAddr() {
        return remoteAddr;
    }

    /**
     * The error message — non-empty only when this request is being passed to
     * the registered error handler (mirrors {@code conduit.error} in the other
     * ports). Empty for normal dispatch.
     */
    public String error() {
        return error;
    }

    /** Named route captures, e.g. {@code /hello/:name} → {@code {name=world}}. */
    public Map<String, String> params() {
        if (params == null) {
            params = parseEncoded(routeParamsEnc);
        }
        return params;
    }

    /** A single named route capture, or {@code null}. */
    public String param(String name) {
        return params().get(name);
    }

    /** Parsed query-string parameters. */
    public Map<String, String> queryParams() {
        if (queryParams == null) {
            queryParams = parseEncoded(queryString);
        }
        return queryParams;
    }

    /** A single query parameter, or {@code null}. */
    public String queryParam(String name) {
        return queryParams().get(name);
    }

    /** Request headers, lower-cased names. */
    public Map<String, String> headers() {
        if (headers == null) {
            headers = parseEncoded(headersEnc);
        }
        return headers;
    }

    /** A single header value by (case-insensitive) name, or {@code null}. */
    public String header(String name) {
        return headers().get(name == null ? null : name.toLowerCase());
    }

    // ── Internal: decode the "k=v&k2=v2" wire format ────────────────────────

    private static Map<String, String> parseEncoded(String enc) {
        if (enc == null || enc.isEmpty()) {
            return Collections.emptyMap();
        }
        Map<String, String> map = new LinkedHashMap<>();
        for (String pair : enc.split("&")) {
            if (pair.isEmpty()) {
                continue;
            }
            int eq = pair.indexOf('=');
            if (eq < 0) {
                map.put(decode(pair), "");
            } else {
                map.put(decode(pair.substring(0, eq)), decode(pair.substring(eq + 1)));
            }
        }
        return map;
    }

    private static String decode(String s) {
        return URLDecoder.decode(s, StandardCharsets.UTF_8);
    }
}
