import XCTest

@testable import HttpCore

final class HttpCoreTests: XCTestCase {
    func testVersionRoundTrip() throws {
        let version = try HttpVersion.parse("HTTP/1.1")
        XCTAssertEqual(version.major, 1)
        XCTAssertEqual(version.minor, 1)
        XCTAssertEqual(version.description, "HTTP/1.1")
    }

    func testCaseInsensitiveHeaderLookup() {
        let headers = [Header(name: "Content-Type", value: "text/plain")]
        XCTAssertEqual(findHeader(headers, name: "content-type"), "text/plain")
    }

    func testContentHelpers() {
        let headers = [
            Header(name: "Content-Length", value: "42"),
            Header(name: "Content-Type", value: "text/html; charset=utf-8"),
        ]

        XCTAssertEqual(parseContentLength(headers), 42)
        let parsed = parseContentType(headers)
        XCTAssertEqual(parsed?.0, "text/html")
        XCTAssertEqual(parsed?.1, "utf-8")
    }

    func testHeadsDelegateToHelpers() {
        let request = RequestHead(
            method: "POST",
            target: "/submit",
            version: HttpVersion(major: 1, minor: 1),
            headers: [Header(name: "Content-Length", value: "5")]
        )
        let response = ResponseHead(
            version: HttpVersion(major: 1, minor: 0),
            status: 200,
            reason: "OK",
            headers: [Header(name: "Content-Type", value: "application/json")]
        )

        XCTAssertEqual(request.contentLength(), 5)
        XCTAssertEqual(response.contentType()?.0, "application/json")
        XCTAssertNil(response.contentType()?.1)
    }
}
