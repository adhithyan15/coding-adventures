import XCTest
import JsonValue
@testable import JsonSerializer

final class JsonSerializerTests: XCTestCase {

    func testCompactSerialization() throws {
        let val: JsonValue = .object([
            "name": .string("Alice"),
            "age": .number(30)
        ])
        
        let out = try serialize(val)
        // Order is non-deterministic, so just check components
        XCTAssertTrue(out.contains("\"name\":\"Alice\""))
        XCTAssertTrue(out.contains("\"age\":30"))
        XCTAssertTrue(out.hasPrefix("{") && out.hasSuffix("}"))
    }

    func testPrettySerialization() throws {
        let val: JsonValue = .object([
            "a": .number(1)
        ])
        
        var config = SerializerConfig()
        config.sortKeys = true
        
        let out = try serializePretty(val, config: config)
        XCTAssertEqual(out, "{\n  \"a\": 1\n}")
    }

    func testStringify() throws {
        let native: [String: Any] = ["list": [1, 2, 3]]
        let out = try stringify(native)
        XCTAssertEqual(out, "{\"list\":[1,2,3]}")
    }

    func testNumberFormatting() throws {
        XCTAssertEqual(try serialize(.number(42)), "42")
        // Since we parse Swift doubles, formatting simple floats might include .0 if it's not exactly an int.
        // Wait, Int(exactly: 42.0) is Int.
        XCTAssertEqual(try serialize(.number(3.14)), "3.14")
    }
}
