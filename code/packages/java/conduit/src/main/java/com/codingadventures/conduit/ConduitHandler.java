package com.codingadventures.conduit;

/**
 * A request handler: turns a {@link Request} into a {@link Response}.
 *
 * <p>This is a {@link FunctionalInterface}, so handlers are usually written as
 * lambdas:
 *
 * <pre>{@code
 * app.get("/hello/:name", req -> Responses.text("Hello " + req.param("name")));
 * }</pre>
 *
 * <h2>Return-value protocol</h2>
 *
 * <ul>
 *   <li><b>Route / not-found / error handlers</b> must return a non-null
 *       {@link Response}.</li>
 *   <li><b>Before filters</b> return {@code null} to continue to routing, or a
 *       {@link Response} to short-circuit.</li>
 *   <li><b>After filters</b> return {@code null} to keep the prior response, or
 *       a {@link Response} to replace it.</li>
 * </ul>
 *
 * <p>A handler may also throw {@link HaltException} for a Sinatra-style deep
 * halt, or any other exception (which routes to the registered error handler).
 */
@FunctionalInterface
public interface ConduitHandler {
    /**
     * Handle a request.
     *
     * @param request the incoming request (never null)
     * @return the response, or {@code null} per the protocol above
     */
    Response handle(Request request);
}
