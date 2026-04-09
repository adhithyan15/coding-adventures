import XCTest
@testable import UUID

final class UUIDTests: XCTestCase {
    func testV4() {
        let u1 = UUID.v4()
        XCTAssertEqual(u1.version, 4)
        XCTAssertEqual(u1.variant, "rfc4122")
        let u2 = UUID.v4()
        XCTAssertNotEqual(u1, u2)
    }

    func testV3() {
        let u = UUID.v3(namespace: UUID.namespaceDNS, name: "python.org")
        XCTAssertEqual(u.description, "6fa459ea-ee8a-3ca4-894e-db77e160355e")
        XCTAssertEqual(u.version, 3)
    }

    func testV5() {
        let u = UUID.v5(namespace: UUID.namespaceDNS, name: "python.org")
        XCTAssertEqual(u.description, "886313e1-3b8a-5372-9b90-0c9aee199e5d")
        XCTAssertEqual(u.version, 5)
    }

    func testV7() {
        let u1 = UUID.v7()
        let u2 = UUID.v7()
        XCTAssertEqual(u1.version, 7)
        XCTAssertEqual(u2.version, 7)
        XCTAssertTrue(UUID.isValid(u1.description))
    }

    func testV1() {
        let u = UUID.v1()
        XCTAssertEqual(u.version, 1)
        XCTAssertEqual(u.variant, "rfc4122")
    }

    func testParse() throws {
        let valid = "550e8400-e29b-41d4-a716-446655440000"
        let u = try UUID.parse(valid)
        XCTAssertEqual(u.description, valid)
        XCTAssertTrue(UUID.isValid(valid))
        
        let upper = "550E8400-E29B-41D4-A716-446655440000"
        let u2 = try UUID.parse(upper)
        XCTAssertEqual(u, u2)
    }
}
