import Testing
import CConduit
@testable import Conduit

@Suite struct ResponseTests {
    @Test func htmlDefaults() {
        let r = Response.html("<h1>Hi</h1>")
        #expect(r.status == 200)
        #expect(r.bodyText == "<h1>Hi</h1>")
        #expect(r.headers.contains { $0.name == "content-type" && $0.value == "text/html; charset=utf-8" })
    }

    @Test func htmlExplicitStatus() {
        #expect(Response.html("x", status: 201).status == 201)
    }

    @Test func jsonContentType() {
        let r = Response.json("{\"ok\":1}")
        #expect(r.headers.contains { $0.name == "content-type" && $0.value == "application/json" })
        #expect(r.bodyText == "{\"ok\":1}")
    }

    @Test func textContentType() {
        let r = Response.text("pong")
        #expect(r.headers.contains { $0.value == "text/plain; charset=utf-8" })
        #expect(r.bodyText == "pong")
    }

    @Test func respondCustom() {
        let r = Response.respond(204, "", headers: [("x-y", "z")])
        #expect(r.status == 204)
        #expect(r.headers.contains { $0.name == "x-y" && $0.value == "z" })
    }

    @Test func redirectDefaults() throws {
        let r = try Response.redirect("/new")
        #expect(r.status == 302)
        #expect(r.headers.contains { $0.name == "location" && $0.value == "/new" })
    }

    @Test func redirectExplicitStatus() throws {
        #expect(try Response.redirect("/old", status: 301).status == 301)
    }

    @Test func redirectRejectsCRLF() {
        #expect(throws: ConduitError.self) {
            _ = try Response.redirect("/x\r\nSet-Cookie: evil=1")
        }
    }

    @Test func bodyTextRoundTrips() {
        let r = Response(status: 200, text: "héllo")
        #expect(r.bodyText == "héllo")
    }

    // toC builds a native response; reading it back must round-trip status/body/headers.
    @Test func toCRoundTrip() {
        let r = Response(status: 200, text: "body", headers: [("x-a", "b"), ("content-type", "text/plain")])
        let c = r.toC()
        #expect(c != nil)
        let back = Response(reading: c!)
        conduit_response_free(c)
        #expect(back.status == 200)
        #expect(back.bodyText == "body")
        #expect(back.headers.contains { $0.name == "x-a" && $0.value == "b" })
    }

    @Test func toCClampsStatus() {
        let c = Response(status: 999).toC()!
        #expect(conduit_response_status(c) == 599)
        conduit_response_free(c)
        let c2 = Response(status: 1).toC()!
        #expect(conduit_response_status(c2) == 100)
        conduit_response_free(c2)
    }

    @Test func toCDropsUnsafeHeaders() {
        // A header value with CR/LF must be dropped at the native boundary.
        let r = Response(status: 200, body: [], headers: [("x-ok", "fine"), ("x-bad", "a\r\nb")])
        let c = r.toC()!
        let back = Response(reading: c)
        conduit_response_free(c)
        #expect(back.headers.contains { $0.name == "x-ok" })
        #expect(!back.headers.contains { $0.name == "x-bad" })
    }
}
