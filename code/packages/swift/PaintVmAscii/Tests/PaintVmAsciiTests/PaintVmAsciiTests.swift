import XCTest
@testable import PaintVmAscii
import PaintInstructions

func glyphRun(from text: String, x: Double = 0, y: Double = 0) -> PaintInstruction {
    var glyphs: [PaintGlyphPlacement] = []
    for (i, scalar) in text.unicodeScalars.enumerated() {
        glyphs.append(PaintGlyphPlacement(glyphId: Int(scalar.value), x: x + Double(i) * 8, y: y))
    }
    return paintGlyphRun(glyphs: glyphs, fontRef: "terminal-mono", fontSize: 16, fill: "#000000")
}

final class PaintVmAsciiTests: XCTestCase {
    // -------------------------------------------------------------------
    // version
    // -------------------------------------------------------------------

    func testVersionIsNonEmptySemver() {
        // Module-qualified: on Darwin, XCTestCase itself has a `version`
        // class member, which shadows a bare reference to this module's
        // top-level `version` constant inside an XCTestCase subclass (a
        // Linux/Windows swift-corelibs-xctest build doesn't have that
        // member, so this only surfaces on macOS).
        XCTAssertFalse(PaintVmAscii.version.isEmpty)
        XCTAssertTrue(PaintVmAscii.version.range(of: #"^\d+\.\d+\.\d+$"#, options: .regularExpression) != nil)
    }

    // -------------------------------------------------------------------
    // scale validation
    // -------------------------------------------------------------------

    func testRejectsZeroScaleX() {
        let scene = createScene(width: 8, height: 16, instructions: [])
        XCTAssertThrowsError(try render(scene, AsciiOptions(scaleX: 0, scaleY: 16))) { error in
            XCTAssertEqual(error as? PaintVmAsciiError, .invalidScaleX(0))
        }
    }

    func testRejectsNegativeScaleY() {
        let scene = createScene(width: 8, height: 16, instructions: [])
        XCTAssertThrowsError(try render(scene, AsciiOptions(scaleX: 8, scaleY: -1))) { error in
            XCTAssertEqual(error as? PaintVmAsciiError, .invalidScaleY(-1))
        }
    }

    // -------------------------------------------------------------------
    // scene dimensions
    // -------------------------------------------------------------------

    func testRejectsNegativeWidth() {
        let scene = createScene(width: -1, height: 16, instructions: [])
        XCTAssertThrowsError(try renderDefault(scene))
    }

    func testRejectsSceneExceedingPerAxisCellCap() {
        let scene = createScene(width: 8 * 3000, height: 16, instructions: [])
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .sceneTooLarge = error as? PaintVmAsciiError else {
                return XCTFail("expected .sceneTooLarge")
            }
        }
    }

    func testRejectsZeroWidthHugeHeightScene() {
        let scene = createScene(width: 0, height: 16 * 5_000_000, instructions: [])
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .sceneTooLarge = error as? PaintVmAsciiError else {
                return XCTFail("expected .sceneTooLarge")
            }
        }
    }

    func testEmptySceneRendersToEmptyString() throws {
        let scene = createScene(width: 8, height: 16, instructions: [])
        XCTAssertEqual(try renderDefault(scene), "")
    }

    // -------------------------------------------------------------------
    // PaintRect
    // -------------------------------------------------------------------

    func testRectRejectsNegativeWidth() {
        let scene = createScene(
            width: 80, height: 16,
            instructions: [.rect(PaintRectInstruction(x: 0, y: 0, width: -1, height: 1, fill: "#000000"))]
        )
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .invalidRectangleGeometry = error as? PaintVmAsciiError else {
                return XCTFail("expected .invalidRectangleGeometry")
            }
        }
    }

    func testRectFillsWithBlockCharacter() throws {
        let scene = createScene(
            width: 24, height: 48,
            instructions: [paintRect(x: 0, y: 0, width: 24, height: 48, fill: "#000000")]
        )
        XCTAssertEqual(try renderDefault(scene), "\u{2588}\u{2588}\u{2588}\n\u{2588}\u{2588}\u{2588}\n\u{2588}\u{2588}\u{2588}")
    }

    func testEmptyFillPaintsNothing() throws {
        // Raw PaintRectInstruction rather than the paintRect() helper: the
        // helper defaults a blank fill to opaque black, so it can't
        // express "no fill" on its own.
        let scene = createScene(
            width: 24, height: 16,
            instructions: [.rect(PaintRectInstruction(x: 0, y: 0, width: 24, height: 16, fill: ""))]
        )
        XCTAssertEqual(try renderDefault(scene), "")
    }

    func testStrokeDrawsBoxBorder() throws {
        let scene = createScene(
            width: 24, height: 48,
            instructions: [.rect(PaintRectInstruction(x: 0, y: 0, width: 24, height: 48, fill: "", stroke: "#000000"))]
        )
        XCTAssertEqual(try renderDefault(scene), "\u{250C}\u{2500}\u{2510}\n\u{2502} \u{2502}\n\u{2514}\u{2500}\u{2518}")
    }

    // -------------------------------------------------------------------
    // PaintLine
    // -------------------------------------------------------------------

    func testLineRejectsNonFiniteCoordinates() {
        let scene = createScene(
            width: 80, height: 16,
            instructions: [.line(PaintLineInstruction(x1: .nan, y1: 0, x2: 10, y2: 0, stroke: "#000000", strokeWidth: 1))]
        )
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .invalidLineGeometry = error as? PaintVmAsciiError else {
                return XCTFail("expected .invalidLineGeometry")
            }
        }
    }

    func testDrawsHorizontalLine() throws {
        let scene = createScene(
            width: 32, height: 16,
            instructions: [paintLine(x1: 0, y1: 0, x2: 24, y2: 0, stroke: "#000000", strokeWidth: 1)]
        )
        XCTAssertEqual(try renderDefault(scene), "\u{2500}\u{2500}\u{2500}\u{2500}")
    }

    func testDrawsVerticalLine() throws {
        let scene = createScene(
            width: 8, height: 48,
            instructions: [paintLine(x1: 0, y1: 0, x2: 0, y2: 32, stroke: "#000000", strokeWidth: 1)]
        )
        XCTAssertEqual(try renderDefault(scene), "\u{2502}\n\u{2502}\n\u{2502}")
    }

    func testDrawsDiagonalLineViaBresenham() throws {
        let scene = createScene(
            width: 32, height: 32,
            instructions: [paintLine(x1: 0, y1: 0, x2: 24, y2: 32, stroke: "#000000", strokeWidth: 1)]
        )
        XCTAssertEqual(try renderDefault(scene), "\u{2500}\u{2500}\n  \u{2500}\u{2500}")
    }

    /// Regression: a zero-seeded Bresenham error term can overshoot the
    /// target row and loop forever. deltaRow=1, deltaCol=3 against a
    /// clip-clamped target is exactly the ratio that exposed the bug in
    /// this package's Haskell/Java/Kotlin siblings (issue #12093) — the
    /// only real assertion here is that render returns at all.
    func testShallowSlopeDiagonalLineTerminates() throws {
        let scene = createScene(
            width: 32, height: 16,
            instructions: [paintLine(x1: 0, y1: 0, x2: 24, y2: 32, stroke: "#000000", strokeWidth: 1)]
        )
        _ = try renderDefault(scene)
    }

    // -------------------------------------------------------------------
    // PaintGlyphRun
    // -------------------------------------------------------------------

    func testPlacesGlyphsAtScaledCoordinates() throws {
        let scene = createScene(width: 32, height: 16, instructions: [glyphRun(from: "Hi")])
        XCTAssertEqual(try renderDefault(scene), "Hi")
    }

    func testNonFiniteGlyphPositionIsSkippedNotFatal() throws {
        let scene = createScene(
            width: 32, height: 16,
            instructions: [
                paintGlyphRun(
                    glyphs: [
                        PaintGlyphPlacement(glyphId: 72, x: 0, y: 0),
                        PaintGlyphPlacement(glyphId: 105, x: .nan, y: 0),
                    ],
                    fontRef: "terminal-mono", fontSize: 16, fill: "#000000"
                )
            ]
        )
        XCTAssertEqual(try renderDefault(scene), "H")
    }

    func testControlCharactersReplacedWithQuestionMark() throws {
        let scene = createScene(
            width: 8, height: 16,
            instructions: [
                paintGlyphRun(glyphs: [PaintGlyphPlacement(glyphId: 0x07, x: 0, y: 0)], fontRef: "terminal-mono", fontSize: 16, fill: "#000000")
            ]
        )
        XCTAssertEqual(try renderDefault(scene), "?")
    }

    func testLoneSurrogateCodePointReplacedWithQuestionMark() throws {
        let scene = createScene(
            width: 8, height: 16,
            instructions: [
                paintGlyphRun(glyphs: [PaintGlyphPlacement(glyphId: 0xD800, x: 0, y: 0)], fontRef: "terminal-mono", fontSize: 16, fill: "#000000")
            ]
        )
        XCTAssertEqual(try renderDefault(scene), "?")
    }

    func testSupplementaryPlaneCodePointRendersAsItsOwnGlyph() throws {
        let scene = createScene(
            width: 8, height: 16,
            instructions: [
                paintGlyphRun(glyphs: [PaintGlyphPlacement(glyphId: 0x1F600, x: 0, y: 0)], fontRef: "terminal-mono", fontSize: 16, fill: "#000000")
            ]
        )
        XCTAssertEqual(try renderDefault(scene), String(Character(Unicode.Scalar(0x1F600)!)))
    }

    // -------------------------------------------------------------------
    // PaintGroup
    // -------------------------------------------------------------------

    func testRendersPlainGroupChildren() throws {
        let scene = createScene(width: 16, height: 16, instructions: [paintGroup(children: [glyphRun(from: "Hi")])])
        XCTAssertEqual(try renderDefault(scene), "Hi")
    }

    func testRejectsNonIdentityTransformOnGroup() {
        let scene = createScene(
            width: 16, height: 16,
            instructions: [.group(PaintGroupInstruction(children: [], transform: Transform2D(a: 2, b: 0, c: 0, d: 1, e: 0, f: 0)))]
        )
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .unsupportedInstruction = error as? PaintVmAsciiError else {
                return XCTFail("expected .unsupportedInstruction")
            }
        }
    }

    func testRejectsNonDefaultOpacityOnGroup() {
        let scene = createScene(width: 16, height: 16, instructions: [.group(PaintGroupInstruction(children: [], opacity: 0.5))])
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .unsupportedInstruction = error as? PaintVmAsciiError else {
                return XCTFail("expected .unsupportedInstruction")
            }
        }
    }

    // -------------------------------------------------------------------
    // PaintClip
    // -------------------------------------------------------------------

    func testClipRejectsNonFiniteGeometry() {
        let scene = createScene(width: 16, height: 16, instructions: [.clip(PaintClipInstruction(x: .nan, y: 0, width: 8, height: 8, children: []))])
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .invalidClipGeometry = error as? PaintVmAsciiError else {
                return XCTFail("expected .invalidClipGeometry")
            }
        }
    }

    func testClipRejectsOverflowingExtent() {
        let scene = createScene(
            width: 16, height: 16,
            instructions: [.clip(PaintClipInstruction(x: .greatestFiniteMagnitude, y: 0, width: .greatestFiniteMagnitude, height: 8, children: []))]
        )
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .invalidClipGeometry = error as? PaintVmAsciiError else {
                return XCTFail("expected .invalidClipGeometry")
            }
        }
    }

    func testClipsChildrenToBounds() throws {
        let scene = createScene(
            width: 16, height: 16,
            instructions: [paintClip(x: 0, y: 0, width: 8, height: 16, children: [glyphRun(from: "Hi")])]
        )
        XCTAssertEqual(try renderDefault(scene), "H")
    }

    // -------------------------------------------------------------------
    // PaintLayer
    // -------------------------------------------------------------------

    func testRendersPlainLayerChildren() throws {
        let scene = createScene(width: 16, height: 16, instructions: [paintLayer(children: [glyphRun(from: "Hi")])])
        XCTAssertEqual(try renderDefault(scene), "Hi")
    }

    func testRejectsLayerWithFilters() {
        let scene = createScene(width: 16, height: 16, instructions: [.layer(PaintLayerInstruction(children: [], hasFilters: true))])
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .unsupportedInstruction = error as? PaintVmAsciiError else {
                return XCTFail("expected .unsupportedInstruction")
            }
        }
    }

    func testRejectsNonNormalBlendMode() {
        let scene = createScene(width: 16, height: 16, instructions: [.layer(PaintLayerInstruction(children: [], blendMode: "multiply"))])
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .unsupportedInstruction = error as? PaintVmAsciiError else {
                return XCTFail("expected .unsupportedInstruction")
            }
        }
    }

    // -------------------------------------------------------------------
    // nesting depth
    // -------------------------------------------------------------------

    func nestedGroups(_ depth: Int, _ leaf: PaintInstruction) -> PaintInstruction {
        var current = leaf
        for _ in 0..<depth {
            current = paintGroup(children: [current])
        }
        return current
    }

    func testDeeplyNestedSceneIsRejected() {
        let scene = createScene(width: 16, height: 16, instructions: [nestedGroups(200, glyphRun(from: "x"))])
        XCTAssertThrowsError(try renderDefault(scene)) { error in
            guard case .sceneTooDeep = error as? PaintVmAsciiError else {
                return XCTFail("expected .sceneTooDeep")
            }
        }
    }

    func testModeratelyNestedSceneStillRenders() throws {
        let scene = createScene(width: 16, height: 16, instructions: [nestedGroups(10, glyphRun(from: "x"))])
        XCTAssertEqual(try renderDefault(scene), "x")
    }

    // -------------------------------------------------------------------
    // box-drawing merges
    // -------------------------------------------------------------------

    func testTwoRectanglesSharingAnEdgeMergeIntoTeeCharacters() throws {
        let scene = createScene(
            width: 40, height: 48,
            instructions: [
                .rect(PaintRectInstruction(x: 0, y: 0, width: 16, height: 48, fill: "", stroke: "#000000")),
                .rect(PaintRectInstruction(x: 16, y: 0, width: 16, height: 48, fill: "", stroke: "#000000")),
            ]
        )
        XCTAssertEqual(
            try renderDefault(scene),
            "\u{250C}\u{2500}\u{252C}\u{2500}\u{2510}\n\u{2502} \u{2502} \u{2502}\n\u{2514}\u{2500}\u{2534}\u{2500}\u{2518}"
        )
    }

    // -------------------------------------------------------------------
    // trailing whitespace trimming
    // -------------------------------------------------------------------

    func testTrimsTrailingSpacesAndBlankLines() throws {
        let scene = createScene(width: 40, height: 32, instructions: [glyphRun(from: "Hi", y: 0)])
        let text = try renderDefault(scene)
        XCTAssertEqual(text, "Hi")
        XCTAssertFalse(text.hasSuffix("\n"))
    }
}
