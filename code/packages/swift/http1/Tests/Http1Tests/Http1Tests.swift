import XCTest
@testable import Http1
import HttpCore

final class Http1Tests: XCTestCase {
    func testParsesSimpleRequest() throws {
        let parsed = try Http1.parseRequestHead(Data("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n".utf8))
        XCTAssertEqual(parsed.head.method, "GET")
        XCTAssertEqual(parsed.head.target, "/")
        XCTAssertEqual(parsed.bodyKind, .none)
    }

    func testParsesRequestContentLength() throws {
        let parsed = try Http1.parseRequestHead(Data("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello".utf8))
        XCTAssertEqual(parsed.bodyKind, .contentLength(5))
    }

    func testParsesResponseHead() throws {
        let parsed = try Http1.parseResponseHead(Data("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody".utf8))
        XCTAssertEqual(parsed.head.status, 200)
        XCTAssertEqual(parsed.head.reason, "OK")
        XCTAssertEqual(parsed.bodyKind, .contentLength(4))
    }

    func testUsesUntilEofWithoutContentLength() throws {
        let parsed = try Http1.parseResponseHead(Data("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n".utf8))
        XCTAssertEqual(parsed.bodyKind, .untilEof)
    }

    func testTreatsBodylessStatusesAsBodyless() throws {
        let parsed = try Http1.parseResponseHead(Data("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n".utf8))
        XCTAssertEqual(parsed.bodyKind, .none)
    }

    func testAcceptsLFOnlyAndPreservesDuplicateHeaders() throws {
        let parsed = try Http1.parseResponseHead(Data("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload".utf8))
        XCTAssertEqual(parsed.head.headers.map(\.value), ["a=1", "b=2"])
    }

    func testRejectsInvalidHeaders() {
        XCTAssertThrowsError(try Http1.parseRequestHead(Data("GET / HTTP/1.1\r\nHost example.com\r\n\r\n".utf8)))
    }

    func testRejectsInvalidContentLength() {
        XCTAssertThrowsError(try Http1.parseResponseHead(Data("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n".utf8)))
    }
}
