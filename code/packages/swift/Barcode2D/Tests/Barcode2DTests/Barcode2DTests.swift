import XCTest
@testable import Barcode2D
import PaintInstructions

// =============================================================================
// Barcode2DTests
// =============================================================================
//
// Comprehensive tests for the Barcode2D Swift package.
//
// Test plan:
//   1. makeModuleGrid — dimensions, all-light, module shape storage
//   2. setModule — immutability, correct mutation, out-of-bounds errors
//   3. layout (square) — dimensions, background rect, dark module count,
//      pixel coordinates, zero-dark-module grid, zero quiet zone
//   4. layout (hex) — dimensions, background rect, hex geometry offsets
//   5. layout validation — moduleSizePx <= 0, quietZoneModules < 0, shape mismatch
//   6. ModuleAnnotation — struct fields and optional values
//   7. AnnotatedModuleGrid — correct grid/annotation pairing
//   8. Barcode2DLayoutConfig — default values
//   9. version constant

final class Barcode2DTests: XCTestCase {

    // =========================================================================
    // 1. makeModuleGrid
    // =========================================================================

    func testMakeModuleGrid_defaultShape_isSquare() {
        let grid = makeModuleGrid(rows: 5, cols: 7)
        XCTAssertEqual(grid.moduleShape, .square)
    }

    func testMakeModuleGrid_dimensions() {
        let grid = makeModuleGrid(rows: 3, cols: 4)
        XCTAssertEqual(grid.rows, 3)
        XCTAssertEqual(grid.cols, 4)
    }

    func testMakeModuleGrid_allLight() {
        let grid = makeModuleGrid(rows: 4, cols: 4)
        for row in 0..<4 {
            for col in 0..<4 {
                XCTAssertFalse(grid.modules[row][col], "Expected light at (\(row),\(col))")
            }
        }
    }

    func testMakeModuleGrid_hexShape() {
        let grid = makeModuleGrid(rows: 33, cols: 30, moduleShape: .hex)
        XCTAssertEqual(grid.moduleShape, .hex)
        XCTAssertEqual(grid.rows, 33)
        XCTAssertEqual(grid.cols, 30)
    }

    func testMakeModuleGrid_singleCell() {
        let grid = makeModuleGrid(rows: 1, cols: 1)
        XCTAssertEqual(grid.rows, 1)
        XCTAssertEqual(grid.cols, 1)
        XCTAssertFalse(grid.modules[0][0])
    }

    func testMakeModuleGrid_rowCount_matchesModulesCount() {
        let grid = makeModuleGrid(rows: 10, cols: 5)
        XCTAssertEqual(grid.modules.count, 10)
        for row in grid.modules {
            XCTAssertEqual(row.count, 5)
        }
    }

    // =========================================================================
    // 2. setModule
    // =========================================================================

    func testSetModule_setsDarkModule() throws {
        let grid = makeModuleGrid(rows: 3, cols: 3)
        let updated = try setModule(grid: grid, row: 1, col: 1, dark: true)
        XCTAssertTrue(updated.modules[1][1])
    }

    func testSetModule_isImmutable() throws {
        let original = makeModuleGrid(rows: 3, cols: 3)
        let _ = try setModule(grid: original, row: 0, col: 0, dark: true)
        // original must not have changed
        XCTAssertFalse(original.modules[0][0])
    }

    func testSetModule_otherCellsUnchanged() throws {
        let grid = makeModuleGrid(rows: 3, cols: 3)
        let updated = try setModule(grid: grid, row: 1, col: 1, dark: true)
        XCTAssertFalse(updated.modules[0][0])
        XCTAssertFalse(updated.modules[2][2])
        XCTAssertFalse(updated.modules[1][0])
    }

    func testSetModule_unsetsDarkModule() throws {
        var grid = makeModuleGrid(rows: 2, cols: 2)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: false)
        XCTAssertFalse(grid.modules[0][0])
    }

    func testSetModule_cornerCells() throws {
        let grid = makeModuleGrid(rows: 5, cols: 5)
        let tl = try setModule(grid: grid, row: 0, col: 0, dark: true)
        let br = try setModule(grid: grid, row: 4, col: 4, dark: true)
        XCTAssertTrue(tl.modules[0][0])
        XCTAssertTrue(br.modules[4][4])
    }

    func testSetModule_rowOutOfBoundsThrows() {
        let grid = makeModuleGrid(rows: 3, cols: 3)
        XCTAssertThrowsError(try setModule(grid: grid, row: 3, col: 0, dark: true)) { error in
            guard case Barcode2DError.invalidConfig(let msg) = error else {
                return XCTFail("Expected invalidConfig error")
            }
            XCTAssertTrue(msg.contains("row"))
        }
    }

    func testSetModule_colOutOfBoundsThrows() {
        let grid = makeModuleGrid(rows: 3, cols: 3)
        XCTAssertThrowsError(try setModule(grid: grid, row: 0, col: 3, dark: true)) { error in
            guard case Barcode2DError.invalidConfig(let msg) = error else {
                return XCTFail("Expected invalidConfig error")
            }
            XCTAssertTrue(msg.contains("col"))
        }
    }

    func testSetModule_negativeRowThrows() {
        let grid = makeModuleGrid(rows: 3, cols: 3)
        XCTAssertThrowsError(try setModule(grid: grid, row: -1, col: 0, dark: true))
    }

    func testSetModule_negativeColThrows() {
        let grid = makeModuleGrid(rows: 3, cols: 3)
        XCTAssertThrowsError(try setModule(grid: grid, row: 0, col: -1, dark: true))
    }

    func testSetModule_chainedUpdates() throws {
        // Simulate building a 3-dark-module grid step by step
        var grid = makeModuleGrid(rows: 3, cols: 3)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
        grid = try setModule(grid: grid, row: 1, col: 1, dark: true)
        grid = try setModule(grid: grid, row: 2, col: 2, dark: true)
        XCTAssertTrue(grid.modules[0][0])
        XCTAssertTrue(grid.modules[1][1])
        XCTAssertTrue(grid.modules[2][2])
        XCTAssertFalse(grid.modules[0][1])
    }

    // =========================================================================
    // 3. layout — square modules
    // =========================================================================

    func testLayout_square_dimensions() throws {
        // 5×5 grid, 10px modules, 4 quiet zone modules
        // Expected: (5 + 2*4) * 10 = 130 × 130
        let grid = makeModuleGrid(rows: 5, cols: 5)
        let scene = try layout(grid: grid)
        XCTAssertEqual(scene.width, 130)
        XCTAssertEqual(scene.height, 130)
    }

    func testLayout_square_backgroundIsFirstInstruction() throws {
        let grid = makeModuleGrid(rows: 3, cols: 3)
        let scene = try layout(grid: grid)
        XCTAssertFalse(scene.instructions.isEmpty)
        let bg = scene.instructions[0]
        XCTAssertEqual(bg.x, 0)
        XCTAssertEqual(bg.y, 0)
        XCTAssertEqual(bg.fill, "#ffffff")
    }

    func testLayout_square_backgroundCoversFullSymbol() throws {
        let grid = makeModuleGrid(rows: 4, cols: 6)
        let config = Barcode2DLayoutConfig(moduleSizePx: 8.0, quietZoneModules: 2)
        let scene = try layout(grid: grid, config: config)
        // totalWidth = (6 + 2*2) * 8 = 80
        // totalHeight = (4 + 2*2) * 8 = 64
        let bg = scene.instructions[0]
        XCTAssertEqual(bg.width, 80)
        XCTAssertEqual(bg.height, 64)
    }

    func testLayout_square_allLightGrid_onlyBackground() throws {
        let grid = makeModuleGrid(rows: 5, cols: 5)
        let scene = try layout(grid: grid)
        // Only the background rect — no dark modules
        XCTAssertEqual(scene.instructions.count, 1)
    }

    func testLayout_square_darkModuleCount() throws {
        // Set 3 dark modules
        var grid = makeModuleGrid(rows: 5, cols: 5)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
        grid = try setModule(grid: grid, row: 2, col: 2, dark: true)
        grid = try setModule(grid: grid, row: 4, col: 4, dark: true)
        let scene = try layout(grid: grid)
        // 1 background + 3 dark module rects
        XCTAssertEqual(scene.instructions.count, 4)
    }

    func testLayout_square_darkModulePosition() throws {
        // moduleSizePx=10, quietZoneModules=4 → quietZonePx=40
        // Module at (row=0, col=0): x=40, y=40
        var grid = makeModuleGrid(rows: 5, cols: 5)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
        let scene = try layout(grid: grid)
        let darkRect = scene.instructions[1]
        XCTAssertEqual(darkRect.x, 40)
        XCTAssertEqual(darkRect.y, 40)
        XCTAssertEqual(darkRect.width, 10)
        XCTAssertEqual(darkRect.height, 10)
    }

    func testLayout_square_darkModulePosition_rowAndCol() throws {
        // Module at (row=2, col=3):
        // x = 40 + 3*10 = 70, y = 40 + 2*10 = 60
        var grid = makeModuleGrid(rows: 5, cols: 5)
        grid = try setModule(grid: grid, row: 2, col: 3, dark: true)
        let scene = try layout(grid: grid)
        let darkRect = scene.instructions[1]
        XCTAssertEqual(darkRect.x, 70)
        XCTAssertEqual(darkRect.y, 60)
    }

    func testLayout_square_customConfig() throws {
        // moduleSizePx=4, quietZoneModules=1
        // 3×3 grid: totalWidth = (3 + 2)*4 = 20, totalHeight = 20
        let grid = makeModuleGrid(rows: 3, cols: 3)
        let config = Barcode2DLayoutConfig(
            moduleSizePx: 4.0,
            quietZoneModules: 1,
            foreground: "#ff0000",
            background: "#00ff00"
        )
        let scene = try layout(grid: grid, config: config)
        XCTAssertEqual(scene.width, 20)
        XCTAssertEqual(scene.height, 20)
        XCTAssertEqual(scene.background, "#00ff00")
        XCTAssertEqual(scene.instructions[0].fill, "#00ff00")
    }

    func testLayout_square_zeroQuietZone() throws {
        // quietZoneModules=0 → no quiet zone, grid fills entire canvas
        let grid = makeModuleGrid(rows: 5, cols: 5)
        let config = Barcode2DLayoutConfig(moduleSizePx: 10.0, quietZoneModules: 0)
        let scene = try layout(grid: grid, config: config)
        // (5 + 0) * 10 = 50
        XCTAssertEqual(scene.width, 50)
        XCTAssertEqual(scene.height, 50)
    }

    func testLayout_square_foregroundColor() throws {
        var grid = makeModuleGrid(rows: 3, cols: 3)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
        let config = Barcode2DLayoutConfig(foreground: "#123456")
        let scene = try layout(grid: grid, config: config)
        let darkRect = scene.instructions[1]
        XCTAssertEqual(darkRect.fill, "#123456")
    }

    func testLayout_square_rectangularGrid() throws {
        // Non-square grids are valid (PDF417, rMQR, Data Matrix variants)
        let grid = makeModuleGrid(rows: 3, cols: 10)
        let config = Barcode2DLayoutConfig(moduleSizePx: 5.0, quietZoneModules: 1)
        let scene = try layout(grid: grid, config: config)
        // Width: (10 + 2) * 5 = 60, Height: (3 + 2) * 5 = 25
        XCTAssertEqual(scene.width, 60)
        XCTAssertEqual(scene.height, 25)
    }

    func testLayout_square_fullGrid_instructionCount() throws {
        // A 3×3 all-dark grid should produce 1 (background) + 9 (dark modules)
        var grid = makeModuleGrid(rows: 3, cols: 3)
        for row in 0..<3 {
            for col in 0..<3 {
                grid = try setModule(grid: grid, row: row, col: col, dark: true)
            }
        }
        let scene = try layout(grid: grid)
        XCTAssertEqual(scene.instructions.count, 10)
    }

    func testLayout_square_backgroundRect_fillsScene() throws {
        let grid = makeModuleGrid(rows: 21, cols: 21)
        let scene = try layout(grid: grid)
        let bg = scene.instructions[0]
        XCTAssertEqual(bg.width, scene.width)
        XCTAssertEqual(bg.height, scene.height)
    }

    // =========================================================================
    // 4. layout — hex modules
    // =========================================================================

    func testLayout_hex_basicRender() throws {
        // A 3×3 hex grid with one dark module
        var grid = makeModuleGrid(rows: 3, cols: 3, moduleShape: .hex)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
        let config = Barcode2DLayoutConfig(
            moduleSizePx: 10.0,
            quietZoneModules: 1,
            moduleShape: .hex
        )
        let scene = try layout(grid: grid, config: config)
        // background + 1 dark module
        XCTAssertEqual(scene.instructions.count, 2)
    }

    func testLayout_hex_totalWidthIncludesOddRowOffset() throws {
        // hexWidth = 10, cols=3, quietZoneModules=1
        // totalWidth = (3 + 2*1) * 10 + 10/2 = 55
        let grid = makeModuleGrid(rows: 2, cols: 3, moduleShape: .hex)
        let config = Barcode2DLayoutConfig(
            moduleSizePx: 10.0,
            quietZoneModules: 1,
            moduleShape: .hex
        )
        let scene = try layout(grid: grid, config: config)
        XCTAssertEqual(scene.width, 55)
    }

    func testLayout_hex_heightUsesHexRowStep() throws {
        // hexHeight = 10 * (√3/2) ≈ 8.66
        // rows=2, quietZoneModules=1
        // totalHeight = (2 + 2) * 8.66 ≈ 34.64 → rounds to 35
        let grid = makeModuleGrid(rows: 2, cols: 2, moduleShape: .hex)
        let config = Barcode2DLayoutConfig(
            moduleSizePx: 10.0,
            quietZoneModules: 1,
            moduleShape: .hex
        )
        let scene = try layout(grid: grid, config: config)
        // Just check it is greater than the row count * 10 (since hex rows are shorter)
        XCTAssertLessThan(scene.height, (2 + 2) * 10)
    }

    func testLayout_hex_oddRowIsOffset() throws {
        // Compare column 0 positions in row 0 vs row 1.
        // Row 1 should have a larger x because of the half-hex offset.
        var grid = makeModuleGrid(rows: 2, cols: 1, moduleShape: .hex)
        grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
        grid = try setModule(grid: grid, row: 1, col: 0, dark: true)
        let config = Barcode2DLayoutConfig(
            moduleSizePx: 10.0,
            quietZoneModules: 0,
            moduleShape: .hex
        )
        let scene = try layout(grid: grid, config: config)
        // instructions[0] = background, [1] = row 0 col 0, [2] = row 1 col 0
        let row0x = scene.instructions[1].x
        let row1x = scene.instructions[2].x
        XCTAssertGreaterThan(row1x, row0x)
    }

    func testLayout_hex_allLightGrid_onlyBackground() throws {
        let grid = makeModuleGrid(rows: 5, cols: 5, moduleShape: .hex)
        let config = Barcode2DLayoutConfig(moduleShape: .hex)
        let scene = try layout(grid: grid, config: config)
        XCTAssertEqual(scene.instructions.count, 1)
    }

    // =========================================================================
    // 5. layout — validation errors
    // =========================================================================

    func testLayout_invalidModuleSizePx_zero() {
        let grid = makeModuleGrid(rows: 5, cols: 5)
        let config = Barcode2DLayoutConfig(moduleSizePx: 0.0)
        XCTAssertThrowsError(try layout(grid: grid, config: config)) { error in
            guard case Barcode2DError.invalidConfig(let msg) = error else {
                return XCTFail("Expected invalidConfig")
            }
            XCTAssertTrue(msg.contains("moduleSizePx"))
        }
    }

    func testLayout_invalidModuleSizePx_negative() {
        let grid = makeModuleGrid(rows: 5, cols: 5)
        let config = Barcode2DLayoutConfig(moduleSizePx: -5.0)
        XCTAssertThrowsError(try layout(grid: grid, config: config))
    }

    func testLayout_invalidQuietZoneModules_negative() {
        let grid = makeModuleGrid(rows: 5, cols: 5)
        let config = Barcode2DLayoutConfig(quietZoneModules: -1)
        XCTAssertThrowsError(try layout(grid: grid, config: config)) { error in
            guard case Barcode2DError.invalidConfig(let msg) = error else {
                return XCTFail("Expected invalidConfig")
            }
            XCTAssertTrue(msg.contains("quietZoneModules"))
        }
    }

    func testLayout_shapeMismatch_squareGridHexConfig() {
        let grid = makeModuleGrid(rows: 5, cols: 5, moduleShape: .square)
        let config = Barcode2DLayoutConfig(moduleShape: .hex)
        XCTAssertThrowsError(try layout(grid: grid, config: config)) { error in
            guard case Barcode2DError.invalidConfig(let msg) = error else {
                return XCTFail("Expected invalidConfig")
            }
            XCTAssertTrue(msg.contains("moduleShape"))
        }
    }

    func testLayout_shapeMismatch_hexGridSquareConfig() {
        let grid = makeModuleGrid(rows: 5, cols: 5, moduleShape: .hex)
        let config = Barcode2DLayoutConfig(moduleShape: .square)
        XCTAssertThrowsError(try layout(grid: grid, config: config))
    }

    // =========================================================================
    // 6. ModuleAnnotation
    // =========================================================================

    func testModuleAnnotation_finderRole() {
        let ann = ModuleAnnotation(role: .finder, dark: true)
        XCTAssertEqual(ann.role, .finder)
        XCTAssertTrue(ann.dark)
        XCTAssertNil(ann.codewordIndex)
        XCTAssertNil(ann.bitIndex)
        XCTAssertTrue(ann.metadata.isEmpty)
    }

    func testModuleAnnotation_dataRoleWithIndices() {
        let ann = ModuleAnnotation(
            role: .data,
            dark: false,
            codewordIndex: 5,
            bitIndex: 3,
            metadata: ["format_role": "qr:data"]
        )
        XCTAssertEqual(ann.role, .data)
        XCTAssertFalse(ann.dark)
        XCTAssertEqual(ann.codewordIndex, 5)
        XCTAssertEqual(ann.bitIndex, 3)
        XCTAssertEqual(ann.metadata["format_role"], "qr:data")
    }

    func testModuleAnnotation_allRoles() {
        // Ensure all roles can be instantiated without crash
        let roles: [ModuleRole] = [.finder, .separator, .timing, .alignment, .format, .data, .ecc, .padding]
        for role in roles {
            let ann = ModuleAnnotation(role: role, dark: true)
            XCTAssertEqual(ann.role, role)
        }
    }

    func testModuleAnnotation_eccRole() {
        let ann = ModuleAnnotation(role: .ecc, dark: true, codewordIndex: 12, bitIndex: 7)
        XCTAssertEqual(ann.role, .ecc)
        XCTAssertEqual(ann.codewordIndex, 12)
        XCTAssertEqual(ann.bitIndex, 7)
    }

    // =========================================================================
    // 7. AnnotatedModuleGrid
    // =========================================================================

    func testAnnotatedModuleGrid_creation() {
        let grid = makeModuleGrid(rows: 2, cols: 2)
        let ann = ModuleAnnotation(role: .finder, dark: false)
        let annotations: [[ModuleAnnotation?]] = [
            [ann, nil],
            [nil, ann],
        ]
        let annotated = AnnotatedModuleGrid(grid: grid, annotations: annotations)
        XCTAssertEqual(annotated.grid.rows, 2)
        XCTAssertEqual(annotated.grid.cols, 2)
        XCTAssertNotNil(annotated.annotations[0][0])
        XCTAssertNil(annotated.annotations[0][1])
    }

    func testAnnotatedModuleGrid_nilAnnotationsAllowed() {
        let grid = makeModuleGrid(rows: 1, cols: 1)
        let annotations: [[ModuleAnnotation?]] = [[nil]]
        let annotated = AnnotatedModuleGrid(grid: grid, annotations: annotations)
        XCTAssertNil(annotated.annotations[0][0])
    }

    // =========================================================================
    // 8. Barcode2DLayoutConfig defaults
    // =========================================================================

    func testLayoutConfig_defaults() {
        let config = Barcode2DLayoutConfig()
        XCTAssertEqual(config.moduleSizePx, 10.0, accuracy: 0.0001)
        XCTAssertEqual(config.quietZoneModules, 4)
        XCTAssertEqual(config.foreground, "#000000")
        XCTAssertEqual(config.background, "#ffffff")
        XCTAssertFalse(config.showAnnotations)
        XCTAssertEqual(config.moduleShape, .square)
    }

    func testLayoutConfig_customValues() {
        let config = Barcode2DLayoutConfig(
            moduleSizePx: 5.0,
            quietZoneModules: 2,
            foreground: "#111111",
            background: "#eeeeee",
            showAnnotations: true,
            moduleShape: .hex
        )
        XCTAssertEqual(config.moduleSizePx, 5.0, accuracy: 0.0001)
        XCTAssertEqual(config.quietZoneModules, 2)
        XCTAssertEqual(config.foreground, "#111111")
        XCTAssertEqual(config.background, "#eeeeee")
        XCTAssertTrue(config.showAnnotations)
        XCTAssertEqual(config.moduleShape, .hex)
    }

    // =========================================================================
    // 9. version constant
    // =========================================================================

    func testVersionConstant() {
        XCTAssertEqual(Barcode2D.version, "0.1.0")
    }

    // =========================================================================
    // 10. Integration — QR v1 corner finder pattern
    // =========================================================================
    //
    // A QR Code v1 finder pattern occupies a 7×7 region. We place a simplified
    // version to verify that layout produces correct pixel coordinates.

    func testLayout_square_finderPatternCornerPixels() throws {
        // Build a 7×7 all-dark grid (simulates a solid finder pattern)
        var grid = makeModuleGrid(rows: 7, cols: 7)
        for row in 0..<7 {
            for col in 0..<7 {
                grid = try setModule(grid: grid, row: row, col: col, dark: true)
            }
        }
        let config = Barcode2DLayoutConfig(moduleSizePx: 10.0, quietZoneModules: 0)
        let scene = try layout(grid: grid, config: config)
        // background + 7*7 = 50 instructions
        XCTAssertEqual(scene.instructions.count, 50)
        // First dark rect is at (0, 0) since quietZoneModules=0
        let first = scene.instructions[1]
        XCTAssertEqual(first.x, 0)
        XCTAssertEqual(first.y, 0)
    }

    // =========================================================================
    // 11. ModuleGrid equality
    // =========================================================================

    func testModuleGrid_equalityAfterNoChange() throws {
        let g1 = makeModuleGrid(rows: 3, cols: 3)
        let g2 = try setModule(grid: g1, row: 0, col: 0, dark: false)
        // Setting light → light should produce an equal grid
        XCTAssertEqual(g1, g2)
    }

    func testModuleGrid_inequalityAfterChange() throws {
        let g1 = makeModuleGrid(rows: 3, cols: 3)
        let g2 = try setModule(grid: g1, row: 0, col: 0, dark: true)
        XCTAssertNotEqual(g1, g2)
    }
}
