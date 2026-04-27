// ============================================================================
// JsonRpcTests.swift — comprehensive tests for the JSON-RPC 2.0 package
// ============================================================================
//
// Test coverage areas:
//
//  1. Message parsing — Request, Notification, Response (success + error)
//  2. Message serialization — messageToMap round-trips
//  3. Error codes — all five standard codes
//  4. Content-Length framing — reader and writer
//  5. Server dispatch — request handling, notification handling, method not found
//  6. Edge cases — missing Content-Length, invalid JSON, empty input
//
// ============================================================================

import XCTest
@testable import JsonRpc
import Foundation

final class JsonRpcTests: XCTestCase {

    // ─── Error Codes ────────────────────────────────────────────────────────

    func testErrorCodeValues() {
        // Verify the standard JSON-RPC 2.0 error code constants match the spec.
        XCTAssertEqual(JsonRpcErrorCodes.parseError, -32700)
        XCTAssertEqual(JsonRpcErrorCodes.invalidRequest, -32600)
        XCTAssertEqual(JsonRpcErrorCodes.methodNotFound, -32601)
        XCTAssertEqual(JsonRpcErrorCodes.invalidParams, -32602)
        XCTAssertEqual(JsonRpcErrorCodes.internalError, -32603)
    }

    // ─── Message Parsing: Request ────────────────────────────────────────────

    func testParseRequestWithIntId() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": ["line": 5],
        ]
        let msg = try parseMessage(dict)
        guard case .request(let req) = msg else {
            XCTFail("Expected Request, got \(msg)")
            return
        }
        XCTAssertEqual(req.id.value as? Int, 1)
        XCTAssertEqual(req.method, "textDocument/hover")
        XCTAssertNotNil(req.params)
    }

    func testParseRequestWithStringId() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "id": "abc-123",
            "method": "ping",
        ]
        let msg = try parseMessage(dict)
        guard case .request(let req) = msg else {
            XCTFail("Expected Request")
            return
        }
        XCTAssertEqual(req.id.value as? String, "abc-123")
        XCTAssertEqual(req.method, "ping")
        XCTAssertNil(req.params)
    }

    func testParseRequestWithoutParams() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "id": 42,
            "method": "shutdown",
        ]
        let msg = try parseMessage(dict)
        guard case .request(let req) = msg else {
            XCTFail("Expected Request")
            return
        }
        XCTAssertEqual(req.method, "shutdown")
        XCTAssertNil(req.params)
    }

    // ─── Message Parsing: Notification ───────────────────────────────────────

    func testParseNotification() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": ["uri": "file:///main.bf"],
        ]
        let msg = try parseMessage(dict)
        guard case .notification(let notif) = msg else {
            XCTFail("Expected Notification, got \(msg)")
            return
        }
        XCTAssertEqual(notif.method, "textDocument/didOpen")
        XCTAssertNotNil(notif.params)
    }

    func testParseNotificationWithoutParams() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "method": "initialized",
        ]
        let msg = try parseMessage(dict)
        guard case .notification(let notif) = msg else {
            XCTFail("Expected Notification")
            return
        }
        XCTAssertEqual(notif.method, "initialized")
        XCTAssertNil(notif.params)
    }

    // ─── Message Parsing: Response ──────────────────────────────────────────

    func testParseSuccessResponse() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "id": 1,
            "result": ["capabilities": [:]],
        ]
        let msg = try parseMessage(dict)
        guard case .response(let resp) = msg else {
            XCTFail("Expected Response")
            return
        }
        XCTAssertEqual(resp.id?.value as? Int, 1)
        XCTAssertNil(resp.error)
        XCTAssertNotNil(resp.result)
    }

    func testParseErrorResponse() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "id": 1,
            "error": [
                "code": -32601,
                "message": "Method not found",
            ],
        ]
        let msg = try parseMessage(dict)
        guard case .response(let resp) = msg else {
            XCTFail("Expected Response")
            return
        }
        XCTAssertEqual(resp.id?.value as? Int, 1)
        XCTAssertNotNil(resp.error)
        XCTAssertEqual(resp.error?.code, -32601)
        XCTAssertEqual(resp.error?.message, "Method not found")
    }

    func testParseNullResultResponse() throws {
        let dict: [String: Any] = [
            "jsonrpc": "2.0",
            "id": 5,
            "result": NSNull(),
        ]
        let msg = try parseMessage(dict)
        guard case .response(let resp) = msg else {
            XCTFail("Expected Response")
            return
        }
        XCTAssertEqual(resp.id?.value as? Int, 5)
        XCTAssertNil(resp.error)
    }

    // ─── Message Parsing: Invalid ────────────────────────────────────────────

    func testParseInvalidMessageShape() {
        // No id, no method, no result, no error → invalid
        let dict: [String: Any] = ["jsonrpc": "2.0"]
        XCTAssertThrowsError(try parseMessage(dict)) { error in
            guard let e = error as? JsonRpcError else {
                XCTFail("Expected JsonRpcError")
                return
            }
            XCTAssertEqual(e.code, JsonRpcErrorCodes.invalidRequest)
        }
    }

    // ─── Message Serialization ──────────────────────────────────────────────

    func testMessageToMapRequest() {
        let req = Request(id: AnySendable(1), method: "foo", params: AnySendable(["x": 42]))
        let dict = messageToMap(.request(req))
        XCTAssertEqual(dict["jsonrpc"] as? String, "2.0")
        XCTAssertEqual(dict["id"] as? Int, 1)
        XCTAssertEqual(dict["method"] as? String, "foo")
        XCTAssertNotNil(dict["params"])
    }

    func testMessageToMapNotification() {
        let notif = Notification(method: "bar")
        let dict = messageToMap(.notification(notif))
        XCTAssertEqual(dict["jsonrpc"] as? String, "2.0")
        XCTAssertEqual(dict["method"] as? String, "bar")
        XCTAssertNil(dict["id"])
        XCTAssertNil(dict["params"])
    }

    func testMessageToMapSuccessResponse() {
        let resp = Response(id: AnySendable(1), result: AnySendable(42))
        let dict = messageToMap(.response(resp))
        XCTAssertEqual(dict["jsonrpc"] as? String, "2.0")
        XCTAssertEqual(dict["id"] as? Int, 1)
        XCTAssertEqual(dict["result"] as? Int, 42)
        XCTAssertNil(dict["error"])
    }

    func testMessageToMapErrorResponse() {
        let err = ResponseError(code: -32601, message: "Method not found")
        let resp = Response(id: AnySendable(1), error: err)
        let dict = messageToMap(.response(resp))
        XCTAssertEqual(dict["jsonrpc"] as? String, "2.0")
        XCTAssertNotNil(dict["error"])
        let errorDict = dict["error"] as? [String: Any]
        XCTAssertEqual(errorDict?["code"] as? Int, -32601)
        XCTAssertEqual(errorDict?["message"] as? String, "Method not found")
    }

    // ─── Reader Tests ───────────────────────────────────────────────────────

    func testReaderReadsSingleMessage() throws {
        let json = """
        {"jsonrpc":"2.0","id":1,"method":"ping"}
        """
        let payload = Data(json.utf8)
        let frame = "Content-Length: \(payload.count)\r\n\r\n" + json
        let reader = MessageReader(frame)

        let msg = try reader.readMessage()
        XCTAssertNotNil(msg)
        guard case .request(let req) = msg else {
            XCTFail("Expected Request")
            return
        }
        XCTAssertEqual(req.method, "ping")
    }

    func testReaderReadsMultipleMessages() throws {
        let json1 = """
        {"jsonrpc":"2.0","id":1,"method":"ping"}
        """
        let json2 = """
        {"jsonrpc":"2.0","method":"initialized"}
        """
        let frame = "Content-Length: \(Data(json1.utf8).count)\r\n\r\n" + json1
                   + "Content-Length: \(Data(json2.utf8).count)\r\n\r\n" + json2
        let reader = MessageReader(frame)

        let msg1 = try reader.readMessage()
        XCTAssertNotNil(msg1)

        let msg2 = try reader.readMessage()
        XCTAssertNotNil(msg2)
        guard case .notification(let notif) = msg2 else {
            XCTFail("Expected Notification")
            return
        }
        XCTAssertEqual(notif.method, "initialized")

        // Third read should return nil (EOF)
        let msg3 = try reader.readMessage()
        XCTAssertNil(msg3)
    }

    func testReaderReturnsNilOnEmptyInput() throws {
        let reader = MessageReader("")
        let msg = try reader.readMessage()
        XCTAssertNil(msg)
    }

    func testReaderThrowsOnInvalidJSON() {
        let badJson = "not valid json at all"
        let frame = "Content-Length: \(Data(badJson.utf8).count)\r\n\r\n" + badJson
        let reader = MessageReader(frame)

        XCTAssertThrowsError(try reader.readMessage()) { error in
            guard let e = error as? JsonRpcError else {
                XCTFail("Expected JsonRpcError")
                return
            }
            XCTAssertEqual(e.code, JsonRpcErrorCodes.parseError)
        }
    }

    func testReaderThrowsOnMissingContentLength() {
        let frame = "X-Custom: foo\r\n\r\n{}"
        let reader = MessageReader(frame)

        XCTAssertThrowsError(try reader.readMessage()) { error in
            guard let e = error as? JsonRpcError else {
                XCTFail("Expected JsonRpcError")
                return
            }
            XCTAssertEqual(e.code, JsonRpcErrorCodes.parseError)
        }
    }

    func testReaderThrowsOnTruncatedPayload() {
        // Content-Length says 100 bytes but only 5 bytes follow
        let frame = "Content-Length: 100\r\n\r\nhello"
        let reader = MessageReader(frame)

        XCTAssertThrowsError(try reader.readMessage()) { error in
            guard let e = error as? JsonRpcError else {
                XCTFail("Expected JsonRpcError")
                return
            }
            XCTAssertEqual(e.code, JsonRpcErrorCodes.parseError)
        }
    }

    // ─── Writer Tests ───────────────────────────────────────────────────────

    func testWriterFramesMessage() throws {
        let output = DataOutput()
        let writer = MessageWriter(output)
        let req = Request(id: AnySendable(1), method: "ping")
        try writer.writeMessage(.request(req))

        let str = output.string
        XCTAssertTrue(str.contains("Content-Length:"))
        XCTAssertTrue(str.contains("\"jsonrpc\":\"2.0\""))
        XCTAssertTrue(str.contains("\"method\":\"ping\""))
    }

    func testWriterWriteRaw() {
        let output = DataOutput()
        let writer = MessageWriter(output)
        let json = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}"
        writer.writeRaw(json)

        let str = output.string
        XCTAssertTrue(str.hasPrefix("Content-Length: \(Data(json.utf8).count)\r\n\r\n"))
        XCTAssertTrue(str.hasSuffix(json))
    }

    func testWriterThenReaderRoundTrip() throws {
        // Write a message, then read it back
        let output = DataOutput()
        let writer = MessageWriter(output)
        let req = Request(id: AnySendable(42), method: "test/roundTrip", params: AnySendable(["key": "value"]))
        try writer.writeMessage(.request(req))

        let reader = MessageReader(output.data)
        let msg = try reader.readMessage()
        guard case .request(let readReq) = msg else {
            XCTFail("Expected Request")
            return
        }
        XCTAssertEqual(readReq.id.value as? Int, 42)
        XCTAssertEqual(readReq.method, "test/roundTrip")
    }

    // ─── Server Dispatch Tests ──────────────────────────────────────────────

    func testServerDispatchesRequest() throws {
        let json = """
        {"jsonrpc":"2.0","id":1,"method":"ping","params":null}
        """
        let inputData = Data(("Content-Length: \(Data(json.utf8).count)\r\n\r\n" + json).utf8)
        let output = DataOutput()

        let server = Server(inputData: inputData, output: output)
        server.onRequest("ping") { id, params in
            return ("pong", nil)
        }
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("\"result\":\"pong\""),
                       "Expected 'pong' result in response: \(responseStr)")
    }

    func testServerDispatchesNotification() throws {
        let json = """
        {"jsonrpc":"2.0","method":"test/notify","params":{"x":1}}
        """
        let inputData = Data(("Content-Length: \(Data(json.utf8).count)\r\n\r\n" + json).utf8)
        let output = DataOutput()

        var receivedParams: Any? = nil
        let server = Server(inputData: inputData, output: output)
        server.onNotification("test/notify") { params in
            receivedParams = params
        }
        server.serve()

        // Notifications produce no response
        XCTAssertEqual(output.data.count, 0, "No output expected for notification")
        XCTAssertNotNil(receivedParams)
    }

    func testServerSendsMethodNotFound() throws {
        let json = """
        {"jsonrpc":"2.0","id":1,"method":"nonexistent"}
        """
        let inputData = Data(("Content-Length: \(Data(json.utf8).count)\r\n\r\n" + json).utf8)
        let output = DataOutput()

        let server = Server(inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("-32601"),
                       "Expected MethodNotFound error code in: \(responseStr)")
    }

    func testServerHandlerReturnsError() throws {
        let json = """
        {"jsonrpc":"2.0","id":1,"method":"fail"}
        """
        let inputData = Data(("Content-Length: \(Data(json.utf8).count)\r\n\r\n" + json).utf8)
        let output = DataOutput()

        let server = Server(inputData: inputData, output: output)
        server.onRequest("fail") { id, params in
            return (nil, ResponseError(code: -32602, message: "bad params"))
        }
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("-32602"),
                       "Expected InvalidParams code in: \(responseStr)")
        XCTAssertTrue(responseStr.contains("bad params"))
    }

    func testServerIgnoresUnknownNotification() throws {
        let json = """
        {"jsonrpc":"2.0","method":"unknown/notification"}
        """
        let inputData = Data(("Content-Length: \(Data(json.utf8).count)\r\n\r\n" + json).utf8)
        let output = DataOutput()

        let server = Server(inputData: inputData, output: output)
        server.serve()

        // No output expected — unknown notifications are silently ignored
        XCTAssertEqual(output.data.count, 0)
    }

    func testServerProcessesMultipleMessages() throws {
        let json1 = """
        {"jsonrpc":"2.0","id":1,"method":"add","params":{"a":2,"b":3}}
        """
        let json2 = """
        {"jsonrpc":"2.0","id":2,"method":"add","params":{"a":10,"b":20}}
        """
        let frame = "Content-Length: \(Data(json1.utf8).count)\r\n\r\n" + json1
                   + "Content-Length: \(Data(json2.utf8).count)\r\n\r\n" + json2
        let inputData = Data(frame.utf8)
        let output = DataOutput()

        var callCount = 0
        let server = Server(inputData: inputData, output: output)
        server.onRequest("add") { id, params in
            callCount += 1
            guard let p = params as? [String: Any],
                  let a = p["a"] as? Int,
                  let b = p["b"] as? Int else {
                return (nil, ResponseError(code: -32602, message: "bad params"))
            }
            return (a + b, nil)
        }
        server.serve()

        XCTAssertEqual(callCount, 2, "Expected handler called twice")
        let responseStr = output.string
        // Both responses should be present
        XCTAssertTrue(responseStr.contains("\"result\":5") || responseStr.contains("\"result\" : 5"),
                       "Expected result 5 in: \(responseStr)")
    }

    // ─── ResponseError Tests ────────────────────────────────────────────────

    func testResponseErrorEquality() {
        let a = ResponseError(code: -32601, message: "Method not found")
        let b = ResponseError(code: -32601, message: "Method not found")
        let c = ResponseError(code: -32602, message: "Invalid params")
        XCTAssertEqual(a, b)
        XCTAssertNotEqual(a, c)
    }

    func testResponseErrorConformsToError() {
        let err: Error = ResponseError(code: -32603, message: "Internal error")
        XCTAssertNotNil(err)
    }

    // ─── AnySendable Tests ──────────────────────────────────────────────────

    func testAnySendableEquality() {
        XCTAssertEqual(AnySendable("hello"), AnySendable("hello"))
        XCTAssertEqual(AnySendable(42), AnySendable(42))
        XCTAssertEqual(AnySendable(true), AnySendable(true))
        XCTAssertNotEqual(AnySendable("a"), AnySendable("b"))
        XCTAssertNotEqual(AnySendable(1), AnySendable(2))
    }

    // ─── normalizeId Tests ──────────────────────────────────────────────────

    func testNormalizeIdDouble() {
        let result = normalizeId(1.0 as Double)
        XCTAssertEqual(result as? Int, 1)
    }

    func testNormalizeIdString() {
        let result = normalizeId("abc")
        XCTAssertEqual(result as? String, "abc")
    }

    func testNormalizeIdInt() {
        let result = normalizeId(42)
        XCTAssertEqual(result as? Int, 42)
    }

    // ─── Unicode in Content-Length ───────────────────────────────────────────

    func testWriterHandlesUnicodePayload() throws {
        let output = DataOutput()
        let writer = MessageWriter(output)
        // The euro sign € is 3 UTF-8 bytes but 1 character
        let resp = Response(id: AnySendable(1), result: AnySendable("€"))
        try writer.writeMessage(.response(resp))

        // Read it back — the reader should handle multi-byte characters correctly
        let reader = MessageReader(output.data)
        let msg = try reader.readMessage()
        XCTAssertNotNil(msg)
    }

    // ─── Server chaining ────────────────────────────────────────────────────

    func testServerMethodChaining() {
        let output = DataOutput()
        let server = Server(inputData: Data(), output: output)
        let returned = server
            .onRequest("a") { _, _ in (nil, nil) }
            .onNotification("b") { _ in }
            .onRequest("c") { _, _ in (nil, nil) }

        // Chaining should return the same server instance
        XCTAssertTrue(returned === server)
    }
}
