import Foundation
import Testing
@testable import Rpc

private struct WireMessage: Codable, Equatable {
    enum Kind: String, Codable {
        case request
        case response
        case errorResponse
        case notification
    }

    var kind: Kind
    var idInteger: Int?
    var idString: String?
    var method: String?
    var params: String?
    var result: String?
    var code: Int?
    var message: String?
    var data: String?
}

private final class MockCodec: RpcCodec {
    typealias Value = String

    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    func encode(_ message: RpcMessage<String>) throws -> Data {
        let wire = Self.toWire(message)
        return try encoder.encode(wire)
    }

    func decode(_ bytes: Data) throws -> RpcMessage<String> {
        if bytes == Data("BAD_BYTES".utf8) {
            throw RpcErrorResponse<String>.parseError(message: "Undecodable bytes")
        }

        if bytes == Data("BAD_SHAPE".utf8) {
            throw RpcErrorResponse<String>.invalidRequest(message: "Not an RPC message")
        }

        do {
            let wire = try decoder.decode(WireMessage.self, from: bytes)
            return try Self.fromWire(wire)
        } catch let error as RpcErrorResponse<String> {
            throw error
        } catch {
            throw RpcErrorResponse<String>.parseError(message: "Undecodable bytes")
        }
    }

    private static func toWire(_ message: RpcMessage<String>) -> WireMessage {
        switch message {
        case .request(let request):
            return WireMessage(
                kind: .request,
                idInteger: wireIDInteger(request.id),
                idString: wireIDString(request.id),
                method: request.method,
                params: request.params,
                result: nil,
                code: nil,
                message: nil,
                data: nil
            )

        case .response(let response):
            return WireMessage(
                kind: .response,
                idInteger: wireIDInteger(response.id),
                idString: wireIDString(response.id),
                method: nil,
                params: nil,
                result: response.result,
                code: nil,
                message: nil,
                data: nil
            )

        case .errorResponse(let error):
            return WireMessage(
                kind: .errorResponse,
                idInteger: wireIDInteger(error.id),
                idString: wireIDString(error.id),
                method: nil,
                params: nil,
                result: nil,
                code: error.code,
                message: error.message,
                data: error.data
            )

        case .notification(let notification):
            return WireMessage(
                kind: .notification,
                idInteger: nil,
                idString: nil,
                method: notification.method,
                params: notification.params,
                result: nil,
                code: nil,
                message: nil,
                data: nil
            )
        }
    }

    private static func fromWire(_ wire: WireMessage) throws -> RpcMessage<String> {
        switch wire.kind {
        case .request:
            guard let id = wireID(from: wire) else {
                throw RpcErrorResponse<String>.invalidRequest(message: "Missing request id")
            }
            guard let method = wire.method else {
                throw RpcErrorResponse<String>.invalidRequest(message: "Missing request method")
            }
            return .request(RpcRequest(id: id, method: method, params: wire.params))

        case .response:
            guard let id = wireID(from: wire) else {
                throw RpcErrorResponse<String>.invalidRequest(message: "Missing response id")
            }
            guard let result = wire.result else {
                throw RpcErrorResponse<String>.invalidRequest(message: "Missing response result")
            }
            return .response(RpcResponse(id: id, result: result))

        case .errorResponse:
            return .errorResponse(
                RpcErrorResponse(
                    id: wireID(from: wire),
                    code: wire.code ?? RpcErrorCodes.invalidRequest,
                    message: wire.message ?? "Invalid request",
                    data: wire.data
                )
            )

        case .notification:
            guard let method = wire.method else {
                throw RpcErrorResponse<String>.invalidRequest(message: "Missing notification method")
            }
            return .notification(RpcNotification(method: method, params: wire.params))
        }
    }

    private static func wireIDInteger(_ id: RpcId?) -> Int? {
        guard let id else {
            return nil
        }

        if case .integer(let value) = id {
            return value
        }
        return nil
    }

    private static func wireIDString(_ id: RpcId?) -> String? {
        guard let id else {
            return nil
        }

        if case .string(let value) = id {
            return value
        }
        return nil
    }

    private static func wireID(from wire: WireMessage) -> RpcId? {
        if let value = wire.idInteger {
            return .integer(value)
        }
        if let value = wire.idString {
            return .string(value)
        }
        return nil
    }
}

private final class MockFramer: RpcFramer {
    private var inputFrames: [Data]
    private(set) var writtenFrames: [Data] = []

    init(_ inputFrames: [Data] = []) {
        self.inputFrames = inputFrames
    }

    func readFrame() throws -> Data? {
        if inputFrames.isEmpty {
            return nil
        }

        return inputFrames.removeFirst()
    }

    func writeFrame(_ bytes: Data) throws {
        writtenFrames.append(bytes)
    }
}

private final class ThrowingFramer: RpcFramer {
    private var readCount = 0
    private(set) var writtenFrames: [Data] = []

    func readFrame() throws -> Data? {
        readCount += 1
        if readCount == 1 {
            throw RpcErrorResponse<String>.parseError(message: "Framing error")
        }
        return nil
    }

    func writeFrame(_ bytes: Data) throws {
        writtenFrames.append(bytes)
    }
}

private func encode(_ message: RpcMessage<String>, using codec: MockCodec = MockCodec()) -> Data {
    do {
        return try codec.encode(message)
    } catch {
        fatalError("Unexpected encoding failure: \(error)")
    }
}

private func decode(_ bytes: Data, using codec: MockCodec = MockCodec()) -> RpcMessage<String> {
    do {
        return try codec.decode(bytes)
    } catch {
        fatalError("Unexpected decoding failure: \(error)")
    }
}

private func runServer(
    _ inputMessages: [RpcMessage<String>],
    setup: (RpcServer<MockCodec, MockFramer>) -> Void
) -> [RpcMessage<String>] {
    let codec = MockCodec()
    let framer = MockFramer(inputMessages.map { encode($0, using: codec) })
    let server = RpcServer(codec: codec, framer: framer)
    setup(server)
    server.serve()
    return framer.writtenFrames.map { decode($0, using: codec) }
}

private func makeClient(
    _ responseMessages: [RpcMessage<String>]
) -> (client: RpcClient<MockCodec, MockFramer>, framer: MockFramer) {
    let codec = MockCodec()
    let framer = MockFramer(responseMessages.map { encode($0, using: codec) })
    let client = RpcClient(codec: codec, framer: framer)
    return (client, framer)
}

@Suite("RpcId")
struct RpcIdTests {
    @Test("supports integer and string literals")
    func literals() {
        let integerId: RpcId = 42
        let stringId: RpcId = "abc"

        #expect(integerId == .integer(42))
        #expect(stringId == .string("abc"))
        #expect(integerId.description == "42")
        #expect(stringId.description == "abc")
    }
}

@Suite("RpcErrorResponse")
struct RpcErrorResponseTests {
    @Test("constructors use the standard error table")
    func standardCodes() {
        #expect(RpcErrorResponse<String>.parseError().code == RpcErrorCodes.parseError)
        #expect(RpcErrorResponse<String>.invalidRequest().code == RpcErrorCodes.invalidRequest)
        #expect(RpcErrorResponse<String>.methodNotFound().code == RpcErrorCodes.methodNotFound)
        #expect(RpcErrorResponse<String>.invalidParams().code == RpcErrorCodes.invalidParams)
        #expect(RpcErrorResponse<String>.internalError().code == RpcErrorCodes.internalError)
    }

    @Test("withID fills in the response id")
    func withID() {
        let error = RpcErrorResponse<String>.internalError(message: "boom")
        let updated = error.withID(7)

        #expect(error.id == nil)
        #expect(updated.id == .integer(7))
        #expect(updated.code == error.code)
    }
}

@Suite("RpcServer")
struct RpcServerTests {
    @Test("dispatches requests to handlers and writes responses")
    func requestDispatch() {
        let responses = runServer(
            [.request(RpcRequest(id: 1, method: "ping"))],
            setup: { server in
                server.onRequest("ping") { id, params in
                    #expect(id == .integer(1))
                    #expect(params == nil)
                    return "pong"
                }
            }
        )

        #expect(responses.count == 1)
        if case .response(let response) = responses[0] {
            #expect(response.id == .integer(1))
            #expect(response.result == "pong")
        } else {
            Issue.record("Expected a success response")
        }
    }

    @Test("passes params to request handlers")
    func requestParams() {
        let responses = runServer(
            [.request(RpcRequest(id: 2, method: "add", params: "3+4"))],
            setup: { server in
                server.onRequest("add") { _, params in
                    #expect(params == "3+4")
                    return "7"
                }
            }
        )

        #expect(responses.count == 1)
        if case .response(let response) = responses[0] {
            #expect(response.result == "7")
        } else {
            Issue.record("Expected a success response")
        }
    }

    @Test("returns method not found for unknown requests")
    func unknownMethod() {
        let responses = runServer([.request(RpcRequest(id: 3, method: "missing"))]) { _ in }

        #expect(responses.count == 1)
        if case .errorResponse(let error) = responses[0] {
            #expect(error.id == .integer(3))
            #expect(error.code == RpcErrorCodes.methodNotFound)
        } else {
            Issue.record("Expected an error response")
        }
    }

    @Test("uses internal error for thrown handler errors")
    func thrownHandlerError() {
        let responses = runServer([.request(RpcRequest(id: 4, method: "boom"))]) { server in
            server.onRequest("boom") { _, _ in
                throw TestError()
            }
        }

        #expect(responses.count == 1)
        if case .errorResponse(let error) = responses[0] {
            #expect(error.id == .integer(4))
            #expect(error.code == RpcErrorCodes.internalError)
        } else {
            Issue.record("Expected an error response")
        }
    }

    @Test("dispatches notifications without writing a response")
    func notificationDispatch() {
        let responses = runServer([.notification(RpcNotification(method: "log", params: "hello"))]) { server in
            server.onNotification("log") { params in
                #expect(params == "hello")
            }
        }

        #expect(responses.isEmpty)
    }

    @Test("silently drops unknown notifications")
    func unknownNotification() {
        let responses = runServer([.notification(RpcNotification(method: "missing"))]) { _ in }
        #expect(responses.isEmpty)
    }

    @Test("reports codec parse errors with nil id")
    func codecParseError() {
        let codec = MockCodec()
        let framer = MockFramer([Data("BAD_BYTES".utf8)])
        let server = RpcServer(codec: codec, framer: framer)
        server.serve()

        #expect(framer.writtenFrames.count == 1)
        if case .errorResponse(let error) = decode(framer.writtenFrames[0], using: codec) {
            #expect(error.id == nil)
            #expect(error.code == RpcErrorCodes.parseError)
        } else {
            Issue.record("Expected a parse error response")
        }
    }

    @Test("reports framing errors with nil id")
    func framingError() {
        let framer = ThrowingFramer()
        let codec = MockCodec()
        let server = RpcServer(codec: codec, framer: framer)
        server.serve()

        #expect(framer.writtenFrames.count == 1)
        if case .errorResponse(let error) = decode(framer.writtenFrames[0], using: codec) {
            #expect(error.id == nil)
            #expect(error.code == RpcErrorCodes.parseError)
        } else {
            Issue.record("Expected a framing error response")
        }
    }

    @Test("handles multiple requests in sequence")
    func multipleRequests() {
        let responses = runServer(
            [
                .request(RpcRequest(id: 10, method: "echo", params: "hello")),
                .request(RpcRequest(id: 11, method: "echo", params: "world")),
            ]
        ) { server in
            server.onRequest("echo") { _, params in
                params ?? ""
            }
        }

        #expect(responses.count == 2)
        if case .response(let first) = responses[0], case .response(let second) = responses[1] {
            #expect(first.result == "hello")
            #expect(second.result == "world")
        } else {
            Issue.record("Expected two success responses")
        }
    }

    @Test("onRequest and onNotification are chainable")
    func chainableRegistration() {
        let server = RpcServer(codec: MockCodec(), framer: MockFramer())
        let returned = server
            .onRequest("a") { _, _ in "" }
            .onNotification("b") { _ in }

        #expect(returned === server)
    }
}

@Suite("RpcClient")
struct RpcClientTests {
    @Test("request returns the matching result")
    func requestSuccess() throws {
        let (client, framer) = makeClient([.response(RpcResponse(id: 1, result: "pong"))])

        let result = try client.request("ping")
        #expect(result == "pong")

        #expect(framer.writtenFrames.count == 1)
        if case .request(let request) = decode(framer.writtenFrames[0]) {
            #expect(request.id == .integer(1))
            #expect(request.method == "ping")
        } else {
            Issue.record("Expected a request frame to be written")
        }
    }

    @Test("request forwards server error responses")
    func requestServerError() {
        let (client, _) = makeClient([
            .errorResponse(RpcErrorResponse(id: 1, code: RpcErrorCodes.methodNotFound, message: "Method not found"))
        ])

        do {
            _ = try client.request("missing")
            Issue.record("Expected the request to throw")
        } catch let error as RpcErrorResponse<String> {
            #expect(error.code == RpcErrorCodes.methodNotFound)
            #expect(error.message == "Method not found")
        } catch {
            Issue.record("Unexpected error: \(error)")
        }
    }

    @Test("request throws on EOF before a response")
    func requestEOF() {
        let (client, _) = makeClient([])

        do {
            _ = try client.request("ping")
            Issue.record("Expected the request to throw")
        } catch let error as RpcErrorResponse<String> {
            #expect(error.code == RpcErrorCodes.internalError)
            #expect(error.id == .integer(1))
        } catch {
            Issue.record("Unexpected error: \(error)")
        }
    }

    @Test("notify writes a notification and does not wait for a response")
    func notify() throws {
        let (client, framer) = makeClient([])

        let returned = try client.notify("log", params: "hello")
        #expect(returned === client)
        #expect(framer.writtenFrames.count == 1)

        if case .notification(let notification) = decode(framer.writtenFrames[0]) {
            #expect(notification.method == "log")
            #expect(notification.params == "hello")
        } else {
            Issue.record("Expected a notification frame")
        }
    }

    @Test("dispatches server-push notifications during request wait")
    func serverPushNotifications() throws {
        let (client, _) = makeClient([
            .notification(RpcNotification(method: "progress", params: "25%")),
            .notification(RpcNotification(method: "progress", params: "50%")),
            .response(RpcResponse(id: 1, result: "done")),
        ])

        var notificationCount = 0
        client.onNotification("progress") { params in
            notificationCount += 1
            #expect(params != nil)
        }

        let result = try client.request("work")
        #expect(result == "done")
        #expect(notificationCount == 2)
    }

    @Test("ignores responses for other ids")
    func ignoresOtherIds() throws {
        let (client, _) = makeClient([
            .response(RpcResponse(id: 99, result: "stale")),
            .response(RpcResponse(id: 1, result: "fresh")),
        ])

        let result = try client.request("ping")
        #expect(result == "fresh")
    }

    @Test("onNotification is chainable")
    func onNotificationChainable() {
        let (client, _) = makeClient([])
        let returned = client
            .onNotification("a") { _ in }
            .onNotification("b") { _ in }

        #expect(returned === client)
    }

    @Test("request ids increment from 1")
    func requestIds() throws {
        let (client, framer) = makeClient([
            .response(RpcResponse(id: 1, result: "first")),
            .response(RpcResponse(id: 2, result: "second")),
        ])

        _ = try client.request("a")
        _ = try client.request("b")

        #expect(framer.writtenFrames.count == 2)
        if case .request(let first) = decode(framer.writtenFrames[0]),
           case .request(let second) = decode(framer.writtenFrames[1]) {
            #expect(first.id == .integer(1))
            #expect(second.id == .integer(2))
        } else {
            Issue.record("Expected two request frames")
        }
    }
}

private struct TestError: Error {}
