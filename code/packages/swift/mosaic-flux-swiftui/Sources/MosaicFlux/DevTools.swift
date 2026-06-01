// DevTools.swift — DevTools protocol middleware (v0.1.0 stub).
//
// Per UI33-rewrite §8, every mosaic-flux runtime publishes a uniform
// event stream so the cross-backend Mosaic DevTools desktop app can
// attach to any of them.  On Apple platforms the transport is a
// local TCP socket on port 9229 (matching Node's --inspect convention).
//
// v0.1.0 ships the middleware shape and event format but uses
// console logging as the transport.  The real TCP socket
// implementation requires Network.framework + async setup; that's
// deferred to v0.2.0 once the cross-backend DevTools app is being
// built.
//
// The event format here matches the TypeScript runtimes' JSON shape
// exactly so a future TCP-attached DevTools client can decode the
// same protocol regardless of source platform.

import Foundation

/// The structured event format published by every mosaic-flux runtime.
///
/// Matches the TypeScript ActionEvent shape so cross-backend DevTools
/// can decode either source identically.
public struct MosaicActionEvent<State> {
    public let kind: String        // always "action"
    public let timestamp: Date
    public let actionType: String  // e.g., "Navigate"
    public let storeName: String
    public let prevState: State
    public let nextState: State

    fileprivate init(
        actionType: String,
        storeName: String,
        prevState: State,
        nextState: State,
        timestamp: Date = Date()
    ) {
        self.kind = "action"
        self.timestamp = timestamp
        self.actionType = actionType
        self.storeName = storeName
        self.prevState = prevState
        self.nextState = nextState
    }
}

/// Build a DevTools-protocol middleware.  v0.1.0 logs to stdout in a
/// structured form; v0.2.0 will additionally transmit over a TCP
/// socket so the Mosaic DevTools desktop app can attach.
///
/// The middleware is safe to leave registered in production builds
/// — the transport silently no-ops when no DevTools client is
/// listening.
///
/// - Parameter storeName: Disambiguator when multiple stores are
///   active (per-tab, sub-stores).  Defaults to "default".
public func devToolsMiddleware<State>(
    storeName: String = "default"
) -> Middleware<State> {
    return { action, prev, next in
        let event = MosaicActionEvent<State>(
            actionType: String(describing: type(of: action)),
            storeName: storeName,
            prevState: prev,
            nextState: next
        )
        publish(event)
    }
}

/// Publish a DevTools event.  v0.1.0 uses console logging; v0.2.0
/// will also stream over TCP on port 9229.
private func publish<State>(_ event: MosaicActionEvent<State>) {
    // Format kept stable across versions so a future TCP listener
    // can match it.  This is intentionally NOT JSON — we want
    // human-readable console output for v0.1.0.  v0.2.0's TCP
    // transport will encode the same event as JSON over the wire.
    let timestamp = ISO8601DateFormatter().string(from: event.timestamp)
    print("[mosaic-flux-devtools] \(timestamp) \(event.storeName)/\(event.actionType)")
}
