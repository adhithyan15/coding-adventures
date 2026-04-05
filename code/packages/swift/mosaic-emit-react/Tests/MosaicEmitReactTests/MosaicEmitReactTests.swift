import XCTest
@testable import MosaicEmitReact

// ============================================================================
// MosaicEmitReactTests
// ============================================================================
//
// Tests verify that the React backend generates correct TSX output.
//
// Tests:
//   1. Output contains React import
//   2. Component name appears in function signature
//   3. Props interface generated for slots
//   4. Primitive node → HTML element
//   5. Column → display:flex flexDirection:column style
//   6. Text content slot ref in JSX
//   7. Filename hint ends in .tsx
//   8. when block → conditional JSX
//   9. each block → .map() expression
//  10. Color property → rgba() in style

final class MosaicEmitReactTests: XCTestCase {

    func emit(_ src: String) throws -> String {
        try emitReact(source: src).code
    }

    // -------------------------------------------------------------------------
    // 1. React import present
    // -------------------------------------------------------------------------

    func testReactImportPresent() throws {
        let code = try emit("component A { Box { } }")
        XCTAssertTrue(code.contains("import React from 'react'"))
    }

    // -------------------------------------------------------------------------
    // 2. Component function name
    // -------------------------------------------------------------------------

    func testComponentFunctionName() throws {
        let code = try emit("component ProfileCard { Box { } }")
        XCTAssertTrue(code.contains("function ProfileCard("))
    }

    // -------------------------------------------------------------------------
    // 3. Props interface
    // -------------------------------------------------------------------------

    func testPropsInterface() throws {
        let code = try emit("component Label { slot text: text; Text { content: @text; } }")
        XCTAssertTrue(code.contains("interface LabelProps"))
        XCTAssertTrue(code.contains("text: string"))
    }

    // -------------------------------------------------------------------------
    // 4. Primitive node → div
    // -------------------------------------------------------------------------

    func testBoxBecomesDiv() throws {
        let code = try emit("component A { Box { } }")
        XCTAssertTrue(code.contains("<div"))
    }

    // -------------------------------------------------------------------------
    // 5. Column gets flex style
    // -------------------------------------------------------------------------

    func testColumnFlexStyle() throws {
        let code = try emit("component A { Column { } }")
        XCTAssertTrue(code.contains("flexDirection"))
    }

    // -------------------------------------------------------------------------
    // 6. Text content with slot ref
    // -------------------------------------------------------------------------

    func testTextContentSlotRef() throws {
        let code = try emit("component A { slot title: text; Text { content: @title; } }")
        XCTAssertTrue(code.contains("title"))
    }

    // -------------------------------------------------------------------------
    // 7. Filename hint
    // -------------------------------------------------------------------------

    func testFilenameHint() throws {
        let result = try emitReact(source: "component MyComp { Box { } }")
        XCTAssertEqual(result.filename, "MyComp.tsx")
    }

    // -------------------------------------------------------------------------
    // 8. when block → conditional expression
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
        let code = try emit(src)
        XCTAssertTrue(code.contains("show &&"))
    }

    // -------------------------------------------------------------------------
    // 9. each block → .map()
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
        let code = try emit(src)
        XCTAssertTrue(code.contains("items.map"))
    }

    // -------------------------------------------------------------------------
    // 10. Color property → rgba
    // -------------------------------------------------------------------------

    func testColorProperty() throws {
        let code = try emit("component A { Box { background: #2563eb; } }")
        XCTAssertTrue(code.contains("rgba("))
    }

    // -------------------------------------------------------------------------
    // 11. Dimension property → px
    // -------------------------------------------------------------------------

    func testDimensionProperty() throws {
        let code = try emit("component A { Box { padding: 16dp; } }")
        XCTAssertTrue(code.contains("16px"))
    }
}
