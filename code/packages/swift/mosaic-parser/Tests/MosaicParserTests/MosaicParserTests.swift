import XCTest
@testable import MosaicParser

// ============================================================================
// MosaicParserTests
// ============================================================================
//
// Tests verify:
//   1. Simple component with no slots
//   2. Component with primitive slot types
//   3. Component with list slot type
//   4. Slot with component-type (imported)
//   5. Property assignments (string, number, dimension, color, keyword, slot ref)
//   6. Slot reference as child
//   7. Nested child nodes
//   8. when block
//   9. each block
//  10. enum_value property (Name.member)
//  11. KEYWORD as property name
//  12. Error: missing component keyword
//  13. Full ProfileCard round-trip

final class MosaicParserTests: XCTestCase {

    // -------------------------------------------------------------------------
    // 1. Minimal component — no slots
    // -------------------------------------------------------------------------

    func testMinimalComponent() throws {
        let ast = try parse("component Empty { Box { } }")
        guard case let .component(name, slots, body) = ast else {
            return XCTFail("Expected .component")
        }
        XCTAssertEqual(name, "Empty")
        XCTAssertEqual(slots.count, 0)
        guard case let .node(type, _, _) = body else {
            return XCTFail("Expected .node body")
        }
        XCTAssertEqual(type, "Box")
    }

    // -------------------------------------------------------------------------
    // 2. Primitive slot types
    // -------------------------------------------------------------------------

    func testPrimitiveSlotTypes() throws {
        let src = """
        component Demo {
          slot title: text;
          slot count: number;
          slot visible: bool;
          slot photo: image;
          slot bg: color;
          Box { }
        }
        """
        let ast = try parse(src)
        guard case let .component(_, slots, _) = ast else { return XCTFail() }
        XCTAssertEqual(slots.count, 5)
        let names = slots.compactMap { node -> String? in
            if case let .slot(n, _) = node { return n }
            return nil
        }
        XCTAssertEqual(names, ["title", "count", "visible", "photo", "bg"])
    }

    // -------------------------------------------------------------------------
    // 3. list<text> slot type
    // -------------------------------------------------------------------------

    func testListSlotType() throws {
        let ast = try parse("component ListDemo { slot items: list<text>; Box { } }")
        guard case let .component(_, slots, _) = ast else { return XCTFail() }
        XCTAssertEqual(slots.count, 1)
        guard case let .slot(name, type) = slots[0] else { return XCTFail() }
        XCTAssertEqual(name, "items")
        guard case let .listType(inner) = type else { return XCTFail("Expected .listType") }
        guard case let .slotType(innerName) = inner else { return XCTFail() }
        XCTAssertEqual(innerName, "text")
    }

    // -------------------------------------------------------------------------
    // 4. Component-type slot
    // -------------------------------------------------------------------------

    func testComponentTypeSlot() throws {
        let ast = try parse("component Card { slot action: Button; Box { } }")
        guard case let .component(_, slots, _) = ast else { return XCTFail() }
        guard case let .slot(_, type) = slots[0] else { return XCTFail() }
        guard case let .slotType(n) = type else { return XCTFail() }
        XCTAssertEqual(n, "Button")
    }

    // -------------------------------------------------------------------------
    // 5. Property assignments — all value kinds
    // -------------------------------------------------------------------------

    func testPropertyStringValue() throws {
        let ast = try parse("""
        component A {
          Text { content: "hello"; }
        }
        """)
        guard case let .component(_, _, body) = ast,
              case let .node(_, props, _) = body,
              case let .property(name, value) = props[0],
              case let .literal(s) = value else { return XCTFail() }
        XCTAssertEqual(name, "content")
        XCTAssertEqual(s, "\"hello\"")
    }

    func testPropertyDimensionValue() throws {
        let ast = try parse("component A { Box { padding: 16dp; } }")
        guard case let .component(_, _, body) = ast,
              case let .node(_, props, _) = body,
              case let .property(_, value) = props[0],
              case let .number(v, unit) = value else { return XCTFail() }
        XCTAssertEqual(v, 16)
        XCTAssertEqual(unit, "dp")
    }

    func testPropertyHexColor() throws {
        let ast = try parse("component A { Box { background: #2563eb; } }")
        guard case let .component(_, _, body) = ast,
              case let .node(_, props, _) = body,
              case let .property(_, value) = props[0],
              case let .color(r, g, b, a) = value else { return XCTFail() }
        XCTAssertEqual(r, 0x25)
        XCTAssertEqual(g, 0x63)
        XCTAssertEqual(b, 0xeb)
        XCTAssertEqual(a, 255)
    }

    func testPropertySlotRef() throws {
        let ast = try parse("component A { slot t: text; Text { content: @t; } }")
        guard case let .component(_, _, body) = ast,
              case let .node(_, props, _) = body,
              case let .property(_, value) = props[0],
              case let .slotRef(name) = value else { return XCTFail() }
        XCTAssertEqual(name, "t")
    }

    // -------------------------------------------------------------------------
    // 6. Slot reference as child
    // -------------------------------------------------------------------------

    func testSlotRefChild() throws {
        let ast = try parse("component A { slot act: node; Column { @act; } }")
        guard case let .component(_, _, body) = ast,
              case let .node(_, _, children) = body,
              case let .slotRef(name) = children[0] else { return XCTFail() }
        XCTAssertEqual(name, "act")
    }

    // -------------------------------------------------------------------------
    // 7. Nested child nodes
    // -------------------------------------------------------------------------

    func testNestedNodes() throws {
        let src = "component A { Column { Row { Text { content: \"hi\"; } } } }"
        let ast = try parse(src)
        guard case let .component(_, _, outer) = ast,
              case let .node("Column", _, outerChildren) = outer,
              case let .node("Row", _, rowChildren) = outerChildren[0],
              case .node("Text", _, _) = rowChildren[0] else {
            return XCTFail("Nesting not parsed correctly")
        }
    }

    // -------------------------------------------------------------------------
    // 8. when block
    // -------------------------------------------------------------------------

    func testWhenBlock() throws {
        let src = """
        component A {
          slot show: bool;
          Column {
            when @show {
              Text { content: "visible"; }
            }
          }
        }
        """
        let ast = try parse(src)
        guard case let .component(_, _, body) = ast,
              case let .node(_, _, children) = body,
              case let .whenBlock(slot, whenChildren) = children[0] else {
            return XCTFail("Expected .whenBlock")
        }
        XCTAssertEqual(slot, "show")
        XCTAssertEqual(whenChildren.count, 1)
    }

    // -------------------------------------------------------------------------
    // 9. each block
    // -------------------------------------------------------------------------

    func testEachBlock() throws {
        let src = """
        component A {
          slot items: list<text>;
          Column {
            each @items as item {
              Text { content: @item; }
            }
          }
        }
        """
        let ast = try parse(src)
        guard case let .component(_, _, body) = ast,
              case let .node(_, _, children) = body,
              case let .eachBlock(slot, item, eachChildren) = children[0] else {
            return XCTFail("Expected .eachBlock")
        }
        XCTAssertEqual(slot, "items")
        XCTAssertEqual(item, "item")
        XCTAssertEqual(eachChildren.count, 1)
    }

    // -------------------------------------------------------------------------
    // 10. enum_value property
    // -------------------------------------------------------------------------

    func testEnumValueProperty() throws {
        let ast = try parse("component A { Text { style: heading.large; } }")
        guard case let .component(_, _, body) = ast,
              case let .node(_, props, _) = body,
              case let .property(name, value) = props[0],
              case let .literal(s) = value else { return XCTFail() }
        XCTAssertEqual(name, "style")
        XCTAssertEqual(s, "heading.large")
    }

    // -------------------------------------------------------------------------
    // 11. KEYWORD as property name (e.g. "color" used as prop name)
    // -------------------------------------------------------------------------

    func testKeywordAsPropertyName() throws {
        let ast = try parse("component A { Box { color: #ff0000; } }")
        guard case let .component(_, _, body) = ast,
              case let .node(_, props, _) = body,
              case let .property(name, _) = props[0] else { return XCTFail() }
        XCTAssertEqual(name, "color")
    }

    // -------------------------------------------------------------------------
    // 12. Error: missing component keyword
    // -------------------------------------------------------------------------

    func testMissingComponentKeyword() {
        XCTAssertThrowsError(try parse("Label { Text { } }")) { err in
            XCTAssertTrue(err is ParseError)
        }
    }

    // -------------------------------------------------------------------------
    // 13. Full ProfileCard round-trip
    // -------------------------------------------------------------------------

    func testProfileCardRoundTrip() throws {
        let src = """
        component ProfileCard {
          slot avatar-url: image;
          slot display-name: text;
          slot bio: text;
          Column {
            Image { source: @avatar-url; corner-radius: 50%; }
            Text { content: @display-name; }
            Text { content: @bio; }
          }
        }
        """
        let ast = try parse(src)
        guard case let .component(name, slots, body) = ast else { return XCTFail() }
        XCTAssertEqual(name, "ProfileCard")
        XCTAssertEqual(slots.count, 3)
        guard case let .node(rootType, _, children) = body else { return XCTFail() }
        XCTAssertEqual(rootType, "Column")
        XCTAssertEqual(children.count, 3)
    }
}
