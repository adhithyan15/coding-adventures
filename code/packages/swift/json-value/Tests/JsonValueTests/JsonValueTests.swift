import XCTest
@testable import JsonValue

final class JsonValueTests: XCTestCase {

    func testParseObject() throws {
        let json = "{\"name\": \"Alice\", \"age\": 30}"
        let val = try parse(json)
        
        switch val {
        case .object(let dict):
            XCTAssertEqual(dict.count, 2)
            XCTAssertEqual(dict["name"], .string("Alice"))
            XCTAssertEqual(dict["age"], .number(30.0))
        default:
            XCTFail("Expected .object")
        }

        // Test custom subscripts
        XCTAssertEqual(val["name"], .string("Alice"))
        XCTAssertNil(val["missing"])
        XCTAssertNil(val[0])
    }

    func testParseArray() throws {
        let json = "[1, 2, true, null]"
        let val = try parse(json)
        
        switch val {
        case .array(let arr):
            XCTAssertEqual(arr.count, 4)
            XCTAssertEqual(arr[0], .number(1.0))
            XCTAssertEqual(arr[2], .bool(true))
            XCTAssertEqual(arr[3], .null)
        default:
            XCTFail("Expected .array")
        }

        // Test custom subscripts
        XCTAssertEqual(val[0], .number(1.0))
        XCTAssertNil(val[10])
        XCTAssertNil(val["key"])
    }

    func testToNative() throws {
        let json = "{\"list\": [1, null, false]}"
        let val = try parse(json)
        let native = toNative(val) as? [String: Any]
        
        XCTAssertNotNil(native)
        let listStr = native?["list"] as? [Any]
        XCTAssertNotNil(listStr)
        XCTAssertEqual(listStr?[0] as? Double, 1.0)
        XCTAssertEqual(listStr?[1] as? NSNull, NSNull())
        XCTAssertEqual(listStr?[2] as? Bool, false)
    }
}
