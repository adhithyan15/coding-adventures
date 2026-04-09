// JsonValueTests.swift
// ============================================================================
// Unit tests for the JsonValue enum.
// ============================================================================
//
// We test every case, every accessor, every subscript, and the Equatable
// conformance. The CustomStringConvertible output is also verified.
//
// Why test so thoroughly?
// A "simple enum" can still have subtle bugs: Did we forget to handle the
// .null case in Equatable? Does the subscript respect negative indices? Does
// the `description` of a whole number format correctly? These tests catch
// such issues immediately.
// ============================================================================
import Testing
@testable import JsonValue

// ============================================================================
// Basic construction and type tests
// ============================================================================
@Suite("JsonValue Construction")
struct ConstructionTests {

    @Test("Null case")
    func testNull() {
        let v = JsonValue.null
        #expect(v.isNull == true)
        #expect(v.stringValue == nil)
        #expect(v.doubleValue == nil)
        #expect(v.boolValue == nil)
        #expect(v.arrayValue == nil)
        #expect(v.objectValue == nil)
    }

    @Test("Bool case — true")
    func testBoolTrue() {
        let v = JsonValue.bool(true)
        #expect(v.boolValue == true)
        #expect(v.isNull == false)
        #expect(v.stringValue == nil)
    }

    @Test("Bool case — false")
    func testBoolFalse() {
        let v = JsonValue.bool(false)
        #expect(v.boolValue == false)
    }

    @Test("Bool factory method")
    func testBoolFactory() {
        let t = JsonValue.from(true)
        let f = JsonValue.from(false)
        #expect(t.boolValue == true)
        #expect(f.boolValue == false)
    }

    @Test("Number case — integer")
    func testNumberInteger() {
        let v = JsonValue.number(42)
        #expect(v.doubleValue == 42.0)
        #expect(v.isNull == false)
    }

    @Test("Number case — floating point")
    func testNumberFloat() {
        let v = JsonValue.number(3.14)
        #expect(v.doubleValue == 3.14)
    }

    @Test("Number case — negative")
    func testNumberNegative() {
        let v = JsonValue.number(-1.5)
        #expect(v.doubleValue == -1.5)
    }

    @Test("String case")
    func testString() {
        let v = JsonValue.string("hello")
        #expect(v.stringValue == "hello")
        #expect(v.isNull == false)
        #expect(v.doubleValue == nil)
    }

    @Test("Empty string")
    func testEmptyString() {
        let v = JsonValue.string("")
        #expect(v.stringValue == "")
    }

    @Test("Array case — empty")
    func testEmptyArray() {
        let v = JsonValue.array([])
        #expect(v.arrayValue?.count == 0)
    }

    @Test("Array case — mixed types")
    func testMixedArray() {
        let v = JsonValue.array([.null, .bool(true), .number(1), .string("x")])
        let arr = v.arrayValue
        #expect(arr?.count == 4)
        #expect(arr?[0].isNull == true)
        #expect(arr?[1].boolValue == true)
        #expect(arr?[2].doubleValue == 1.0)
        #expect(arr?[3].stringValue == "x")
    }

    @Test("Object case — empty")
    func testEmptyObject() {
        let v = JsonValue.object([])
        #expect(v.objectValue?.count == 0)
    }

    @Test("Object case — with pairs")
    func testObjectWithPairs() {
        let v = JsonValue.object([
            (key: "name", value: .string("Alice")),
            (key: "age", value: .number(30)),
        ])
        let pairs = v.objectValue
        #expect(pairs?.count == 2)
        #expect(pairs?[0].key == "name")
        #expect(pairs?[0].value.stringValue == "Alice")
        #expect(pairs?[1].key == "age")
        #expect(pairs?[1].value.doubleValue == 30.0)
    }
}

// ============================================================================
// Equatable conformance
// ============================================================================
@Suite("JsonValue Equatable")
struct EquatableTests {

    @Test("Null equals null")
    func testNullEquality() {
        #expect(JsonValue.null == JsonValue.null)
    }

    @Test("Bool equality")
    func testBoolEquality() {
        #expect(JsonValue.bool(true) == JsonValue.bool(true))
        #expect(JsonValue.bool(false) == JsonValue.bool(false))
        #expect(JsonValue.bool(true) != JsonValue.bool(false))
    }

    @Test("Number equality")
    func testNumberEquality() {
        #expect(JsonValue.number(42) == JsonValue.number(42))
        #expect(JsonValue.number(1.5) != JsonValue.number(1.6))
    }

    @Test("String equality")
    func testStringEquality() {
        #expect(JsonValue.string("foo") == JsonValue.string("foo"))
        #expect(JsonValue.string("foo") != JsonValue.string("bar"))
    }

    @Test("Array equality")
    func testArrayEquality() {
        let a = JsonValue.array([.null, .bool(true)])
        let b = JsonValue.array([.null, .bool(true)])
        let c = JsonValue.array([.null, .bool(false)])
        #expect(a == b)
        #expect(a != c)
    }

    @Test("Object equality — same order")
    func testObjectEquality() {
        let a = JsonValue.object([(key: "x", value: .number(1))])
        let b = JsonValue.object([(key: "x", value: .number(1))])
        let c = JsonValue.object([(key: "x", value: .number(2))])
        #expect(a == b)
        #expect(a != c)
    }

    @Test("Object equality — different key order means not equal")
    func testObjectKeyOrderMatters() {
        // We preserve insertion order, so {a:1, b:2} != {b:2, a:1}
        let a = JsonValue.object([(key: "a", value: .number(1)), (key: "b", value: .number(2))])
        let b = JsonValue.object([(key: "b", value: .number(2)), (key: "a", value: .number(1))])
        #expect(a != b)
    }

    @Test("Object equality — different count")
    func testObjectDifferentCount() {
        let a = JsonValue.object([(key: "x", value: .number(1))])
        let b = JsonValue.object([(key: "x", value: .number(1)), (key: "y", value: .number(2))])
        #expect(a != b)
    }

    @Test("Cross-case inequality")
    func testCrossCaseInequality() {
        #expect(JsonValue.null != JsonValue.bool(false))
        #expect(JsonValue.number(0) != JsonValue.null)
        #expect(JsonValue.string("true") != JsonValue.bool(true))
    }
}

// ============================================================================
// Subscript access
// ============================================================================
@Suite("JsonValue Subscript")
struct SubscriptTests {

    @Test("String key lookup in object")
    func testObjectSubscript() {
        let v = JsonValue.object([
            (key: "hello", value: .string("world")),
            (key: "count", value: .number(3)),
        ])
        #expect(v["hello"]?.stringValue == "world")
        #expect(v["count"]?.doubleValue == 3.0)
    }

    @Test("Missing key returns nil")
    func testMissingKey() {
        let v = JsonValue.object([(key: "a", value: .null)])
        #expect(v["missing"] == nil)
    }

    @Test("Key subscript on non-object returns nil")
    func testKeySubscriptOnNonObject() {
        #expect(JsonValue.null["key"] == nil)
        #expect(JsonValue.array([])["key"] == nil)
        #expect(JsonValue.string("x")["key"] == nil)
    }

    @Test("Integer index in array")
    func testArraySubscript() {
        let v = JsonValue.array([.null, .bool(true), .number(99)])
        #expect(v[0]?.isNull == true)
        #expect(v[1]?.boolValue == true)
        #expect(v[2]?.doubleValue == 99)
    }

    @Test("Out-of-bounds index returns nil")
    func testOutOfBoundsIndex() {
        let v = JsonValue.array([.null])
        #expect(v[1] == nil)
        #expect(v[-1] == nil)
    }

    @Test("Index subscript on non-array returns nil")
    func testIndexSubscriptOnNonArray() {
        #expect(JsonValue.null[0] == nil)
        #expect(JsonValue.object([])[0] == nil)
    }

    @Test("Chained subscript access")
    func testChainedAccess() {
        // Simulate: {"users": [{"name": "Alice"}]}
        let v = JsonValue.object([
            (key: "users", value: .array([
                .object([(key: "name", value: .string("Alice"))])
            ]))
        ])
        let name = v["users"]?[0]?["name"]?.stringValue
        #expect(name == "Alice")
    }
}

// ============================================================================
// CustomStringConvertible (description)
// ============================================================================
@Suite("JsonValue Description")
struct DescriptionTests {

    @Test("Null description")
    func testNullDescription() {
        #expect(JsonValue.null.description == "null")
    }

    @Test("Bool descriptions")
    func testBoolDescriptions() {
        #expect(JsonValue.bool(true).description == "true")
        #expect(JsonValue.bool(false).description == "false")
    }

    @Test("Whole number description")
    func testWholeNumberDescription() {
        // 42.0 should render as "42", not "42.0"
        #expect(JsonValue.number(42).description == "42")
        #expect(JsonValue.number(0).description == "0")
        #expect(JsonValue.number(-100).description == "-100")
    }

    @Test("Fractional number description")
    func testFractionalNumberDescription() {
        // 3.14 stays as "3.14"
        #expect(JsonValue.number(3.14).description == "3.14")
    }

    @Test("String description")
    func testStringDescription() {
        #expect(JsonValue.string("hi").description == "\"hi\"")
    }

    @Test("Empty array description")
    func testEmptyArrayDescription() {
        #expect(JsonValue.array([]).description == "[]")
    }

    @Test("Array description")
    func testArrayDescription() {
        let v = JsonValue.array([.null, .bool(true)])
        #expect(v.description == "[null, true]")
    }

    @Test("Empty object description")
    func testEmptyObjectDescription() {
        #expect(JsonValue.object([]).description == "{}")
    }

    @Test("Object description")
    func testObjectDescription() {
        let v = JsonValue.object([(key: "x", value: .number(1))])
        #expect(v.description == "{\"x\": 1}")
    }
}
