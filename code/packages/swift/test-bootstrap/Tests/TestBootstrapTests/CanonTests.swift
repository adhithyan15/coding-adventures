import XCTest
@testable import TestBootstrap

final class CanonTests: XCTestCase {
    func testScalarsAndStrings() {
        XCTAssertEqual(Canon.render(true), "true")
        XCTAssertEqual(Canon.render(-5), "-5")
        XCTAssertEqual(Canon.render(UInt8(7)), "7")
        XCTAssertEqual(Canon.render(UInt64.max), "18446744073709551615")
        XCTAssertEqual(Canon.render(1.5), "1.5")
        XCTAssertEqual(Canon.render(BigInt(-3)), "-3")
        XCTAssertEqual(Canon.render("a\"b\\c"), "\"a\\\"b\\\\c\"")
        XCTAssertEqual(Canon.render([UInt8]([0, 255, 16])), "0x00ff10")
        XCTAssertEqual(Canon.render(Optional<Int>.none as Any), "nil")
        XCTAssertEqual(Canon.render(Optional(4) as Any), "4")
    }

    func testControlCharactersAreEscaped() {
        XCTAssertEqual(Canon.render("a\nb\r\tc"), "\"a\\nb\\r\\tc\"")
        XCTAssertEqual(Canon.render("\u{1b}[31m\u{7f}\u{0}"), "\"\\u001b[31m\\u007f\\u0000\"")
    }

    func testCollectionsDictionariesTuplesAndStructs() {
        XCTAssertEqual(Canon.render([1, 2, 3]), "[1,2,3]")
        XCTAssertEqual(Canon.render([[1], [2, 3]]), "[[1],[2,3]]")
        XCTAssertEqual(Canon.render(["b": 2, "a": 1]), "{\"a\":1,\"b\":2}")
        XCTAssertEqual(Canon.render((1, "x")), "{1,\"x\"}")
        struct P { let x: Int; let y: [Int] }
        XCTAssertEqual(Canon.render(P(x: 1, y: [2])), "{1,[2]}")
        XCTAssertEqual(Canon.render(Set([7])), "[7]")
    }

    func testEnumsRenderWithoutUserDescription() {
        enum Color { case red, green }
        enum Shape { case circle(Int), square(String) }
        XCTAssertEqual(Canon.render(Color.green), "green")
        XCTAssertEqual(Canon.render(Shape.circle(2)), "circle(2)")
        XCTAssertEqual(Canon.render(Shape.square("s")), "square(\"s\")")
    }

    func testAHostileDescriptionIsNeverCalled() {
        struct Hostile: CustomStringConvertible {
            let v = 1
            var description: String { fatalError("description must never be called") }
        }
        struct HostileDebug: CustomDebugStringConvertible {
            var debugDescription: String { fatalError("debugDescription must never be called") }
        }
        enum HostileEnum: CustomStringConvertible {
            case a
            var description: String { fatalError("enum description must never be called") }
        }
        XCTAssertEqual(Canon.render(Hostile()), "{1}")
        XCTAssertEqual(Canon.render([Hostile(), Hostile()]), "[{1},{1}]")
        _ = Canon.render(HostileDebug())
        _ = Canon.render(HostileEnum.a)
        final class Node { var next: Node?; init() {} }
        let a = Node(), b = Node()
        a.next = b; b.next = a      // a cycle: the depth cap stops it
        XCTAssertTrue(Canon.render(a).contains("..."))
    }

    func testDepthCapAtSixteen() {
        var v: Any = 1
        for _ in 0..<1000 { v = [v] }
        let r = Canon.render(v)
        XCTAssertTrue(r.contains("..."))
        XCTAssertLessThan(r.count, 100)
    }

    func testNodeCapAndOutputCap() {
        let many = Array(repeating: 1, count: 50_000)
        let r = Canon.render(many)
        XCTAssertTrue(r.hasSuffix("..."))
        XCTAssertLessThan(r.utf8.count, 30_000)
        let huge = String(repeating: "x", count: 5 << 20)
        let rs = Canon.render(huge)
        XCTAssertLessThanOrEqual(rs.utf8.count, Canon.maxOutput + 8)
        XCTAssertTrue(rs.hasSuffix("..."))
        let bytes = Canon.render([UInt8](repeating: 0xab, count: 2 << 20))
        XCTAssertLessThanOrEqual(bytes.utf8.count, Canon.maxOutput + 8)
        let wide = (0..<9_000).map { _ in [1, 2, 3] }
        XCTAssertTrue(Canon.render(wide).hasSuffix("..."))
    }

    func testUnknownLeafRendersAsItsTypeName() {
        final class Opaque {}
        _ = Canon.render(Opaque())
        XCTAssertFalse(Canon.render(Opaque()).isEmpty)
        XCTAssertEqual(Canon.unrenderable, "<unrenderable>")
        XCTAssertEqual(clip("abcdef", 3), "abc...")
        XCTAssertEqual(clip("ab", 3), "ab")
    }
}
