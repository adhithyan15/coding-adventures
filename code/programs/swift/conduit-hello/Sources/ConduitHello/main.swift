import Conduit

// Entry point. Binds 127.0.0.1:<port> (default 3000) and serves in the
// foreground — the reactor dispatches handlers on this thread. Ctrl-C to stop.
//
//   swift run ConduitHello           # port 3000
//   swift run ConduitHello 8080      # or choose a port
//
// Then:  curl http://127.0.0.1:3000/hello/Ada

let port: UInt16 = CommandLine.arguments.count > 1
    ? (UInt16(CommandLine.arguments[1]) ?? 3000)
    : 3000

do {
    let server = try makeApp().bind(host: "127.0.0.1", port: port)
    print("conduit-hello listening on http://127.0.0.1:\(server.localPort)/  (Ctrl-C to stop)")
    server.serve()
} catch {
    print("failed to start: \(error)")
}
