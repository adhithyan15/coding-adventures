// JsonParserTests.swift
// ============================================================================
// Unit tests for the JsonParser recursive-descent parser.
// ============================================================================
//
// We test:
//   - All six JSON value types at the top level
//   - Empty and nested arrays
//   - Empty and nested objects
//   - Complex mixed structures
//   - Error cases: trailing commas, missing separators, bad key types, etc.
// ============================================================================
import Testing
@testable import JsonParser
import JsonValue
import JsonLexer

// ============================================================================
// Primitive values
// ============================================================================
@Suite("Primitive values")
struct PrimitiveTests {

    let parser = JsonParser()

    @Test("Parse null")
    func testNull() throws {
        let v = try parser.parse("null")
        #expect(v == .null)
    }

    @Test("Parse true")
    func testTrue() throws {
        let v = try parser.parse("true")
        #expect(v == .bool(true))
    }

    @Test("Parse false")
    func testFalse() throws {
        let v = try parser.parse("false")
        #expect(v == .bool(false))
    }

    @Test("Parse integer number")
    func testInteger() throws {
        let v = try parser.parse("42")
        #expect(v == .number(42))
    }

    @Test("Parse negative number")
    func testNegativeNumber() throws {
        let v = try parser.parse("-7")
        #expect(v == .number(-7))
    }

    @Test("Parse floating-point number")
    func testFloat() throws {
        let v = try parser.parse("3.14")
        if case .number(let n) = v {
            #expect(abs(n - 3.14) < 1e-10)
        } else {
            Issue.record("Expected .number")
        }
    }

    @Test("Parse string")
    func testString() throws {
        let v = try parser.parse("\"hello world\"")
        #expect(v == .string("hello world"))
    }

    @Test("Parse empty string")
    func testEmptyString() throws {
        let v = try parser.parse("\"\"")
        #expect(v == .string(""))
    }
}

// ============================================================================
// Arrays
// ============================================================================
@Suite("Arrays")
struct ArrayTests {

    let parser = JsonParser()

    @Test("Parse empty array")
    func testEmptyArray() throws {
        let v = try parser.parse("[]")
        #expect(v == .array([]))
    }

    @Test("Parse array of nulls")
    func testNullArray() throws {
        let v = try parser.parse("[null, null, null]")
        #expect(v == .array([.null, .null, .null]))
    }

    @Test("Parse array of booleans")
    func testBoolArray() throws {
        let v = try parser.parse("[true, false, true]")
        #expect(v == .array([.bool(true), .bool(false), .bool(true)]))
    }

    @Test("Parse array of numbers")
    func testNumberArray() throws {
        let v = try parser.parse("[1, 2, 3]")
        #expect(v == .array([.number(1), .number(2), .number(3)]))
    }

    @Test("Parse array of strings")
    func testStringArray() throws {
        let v = try parser.parse("[\"a\", \"b\", \"c\"]")
        #expect(v == .array([.string("a"), .string("b"), .string("c")]))
    }

    @Test("Parse mixed-type array")
    func testMixedArray() throws {
        let v = try parser.parse("[null, true, 42, \"hello\"]")
        #expect(v == .array([.null, .bool(true), .number(42), .string("hello")]))
    }

    @Test("Parse nested arrays")
    func testNestedArrays() throws {
        let v = try parser.parse("[[1, 2], [3, 4]]")
        #expect(v == .array([
            .array([.number(1), .number(2)]),
            .array([.number(3), .number(4)]),
        ]))
    }

    @Test("Parse single-element array")
    func testSingleElement() throws {
        let v = try parser.parse("[42]")
        #expect(v == .array([.number(42)]))
    }
}

// ============================================================================
// Objects
// ============================================================================
@Suite("Objects")
struct ObjectTests {

    let parser = JsonParser()

    @Test("Parse empty object")
    func testEmptyObject() throws {
        let v = try parser.parse("{}")
        #expect(v == .object([]))
    }

    @Test("Parse single key-value pair")
    func testSinglePair() throws {
        let v = try parser.parse("{\"key\": \"value\"}")
        #expect(v == .object([(key: "key", value: .string("value"))]))
    }

    @Test("Parse multiple pairs")
    func testMultiplePairs() throws {
        let v = try parser.parse("{\"a\": 1, \"b\": 2}")
        #expect(v == .object([
            (key: "a", value: .number(1)),
            (key: "b", value: .number(2)),
        ]))
    }

    @Test("Parse object with mixed value types")
    func testMixedObject() throws {
        let v = try parser.parse("{\"name\": \"Alice\", \"age\": 30, \"active\": true}")
        #expect(v == .object([
            (key: "name",   value: .string("Alice")),
            (key: "age",    value: .number(30)),
            (key: "active", value: .bool(true)),
        ]))
    }

    @Test("Parse nested objects")
    func testNestedObjects() throws {
        let v = try parser.parse("{\"outer\": {\"inner\": 1}}")
        #expect(v == .object([
            (key: "outer", value: .object([
                (key: "inner", value: .number(1))
            ]))
        ]))
    }

    @Test("Insertion order is preserved")
    func testInsertionOrder() throws {
        // Keys should appear in the order they were written
        let v = try parser.parse("{\"z\": 1, \"a\": 2, \"m\": 3}")
        guard case .object(let pairs) = v else {
            Issue.record("Expected object"); return
        }
        #expect(pairs[0].key == "z")
        #expect(pairs[1].key == "a")
        #expect(pairs[2].key == "m")
    }
}

// ============================================================================
// Complex structures
// ============================================================================
@Suite("Complex structures")
struct ComplexTests {

    let parser = JsonParser()

    @Test("Array of objects")
    func testArrayOfObjects() throws {
        let v = try parser.parse("[{\"x\": 1}, {\"x\": 2}]")
        #expect(v == .array([
            .object([(key: "x", value: .number(1))]),
            .object([(key: "x", value: .number(2))]),
        ]))
    }

    @Test("Object with array value")
    func testObjectWithArray() throws {
        let v = try parser.parse("{\"nums\": [1, 2, 3]}")
        #expect(v == .object([
            (key: "nums", value: .array([.number(1), .number(2), .number(3)]))
        ]))
    }

    @Test("Deeply nested structure")
    func testDeeplyNested() throws {
        let v = try parser.parse("{\"a\": {\"b\": {\"c\": null}}}")
        guard case .object(let outer) = v,
              case .object(let mid) = outer[0].value,
              case .object(let inner) = mid[0].value else {
            Issue.record("Structure mismatch"); return
        }
        #expect(inner[0].key == "c")
        #expect(inner[0].value == .null)
    }

    @Test("Parse from pre-lexed tokens")
    func testParseTokens() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("42")
        let v = try parser.parseTokens(tokens)
        #expect(v == .number(42))
    }

    @Test("Whitespace is ignored")
    func testWhitespace() throws {
        let v = try parser.parse("  {  \"k\"  :  1  }  ")
        #expect(v == .object([(key: "k", value: .number(1))]))
    }
}

// ============================================================================
// Error cases
// ============================================================================
@Suite("Error cases")
struct ErrorTests {

    let parser = JsonParser()

    @Test("Empty input throws")
    func testEmptyInput() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("")
        }
    }

    @Test("Trailing comma in array throws")
    func testTrailingCommaArray() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("[1, 2,]")
        }
    }

    @Test("Trailing comma in object throws")
    func testTrailingCommaObject() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("{\"a\": 1,}")
        }
    }

    @Test("Missing colon in object throws")
    func testMissingColon() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("{\"key\" 42}")
        }
    }

    @Test("Number as object key throws")
    func testNumberKey() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("{42: \"value\"}")
        }
    }

    @Test("Boolean as object key throws")
    func testBoolKey() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("{true: \"value\"}")
        }
    }

    @Test("Missing value in pair throws")
    func testMissingValue() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("{\"key\":}")
        }
    }

    @Test("Unterminated array throws")
    func testUnterminatedArray() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("[1, 2")
        }
    }

    @Test("Unterminated object throws")
    func testUnterminatedObject() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("{\"a\": 1")
        }
    }

    @Test("Extra tokens after value throws")
    func testExtraTokens() throws {
        #expect(throws: (any Error).self) {
            try parser.parse("1 2")
        }
    }

    @Test("Bare comma throws")
    func testBareComma() throws {
        #expect(throws: (any Error).self) {
            try parser.parse(",")
        }
    }
}
