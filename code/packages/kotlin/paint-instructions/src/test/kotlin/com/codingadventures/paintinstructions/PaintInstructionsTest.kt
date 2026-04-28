package com.codingadventures.paintinstructions

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

/**
 * Tests for the paint-instructions package.
 *
 * We aim for ≥ 90 % line coverage.  Every public API function and every sealed
 * class branch is exercised.  The tests are grouped by type and labelled with
 * their equivalence class so it's clear what each test proves.
 *
 * ## Testing philosophy
 *
 * A paint-instructions package is foundational: it has no dependencies of its
 * own and every higher-level package (barcode-2d, paint-vm backends, …) relies
 * on it.  Bugs here propagate everywhere.  So we test every function, every
 * default, every error path, and every sealed class variant.
 */
class PaintInstructionsTest {

    // =========================================================================
    // VERSION
    // =========================================================================

    @Test
    fun `VERSION is 0_1_0`() {
        assertEquals("0.1.0", VERSION)
    }

    // =========================================================================
    // PathCommand sealed hierarchy
    // =========================================================================

    @Test
    fun `PathCommand MoveTo stores x and y`() {
        val cmd = PathCommand.MoveTo(3.5, 7.25)
        assertEquals(3.5, cmd.x)
        assertEquals(7.25, cmd.y)
    }

    @Test
    fun `PathCommand LineTo stores x and y`() {
        val cmd = PathCommand.LineTo(100.0, 200.0)
        assertEquals(100.0, cmd.x)
        assertEquals(200.0, cmd.y)
    }

    @Test
    fun `PathCommand ClosePath is a singleton data object`() {
        // data object means every reference is the same object.
        val a = PathCommand.ClosePath
        val b = PathCommand.ClosePath
        assertEquals(a, b)
    }

    @Test
    fun `PathCommand MoveTo equality uses structural equality`() {
        // Two MoveTos with the same coordinates are equal.
        assertEquals(PathCommand.MoveTo(1.0, 2.0), PathCommand.MoveTo(1.0, 2.0))
    }

    @Test
    fun `PathCommand MoveTo inequality on different coordinates`() {
        assertNotEquals(PathCommand.MoveTo(1.0, 2.0), PathCommand.MoveTo(1.0, 3.0))
    }

    @Test
    fun `PathCommand when expression is exhaustive at compile time`() {
        // This test just exercises the when — if a new variant is added and
        // this when is not updated, the code won't compile.
        val cmds: List<PathCommand> = listOf(
            PathCommand.MoveTo(0.0, 0.0),
            PathCommand.LineTo(10.0, 0.0),
            PathCommand.ClosePath,
        )
        val kinds = cmds.map { cmd ->
            when (cmd) {
                is PathCommand.MoveTo -> "move"
                is PathCommand.LineTo -> "line"
                is PathCommand.ClosePath -> "close"
            }
        }
        assertEquals(listOf("move", "line", "close"), kinds)
    }

    // =========================================================================
    // PaintColorRGBA8
    // =========================================================================

    @Test
    fun `PaintColorRGBA8 stores all four channels`() {
        val c = PaintColorRGBA8(r = 10, g = 20, b = 30, a = 200)
        assertEquals(10, c.r)
        assertEquals(20, c.g)
        assertEquals(30, c.b)
        assertEquals(200, c.a)
    }

    @Test
    fun `PaintColorRGBA8 equality uses structural equality`() {
        val c1 = PaintColorRGBA8(255, 0, 0, 255)
        val c2 = PaintColorRGBA8(255, 0, 0, 255)
        assertEquals(c1, c2)
    }

    @Test
    fun `PaintColorRGBA8 inequality on different alpha`() {
        assertNotEquals(
            PaintColorRGBA8(255, 0, 0, 255),
            PaintColorRGBA8(255, 0, 0, 128),
        )
    }

    // =========================================================================
    // parseColorRGBA8
    // =========================================================================

    @Test
    fun `parseColorRGBA8 parses 3-digit rgb shorthand`() {
        // #f00 → red with full alpha
        val c = parseColorRGBA8("#f00")
        assertEquals(PaintColorRGBA8(0xff, 0x00, 0x00, 0xff), c)
    }

    @Test
    fun `parseColorRGBA8 parses 4-digit rgba shorthand`() {
        // #f008 → red with alpha 0x88 = 136
        val c = parseColorRGBA8("#f008")
        assertEquals(PaintColorRGBA8(0xff, 0x00, 0x00, 0x88), c)
    }

    @Test
    fun `parseColorRGBA8 parses 6-digit rrggbb with full alpha`() {
        val c = parseColorRGBA8("#ffffff")
        assertEquals(PaintColorRGBA8(255, 255, 255, 255), c)
    }

    @Test
    fun `parseColorRGBA8 parses 8-digit rrggbbaa`() {
        val c = parseColorRGBA8("#ff000080")
        assertEquals(PaintColorRGBA8(255, 0, 0, 128), c)
    }

    @Test
    fun `parseColorRGBA8 is case-insensitive`() {
        // Upper-case hex digits are valid CSS.
        assertEquals(parseColorRGBA8("#FFFFFF"), parseColorRGBA8("#ffffff"))
    }

    @Test
    fun `parseColorRGBA8 trims whitespace`() {
        assertEquals(parseColorRGBA8("#000000"), parseColorRGBA8("  #000000  "))
    }

    @Test
    fun `parseColorRGBA8 throws without leading hash`() {
        assertThrows<IllegalArgumentException> {
            parseColorRGBA8("000000")
        }
    }

    @Test
    fun `parseColorRGBA8 throws for 5-digit string`() {
        assertThrows<IllegalArgumentException> {
            parseColorRGBA8("#12345")
        }
    }

    @Test
    fun `parseColorRGBA8 parses black correctly`() {
        val c = parseColorRGBA8("#000")
        assertEquals(PaintColorRGBA8(0, 0, 0, 255), c)
    }

    @Test
    fun `parseColorRGBA8 parses fully transparent colour`() {
        val c = parseColorRGBA8("#00000000")
        assertEquals(PaintColorRGBA8(0, 0, 0, 0), c)
    }

    // =========================================================================
    // PaintInstruction sealed hierarchy
    // =========================================================================

    @Test
    fun `PaintInstruction PaintRect stores geometry and fill`() {
        val r = PaintInstruction.PaintRect(x = 5, y = 10, width = 20, height = 30, fill = "#ff0000")
        assertEquals(5, r.x)
        assertEquals(10, r.y)
        assertEquals(20, r.width)
        assertEquals(30, r.height)
        assertEquals("#ff0000", r.fill)
    }

    @Test
    fun `PaintInstruction PaintRect default metadata is emptyMap`() {
        val r = PaintInstruction.PaintRect(0, 0, 10, 10, "#000")
        assertEquals(emptyMap(), r.metadata)
    }

    @Test
    fun `PaintInstruction PaintRect stores custom metadata`() {
        val r = PaintInstruction.PaintRect(
            0, 0, 10, 10, "#000",
            metadata = mapOf("role" to "finder"),
        )
        assertEquals("finder", r.metadata["role"])
    }

    @Test
    fun `PaintInstruction PaintPath stores commands and fill`() {
        val cmds = listOf(
            PathCommand.MoveTo(0.0, 0.0),
            PathCommand.LineTo(10.0, 0.0),
            PathCommand.ClosePath,
        )
        val p = PaintInstruction.PaintPath(commands = cmds, fill = "#0000ff")
        assertEquals(cmds, p.commands)
        assertEquals("#0000ff", p.fill)
    }

    @Test
    fun `PaintInstruction PaintPath default metadata is emptyMap`() {
        val p = PaintInstruction.PaintPath(emptyList(), "#000")
        assertEquals(emptyMap(), p.metadata)
    }

    @Test
    fun `PaintInstruction when expression exhaustive`() {
        val instructions: List<PaintInstruction> = listOf(
            PaintInstruction.PaintRect(0, 0, 10, 10, "#000"),
            PaintInstruction.PaintPath(emptyList(), "#fff"),
        )
        val kinds = instructions.map { instr ->
            when (instr) {
                is PaintInstruction.PaintRect -> "rect"
                is PaintInstruction.PaintPath -> "path"
            }
        }
        assertEquals(listOf("rect", "path"), kinds)
    }

    // =========================================================================
    // paintRect helper
    // =========================================================================

    @Test
    fun `paintRect returns PaintRect with correct fields`() {
        val r = paintRect(x = 1, y = 2, width = 3, height = 4, fill = "#aabbcc")
        assertIs<PaintInstruction.PaintRect>(r)
        assertEquals(1, r.x)
        assertEquals(2, r.y)
        assertEquals(3, r.width)
        assertEquals(4, r.height)
        assertEquals("#aabbcc", r.fill)
    }

    @Test
    fun `paintRect defaults fill to black when blank`() {
        val r = paintRect(0, 0, 10, 10, fill = "")
        assertEquals("#000000", r.fill)
    }

    @Test
    fun `paintRect defaults fill to black when only whitespace`() {
        val r = paintRect(0, 0, 10, 10, fill = "   ")
        assertEquals("#000000", r.fill)
    }

    @Test
    fun `paintRect with all defaults`() {
        val r = paintRect(0, 0, 5, 5)
        assertEquals("#000000", r.fill)
        assertEquals(emptyMap(), r.metadata)
    }

    @Test
    fun `paintRect stores metadata`() {
        val r = paintRect(0, 0, 10, 10, metadata = mapOf("src" to "qr"))
        assertEquals("qr", r.metadata["src"])
    }

    // =========================================================================
    // paintPath helper
    // =========================================================================

    @Test
    fun `paintPath returns PaintPath with correct fields`() {
        val cmds = listOf(PathCommand.MoveTo(0.0, 0.0), PathCommand.ClosePath)
        val p = paintPath(commands = cmds, fill = "#123456")
        assertIs<PaintInstruction.PaintPath>(p)
        assertEquals(cmds, p.commands)
        assertEquals("#123456", p.fill)
    }

    @Test
    fun `paintPath defaults fill to black when blank`() {
        val p = paintPath(emptyList(), fill = "")
        assertEquals("#000000", p.fill)
    }

    @Test
    fun `paintPath stores metadata`() {
        val p = paintPath(emptyList(), metadata = mapOf("shape" to "hex"))
        assertEquals("hex", p.metadata["shape"])
    }

    // =========================================================================
    // createScene helper
    // =========================================================================

    @Test
    fun `createScene stores all fields`() {
        val instructions = listOf(paintRect(0, 0, 10, 10))
        val scene = createScene(
            width = 100,
            height = 200,
            background = "#aabbcc",
            instructions = instructions,
            metadata = mapOf("version" to "1"),
        )
        assertEquals(100, scene.width)
        assertEquals(200, scene.height)
        assertEquals("#aabbcc", scene.background)
        assertEquals(instructions, scene.instructions)
        assertEquals("1", scene.metadata["version"])
    }

    @Test
    fun `createScene defaults background to white when blank`() {
        val scene = createScene(100, 100, background = "")
        assertEquals("#ffffff", scene.background)
    }

    @Test
    fun `createScene defaults background to white when whitespace`() {
        val scene = createScene(100, 100, background = "   ")
        assertEquals("#ffffff", scene.background)
    }

    @Test
    fun `createScene defaults instructions to empty list`() {
        val scene = createScene(50, 50)
        assertEquals(emptyList(), scene.instructions)
    }

    @Test
    fun `createScene defaults metadata to empty map`() {
        val scene = createScene(50, 50)
        assertEquals(emptyMap(), scene.metadata)
    }

    // =========================================================================
    // PaintScene data class
    // =========================================================================

    @Test
    fun `PaintScene equality based on content`() {
        val s1 = PaintScene(100, 100, "#fff", emptyList())
        val s2 = PaintScene(100, 100, "#fff", emptyList())
        assertEquals(s1, s2)
    }

    @Test
    fun `PaintScene inequality when width differs`() {
        val s1 = PaintScene(100, 100, "#fff", emptyList())
        val s2 = PaintScene(200, 100, "#fff", emptyList())
        assertNotEquals(s1, s2)
    }

    @Test
    fun `PaintScene copy preserves unchanged fields`() {
        val s1 = PaintScene(100, 200, "#fff", emptyList())
        val s2 = s1.copy(width = 300)
        assertEquals(300, s2.width)
        assertEquals(200, s2.height)
        assertEquals("#fff", s2.background)
    }

    // =========================================================================
    // Integration: full scene with mixed instructions
    // =========================================================================

    @Test
    fun `full scene with rect and path instructions`() {
        // Simulate how a barcode layout engine builds a scene:
        // 1) A background rect covering the whole canvas.
        // 2) Dark square modules as PaintRects.
        // 3) A hex module as a PaintPath.

        val hexCommands = (0 until 6).map { i ->
            val angle = Math.toRadians(i * 60.0)
            val r = 6.0
            if (i == 0)
                PathCommand.MoveTo(50.0 + r * kotlin.math.cos(angle), 50.0 + r * kotlin.math.sin(angle))
            else
                PathCommand.LineTo(50.0 + r * kotlin.math.cos(angle), 50.0 + r * kotlin.math.sin(angle))
        } + listOf(PathCommand.ClosePath)

        val instructions = listOf(
            paintRect(0, 0, 210, 210, fill = "#ffffff"),
            paintRect(10, 10, 10, 10, fill = "#000000"),
            paintPath(hexCommands, fill = "#1a1a1a"),
        )

        val scene = createScene(width = 210, height = 210, instructions = instructions)

        assertEquals(210, scene.width)
        assertEquals(210, scene.height)
        assertEquals(3, scene.instructions.size)
        assertIs<PaintInstruction.PaintRect>(scene.instructions[0])
        assertIs<PaintInstruction.PaintRect>(scene.instructions[1])
        assertIs<PaintInstruction.PaintPath>(scene.instructions[2])

        val path = scene.instructions[2] as PaintInstruction.PaintPath
        // 6 vertex commands + 1 ClosePath = 7 commands
        assertEquals(7, path.commands.size)
        assertIs<PathCommand.MoveTo>(path.commands.first())
        assertIs<PathCommand.ClosePath>(path.commands.last())
    }

    @Test
    fun `scene instruction list is immutable under copy`() {
        val original = createScene(
            100, 100,
            instructions = listOf(paintRect(0, 0, 10, 10)),
        )
        // Kotlin data class copy is a shallow copy — the list reference itself
        // is copied but the list is immutable so there's no aliasing hazard.
        val modified = original.copy(width = 200)
        assertEquals(1, modified.instructions.size)
        assertEquals(1, original.instructions.size)
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    @Test
    fun `paintRect with zero dimensions is valid`() {
        // Some renderers may optimise away zero-size rects, but the model
        // itself must accept them.
        val r = paintRect(0, 0, 0, 0)
        assertEquals(0, r.width)
        assertEquals(0, r.height)
    }

    @Test
    fun `paintPath with empty command list is valid`() {
        // An empty path draws nothing, but should not throw at construction.
        val p = paintPath(emptyList())
        assertTrue(p.commands.isEmpty())
    }

    @Test
    fun `createScene with zero dimensions is valid`() {
        val scene = createScene(0, 0)
        assertEquals(0, scene.width)
        assertEquals(0, scene.height)
    }

    @Test
    fun `parseColorRGBA8 mixed case channel digits`() {
        // "aAbB" in mixed case should still parse.
        val c = parseColorRGBA8("#AaBb11FF")
        assertEquals(0xAA, c.r)
        assertEquals(0xBB, c.g)
        assertEquals(0x11, c.b)
        assertEquals(0xFF, c.a)
    }
}
