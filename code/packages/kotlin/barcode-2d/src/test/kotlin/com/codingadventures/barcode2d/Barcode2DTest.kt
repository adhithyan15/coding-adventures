package com.codingadventures.barcode2d

import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintinstructions.PathCommand
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.math.sqrt
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNotSame
import kotlin.test.assertSame
import kotlin.test.assertTrue

/**
 * Tests for the barcode-2d package.
 *
 * We aim for ≥ 90 % line coverage.  Every public API function, every sealed
 * class branch, and every error path is exercised.  Tests are grouped by
 * type and labelled with their equivalence class.
 *
 * ## Testing philosophy
 *
 * barcode-2d is the single layer that converts abstract module coordinates
 * into real pixel instructions.  Bugs here produce visually wrong barcodes
 * that scanners cannot read.  We test every formula exactly:
 *
 *   x = quietZonePx + col * moduleSizePx
 *   y = quietZonePx + row * moduleSizePx
 *   totalWidth  = (cols + 2 * quietZoneModules) * moduleSizePx
 *   totalHeight = (rows + 2 * quietZoneModules) * moduleSizePx
 *
 * and the hex-geometry formulas for MaxiCode.
 */
class Barcode2DTest {

    // =========================================================================
    // VERSION
    // =========================================================================

    @Test
    fun `VERSION is 0_1_0`() {
        assertEquals("0.1.0", VERSION)
    }

    // =========================================================================
    // ModuleShape enum
    // =========================================================================

    @Test
    fun `ModuleShape SQUARE and HEX are distinct`() {
        assertNotEquals(ModuleShape.SQUARE, ModuleShape.HEX)
    }

    @Test
    fun `ModuleShape when expression is exhaustive`() {
        val shapes = listOf(ModuleShape.SQUARE, ModuleShape.HEX)
        val names = shapes.map { shape ->
            when (shape) {
                ModuleShape.SQUARE -> "square"
                ModuleShape.HEX    -> "hex"
            }
        }
        assertEquals(listOf("square", "hex"), names)
    }

    // =========================================================================
    // ModuleGrid construction
    // =========================================================================

    @Test
    fun `makeModuleGrid creates grid with correct dimensions`() {
        val grid = makeModuleGrid(rows = 5, cols = 7)
        assertEquals(5, grid.rows)
        assertEquals(7, grid.cols)
    }

    @Test
    fun `makeModuleGrid all modules are false by default`() {
        val grid = makeModuleGrid(rows = 3, cols = 4)
        for (row in 0 until grid.rows) {
            for (col in 0 until grid.cols) {
                assertFalse(grid.modules[row][col], "Module ($row,$col) should be false")
            }
        }
    }

    @Test
    fun `makeModuleGrid default moduleShape is SQUARE`() {
        val grid = makeModuleGrid(rows = 2, cols = 2)
        assertEquals(ModuleShape.SQUARE, grid.moduleShape)
    }

    @Test
    fun `makeModuleGrid HEX shape is stored correctly`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        assertEquals(ModuleShape.HEX, grid.moduleShape)
    }

    @Test
    fun `makeModuleGrid 1x1 grid`() {
        val grid = makeModuleGrid(rows = 1, cols = 1)
        assertEquals(1, grid.rows)
        assertEquals(1, grid.cols)
        assertFalse(grid.modules[0][0])
    }

    @Test
    fun `makeModuleGrid 21x21 QR v1 grid has correct module count`() {
        val grid = makeModuleGrid(rows = 21, cols = 21)
        assertEquals(21, grid.modules.size)
        assertEquals(21, grid.modules[0].size)
    }

    // =========================================================================
    // ModuleGrid equality
    // =========================================================================

    @Test
    fun `ModuleGrid equality uses structural equality`() {
        val g1 = makeModuleGrid(rows = 2, cols = 2)
        val g2 = makeModuleGrid(rows = 2, cols = 2)
        assertEquals(g1, g2)
    }

    @Test
    fun `ModuleGrid inequality on different dimensions`() {
        val g1 = makeModuleGrid(rows = 2, cols = 2)
        val g2 = makeModuleGrid(rows = 3, cols = 2)
        assertNotEquals(g1, g2)
    }

    // =========================================================================
    // setModule — immutability and correctness
    // =========================================================================

    @Test
    fun `setModule sets a dark module correctly`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        val g2 = setModule(grid, row = 1, col = 1, dark = true)
        assertTrue(g2.modules[1][1])
    }

    @Test
    fun `setModule leaves the original grid unchanged`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        setModule(grid, row = 1, col = 1, dark = true)
        assertFalse(grid.modules[1][1], "Original must not be mutated")
    }

    @Test
    fun `setModule returns a new object`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        assertNotSame(grid, g2)
    }

    @Test
    fun `setModule does not affect other modules`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        val g2 = setModule(grid, row = 1, col = 1, dark = true)
        // All modules except (1,1) should still be false.
        for (row in 0 until 3) {
            for (col in 0 until 3) {
                if (row == 1 && col == 1) continue
                assertFalse(g2.modules[row][col], "Module ($row,$col) should still be false")
            }
        }
    }

    @Test
    fun `setModule shares unchanged rows with original`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        // Rows 1 and 2 are not modified; Gradle should share the same list object.
        assertSame(grid.modules[1], g2.modules[1])
        assertSame(grid.modules[2], g2.modules[2])
    }

    @Test
    fun `setModule sets module to false on a dark module`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        val g1 = setModule(grid, row = 0, col = 0, dark = true)
        val g2 = setModule(g1, row = 0, col = 0, dark = false)
        assertFalse(g2.modules[0][0])
    }

    @Test
    fun `setModule top-left corner`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        assertTrue(g2.modules[0][0])
    }

    @Test
    fun `setModule bottom-right corner`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 4, col = 4, dark = true)
        assertTrue(g2.modules[4][4])
    }

    @Test
    fun `setModule throws on negative row`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        assertThrows<IndexOutOfBoundsException> {
            setModule(grid, row = -1, col = 0, dark = true)
        }
    }

    @Test
    fun `setModule throws on row out of bounds`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        assertThrows<IndexOutOfBoundsException> {
            setModule(grid, row = 3, col = 0, dark = true)
        }
    }

    @Test
    fun `setModule throws on negative col`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        assertThrows<IndexOutOfBoundsException> {
            setModule(grid, row = 0, col = -1, dark = true)
        }
    }

    @Test
    fun `setModule throws on col out of bounds`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        assertThrows<IndexOutOfBoundsException> {
            setModule(grid, row = 0, col = 3, dark = true)
        }
    }

    // =========================================================================
    // Barcode2DLayoutConfig — defaults
    // =========================================================================

    @Test
    fun `Barcode2DLayoutConfig default moduleSizePx is 10`() {
        val cfg = Barcode2DLayoutConfig()
        assertEquals(10, cfg.moduleSizePx)
    }

    @Test
    fun `Barcode2DLayoutConfig default quietZoneModules is 4`() {
        val cfg = Barcode2DLayoutConfig()
        assertEquals(4, cfg.quietZoneModules)
    }

    @Test
    fun `Barcode2DLayoutConfig default foreground is black`() {
        val cfg = Barcode2DLayoutConfig()
        assertEquals("#000000", cfg.foreground)
    }

    @Test
    fun `Barcode2DLayoutConfig default background is white`() {
        val cfg = Barcode2DLayoutConfig()
        assertEquals("#ffffff", cfg.background)
    }

    @Test
    fun `Barcode2DLayoutConfig default showAnnotations is false`() {
        val cfg = Barcode2DLayoutConfig()
        assertFalse(cfg.showAnnotations)
    }

    @Test
    fun `Barcode2DLayoutConfig default moduleShape is SQUARE`() {
        val cfg = Barcode2DLayoutConfig()
        assertEquals(ModuleShape.SQUARE, cfg.moduleShape)
    }

    @Test
    fun `Barcode2DLayoutConfig custom values are stored`() {
        val cfg = Barcode2DLayoutConfig(
            moduleSizePx = 5,
            quietZoneModules = 2,
            foreground = "#ff0000",
            background = "#eeeeee",
            showAnnotations = true,
            moduleShape = ModuleShape.HEX,
        )
        assertEquals(5, cfg.moduleSizePx)
        assertEquals(2, cfg.quietZoneModules)
        assertEquals("#ff0000", cfg.foreground)
        assertEquals("#eeeeee", cfg.background)
        assertTrue(cfg.showAnnotations)
        assertEquals(ModuleShape.HEX, cfg.moduleShape)
    }

    // =========================================================================
    // layout — validation errors
    // =========================================================================

    @Test
    fun `layout throws InvalidBarcode2DConfigException when moduleSizePx is 0`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val cfg = Barcode2DLayoutConfig(moduleSizePx = 0)
        assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
    }

    @Test
    fun `layout throws when moduleSizePx is negative`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val cfg = Barcode2DLayoutConfig(moduleSizePx = -3)
        assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
    }

    @Test
    fun `layout throws when quietZoneModules is negative`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val cfg = Barcode2DLayoutConfig(quietZoneModules = -1)
        assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
    }

    @Test
    fun `layout throws when moduleShape mismatches grid shape`() {
        // Grid is SQUARE but config says HEX
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.SQUARE)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
    }

    @Test
    fun `layout throws when config is HEX but grid is SQUARE`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.SQUARE)
        assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
    }

    @Test
    fun `InvalidBarcode2DConfigException is a Barcode2DException`() {
        val e = InvalidBarcode2DConfigException("test")
        assertTrue(e is Barcode2DException)
    }

    // =========================================================================
    // layoutSquare — canvas dimensions
    // =========================================================================

    @Test
    fun `layoutSquare default config all-light grid produces correct canvas size`() {
        // 21×21 QR v1, default config: moduleSizePx=10, quietZoneModules=4
        // totalWidth  = (21 + 2*4) * 10 = 290
        // totalHeight = (21 + 2*4) * 10 = 290
        val grid = makeModuleGrid(rows = 21, cols = 21)
        val scene = layout(grid)
        assertEquals(290, scene.width)
        assertEquals(290, scene.height)
    }

    @Test
    fun `layoutSquare 1x1 grid default config canvas size`() {
        // totalWidth  = (1 + 2*4) * 10 = 90
        // totalHeight = (1 + 2*4) * 10 = 90
        val grid = makeModuleGrid(rows = 1, cols = 1)
        val scene = layout(grid)
        assertEquals(90, scene.width)
        assertEquals(90, scene.height)
    }

    @Test
    fun `layoutSquare custom moduleSizePx and quietZone`() {
        // 5×5 grid, moduleSizePx=5, quietZoneModules=2
        // totalWidth  = (5 + 2*2) * 5 = 45
        // totalHeight = (5 + 2*2) * 5 = 45
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val cfg = Barcode2DLayoutConfig(moduleSizePx = 5, quietZoneModules = 2)
        val scene = layout(grid, cfg)
        assertEquals(45, scene.width)
        assertEquals(45, scene.height)
    }

    @Test
    fun `layoutSquare zero quietZone`() {
        // 4×4 grid, moduleSizePx=10, quietZoneModules=0
        // totalWidth  = (4 + 0) * 10 = 40
        val grid = makeModuleGrid(rows = 4, cols = 4)
        val cfg = Barcode2DLayoutConfig(quietZoneModules = 0)
        val scene = layout(grid, cfg)
        assertEquals(40, scene.width)
        assertEquals(40, scene.height)
    }

    @Test
    fun `layoutSquare non-square grid dimensions`() {
        // 10 rows × 20 cols, moduleSizePx=10, quietZoneModules=4
        // totalWidth  = (20 + 8) * 10 = 280
        // totalHeight = (10 + 8) * 10 = 180
        val grid = makeModuleGrid(rows = 10, cols = 20)
        val scene = layout(grid)
        assertEquals(280, scene.width)
        assertEquals(180, scene.height)
    }

    // =========================================================================
    // layoutSquare — instruction count and background
    // =========================================================================

    @Test
    fun `layoutSquare all-light grid produces only background instruction`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val scene = layout(grid)
        // Only the background rect; no dark modules.
        assertEquals(1, scene.instructions.size)
    }

    @Test
    fun `layoutSquare background instruction is a PaintRect`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val scene = layout(grid)
        assertIs<PaintInstruction.PaintRect>(scene.instructions[0])
    }

    @Test
    fun `layoutSquare background rect covers full canvas`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val scene = layout(grid)
        val bg = scene.instructions[0] as PaintInstruction.PaintRect
        assertEquals(0, bg.x)
        assertEquals(0, bg.y)
        assertEquals(scene.width, bg.width)
        assertEquals(scene.height, bg.height)
    }

    @Test
    fun `layoutSquare background rect uses config background colour`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        val cfg = Barcode2DLayoutConfig(background = "#eeeeee")
        val scene = layout(grid, cfg)
        val bg = scene.instructions[0] as PaintInstruction.PaintRect
        assertEquals("#eeeeee", bg.fill)
    }

    @Test
    fun `layoutSquare scene background colour matches config`() {
        val grid = makeModuleGrid(rows = 3, cols = 3)
        val cfg = Barcode2DLayoutConfig(background = "#ffffff")
        val scene = layout(grid, cfg)
        assertEquals("#ffffff", scene.background)
    }

    @Test
    fun `layoutSquare single dark module produces two instructions`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        val scene = layout(g2)
        // 1 background + 1 dark module
        assertEquals(2, scene.instructions.size)
    }

    @Test
    fun `layoutSquare dark module instruction is a PaintRect`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        val scene = layout(g2)
        assertIs<PaintInstruction.PaintRect>(scene.instructions[1])
    }

    @Test
    fun `layoutSquare dark module x position accounts for quiet zone`() {
        // row=0, col=2, moduleSizePx=10, quietZoneModules=4
        // x = 4*10 + 2*10 = 60
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 0, col = 2, dark = true)
        val scene = layout(g2)
        val rect = scene.instructions[1] as PaintInstruction.PaintRect
        assertEquals(60, rect.x)
    }

    @Test
    fun `layoutSquare dark module y position accounts for quiet zone`() {
        // row=3, col=0, moduleSizePx=10, quietZoneModules=4
        // y = 4*10 + 3*10 = 70
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 3, col = 0, dark = true)
        val scene = layout(g2)
        val rect = scene.instructions[1] as PaintInstruction.PaintRect
        assertEquals(70, rect.y)
    }

    @Test
    fun `layoutSquare dark module has correct width and height`() {
        // moduleSizePx=10
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        val scene = layout(g2)
        val rect = scene.instructions[1] as PaintInstruction.PaintRect
        assertEquals(10, rect.width)
        assertEquals(10, rect.height)
    }

    @Test
    fun `layoutSquare dark module uses config foreground colour`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        val cfg = Barcode2DLayoutConfig(foreground = "#000000")
        val scene = layout(g2, cfg)
        val rect = scene.instructions[1] as PaintInstruction.PaintRect
        assertEquals("#000000", rect.fill)
    }

    @Test
    fun `layoutSquare multiple dark modules produce correct instruction count`() {
        var grid = makeModuleGrid(rows = 5, cols = 5)
        grid = setModule(grid, row = 0, col = 0, dark = true)
        grid = setModule(grid, row = 1, col = 1, dark = true)
        grid = setModule(grid, row = 2, col = 2, dark = true)
        val scene = layout(grid)
        // 1 background + 3 dark modules
        assertEquals(4, scene.instructions.size)
    }

    @Test
    fun `layoutSquare instructions are in row-major scan order`() {
        // Dark modules at (0,1) and (1,0): row-major means (0,1) comes first
        var grid = makeModuleGrid(rows = 5, cols = 5)
        grid = setModule(grid, row = 0, col = 1, dark = true)
        grid = setModule(grid, row = 1, col = 0, dark = true)
        val scene = layout(grid)
        val r1 = scene.instructions[1] as PaintInstruction.PaintRect
        val r2 = scene.instructions[2] as PaintInstruction.PaintRect
        // (0,1) → x=50, y=40; (1,0) → x=40, y=50
        // quietZonePx=40, so (0,1): x=40+1*10=50, y=40+0*10=40
        assertEquals(50, r1.x)
        assertEquals(40, r1.y)
        // (1,0): x=40+0*10=40, y=40+1*10=50
        assertEquals(40, r2.x)
        assertEquals(50, r2.y)
    }

    @Test
    fun `layoutSquare first module at row=0 col=0 has coordinates at quietZonePx`() {
        // x = y = quietZoneModules * moduleSizePx = 4 * 10 = 40
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 0, col = 0, dark = true)
        val scene = layout(g2)
        val rect = scene.instructions[1] as PaintInstruction.PaintRect
        assertEquals(40, rect.x)
        assertEquals(40, rect.y)
    }

    @Test
    fun `layoutSquare custom moduleSizePx affects dark module position`() {
        // row=1, col=2, moduleSizePx=5, quietZoneModules=4
        // x = 4*5 + 2*5 = 30
        // y = 4*5 + 1*5 = 25
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val g2 = setModule(grid, row = 1, col = 2, dark = true)
        val cfg = Barcode2DLayoutConfig(moduleSizePx = 5)
        val scene = layout(g2, cfg)
        val rect = scene.instructions[1] as PaintInstruction.PaintRect
        assertEquals(30, rect.x)
        assertEquals(25, rect.y)
    }

    // =========================================================================
    // layoutHex — canvas dimensions
    // =========================================================================

    @Test
    fun `layoutHex 33x30 grid with default config produces non-zero canvas`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(grid, cfg)
        assertTrue(scene.width > 0)
        assertTrue(scene.height > 0)
    }

    @Test
    fun `layoutHex canvas width accounts for odd-row offset`() {
        // hexWidth = 10
        // totalWidth = ((30 + 2*4) * 10 + 10/2.0).toInt() = (380 + 5).toInt() = 385
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(grid, cfg)
        val hexWidth = 10.0
        val expected = ((30 + 2 * 4) * hexWidth + hexWidth / 2.0).toInt()
        assertEquals(expected, scene.width)
    }

    @Test
    fun `layoutHex canvas height uses hexHeight row step`() {
        // hexHeight = moduleSizePx * (sqrt(3)/2) ≈ 8.66
        // totalHeight = ((33 + 2*4) * hexHeight).toInt()
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(grid, cfg)
        val hexHeight = 10.0 * (sqrt(3.0) / 2.0)
        val expected = ((33 + 2 * 4) * hexHeight).toInt()
        assertEquals(expected, scene.height)
    }

    // =========================================================================
    // layoutHex — instruction types and hex geometry
    // =========================================================================

    @Test
    fun `layoutHex all-light grid produces only background instruction`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(grid, cfg)
        assertEquals(1, scene.instructions.size)
        assertIs<PaintInstruction.PaintRect>(scene.instructions[0])
    }

    @Test
    fun `layoutHex single dark module produces PaintPath instruction`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        var g2 = setModule(grid, row = 0, col = 0, dark = true)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(g2, cfg)
        assertEquals(2, scene.instructions.size)
        assertIs<PaintInstruction.PaintPath>(scene.instructions[1])
    }

    @Test
    fun `layoutHex PaintPath has 7 commands for a hexagon`() {
        // A flat-top hexagon: MoveTo + 5 LineTo + ClosePath = 7 commands
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        var g2 = setModule(grid, row = 0, col = 0, dark = true)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(g2, cfg)
        val path = scene.instructions[1] as PaintInstruction.PaintPath
        assertEquals(7, path.commands.size)
    }

    @Test
    fun `layoutHex PaintPath starts with MoveTo`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        var g2 = setModule(grid, row = 0, col = 0, dark = true)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(g2, cfg)
        val path = scene.instructions[1] as PaintInstruction.PaintPath
        assertIs<PathCommand.MoveTo>(path.commands[0])
    }

    @Test
    fun `layoutHex PaintPath ends with ClosePath`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        var g2 = setModule(grid, row = 0, col = 0, dark = true)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(g2, cfg)
        val path = scene.instructions[1] as PaintInstruction.PaintPath
        assertIs<PathCommand.ClosePath>(path.commands[6])
    }

    @Test
    fun `layoutHex PaintPath commands 1-5 are LineTo`() {
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        var g2 = setModule(grid, row = 0, col = 0, dark = true)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene = layout(g2, cfg)
        val path = scene.instructions[1] as PaintInstruction.PaintPath
        for (i in 1..5) {
            assertIs<PathCommand.LineTo>(path.commands[i], "Command $i should be LineTo")
        }
    }

    @Test
    fun `layoutHex odd row is offset by half hexWidth`() {
        // For row=1 (odd), cx should include an extra hexWidth/2
        // row=1, col=0, moduleSizePx=10, quietZoneModules=4
        // hexWidth=10, hexHeight=10*sqrt(3)/2
        // cx(row=0,col=0) = 4*10 + 0*10 + 0   = 40
        // cx(row=1,col=0) = 4*10 + 0*10 + 10/2 = 45
        val grid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
        var g1 = setModule(grid, row = 0, col = 0, dark = true)
        var g2 = setModule(grid, row = 1, col = 0, dark = true)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val scene1 = layout(g1, cfg)
        val scene2 = layout(g2, cfg)

        val path1 = scene1.instructions[1] as PaintInstruction.PaintPath
        val path2 = scene2.instructions[1] as PaintInstruction.PaintPath

        // Check the x component of the first MoveTo vertex
        val move1 = path1.commands[0] as PathCommand.MoveTo
        val move2 = path2.commands[0] as PathCommand.MoveTo

        // Row 0 center x ≈ 40 + R*cos(0) = 40 + circumR
        // Row 1 center x ≈ 45 + R*cos(0) = 45 + circumR
        // So move2.x should be 5 more than move1.x
        val diff = move2.x - move1.x
        assertEquals(5.0, diff, 1e-9)
    }

    // =========================================================================
    // buildFlatTopHexPath — geometry helper
    // =========================================================================

    @Test
    fun `buildFlatTopHexPath returns 7 commands`() {
        val cmds = buildFlatTopHexPath(cx = 50.0, cy = 50.0, circumR = 10.0)
        assertEquals(7, cmds.size)
    }

    @Test
    fun `buildFlatTopHexPath first command is MoveTo`() {
        val cmds = buildFlatTopHexPath(cx = 50.0, cy = 50.0, circumR = 10.0)
        assertIs<PathCommand.MoveTo>(cmds[0])
    }

    @Test
    fun `buildFlatTopHexPath last command is ClosePath`() {
        val cmds = buildFlatTopHexPath(cx = 50.0, cy = 50.0, circumR = 10.0)
        assertIs<PathCommand.ClosePath>(cmds[6])
    }

    @Test
    fun `buildFlatTopHexPath vertex 0 is at angle 0 from center`() {
        val cx = 100.0
        val cy = 100.0
        val r = 20.0
        val cmds = buildFlatTopHexPath(cx, cy, r)
        val v0 = cmds[0] as PathCommand.MoveTo
        // angle=0: x=cx+r*cos(0)=cx+r, y=cy+r*sin(0)=cy
        assertEquals(cx + r, v0.x, 1e-9)
        assertEquals(cy, v0.y, 1e-9)
    }

    @Test
    fun `buildFlatTopHexPath vertex 1 is at angle 60 degrees`() {
        val cx = 0.0
        val cy = 0.0
        val r = 10.0
        val cmds = buildFlatTopHexPath(cx, cy, r)
        val v1 = cmds[1] as PathCommand.LineTo
        val angle = Math.PI / 3.0  // 60 degrees
        assertEquals(r * Math.cos(angle), v1.x, 1e-9)
        assertEquals(r * Math.sin(angle), v1.y, 1e-9)
    }

    @Test
    fun `buildFlatTopHexPath all vertices are at circumRadius from center`() {
        val cx = 50.0
        val cy = 50.0
        val r = 15.0
        val cmds = buildFlatTopHexPath(cx, cy, r)
        for (i in 0..5) {
            val (vx, vy) = when (val cmd = cmds[i]) {
                is PathCommand.MoveTo -> Pair(cmd.x, cmd.y)
                is PathCommand.LineTo -> Pair(cmd.x, cmd.y)
                else                  -> continue
            }
            val dist = Math.sqrt((vx - cx) * (vx - cx) + (vy - cy) * (vy - cy))
            assertEquals(r, dist, 1e-9, "Vertex $i should be at distance $r from center")
        }
    }

    // =========================================================================
    // ModuleRole enum
    // =========================================================================

    @Test
    fun `ModuleRole all values are distinct`() {
        val roles = ModuleRole.values()
        assertEquals(8, roles.size)
        assertEquals(8, roles.toSet().size)
    }

    @Test
    fun `ModuleRole when expression is exhaustive`() {
        val role = ModuleRole.DATA
        val name = when (role) {
            ModuleRole.FINDER    -> "finder"
            ModuleRole.SEPARATOR -> "separator"
            ModuleRole.TIMING    -> "timing"
            ModuleRole.ALIGNMENT -> "alignment"
            ModuleRole.FORMAT    -> "format"
            ModuleRole.DATA      -> "data"
            ModuleRole.ECC       -> "ecc"
            ModuleRole.PADDING   -> "padding"
        }
        assertEquals("data", name)
    }

    // =========================================================================
    // ModuleAnnotation
    // =========================================================================

    @Test
    fun `ModuleAnnotation stores role and dark flag`() {
        val ann = ModuleAnnotation(role = ModuleRole.DATA, dark = true)
        assertEquals(ModuleRole.DATA, ann.role)
        assertTrue(ann.dark)
    }

    @Test
    fun `ModuleAnnotation optional fields default to null`() {
        val ann = ModuleAnnotation(role = ModuleRole.ECC, dark = false)
        assertEquals(null, ann.codewordIndex)
        assertEquals(null, ann.bitIndex)
    }

    @Test
    fun `ModuleAnnotation stores codeword and bit indices`() {
        val ann = ModuleAnnotation(
            role = ModuleRole.DATA,
            dark = true,
            codewordIndex = 3,
            bitIndex = 7,
        )
        assertEquals(3, ann.codewordIndex)
        assertEquals(7, ann.bitIndex)
    }

    @Test
    fun `ModuleAnnotation metadata is empty by default`() {
        val ann = ModuleAnnotation(role = ModuleRole.FINDER, dark = true)
        assertEquals(emptyMap<String, String>(), ann.metadata)
    }

    @Test
    fun `ModuleAnnotation stores metadata`() {
        val ann = ModuleAnnotation(
            role = ModuleRole.FORMAT,
            dark = false,
            metadata = mapOf("qr:mask" to "3"),
        )
        assertEquals("3", ann.metadata["qr:mask"])
    }

    // =========================================================================
    // AnnotatedModuleGrid
    // =========================================================================

    @Test
    fun `AnnotatedModuleGrid stores grid and annotations`() {
        val grid = makeModuleGrid(rows = 2, cols = 2)
        val anns: List<List<ModuleAnnotation?>> = listOf(
            listOf(null, null),
            listOf(null, null),
        )
        val ag = AnnotatedModuleGrid(grid = grid, annotations = anns)
        assertEquals(grid, ag.grid)
        assertEquals(anns, ag.annotations)
    }

    @Test
    fun `AnnotatedModuleGrid annotation at 0_0 is accessible`() {
        val grid = makeModuleGrid(rows = 2, cols = 2)
        val ann = ModuleAnnotation(role = ModuleRole.FINDER, dark = true)
        val anns: List<List<ModuleAnnotation?>> = listOf(
            listOf(ann, null),
            listOf(null, null),
        )
        val ag = AnnotatedModuleGrid(grid = grid, annotations = anns)
        assertEquals(ann, ag.annotations[0][0])
    }

    // =========================================================================
    // Exception hierarchy
    // =========================================================================

    @Test
    fun `Barcode2DException extends Exception`() {
        val e = Barcode2DException("msg")
        assertTrue(e is Exception)
        assertEquals("msg", e.message)
    }

    @Test
    fun `InvalidBarcode2DConfigException message is preserved`() {
        val e = InvalidBarcode2DConfigException("moduleSizePx must be > 0")
        assertEquals("moduleSizePx must be > 0", e.message)
    }

    @Test
    fun `layout error message mentions moduleSizePx when moduleSizePx is 0`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val cfg = Barcode2DLayoutConfig(moduleSizePx = 0)
        val e = assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
        assertTrue(e.message!!.contains("moduleSizePx"))
    }

    @Test
    fun `layout error message mentions quietZoneModules when negative`() {
        val grid = makeModuleGrid(rows = 5, cols = 5)
        val cfg = Barcode2DLayoutConfig(quietZoneModules = -1)
        val e = assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
        assertTrue(e.message!!.contains("quietZoneModules"))
    }

    @Test
    fun `layout error message mentions moduleShape on mismatch`() {
        val grid = makeModuleGrid(rows = 5, cols = 5, moduleShape = ModuleShape.SQUARE)
        val cfg = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
        val e = assertThrows<InvalidBarcode2DConfigException> {
            layout(grid, cfg)
        }
        assertTrue(e.message!!.contains("moduleShape"))
    }
}
