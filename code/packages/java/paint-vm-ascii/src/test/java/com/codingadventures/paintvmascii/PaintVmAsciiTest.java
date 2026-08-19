package com.codingadventures.paintvmascii;

import com.codingadventures.paintinstructions.PaintGlyphPlacement;
import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintInstructions;
import com.codingadventures.paintinstructions.PaintScene;
import com.codingadventures.paintinstructions.Transform2D;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.*;

class PaintVmAsciiTest {

    private static PaintScene withInstructions(int width, int height, String background, List<PaintInstruction> instructions) {
        return new PaintScene(width, height, background, instructions);
    }

    private static String okText(PaintVmAsciiResult result) {
        assertInstanceOf(PaintVmAsciiResult.Ok.class, result, "expected Ok, got " + result);
        return ((PaintVmAsciiResult.Ok) result).text();
    }

    private static PaintVmAsciiError errorOf(PaintVmAsciiResult result) {
        assertInstanceOf(PaintVmAsciiResult.Err.class, result, "expected Err, got " + result);
        return ((PaintVmAsciiResult.Err) result).error();
    }

    @Nested
    @DisplayName("metadata")
    class MetadataTests {
        @Test
        @DisplayName("reports the shared package version")
        void version() {
            assertEquals("0.1.0", PaintVmAscii.VERSION);
        }

        @Test
        @DisplayName("default options use the shared 8x16 cell scale")
        void defaultOptions() {
            assertEquals(8, AsciiOptions.DEFAULT.scaleX);
            assertEquals(16, AsciiOptions.DEFAULT.scaleY);
        }
    }

    @Nested
    @DisplayName("render — rect")
    class RectRenderTests {
        @Test
        @DisplayName("renders a filled rectangle inclusively")
        void filledRectangleInclusive() {
            var scene = withInstructions(4, 3, "#ffffff",
                    List.of(PaintInstructions.paintRect(0, 0, 2, 1, "#000000")));
            var result = PaintVmAscii.render(scene, new AsciiOptions(1, 1));
            assertEquals("███\n███", okText(result));
        }

        @Test
        @DisplayName("uses painter order while clipping rectangles to the buffer")
        void painterOrderClipped() {
            var scene = withInstructions(3, 2, "transparent", List.of(
                    PaintInstructions.paintRect(-2, -2, 3, 3, "red"),
                    PaintInstructions.paintRect(2, 1, 4, 4, "blue")));
            var result = PaintVmAscii.render(scene, new AsciiOptions(1, 1));
            assertEquals("██\n███", okText(result));
        }

        @Test
        @DisplayName("skips empty and transparent fills after trimming whitespace")
        void skipsInvisibleFills() {
            // Uses the raw constructor, not the PaintInstructions.paintRect
            // builder — that builder defaults a blank fill to black, which
            // would defeat the point of this test.
            var scene = withInstructions(3, 2, "transparent", List.of(
                    new PaintInstruction.PaintRect(0, 0, 2, 1, ""),
                    new PaintInstruction.PaintRect(0, 0, 2, 1, " transparent "),
                    new PaintInstruction.PaintRect(0, 0, 2, 1, "none")));
            var result = PaintVmAscii.render(scene, new AsciiOptions(1, 1));
            assertEquals("", okText(result));
        }

        @Test
        @DisplayName("maps coordinates through the default scale")
        void mapsCoordinatesThroughScale() {
            var scene = withInstructions(16, 32, "transparent",
                    List.of(PaintInstructions.paintRect(8, 16, 0, 0, "black")));
            var result = PaintVmAscii.renderDefault(scene);
            assertEquals("\n █", okText(result));
        }

        @Test
        @DisplayName("renders a zero-sized scene as empty text")
        void zeroSizedScene() {
            var scene = withInstructions(0, 0, "transparent", List.of());
            assertEquals("", okText(PaintVmAscii.renderDefault(scene)));
        }

        @Test
        @DisplayName("rejects paths instead of returning an incomplete rendering")
        void rejectsPaths() {
            var scene = withInstructions(10, 10, "transparent",
                    List.of(PaintInstructions.paintPath(List.of(), "black")));
            var error = errorOf(PaintVmAscii.renderDefault(scene));
            assertInstanceOf(PaintVmAsciiError.UnsupportedInstruction.class, error);
            assertEquals("path", ((PaintVmAsciiError.UnsupportedInstruction) error).reason());
        }

        @Test
        @DisplayName("rejects non-positive horizontal scale")
        void rejectsNonPositiveScaleX() {
            var scene = withInstructions(1, 1, "transparent", List.of());
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(0, 1)));
            assertEquals(new PaintVmAsciiError.InvalidScaleX(0), error);
        }

        @Test
        @DisplayName("rejects non-positive vertical scale")
        void rejectsNonPositiveScaleY() {
            var scene = withInstructions(1, 1, "transparent", List.of());
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(1, -1)));
            assertEquals(new PaintVmAsciiError.InvalidScaleY(-1), error);
        }

        @Test
        @DisplayName("rejects negative scene dimensions")
        void rejectsNegativeSceneDimensions() {
            var scene = withInstructions(-1, 1, "transparent", List.of());
            var error = errorOf(PaintVmAscii.renderDefault(scene));
            assertEquals(new PaintVmAsciiError.InvalidSceneDimensions(-1, 1), error);
        }

        @Test
        @DisplayName("rejects invalid rectangle geometry")
        void rejectsInvalidRectangleGeometry() {
            var scene = withInstructions(2, 2, "transparent",
                    List.of(PaintInstructions.paintRect(0, 0, -1, 1, "black")));
            var error = errorOf(PaintVmAscii.renderDefault(scene));
            assertEquals(new PaintVmAsciiError.InvalidRectangleGeometry(0, 0, -1, 1), error);
        }

        @Test
        @DisplayName("rejects an enormous scene instead of hanging")
        void rejectsEnormousScene() {
            var scene = withInstructions(1_000_000_000, 1_000_000_000, "transparent", List.of());
            var error = errorOf(PaintVmAscii.render(scene, AsciiOptions.DEFAULT));
            assertInstanceOf(PaintVmAsciiError.SceneTooLarge.class, error);
        }

        @Test
        @DisplayName("rejects a zero-width, huge-height scene instead of hanging (product-only check bypass)")
        void rejectsZeroWidthHugeHeight() {
            var scene = withInstructions(0, 1_000_000_000, "transparent", List.of());
            var error = errorOf(PaintVmAscii.render(scene, AsciiOptions.DEFAULT));
            assertInstanceOf(PaintVmAsciiError.SceneTooLarge.class, error);
        }

        @Test
        @DisplayName("rejects a huge-width, zero-height scene instead of hanging (product-only check bypass)")
        void rejectsHugeWidthZeroHeight() {
            var scene = withInstructions(1_000_000_000, 0, "transparent", List.of());
            var error = errorOf(PaintVmAscii.render(scene, AsciiOptions.DEFAULT));
            assertInstanceOf(PaintVmAsciiError.SceneTooLarge.class, error);
        }
    }

    @Nested
    @DisplayName("stroked rect")
    class StrokedRectTests {
        @Test
        @DisplayName("draws box-drawing corners and edges")
        void boxDrawing() {
            var rect = PaintInstructions.paintRect(0, 0, 16, 16, "", "#000000", 1);
            var scene = withInstructions(24, 32, "transparent", List.of(rect));
            var result = PaintVmAscii.render(scene, new AsciiOptions(8, 16));
            assertEquals("┌─┐\n└─┘", okText(result));
        }

        @Test
        @DisplayName("clamps an enormous rectangle to the clip bounds instead of hanging")
        void clampsEnormousRectangle() {
            var scene = withInstructions(8, 8, "transparent",
                    List.of(PaintInstructions.paintRect(0, 0, Integer.MAX_VALUE, Integer.MAX_VALUE, "#000000")));
            assertEquals("█", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 8))));
        }
    }

    @Nested
    @DisplayName("glyph_run")
    class GlyphRunTests {
        @Test
        @DisplayName("places literal characters at their scene positions")
        void placesLiteralCharacters() {
            var run = PaintInstructions.paintGlyphRun(
                    List.of(new PaintGlyphPlacement('h', 0, 0), new PaintGlyphPlacement('i', 8, 0)),
                    "terminal-mono", 16, "#000000");
            var scene = withInstructions(16, 16, "transparent", List.of(run));
            assertEquals("hi", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("maps unsafe control code points to a placeholder")
        void mapsControlCodePoints() {
            var run = PaintInstructions.paintGlyphRun(
                    List.of(new PaintGlyphPlacement(0x07, 0, 0)), "terminal-mono", 16, "#000000");
            var scene = withInstructions(16, 16, "transparent", List.of(run));
            assertEquals("?", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("maps a UTF-16 surrogate code point to a placeholder")
        void mapsSurrogateCodePoint() {
            var run = PaintInstructions.paintGlyphRun(
                    List.of(new PaintGlyphPlacement(0xDC80, 0, 0)), "terminal-mono", 16, "#000000");
            var scene = withInstructions(16, 16, "transparent", List.of(run));
            assertEquals("?", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("maps a supplementary-plane code point (needs a surrogate pair) to a placeholder")
        void mapsSupplementaryPlaneCodePoint() {
            var run = PaintInstructions.paintGlyphRun(
                    List.of(new PaintGlyphPlacement(0x1F600, 0, 0)), "terminal-mono", 16, "#000000");
            var scene = withInstructions(16, 16, "transparent", List.of(run));
            assertEquals("?", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("skips a glyph with a non-finite position instead of failing the render")
        void skipsNonFinitePosition() {
            var run = PaintInstructions.paintGlyphRun(List.of(
                    new PaintGlyphPlacement('h', Double.POSITIVE_INFINITY, 0),
                    new PaintGlyphPlacement('i', 8, 0)), "terminal-mono", 16, "#000000");
            var scene = withInstructions(16, 16, "transparent", List.of(run));
            assertEquals(" i", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }
    }

    @Nested
    @DisplayName("line")
    class LineTests {
        @Test
        @DisplayName("draws a horizontal box-drawing run")
        void horizontal() {
            var line = PaintInstructions.paintLine(0, 0, 24, 0, "#000000", 1);
            var scene = withInstructions(32, 16, "transparent", List.of(line));
            assertEquals("────", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("draws a vertical box-drawing run")
        void vertical() {
            var line = PaintInstructions.paintLine(0, 0, 0, 32, "#000000", 1);
            var scene = withInstructions(8, 48, "transparent", List.of(line));
            assertEquals("│\n│\n│", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("rejects a line with a non-finite coordinate")
        void rejectsNonFiniteCoordinate() {
            var line = PaintInstructions.paintLine(Double.POSITIVE_INFINITY, 0, 8, 8, "#000000", 1);
            var scene = withInstructions(8, 8, "transparent", List.of(line));
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(8, 8)));
            assertEquals(new PaintVmAsciiError.InvalidLineGeometry(Double.POSITIVE_INFINITY, 0, 8, 8), error);
        }

        @Test
        @DisplayName("clamps an enormous diagonal line to the clip bounds instead of hanging")
        void clampsEnormousDiagonalLine() {
            var line = PaintInstructions.paintLine(0, 0, 1.0e12, 1.0e12, "#000000", 1);
            var scene = withInstructions(8, 8, "transparent", List.of(line));
            var text = okText(PaintVmAscii.render(scene, new AsciiOptions(8, 8)));
            assertTrue(text.length() <= 3, "expected a bounded render, got: " + text);
        }
    }

    @Nested
    @DisplayName("group")
    class GroupTests {
        @Test
        @DisplayName("recurses into its children")
        void recursesIntoChildren() {
            var group = PaintInstructions.paintGroup(
                    List.of(PaintInstructions.paintRect(0, 0, 8, 16, "#000000")));
            var scene = withInstructions(16, 16, "transparent", List.of(group));
            assertEquals("██", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("rejects a group with a non-identity transform")
        void rejectsNonIdentityTransform() {
            var group = new PaintInstruction.PaintGroup(
                    List.of(), Optional.of(new Transform2D(2, 0, 0, 1, 0, 0)), Optional.empty(), Map.of());
            var scene = withInstructions(16, 16, "transparent", List.of(group));
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(8, 16)));
            assertEquals(
                    new PaintVmAsciiError.UnsupportedInstruction("group with a non-identity transform"), error);
        }

        @Test
        @DisplayName("rejects a group with non-default opacity")
        void rejectsNonDefaultOpacity() {
            var group = new PaintInstruction.PaintGroup(
                    List.of(), Optional.empty(), Optional.of(0.5), Map.of());
            var scene = withInstructions(16, 16, "transparent", List.of(group));
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(8, 16)));
            assertEquals(new PaintVmAsciiError.UnsupportedInstruction("group with non-default opacity"), error);
        }
    }

    @Nested
    @DisplayName("clip")
    class ClipTests {
        @Test
        @DisplayName("drops children outside the clip rectangle")
        void dropsOutsideChildren() {
            var run = PaintInstructions.paintGlyphRun(List.of(
                    new PaintGlyphPlacement('a', 0, 0),
                    new PaintGlyphPlacement('b', 8, 0)), "terminal-mono", 16, "#000000");
            var clip = PaintInstructions.paintClip(0, 0, 8, 16, List.of(run));
            var scene = withInstructions(16, 16, "transparent", List.of(clip));
            assertEquals("a", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("rejects a clip with a non-finite coordinate")
        void rejectsNonFiniteCoordinate() {
            var clip = PaintInstructions.paintClip(Double.POSITIVE_INFINITY, 0, 8, 16, List.of());
            var scene = withInstructions(16, 16, "transparent", List.of(clip));
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(8, 16)));
            assertEquals(
                    new PaintVmAsciiError.InvalidClipGeometry(Double.POSITIVE_INFINITY, 0, 8, 16), error);
        }

        @Test
        @DisplayName("rejects a clip whose individually-finite x+width overflows to infinity")
        void rejectsSumOverflow() {
            double hugeX = 1.7e308;
            double hugeW = 1.0e308;
            var clip = PaintInstructions.paintClip(hugeX, 0, hugeW, 16, List.of());
            var scene = withInstructions(16, 16, "transparent", List.of(clip));
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(8, 16)));
            assertEquals(new PaintVmAsciiError.InvalidClipGeometry(hugeX, 0, hugeW, 16), error);
        }

        @Test
        @DisplayName("does not let a large clip extent unclamp a nested rect's fill range")
        void largeClipExtentStaysClamped() {
            var rect = PaintInstructions.paintRect(0, 0, Integer.MAX_VALUE, 16, "#000000");
            var clip = PaintInstructions.paintClip(0, 0, 6.6461399789245786e35, 16, List.of(rect));
            var scene = withInstructions(800, 16, "transparent", List.of(clip));
            var text = okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16)));
            assertTrue(text.length() <= 100, "expected a bounded render, got length " + text.length());
        }
    }

    @Nested
    @DisplayName("layer")
    class LayerTests {
        @Test
        @DisplayName("recurses into its children when plain")
        void recursesWhenPlain() {
            var layer = PaintInstructions.paintLayer(
                    List.of(PaintInstructions.paintRect(0, 0, 8, 16, "#000000")));
            var scene = withInstructions(16, 16, "transparent", List.of(layer));
            assertEquals("██", okText(PaintVmAscii.render(scene, new AsciiOptions(8, 16))));
        }

        @Test
        @DisplayName("rejects a layer with filters")
        void rejectsFilters() {
            var layer = new PaintInstruction.PaintLayer(
                    List.of(), true, Optional.empty(), Optional.empty(), Optional.empty(), Map.of());
            var scene = withInstructions(16, 16, "transparent", List.of(layer));
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(8, 16)));
            assertEquals(new PaintVmAsciiError.UnsupportedInstruction("layer with filters"), error);
        }

        @Test
        @DisplayName("rejects a layer with a non-normal blend mode")
        void rejectsNonNormalBlendMode() {
            var layer = new PaintInstruction.PaintLayer(
                    List.of(), false, Optional.of("multiply"), Optional.empty(), Optional.empty(), Map.of());
            var scene = withInstructions(16, 16, "transparent", List.of(layer));
            var error = errorOf(PaintVmAscii.render(scene, new AsciiOptions(8, 16)));
            assertEquals(
                    new PaintVmAsciiError.UnsupportedInstruction("layer with a non-normal blend mode"), error);
        }
    }
}
