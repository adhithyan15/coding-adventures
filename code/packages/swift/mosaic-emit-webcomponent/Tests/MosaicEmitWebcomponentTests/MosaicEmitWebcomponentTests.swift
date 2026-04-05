import XCTest
@testable import MosaicEmitWebcomponent

// ============================================================================
// MosaicEmitWebcomponentTests
// ============================================================================
//
// Tests verify that the Web Component backend generates correct TypeScript.
//
// Tests:
//   1. Class extends HTMLElement
//   2. customElements.define called with kebab-case name
//   3. Private field generated for each slot
//   4. Public setter generated for each slot
//   5. connectedCallback present
//   6. _render method present
//   7. Filename hint ends in .ts
//   8. when block → if statement
//   9. each block → forEach
//  10. Color property → rgba() in style

final class MosaicEmitWebcomponentTests: XCTestCase {

    func emit(_ src: String) throws -> String {
        try emitWebComponent(source: src).code
    }

    // -------------------------------------------------------------------------
    // 1. Class extends HTMLElement
    // -------------------------------------------------------------------------

    func testClassExtendsHTMLElement() throws {
        let code = try emit("component Badge { Box { } }")
        XCTAssertTrue(code.contains("extends HTMLElement"))
    }

    // -------------------------------------------------------------------------
    // 2. customElements.define with kebab-case name
    // -------------------------------------------------------------------------

    func testCustomElementDefine() throws {
        let code = try emit("component ProfileCard { Box { } }")
        XCTAssertTrue(code.contains("customElements.define('mosaic-profile-card'"))
    }

    func testSimpleNameKebab() throws {
        let code = try emit("component Badge { Box { } }")
        XCTAssertTrue(code.contains("'mosaic-badge'"))
    }

    // -------------------------------------------------------------------------
    // 3. Private field for slot
    // -------------------------------------------------------------------------

    func testPrivateFieldForSlot() throws {
        let code = try emit("component A { slot title: text; Text { content: @title; } }")
        XCTAssertTrue(code.contains("private _title"))
    }

    // -------------------------------------------------------------------------
    // 4. Public setter for slot
    // -------------------------------------------------------------------------

    func testPublicSetterForSlot() throws {
        let code = try emit("component A { slot title: text; Text { content: @title; } }")
        XCTAssertTrue(code.contains("set title("))
    }

    // -------------------------------------------------------------------------
    // 5. connectedCallback
    // -------------------------------------------------------------------------

    func testConnectedCallback() throws {
        let code = try emit("component A { Box { } }")
        XCTAssertTrue(code.contains("connectedCallback()"))
    }

    // -------------------------------------------------------------------------
    // 6. _render method
    // -------------------------------------------------------------------------

    func testRenderMethod() throws {
        let code = try emit("component A { Box { } }")
        XCTAssertTrue(code.contains("_render()"))
    }

    // -------------------------------------------------------------------------
    // 7. Filename hint ends in .ts
    // -------------------------------------------------------------------------

    func testFilenameHint() throws {
        let result = try emitWebComponent(source: "component MyComp { Box { } }")
        XCTAssertEqual(result.filename, "MyComp.ts")
    }

    // -------------------------------------------------------------------------
    // 8. when block → if statement
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
        XCTAssertTrue(code.contains("if (this._show)"))
    }

    // -------------------------------------------------------------------------
    // 9. each block → forEach
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
        XCTAssertTrue(code.contains("forEach"))
    }

    // -------------------------------------------------------------------------
    // 10. Color property → rgba in style attribute
    // -------------------------------------------------------------------------

    func testColorProperty() throws {
        let code = try emit("component A { Box { background: #2563eb; } }")
        XCTAssertTrue(code.contains("rgba("))
    }

    // -------------------------------------------------------------------------
    // 11. _escapeHtml helper present
    // -------------------------------------------------------------------------

    func testEscapeHtmlHelper() throws {
        let code = try emit("component A { Box { } }")
        XCTAssertTrue(code.contains("_escapeHtml"))
    }
}
