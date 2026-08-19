package com.codingadventures.paintvmascii

import com.codingadventures.paintinstructions.PaintGlyphPlacement
import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintinstructions.PaintScene
import com.codingadventures.paintinstructions.Transform2D
import com.codingadventures.paintinstructions.paintClip
import com.codingadventures.paintinstructions.paintGlyphRun
import com.codingadventures.paintinstructions.paintGroup
import com.codingadventures.paintinstructions.paintLayer
import com.codingadventures.paintinstructions.paintLine
import com.codingadventures.paintinstructions.paintPath
import com.codingadventures.paintinstructions.paintRect
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class PaintVmAsciiTest {

    private fun scene(width: Int, height: Int, background: String, instructions: List<PaintInstruction>): PaintScene =
        PaintScene(width, height, background, instructions)

    private fun okText(result: PaintVmAsciiResult): String {
        val ok = assertIs<PaintVmAsciiResult.Ok>(result)
        return ok.text
    }

    private fun errorOf(result: PaintVmAsciiResult): PaintVmAsciiError {
        val err = assertIs<PaintVmAsciiResult.Err>(result)
        return err.error
    }

    // -------------------------------------------------------------------
    // metadata
    // -------------------------------------------------------------------

    @Test
    fun `reports the shared package version`() {
        assertEquals("0.1.0", VERSION)
    }

    @Test
    fun `default options use the shared 8x16 cell scale`() {
        assertEquals(8, AsciiOptions.DEFAULT.scaleX)
        assertEquals(16, AsciiOptions.DEFAULT.scaleY)
    }

    // -------------------------------------------------------------------
    // render - rect
    // -------------------------------------------------------------------

    @Test
    fun `renders a filled rectangle inclusively`() {
        val s = scene(4, 3, "#ffffff", listOf(paintRect(0, 0, 2, 1, "#000000")))
        assertEquals("███\n███", okText(render(s, AsciiOptions(1, 1))))
    }

    @Test
    fun `uses painter order while clipping rectangles to the buffer`() {
        val s = scene(3, 2, "transparent", listOf(paintRect(-2, -2, 3, 3, "red"), paintRect(2, 1, 4, 4, "blue")))
        assertEquals("██\n███", okText(render(s, AsciiOptions(1, 1))))
    }

    @Test
    fun `skips empty and transparent fills after trimming whitespace`() {
        // Uses the raw constructor, not the paintRect() builder -- that
        // builder defaults a blank fill to black, which would defeat the
        // point of this test.
        val s = scene(
            3, 2, "transparent",
            listOf(
                PaintInstruction.PaintRect(0, 0, 2, 1, ""),
                PaintInstruction.PaintRect(0, 0, 2, 1, " transparent "),
                PaintInstruction.PaintRect(0, 0, 2, 1, "none"),
            ),
        )
        assertEquals("", okText(render(s, AsciiOptions(1, 1))))
    }

    @Test
    fun `maps coordinates through the default scale`() {
        val s = scene(16, 32, "transparent", listOf(paintRect(8, 16, 0, 0, "black")))
        assertEquals("\n █", okText(renderDefault(s)))
    }

    @Test
    fun `renders a zero-sized scene as empty text`() {
        assertEquals("", okText(renderDefault(scene(0, 0, "transparent", emptyList()))))
    }

    @Test
    fun `rejects paths instead of returning an incomplete rendering`() {
        val s = scene(10, 10, "transparent", listOf(paintPath(emptyList(), "black")))
        val error = errorOf(renderDefault(s))
        val unsupported = assertIs<PaintVmAsciiError.UnsupportedInstruction>(error)
        assertEquals("path", unsupported.reason)
    }

    @Test
    fun `rejects non-positive horizontal scale`() {
        val s = scene(1, 1, "transparent", emptyList())
        assertEquals(PaintVmAsciiError.InvalidScaleX(0), errorOf(render(s, AsciiOptions(0, 1))))
    }

    @Test
    fun `rejects non-positive vertical scale`() {
        val s = scene(1, 1, "transparent", emptyList())
        assertEquals(PaintVmAsciiError.InvalidScaleY(-1), errorOf(render(s, AsciiOptions(1, -1))))
    }

    @Test
    fun `rejects negative scene dimensions`() {
        val s = scene(-1, 1, "transparent", emptyList())
        assertEquals(PaintVmAsciiError.InvalidSceneDimensions(-1, 1), errorOf(renderDefault(s)))
    }

    @Test
    fun `rejects invalid rectangle geometry`() {
        val s = scene(2, 2, "transparent", listOf(paintRect(0, 0, -1, 1, "black")))
        assertEquals(PaintVmAsciiError.InvalidRectangleGeometry(0, 0, -1, 1), errorOf(renderDefault(s)))
    }

    @Test
    fun `rejects an enormous scene instead of hanging`() {
        val s = scene(1_000_000_000, 1_000_000_000, "transparent", emptyList())
        assertIs<PaintVmAsciiError.SceneTooLarge>(errorOf(render(s, AsciiOptions.DEFAULT)))
    }

    @Test
    fun `rejects a zero-width huge-height scene instead of hanging (product-only check bypass)`() {
        val s = scene(0, 1_000_000_000, "transparent", emptyList())
        assertIs<PaintVmAsciiError.SceneTooLarge>(errorOf(render(s, AsciiOptions.DEFAULT)))
    }

    @Test
    fun `rejects a huge-width zero-height scene instead of hanging (product-only check bypass)`() {
        val s = scene(1_000_000_000, 0, "transparent", emptyList())
        assertIs<PaintVmAsciiError.SceneTooLarge>(errorOf(render(s, AsciiOptions.DEFAULT)))
    }

    // -------------------------------------------------------------------
    // stroked rect
    // -------------------------------------------------------------------

    @Test
    fun `draws box-drawing corners and edges`() {
        // Raw constructor, not the paintRect() builder -- that builder
        // defaults a blank fill to black, which would defeat the point of
        // this stroke-only test.
        val rect = PaintInstruction.PaintRect(0, 0, 16, 16, "", stroke = "#000000", strokeWidth = 1.0)
        val s = scene(24, 32, "transparent", listOf(rect))
        assertEquals("┌─┐\n└─┘", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `clamps an enormous rectangle to the clip bounds instead of hanging`() {
        val s = scene(8, 8, "transparent", listOf(paintRect(0, 0, Int.MAX_VALUE, Int.MAX_VALUE, "#000000")))
        assertEquals("█", okText(render(s, AsciiOptions(8, 8))))
    }

    // -------------------------------------------------------------------
    // glyph_run
    // -------------------------------------------------------------------

    @Test
    fun `places literal characters at their scene positions`() {
        val run = paintGlyphRun(
            listOf(PaintGlyphPlacement('h'.code, 0.0, 0.0), PaintGlyphPlacement('i'.code, 8.0, 0.0)),
            "terminal-mono", 16.0, "#000000",
        )
        val s = scene(16, 16, "transparent", listOf(run))
        assertEquals("hi", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `maps unsafe control code points to a placeholder`() {
        val run = paintGlyphRun(listOf(PaintGlyphPlacement(0x07, 0.0, 0.0)), "terminal-mono", 16.0, "#000000")
        val s = scene(16, 16, "transparent", listOf(run))
        assertEquals("?", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `maps a UTF-16 surrogate code point to a placeholder`() {
        val run = paintGlyphRun(listOf(PaintGlyphPlacement(0xDC80, 0.0, 0.0)), "terminal-mono", 16.0, "#000000")
        val s = scene(16, 16, "transparent", listOf(run))
        assertEquals("?", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `maps a supplementary-plane code point to a placeholder`() {
        val run = paintGlyphRun(listOf(PaintGlyphPlacement(0x1F600, 0.0, 0.0)), "terminal-mono", 16.0, "#000000")
        val s = scene(16, 16, "transparent", listOf(run))
        assertEquals("?", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `skips a glyph with a non-finite position instead of failing the render`() {
        val run = paintGlyphRun(
            listOf(PaintGlyphPlacement('h'.code, Double.POSITIVE_INFINITY, 0.0), PaintGlyphPlacement('i'.code, 8.0, 0.0)),
            "terminal-mono", 16.0, "#000000",
        )
        val s = scene(16, 16, "transparent", listOf(run))
        assertEquals(" i", okText(render(s, AsciiOptions(8, 16))))
    }

    // -------------------------------------------------------------------
    // line
    // -------------------------------------------------------------------

    @Test
    fun `draws a horizontal box-drawing run`() {
        val s = scene(32, 16, "transparent", listOf(paintLine(0.0, 0.0, 24.0, 0.0, "#000000", 1.0)))
        assertEquals("────", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `draws a vertical box-drawing run`() {
        val s = scene(8, 48, "transparent", listOf(paintLine(0.0, 0.0, 0.0, 32.0, "#000000", 1.0)))
        assertEquals("│\n│\n│", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `rejects a line with a non-finite coordinate`() {
        val s = scene(8, 8, "transparent", listOf(paintLine(Double.POSITIVE_INFINITY, 0.0, 8.0, 8.0, "#000000", 1.0)))
        assertEquals(
            PaintVmAsciiError.InvalidLineGeometry(Double.POSITIVE_INFINITY, 0.0, 8.0, 8.0),
            errorOf(render(s, AsciiOptions(8, 8))),
        )
    }

    @Test
    fun `clamps an enormous diagonal line to the clip bounds instead of hanging`() {
        val s = scene(8, 8, "transparent", listOf(paintLine(0.0, 0.0, 1.0e12, 1.0e12, "#000000", 1.0)))
        val text = okText(render(s, AsciiOptions(8, 8)))
        assertTrue(text.length <= 3, "expected a bounded render, got: $text")
    }

    // -------------------------------------------------------------------
    // group
    // -------------------------------------------------------------------

    @Test
    fun `recurses into group children`() {
        val s = scene(16, 16, "transparent", listOf(paintGroup(listOf(paintRect(0, 0, 8, 16, "#000000")))))
        assertEquals("██", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `rejects a group with a non-identity transform`() {
        val group = PaintInstruction.PaintGroup(emptyList(), transform = Transform2D(2.0, 0.0, 0.0, 1.0, 0.0, 0.0))
        val s = scene(16, 16, "transparent", listOf(group))
        assertEquals(
            PaintVmAsciiError.UnsupportedInstruction("group with a non-identity transform"),
            errorOf(render(s, AsciiOptions(8, 16))),
        )
    }

    @Test
    fun `rejects a group with non-default opacity`() {
        val group = PaintInstruction.PaintGroup(emptyList(), opacity = 0.5)
        val s = scene(16, 16, "transparent", listOf(group))
        assertEquals(
            PaintVmAsciiError.UnsupportedInstruction("group with non-default opacity"),
            errorOf(render(s, AsciiOptions(8, 16))),
        )
    }

    // -------------------------------------------------------------------
    // clip
    // -------------------------------------------------------------------

    @Test
    fun `drops children outside the clip rectangle`() {
        val run = paintGlyphRun(
            listOf(PaintGlyphPlacement('a'.code, 0.0, 0.0), PaintGlyphPlacement('b'.code, 8.0, 0.0)),
            "terminal-mono", 16.0, "#000000",
        )
        val s = scene(16, 16, "transparent", listOf(paintClip(0.0, 0.0, 8.0, 16.0, listOf(run))))
        assertEquals("a", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `rejects a clip with a non-finite coordinate`() {
        val s = scene(16, 16, "transparent", listOf(paintClip(Double.POSITIVE_INFINITY, 0.0, 8.0, 16.0, emptyList())))
        assertEquals(
            PaintVmAsciiError.InvalidClipGeometry(Double.POSITIVE_INFINITY, 0.0, 8.0, 16.0),
            errorOf(render(s, AsciiOptions(8, 16))),
        )
    }

    @Test
    fun `rejects a clip whose individually-finite x+width overflows to infinity`() {
        val hugeX = 1.7e308
        val hugeW = 1.0e308
        val s = scene(16, 16, "transparent", listOf(paintClip(hugeX, 0.0, hugeW, 16.0, emptyList())))
        assertEquals(PaintVmAsciiError.InvalidClipGeometry(hugeX, 0.0, hugeW, 16.0), errorOf(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `does not let a large clip extent unclamp a nested rect's fill range`() {
        val rect = paintRect(0, 0, Int.MAX_VALUE, 16, "#000000")
        val clip = paintClip(0.0, 0.0, 6.6461399789245786e35, 16.0, listOf(rect))
        val s = scene(800, 16, "transparent", listOf(clip))
        val text = okText(render(s, AsciiOptions(8, 16)))
        assertTrue(text.length <= 100, "expected a bounded render, got length ${text.length}")
    }

    // -------------------------------------------------------------------
    // layer
    // -------------------------------------------------------------------

    @Test
    fun `recurses into layer children when plain`() {
        val s = scene(16, 16, "transparent", listOf(paintLayer(listOf(paintRect(0, 0, 8, 16, "#000000")))))
        assertEquals("██", okText(render(s, AsciiOptions(8, 16))))
    }

    @Test
    fun `rejects a layer with filters`() {
        val layer = PaintInstruction.PaintLayer(emptyList(), hasFilters = true)
        val s = scene(16, 16, "transparent", listOf(layer))
        assertEquals(
            PaintVmAsciiError.UnsupportedInstruction("layer with filters"),
            errorOf(render(s, AsciiOptions(8, 16))),
        )
    }

    @Test
    fun `rejects a layer with a non-normal blend mode`() {
        val layer = PaintInstruction.PaintLayer(emptyList(), blendMode = "multiply")
        val s = scene(16, 16, "transparent", listOf(layer))
        assertEquals(
            PaintVmAsciiError.UnsupportedInstruction("layer with a non-normal blend mode"),
            errorOf(render(s, AsciiOptions(8, 16))),
        )
    }
}
