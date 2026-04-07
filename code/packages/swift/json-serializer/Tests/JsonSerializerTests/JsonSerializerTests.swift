// JsonSerializerTests.swift
// ============================================================================
// Unit tests for the JsonSerializer codec.
// ============================================================================
//
// We test:
//   - Compact serialization of all six JSON value types
//   - Pretty serialization (indented output)
//   - String escape sequences
//   - Number formatting (integer vs float)
//   - Roundtrip: serialize → deserialize → re-serialize (must produce equal output)
//   - Deserialization (via JsonParser wrapper)
// ============================================================================
import Testing
@testable import JsonSerializer
import JsonValue

// ============================================================================
// Compact serialization
// ============================================================================
@Suite("Compact serialization")
struct CompactTests {

    let s = JsonSerializer(mode: .compact)

    @Test("Null")
    func testNull() {
        #expect(s.serialize(.null) == "null")
    }

    @Test("True")
    func testTrue() {
        #expect(s.serialize(.bool(true)) == "true")
    }

    @Test("False")
    func testFalse() {
        #expect(s.serialize(.bool(false)) == "false")
    }

    @Test("Whole number as integer string")
    func testWholeNumber() {
        // 42.0 should serialize as "42", not "42.0"
        #expect(s.serialize(.number(42)) == "42")
        #expect(s.serialize(.number(0)) == "0")
        #expect(s.serialize(.number(-100)) == "-100")
    }

    @Test("Fractional number")
    func testFractionalNumber() {
        #expect(s.serialize(.number(3.14)) == "3.14")
    }

    @Test("Large integer stays as integer")
    func testLargeInteger() {
        #expect(s.serialize(.number(1_000_000)) == "1000000")
    }

    @Test("Simple string")
    func testSimpleString() {
        #expect(s.serialize(.string("hello")) == "\"hello\"")
    }

    @Test("Empty string")
    func testEmptyString() {
        #expect(s.serialize(.string("")) == "\"\"")
    }

    @Test("Empty array")
    func testEmptyArray() {
        #expect(s.serialize(.array([])) == "[]")
    }

    @Test("Array of numbers")
    func testNumberArray() {
        #expect(s.serialize(.array([.number(1), .number(2), .number(3)])) == "[1,2,3]")
    }

    @Test("Mixed array")
    func testMixedArray() {
        #expect(s.serialize(.array([.null, .bool(true), .number(1), .string("x")])) == "[null,true,1,\"x\"]")
    }

    @Test("Empty object")
    func testEmptyObject() {
        #expect(s.serialize(.object([])) == "{}")
    }

    @Test("Object with one pair")
    func testSinglePair() {
        #expect(s.serialize(.object([(key: "a", value: .number(1))])) == "{\"a\":1}")
    }

    @Test("Object with multiple pairs")
    func testMultiplePairs() {
        let v = JsonValue.object([
            (key: "name", value: .string("Alice")),
            (key: "age", value: .number(30)),
        ])
        #expect(s.serialize(v) == "{\"name\":\"Alice\",\"age\":30}")
    }

    @Test("Nested structure")
    func testNested() {
        let v = JsonValue.object([
            (key: "x", value: .array([.number(1), .number(2)]))
        ])
        #expect(s.serialize(v) == "{\"x\":[1,2]}")
    }

    @Test("Key order is preserved")
    func testKeyOrder() {
        let v = JsonValue.object([
            (key: "z", value: .number(1)),
            (key: "a", value: .number(2)),
        ])
        #expect(s.serialize(v) == "{\"z\":1,\"a\":2}")
    }
}

// ============================================================================
// String escape sequences
// ============================================================================
@Suite("String escape sequences")
struct EscapeTests {

    let s = JsonSerializer(mode: .compact)

    @Test("Escape double quote")
    func testEscapeQuote() {
        #expect(s.serialize(.string("say \"hi\"")) == "\"say \\\"hi\\\"\"")
    }

    @Test("Escape backslash")
    func testEscapeBackslash() {
        #expect(s.serialize(.string("a\\b")) == "\"a\\\\b\"")
    }

    @Test("Escape newline")
    func testEscapeNewline() {
        #expect(s.serialize(.string("line1\nline2")) == "\"line1\\nline2\"")
    }

    @Test("Escape carriage return")
    func testEscapeCarriageReturn() {
        #expect(s.serialize(.string("\r")) == "\"\\r\"")
    }

    @Test("Escape tab")
    func testEscapeTab() {
        #expect(s.serialize(.string("col1\tcol2")) == "\"col1\\tcol2\"")
    }

    @Test("Escape backspace")
    func testEscapeBackspace() {
        #expect(s.serialize(.string("\u{08}")) == "\"\\b\"")
    }

    @Test("Escape form feed")
    func testEscapeFormFeed() {
        #expect(s.serialize(.string("\u{0C}")) == "\"\\f\"")
    }

    @Test("Escape other control characters with \\uXXXX")
    func testEscapeControlChar() {
        // U+0001 (SOH) has no named escape, so it becomes \u0001
        #expect(s.serialize(.string("\u{01}")) == "\"\\u0001\"")
    }

    @Test("Unicode characters pass through unescaped")
    func testUnicodePassThrough() {
        // Non-ASCII Unicode characters don't need escaping in JSON
        #expect(s.serialize(.string("αβγ")) == "\"αβγ\"")
        #expect(s.serialize(.string("😀")) == "\"😀\"")
    }
}

// ============================================================================
// Pretty serialization
// ============================================================================
@Suite("Pretty serialization")
struct PrettyTests {

    let s = JsonSerializer(mode: .pretty)

    @Test("Primitives are unchanged")
    func testPrimitivesUnchanged() {
        #expect(s.serialize(.null) == "null")
        #expect(s.serialize(.bool(true)) == "true")
        #expect(s.serialize(.number(42)) == "42")
        #expect(s.serialize(.string("hi")) == "\"hi\"")
    }

    @Test("Empty array is compact")
    func testEmptyArrayCompact() {
        #expect(s.serialize(.array([])) == "[]")
    }

    @Test("Empty object is compact")
    func testEmptyObjectCompact() {
        #expect(s.serialize(.object([])) == "{}")
    }

    @Test("Array is indented")
    func testArrayIndented() {
        let v = JsonValue.array([.number(1), .number(2)])
        let expected = "[\n  1,\n  2\n]"
        #expect(s.serialize(v) == expected)
    }

    @Test("Object is indented with colon space")
    func testObjectIndented() {
        let v = JsonValue.object([(key: "a", value: .number(1))])
        let expected = "{\n  \"a\": 1\n}"
        #expect(s.serialize(v) == expected)
    }

    @Test("Nested arrays are double-indented")
    func testNestedArrayIndented() {
        let v = JsonValue.array([.array([.number(1), .number(2)])])
        let expected = "[\n  [\n    1,\n    2\n  ]\n]"
        #expect(s.serialize(v) == expected)
    }

    @Test("Nested object")
    func testNestedObjectIndented() {
        let v = JsonValue.object([
            (key: "outer", value: .object([
                (key: "inner", value: .number(1))
            ]))
        ])
        let expected = "{\n  \"outer\": {\n    \"inner\": 1\n  }\n}"
        #expect(s.serialize(v) == expected)
    }
}

// ============================================================================
// Roundtrip tests
// ============================================================================
@Suite("Roundtrip")
struct RoundtripTests {

    let s = JsonSerializer(mode: .compact)

    /// Serialize a value, deserialize the result, re-serialize — the two
    /// serialized strings must be identical.
    private func roundtrip(_ v: JsonValue) throws -> String {
        let first = s.serialize(v)
        let parsed = try s.deserialize(first)
        return s.serialize(parsed)
    }

    @Test("Roundtrip null")
    func testNull() throws {
        let v = JsonValue.null
        #expect(try roundtrip(v) == s.serialize(v))
    }

    @Test("Roundtrip bool")
    func testBool() throws {
        #expect(try roundtrip(.bool(true)) == "true")
        #expect(try roundtrip(.bool(false)) == "false")
    }

    @Test("Roundtrip number")
    func testNumber() throws {
        #expect(try roundtrip(.number(42)) == "42")
        #expect(try roundtrip(.number(3.14)) == "3.14")
    }

    @Test("Roundtrip string with escapes")
    func testStringEscapes() throws {
        let v = JsonValue.string("line1\nline2\t\"tab\"")
        #expect(try roundtrip(v) == s.serialize(v))
    }

    @Test("Roundtrip array")
    func testArray() throws {
        let v = JsonValue.array([.number(1), .string("x"), .null])
        #expect(try roundtrip(v) == s.serialize(v))
    }

    @Test("Roundtrip object")
    func testObject() throws {
        let v = JsonValue.object([
            (key: "name", value: .string("Alice")),
            (key: "scores", value: .array([.number(10), .number(20)])),
        ])
        #expect(try roundtrip(v) == s.serialize(v))
    }

    @Test("Roundtrip complex nested structure")
    func testComplex() throws {
        let v = JsonValue.object([
            (key: "users", value: .array([
                .object([
                    (key: "id",     value: .number(1)),
                    (key: "name",   value: .string("Alice")),
                    (key: "active", value: .bool(true)),
                ]),
                .object([
                    (key: "id",     value: .number(2)),
                    (key: "name",   value: .string("Bob")),
                    (key: "active", value: .bool(false)),
                ]),
            ])),
            (key: "total", value: .number(2)),
        ])
        #expect(try roundtrip(v) == s.serialize(v))
    }
}

// ============================================================================
// Deserialization (JsonParser wrapper)
// ============================================================================
@Suite("Deserialization")
struct DeserializationTests {

    let s = JsonSerializer()

    @Test("Deserialize null")
    func testNull() throws {
        #expect(try s.deserialize("null") == .null)
    }

    @Test("Deserialize object")
    func testObject() throws {
        let v = try s.deserialize("{\"x\":1}")
        #expect(v == .object([(key: "x", value: .number(1))]))
    }

    @Test("Deserialize malformed input throws")
    func testMalformed() throws {
        #expect(throws: (any Error).self) {
            try s.deserialize("not-json")
        }
    }
}
