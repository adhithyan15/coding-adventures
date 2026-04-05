import XCTest
@testable import MosaicVm
import MosaicAnalyzer

// ============================================================================
// MosaicVmTests
// ============================================================================
//
// Tests use a RecordingRenderer that captures all method calls, then assert
// on the call sequence and resolved values.
//
// Tests cover:
//   1.  beginComponent / endComponent called with correct name + slots
//   2.  beginNode / endNode called in correct order
//   3.  Resolved string property
//   4.  Resolved number property
//   5.  Resolved dimension property
//   6.  Resolved color property (RGBA)
//   7.  Resolved slot ref property (component slot)
//   8.  renderSlotChild called for @slotRef child
//   9.  beginWhen / endWhen called for when block
//  10.  beginEach / endEach called for each block; loop scope pushed
//  11.  Nested nodes — correct depth-first order
//  12.  Error: unresolved slot reference

final class MosaicVmTests: XCTestCase {

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    func makeVM(_ src: String) throws -> MosaicVM {
        let comp = try analyze(src)
        return MosaicVM(component: comp)
    }

    // -------------------------------------------------------------------------
    // Recording renderer — captures all calls for assertion
    // -------------------------------------------------------------------------

    enum Call: Equatable {
        case beginComponent(name: String)
        case endComponent
        case beginNode(tag: String, isPrimitive: Bool)
        case endNode(tag: String)
        case renderSlotChild(slotName: String)
        case beginWhen(slotName: String)
        case endWhen
        case beginEach(slotName: String, itemName: String)
        case endEach
    }

    class RecordingRenderer: MosaicRenderer {
        var calls: [Call] = []
        var resolvedProps: [String: ResolvedValue] = [:]

        func beginComponent(name: String, slots: [MosaicSlot]) {
            calls.append(.beginComponent(name: name))
        }
        func endComponent() { calls.append(.endComponent) }
        func beginNode(tag: String, isPrimitive: Bool, properties: [ResolvedProperty], ctx: SlotContext) {
            calls.append(.beginNode(tag: tag, isPrimitive: isPrimitive))
            for p in properties { resolvedProps[p.name] = p.value }
        }
        func endNode(tag: String) { calls.append(.endNode(tag: tag)) }
        func renderSlotChild(slotName: String, slotType: MosaicType, ctx: SlotContext) {
            calls.append(.renderSlotChild(slotName: slotName))
        }
        func beginWhen(slotName: String, ctx: SlotContext) {
            calls.append(.beginWhen(slotName: slotName))
        }
        func endWhen() { calls.append(.endWhen) }
        func beginEach(slotName: String, itemName: String, elementType: MosaicType, ctx: SlotContext) {
            calls.append(.beginEach(slotName: slotName, itemName: itemName))
        }
        func endEach() { calls.append(.endEach) }
        func emit() -> EmitResult { EmitResult(code: "// recorded", filename: nil) }
    }

    // -------------------------------------------------------------------------
    // 1. beginComponent / endComponent
    // -------------------------------------------------------------------------

    func testBeginEndComponent() throws {
        let vm = try makeVM("component MyCard { Box { } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertEqual(r.calls.first, .beginComponent(name: "MyCard"))
        XCTAssertEqual(r.calls.last, .endComponent)
    }

    // -------------------------------------------------------------------------
    // 2. beginNode / endNode order
    // -------------------------------------------------------------------------

    func testNodeCallOrder() throws {
        let vm = try makeVM("component A { Box { } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertTrue(r.calls.contains(.beginNode(tag: "Box", isPrimitive: true)))
        XCTAssertTrue(r.calls.contains(.endNode(tag: "Box")))
        // begin must precede end
        let beginIdx = r.calls.firstIndex(of: .beginNode(tag: "Box", isPrimitive: true))!
        let endIdx   = r.calls.firstIndex(of: .endNode(tag: "Box"))!
        XCTAssertLessThan(beginIdx, endIdx)
    }

    // -------------------------------------------------------------------------
    // 3. Resolved string property
    // -------------------------------------------------------------------------

    func testResolvedStringProperty() throws {
        let vm = try makeVM("component A { Text { content: \"hello\"; } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertEqual(r.resolvedProps["content"], .string("hello"))
    }

    // -------------------------------------------------------------------------
    // 4. Resolved number property
    // -------------------------------------------------------------------------

    func testResolvedNumberProperty() throws {
        let vm = try makeVM("component A { Box { gap: 8; } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertEqual(r.resolvedProps["gap"], .number(8))
    }

    // -------------------------------------------------------------------------
    // 5. Resolved dimension property
    // -------------------------------------------------------------------------

    func testResolvedDimensionProperty() throws {
        let vm = try makeVM("component A { Box { padding: 16dp; } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertEqual(r.resolvedProps["padding"], .dimension(ResolvedDimension(value: 16, unit: "dp")))
    }

    // -------------------------------------------------------------------------
    // 6. Resolved color property
    // -------------------------------------------------------------------------

    func testResolvedColorProperty() throws {
        let vm = try makeVM("component A { Box { background: #2563eb; } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertEqual(r.resolvedProps["background"], .color(ResolvedColor(r: 0x25, g: 0x63, b: 0xeb, a: 255)))
    }

    func testColorCssString() {
        let c = ResolvedColor(r: 37, g: 99, b: 235, a: 255)
        // alpha = 255/255 = 1.0 → "rgba(37, 99, 235, 1.0)"
        XCTAssertTrue(c.cssString.hasPrefix("rgba(37, 99, 235,"))
    }

    // -------------------------------------------------------------------------
    // 7. Resolved slot ref property
    // -------------------------------------------------------------------------

    func testResolvedSlotRefProperty() throws {
        let vm = try makeVM("component A { slot title: text; Text { content: @title; } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertEqual(r.resolvedProps["content"], .slotRef(name: "title", slotType: .primitive("text"), isLoopVar: false))
    }

    // -------------------------------------------------------------------------
    // 8. renderSlotChild
    // -------------------------------------------------------------------------

    func testRenderSlotChild() throws {
        let vm = try makeVM("component A { slot act: node; Column { @act; } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertTrue(r.calls.contains(.renderSlotChild(slotName: "act")))
    }

    // -------------------------------------------------------------------------
    // 9. when block
    // -------------------------------------------------------------------------

    func testWhenBlock() throws {
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
        let vm = try makeVM(src)
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertTrue(r.calls.contains(.beginWhen(slotName: "show")))
        XCTAssertTrue(r.calls.contains(.endWhen))
    }

    // -------------------------------------------------------------------------
    // 10. each block
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
        let vm = try makeVM(src)
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        XCTAssertTrue(r.calls.contains(.beginEach(slotName: "items", itemName: "item")))
        XCTAssertTrue(r.calls.contains(.endEach))
    }

    // -------------------------------------------------------------------------
    // 11. Nested nodes — depth-first order
    // -------------------------------------------------------------------------

    func testNestedNodeOrder() throws {
        let vm = try makeVM("component A { Column { Text { content: \"hi\"; } } }")
        let r = RecordingRenderer()
        _ = try vm.run(renderer: r)
        let expected: [Call] = [
            .beginComponent(name: "A"),
            .beginNode(tag: "Column", isPrimitive: true),
            .beginNode(tag: "Text", isPrimitive: true),
            .endNode(tag: "Text"),
            .endNode(tag: "Column"),
            .endComponent,
        ]
        XCTAssertEqual(r.calls, expected)
    }

    // -------------------------------------------------------------------------
    // 12. Error: unresolved slot reference
    // -------------------------------------------------------------------------

    func testUnresolvedSlotReference() throws {
        // Property references @ghost which is not declared as a slot
        let comp = try analyze("component A { Text { content: @ghost; } }")
        let vm = MosaicVM(component: comp)
        let r = RecordingRenderer()
        XCTAssertThrowsError(try vm.run(renderer: r)) { err in
            XCTAssertTrue(err is MosaicVMError)
        }
    }
}
