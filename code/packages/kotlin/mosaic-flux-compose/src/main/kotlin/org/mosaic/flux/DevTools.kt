// DevTools.kt — DevTools protocol middleware (v0.1.0 stub).
//
// Per UI33-rewrite §8, every mosaic-flux runtime publishes a uniform
// event stream so the Mosaic DevTools desktop app can attach.  On
// JVM platforms the transport is a local TCP socket on port 9229.
//
// v0.1.0 ships the middleware shape and logs each event to stdout
// in a format the future DevTools client will recognise.  The TCP
// socket implementation requires kotlinx-io or java.net.Socket plus
// async setup — deferred to v0.2.0.

package org.mosaic.flux

import java.time.Instant

/**
 * Build a DevTools-protocol middleware.  Logs structured events to
 * stdout; v0.2.0 will additionally transmit to localhost:9229 so the
 * Mosaic DevTools desktop app can attach.
 *
 * @param storeName Disambiguator when multiple stores are active.
 */
fun <S> devToolsMiddleware(storeName: String = "default"): Middleware<S> {
    return { action, _, _ ->
        val timestamp = Instant.now().toString()
        val actionType = action::class.simpleName ?: "Action"
        println("[mosaic-flux-devtools] $timestamp $storeName/$actionType")
    }
}
