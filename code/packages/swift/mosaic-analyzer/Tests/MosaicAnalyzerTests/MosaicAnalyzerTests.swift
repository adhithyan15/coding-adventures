import XCTest
@testable import MosaicAnalyzer

// ============================================================================
// MosaicAnalyzerTests
// ============================================================================
//
// Tests cover:
//   1. Component name extraction
//   2. Primitive slot types (text, number, bool, image, color, node)
//   3. List slot type
//   4. Component-type slot
//   5. Root node type and isPrimitive flag
//   6. Property with string value (quotes stripped)
//   7. Property with dimension value
//   8. Property with hex color → RGBA
//   9. Property with slot reference
//  10. Child node in tree
//  11. when block child
//  12. each block child
//  13. slot ref as child
//  14. Full ProfileCard round-trip
//  15. Error: bad AST

final class MosaicAnalyzerTests: XCTestCase {

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    func comp(_ src: String) throws -> MosaicComponent {
        try analyze(src)
    }

    // -------------------------------------------------------------------------
    // 1. Component name
    // -------------------------------------------------------------------------

    func testComponentName() throws {
        let c = try comp("component MyCard { Box { } }")
        XCTAssertEqual(c.name, "MyCard")
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
          slot flex: node;
          Box { }
        }
        """
        let c = try comp(src)
        XCTAssertEqual(c.slots.count, 6)
        XCTAssertEqual(c.slots[0].slotType, .primitive("text"))
        XCTAssertEqual(c.slots[1].slotType, .primitive("number"))
        XCTAssertEqual(c.slots[2].slotType, .primitive("bool"))
        XCTAssertEqual(c.slots[3].slotType, .primitive("image"))
        XCTAssertEqual(c.slots[4].slotType, .primitive("color"))
        XCTAssertEqual(c.slots[5].slotType, .primitive("node"))
    }

    // -------------------------------------------------------------------------
    // 3. List slot type
    // -------------------------------------------------------------------------

    func testListSlotType() throws {
        let c = try comp("component L { slot items: list<text>; Box { } }")
        XCTAssertEqual(c.slots[0].slotType, .list(.primitive("text")))
    }

    func testListOfListType() throws {
        let c = try comp("component L { slot matrix: list<list<number>>; Box { } }")
        XCTAssertEqual(c.slots[0].slotType, .list(.list(.primitive("number"))))
    }

    // -------------------------------------------------------------------------
    // 4. Component-type slot
    // -------------------------------------------------------------------------

    func testComponentTypeSlot() throws {
        let c = try comp("component Card { slot action: Button; Box { } }")
        XCTAssertEqual(c.slots[0].slotType, .component("Button"))
    }

    // -------------------------------------------------------------------------
    // 5. Root node type and isPrimitive
    // -------------------------------------------------------------------------

    func testPrimitiveRootNode() throws {
        let c = try comp("component A { Column { } }")
        XCTAssertEqual(c.root.nodeType, "Column")
        XCTAssertTrue(c.root.isPrimitive)
    }

    func testComponentRootNode() throws {
        let c = try comp("component A { MyWidget { } }")
        XCTAssertEqual(c.root.nodeType, "MyWidget")
        XCTAssertFalse(c.root.isPrimitive)
    }

    // -------------------------------------------------------------------------
    // 6. Property string value — quotes stripped
    // -------------------------------------------------------------------------

    func testStringPropertyStripped() throws {
        let c = try comp("""
        component A { Text { content: "hello world"; } }
        """)
        let prop = c.root.properties[0]
        XCTAssertEqual(prop.name, "content")
        XCTAssertEqual(prop.value, .literal("hello world"))
    }

    // -------------------------------------------------------------------------
    // 7. Property dimension value
    // -------------------------------------------------------------------------

    func testDimensionProperty() throws {
        let c = try comp("component A { Box { padding: 16dp; } }")
        XCTAssertEqual(c.root.properties[0].value, .number(16, "dp"))
    }

    func testPercentDimension() throws {
        let c = try comp("component A { Box { width: 50%; } }")
        XCTAssertEqual(c.root.properties[0].value, .number(50, "%"))
    }

    // -------------------------------------------------------------------------
    // 8. Property hex color → RGBA
    // -------------------------------------------------------------------------

    func testHexColorProperty() throws {
        let c = try comp("component A { Box { background: #2563eb; } }")
        XCTAssertEqual(c.root.properties[0].value, .color(0x25, 0x63, 0xeb, 255))
    }

    func testShortHexColor() throws {
        let c = try comp("component A { Box { background: #fff; } }")
        XCTAssertEqual(c.root.properties[0].value, .color(255, 255, 255, 255))
    }

    // -------------------------------------------------------------------------
    // 9. Property slot reference
    // -------------------------------------------------------------------------

    func testSlotRefProperty() throws {
        let c = try comp("component A { slot t: text; Text { content: @t; } }")
        XCTAssertEqual(c.root.properties[0].value, .slotRef("t"))
    }

    // -------------------------------------------------------------------------
    // 10. Child node
    // -------------------------------------------------------------------------

    func testChildNode() throws {
        let c = try comp("component A { Column { Text { content: \"hi\"; } } }")
        XCTAssertEqual(c.root.children.count, 1)
        guard case let .node(child) = c.root.children[0] else { return XCTFail() }
        XCTAssertEqual(child.nodeType, "Text")
    }

    // -------------------------------------------------------------------------
    // 11. when block child
    // -------------------------------------------------------------------------

    func testWhenBlockChild() throws {
        let src = """
        component A {
          slot show: bool;
          Column {
            when @show {
              Text { content: "yes"; }
            }
          }
        }
        """
        let c = try comp(src)
        guard case let .whenBlock(slot, body) = c.root.children[0] else {
            return XCTFail("Expected whenBlock")
        }
        XCTAssertEqual(slot, "show")
        XCTAssertEqual(body.count, 1)
        XCTAssertEqual(body[0].nodeType, "Text")
    }

    // -------------------------------------------------------------------------
    // 12. each block child
    // -------------------------------------------------------------------------

    func testEachBlockChild() throws {
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
        let c = try comp(src)
        guard case let .eachBlock(slot, item, body) = c.root.children[0] else {
            return XCTFail("Expected eachBlock")
        }
        XCTAssertEqual(slot, "items")
        XCTAssertEqual(item, "item")
        XCTAssertEqual(body.count, 1)
    }

    // -------------------------------------------------------------------------
    // 13. slot ref as child
    // -------------------------------------------------------------------------

    func testSlotRefChild() throws {
        let c = try comp("component A { slot act: node; Column { @act; } }")
        guard case let .slotRef(name) = c.root.children[0] else {
            return XCTFail("Expected slotRef child")
        }
        XCTAssertEqual(name, "act")
    }

    // -------------------------------------------------------------------------
    // 14. Full ProfileCard
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
        let c = try comp(src)
        XCTAssertEqual(c.name, "ProfileCard")
        XCTAssertEqual(c.slots.count, 3)
        XCTAssertEqual(c.slots[0].slotType, .primitive("image"))
        XCTAssertEqual(c.root.nodeType, "Column")
        XCTAssertEqual(c.root.children.count, 3)
    }

    // -------------------------------------------------------------------------
    // 15. Zero slots is valid
    // -------------------------------------------------------------------------

    func testZeroSlots() throws {
        let c = try comp("component Plain { Box { } }")
        XCTAssertEqual(c.slots.count, 0)
    }
}
