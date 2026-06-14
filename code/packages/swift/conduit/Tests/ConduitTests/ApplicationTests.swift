import Testing
@testable import Conduit

@Suite struct ApplicationTests {
    @Test func settingsRoundTrip() {
        let app = Application()
        app.set("views", "tmpl")
        #expect(app.getSetting("views") == "tmpl")
    }

    @Test func missingSettingIsNil() {
        let app = Application()
        #expect(app.getSetting("nope") == nil)
    }

    @Test func setIsChainable() {
        let app = Application()
        #expect(app.set("a", "1") === app)
    }

    @Test func routeRegistrationsChain() {
        let app = Application()
        #expect(app.get("/") { _ in .text("x") } === app)
        #expect(app.post("/x") { _ in .text("x") } === app)
        #expect(app.put("/x") { _ in .text("x") } === app)
        #expect(app.delete("/x") { _ in .text("x") } === app)
        #expect(app.patch("/x") { _ in .text("x") } === app)
    }

    @Test func hookRegistrationsChain() {
        let app = Application()
        #expect(app.before { _ in nil } === app)
        #expect(app.after { _, r in r } === app)
        #expect(app.notFound { _ in .text("nf", status: 404) } === app)
        #expect(app.onError { _ in .text("err", status: 500) } === app)
    }

    @Test func bindReturnsServerWithPort() throws {
        let app = Application()
        app.get("/") { _ in .text("x") }
        let server = try app.bind(host: "127.0.0.1", port: 0)
        #expect(server.localPort > 0)
        server.stop()
    }

    @Test func doubleBindThrows() throws {
        let app = Application()
        app.get("/") { _ in .text("x") }
        let s = try app.bind(host: "127.0.0.1", port: 0)
        #expect(throws: ConduitError.self) {
            _ = try app.bind(host: "127.0.0.1", port: 0)
        }
        s.stop()
    }
}
