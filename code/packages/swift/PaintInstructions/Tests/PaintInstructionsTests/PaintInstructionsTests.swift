import XCTest
@testable import PaintInstructions

final class PaintInstructionsTests: XCTestCase {
    func testParseHexColor() {
        XCTAssertEqual(
            parsePaintColor("#336699"),
            PaintColorRGBA8(r: 0x33, g: 0x66, b: 0x99, a: 255)
        )
    }

    func testPaintSceneDefaults() {
        let scene = paintScene(width: 32, height: 16, instructions: [])
        XCTAssertEqual(scene.background, "#ffffff")
        XCTAssertEqual(scene.width, 32)
        XCTAssertEqual(scene.height, 16)
    }

    // -------------------------------------------------------------------
    // PaintInstruction exhaustive switch — every case has a distinct kind
    // -------------------------------------------------------------------

    func testEveryInstructionKindHasADistinctKindString() {
        func kindOf(_ instruction: PaintInstruction) -> String {
            switch instruction {
            case .rect: return "rect"
            case .line: return "line"
            case .glyphRun: return "glyph_run"
            case .group: return "group"
            case .clip: return "clip"
            case .layer: return "layer"
            }
        }

        XCTAssertEqual(kindOf(paintRect(x: 0, y: 0, width: 1, height: 1)), "rect")
        XCTAssertEqual(kindOf(paintLine(x1: 0, y1: 0, x2: 1, y2: 1, stroke: "#000000", strokeWidth: 1)), "line")
        XCTAssertEqual(kindOf(paintGlyphRun(glyphs: [], fontRef: "mono", fontSize: 16, fill: "#000000")), "glyph_run")
        XCTAssertEqual(kindOf(paintGroup(children: [])), "group")
        XCTAssertEqual(kindOf(paintClip(x: 0, y: 0, width: 1, height: 1, children: [])), "clip")
        XCTAssertEqual(kindOf(paintLayer(children: [])), "layer")
    }

    // -------------------------------------------------------------------
    // PaintRectInstruction stroke
    // -------------------------------------------------------------------

    func testPaintRectStrokeDefaults() {
        guard case .rect(let r) = paintRect(x: 0, y: 0, width: 1, height: 1) else {
            return XCTFail("expected .rect")
        }
        XCTAssertEqual(r.stroke, "")
        XCTAssertEqual(r.strokeWidth, 0.0)
    }

    func testPaintRectStrokeExplicit() {
        guard case .rect(let r) = paintRect(x: 0, y: 0, width: 1, height: 1, stroke: "#ff0000", strokeWidth: 2) else {
            return XCTFail("expected .rect")
        }
        XCTAssertEqual(r.stroke, "#ff0000")
        XCTAssertEqual(r.strokeWidth, 2.0)
    }

    // -------------------------------------------------------------------
    // Transform2D
    // -------------------------------------------------------------------

    func testTransform2DIdentityReportsIsIdentity() {
        XCTAssertTrue(Transform2D.identity.isIdentity)
    }

    func testTransform2DNonIdentityReportsNotIdentity() {
        let t = Transform2D(a: 2, b: 0, c: 0, d: 1, e: 0, f: 0)
        XCTAssertFalse(t.isIdentity)
    }

    // -------------------------------------------------------------------
    // PaintGlyphPlacement / paintGlyphRun
    // -------------------------------------------------------------------

    func testPaintGlyphPlacementStoresFields() {
        let glyph = PaintGlyphPlacement(glyphId: 72, x: 3.0, y: 4.0)
        XCTAssertEqual(glyph.glyphId, 72)
        XCTAssertEqual(glyph.x, 3.0)
        XCTAssertEqual(glyph.y, 4.0)
    }

    func testPaintLineStoresEndpointsAndStroke() {
        guard case .line(let l) = paintLine(x1: 1, y1: 2, x2: 3, y2: 4, stroke: "#000000", strokeWidth: 1.5) else {
            return XCTFail("expected .line")
        }
        XCTAssertEqual(l.x1, 1)
        XCTAssertEqual(l.y1, 2)
        XCTAssertEqual(l.x2, 3)
        XCTAssertEqual(l.y2, 4)
        XCTAssertEqual(l.stroke, "#000000")
        XCTAssertEqual(l.strokeWidth, 1.5)
    }

    func testPaintGlyphRunStoresGlyphsFontRefFontSizeFill() {
        let glyphs = [PaintGlyphPlacement(glyphId: 72, x: 0, y: 0), PaintGlyphPlacement(glyphId: 105, x: 8, y: 0)]
        guard case .glyphRun(let run) = paintGlyphRun(glyphs: glyphs, fontRef: "terminal-mono", fontSize: 16, fill: "#000000") else {
            return XCTFail("expected .glyphRun")
        }
        XCTAssertEqual(run.glyphs, glyphs)
        XCTAssertEqual(run.fontRef, "terminal-mono")
        XCTAssertEqual(run.fontSize, 16)
        XCTAssertEqual(run.fill, "#000000")
    }

    // -------------------------------------------------------------------
    // PaintGroup / PaintClip / PaintLayer
    // -------------------------------------------------------------------

    func testPaintGroupDefaultsToNoTransformOrOpacity() {
        guard case .group(let g) = paintGroup(children: []) else {
            return XCTFail("expected .group")
        }
        XCTAssertNil(g.transform)
        XCTAssertNil(g.opacity)
    }

    func testPaintGroupPreservesChildOrder() {
        let a = paintRect(x: 0, y: 0, width: 1, height: 1)
        let b = paintRect(x: 1, y: 1, width: 1, height: 1)
        guard case .group(let g) = paintGroup(children: [a, b]) else {
            return XCTFail("expected .group")
        }
        XCTAssertEqual(g.children, [a, b])
    }

    func testPaintClipStoresBoundsAndChildren() {
        let child = paintRect(x: 0, y: 0, width: 1, height: 1)
        guard case .clip(let c) = paintClip(x: 1, y: 2, width: 3, height: 4, children: [child]) else {
            return XCTFail("expected .clip")
        }
        XCTAssertEqual(c.x, 1)
        XCTAssertEqual(c.y, 2)
        XCTAssertEqual(c.width, 3)
        XCTAssertEqual(c.height, 4)
        XCTAssertEqual(c.children, [child])
    }

    func testPaintLayerDefaultsToNoFiltersBlendModeOpacityTransform() {
        guard case .layer(let l) = paintLayer(children: []) else {
            return XCTFail("expected .layer")
        }
        XCTAssertFalse(l.hasFilters)
        XCTAssertNil(l.blendMode)
        XCTAssertNil(l.opacity)
        XCTAssertNil(l.transform)
    }

    func testPaintLayerHasFiltersCanBeSetExplicitly() {
        guard case .layer(let l) = paintLayer(children: [], hasFilters: true) else {
            return XCTFail("expected .layer")
        }
        XCTAssertTrue(l.hasFilters)
    }
}
