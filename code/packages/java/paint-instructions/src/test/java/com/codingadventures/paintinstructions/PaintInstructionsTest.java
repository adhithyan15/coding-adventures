package com.codingadventures.paintinstructions;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for the paint-instructions package.
 *
 * <p>Tests cover:
 * <ul>
 *   <li>{@link PathCommand} — all three variants (MoveTo, LineTo, ClosePath).</li>
 *   <li>{@link PaintInstruction.PaintRect} — construction, equality, defaults.</li>
 *   <li>{@link PaintInstruction.PaintPath} — construction, equality, command list.</li>
 *   <li>{@link PaintScene} — construction, equality, immutability.</li>
 *   <li>{@link PaintInstructions} builder helpers — paintRect, paintPath, createScene.</li>
 * </ul>
 *
 * <p>Cross-validated against the TypeScript and Kotlin reference implementations.
 */
class PaintInstructionsTest {

    // =========================================================================
    // Version
    // =========================================================================

    @Test
    @DisplayName("VERSION constant is present")
    void testVersion() {
        assertNotNull(PaintInstructions.VERSION);
        assertFalse(PaintInstructions.VERSION.isBlank());
        assertEquals("0.1.0", PaintInstructions.VERSION);
    }

    // =========================================================================
    // PathCommand.MoveTo
    // =========================================================================

    @Nested
    @DisplayName("PathCommand.MoveTo")
    class MoveToTests {

        @Test
        @DisplayName("stores x and y coordinates")
        void storesCoordinates() {
            var cmd = new PathCommand.MoveTo(10.5, 20.7);
            assertEquals(10.5, cmd.x, 1e-9);
            assertEquals(20.7, cmd.y, 1e-9);
        }

        @Test
        @DisplayName("toString includes kind and coordinates")
        void toStringFormat() {
            var cmd = new PathCommand.MoveTo(1.0, 2.0);
            assertTrue(cmd.toString().contains("MoveTo"));
            assertTrue(cmd.toString().contains("1.0"));
        }

        @Test
        @DisplayName("equality — same values equal")
        void equalitySameValues() {
            var a = new PathCommand.MoveTo(5.0, 10.0);
            var b = new PathCommand.MoveTo(5.0, 10.0);
            assertEquals(a, b);
            assertEquals(a.hashCode(), b.hashCode());
        }

        @Test
        @DisplayName("equality — different values not equal")
        void equalityDifferentValues() {
            var a = new PathCommand.MoveTo(5.0, 10.0);
            var b = new PathCommand.MoveTo(5.0, 11.0);
            assertNotEquals(a, b);
        }

        @Test
        @DisplayName("zero coordinates are valid")
        void zeroCoordinates() {
            var cmd = new PathCommand.MoveTo(0.0, 0.0);
            assertEquals(0.0, cmd.x, 1e-9);
            assertEquals(0.0, cmd.y, 1e-9);
        }

        @Test
        @DisplayName("negative coordinates are valid (path can start anywhere)")
        void negativeCoordinates() {
            var cmd = new PathCommand.MoveTo(-5.5, -3.2);
            assertEquals(-5.5, cmd.x, 1e-9);
            assertEquals(-3.2, cmd.y, 1e-9);
        }

        @Test
        @DisplayName("is instance of PathCommand (sealed hierarchy)")
        void isPathCommand() {
            PathCommand cmd = new PathCommand.MoveTo(1.0, 2.0);
            assertInstanceOf(PathCommand.MoveTo.class, cmd);
        }
    }

    // =========================================================================
    // PathCommand.LineTo
    // =========================================================================

    @Nested
    @DisplayName("PathCommand.LineTo")
    class LineToTests {

        @Test
        @DisplayName("stores x and y coordinates")
        void storesCoordinates() {
            var cmd = new PathCommand.LineTo(30.0, 40.5);
            assertEquals(30.0, cmd.x, 1e-9);
            assertEquals(40.5, cmd.y, 1e-9);
        }

        @Test
        @DisplayName("toString includes kind and coordinates")
        void toStringFormat() {
            var cmd = new PathCommand.LineTo(3.0, 4.0);
            assertTrue(cmd.toString().contains("LineTo"));
        }

        @Test
        @DisplayName("equality — same values equal")
        void equalitySameValues() {
            var a = new PathCommand.LineTo(3.0, 7.0);
            var b = new PathCommand.LineTo(3.0, 7.0);
            assertEquals(a, b);
            assertEquals(a.hashCode(), b.hashCode());
        }

        @Test
        @DisplayName("equality — different from MoveTo even with same coordinates")
        void notEqualToMoveTo() {
            PathCommand line = new PathCommand.LineTo(5.0, 5.0);
            PathCommand move = new PathCommand.MoveTo(5.0, 5.0);
            assertNotEquals(line, move);
        }

        @Test
        @DisplayName("is instance of PathCommand (sealed hierarchy)")
        void isPathCommand() {
            PathCommand cmd = new PathCommand.LineTo(1.0, 2.0);
            assertInstanceOf(PathCommand.LineTo.class, cmd);
        }
    }

    // =========================================================================
    // PathCommand.ClosePath
    // =========================================================================

    @Nested
    @DisplayName("PathCommand.ClosePath")
    class ClosePathTests {

        @Test
        @DisplayName("singleton INSTANCE is not null")
        void singletonIsNotNull() {
            assertNotNull(PathCommand.ClosePath.INSTANCE);
        }

        @Test
        @DisplayName("INSTANCE equals itself")
        void equalsItself() {
            assertEquals(PathCommand.ClosePath.INSTANCE, PathCommand.ClosePath.INSTANCE);
        }

        @Test
        @DisplayName("toString returns ClosePath")
        void toStringValue() {
            assertEquals("ClosePath", PathCommand.ClosePath.INSTANCE.toString());
        }

        @Test
        @DisplayName("is instance of PathCommand (sealed hierarchy)")
        void isPathCommand() {
            PathCommand cmd = PathCommand.ClosePath.INSTANCE;
            assertInstanceOf(PathCommand.ClosePath.class, cmd);
        }
    }

    // =========================================================================
    // PaintInstruction.PaintRect
    // =========================================================================

    @Nested
    @DisplayName("PaintInstruction.PaintRect")
    class PaintRectTests {

        @Test
        @DisplayName("stores all fields correctly")
        void storesFields() {
            var r = new PaintInstruction.PaintRect(10, 20, 100, 50, "#ff0000");
            assertEquals(10, r.x);
            assertEquals(20, r.y);
            assertEquals(100, r.width);
            assertEquals(50, r.height);
            assertEquals("#ff0000", r.fill);
        }

        @Test
        @DisplayName("metadata is stored and accessible")
        void storesMetadata() {
            var meta = Map.of("key", "value");
            var r = new PaintInstruction.PaintRect(0, 0, 10, 10, "#000", meta);
            assertEquals("value", r.metadata.get("key"));
        }

        @Test
        @DisplayName("metadata is immutable")
        void metadataImmutable() {
            var meta = new java.util.HashMap<String, String>();
            meta.put("k", "v");
            var r = new PaintInstruction.PaintRect(0, 0, 10, 10, "#000", meta);
            assertThrows(UnsupportedOperationException.class, () -> r.metadata.put("x", "y"));
        }

        @Test
        @DisplayName("null fill throws NullPointerException")
        void nullFillThrows() {
            assertThrows(NullPointerException.class,
                    () -> new PaintInstruction.PaintRect(0, 0, 10, 10, null));
        }

        @Test
        @DisplayName("equality — same fields equal")
        void equalitySameFields() {
            var a = new PaintInstruction.PaintRect(5, 5, 20, 20, "#abc");
            var b = new PaintInstruction.PaintRect(5, 5, 20, 20, "#abc");
            assertEquals(a, b);
            assertEquals(a.hashCode(), b.hashCode());
        }

        @Test
        @DisplayName("equality — different fill not equal")
        void equalityDifferentFill() {
            var a = new PaintInstruction.PaintRect(5, 5, 20, 20, "#000");
            var b = new PaintInstruction.PaintRect(5, 5, 20, 20, "#fff");
            assertNotEquals(a, b);
        }

        @Test
        @DisplayName("zero dimensions are valid (empty rect)")
        void zeroDimensions() {
            var r = new PaintInstruction.PaintRect(0, 0, 0, 0, "#000");
            assertEquals(0, r.width);
            assertEquals(0, r.height);
        }

        @Test
        @DisplayName("toString includes kind and coordinates")
        void toStringFormat() {
            var r = new PaintInstruction.PaintRect(1, 2, 3, 4, "#red");
            String s = r.toString();
            assertTrue(s.contains("PaintRect"));
            assertTrue(s.contains("1"));
        }

        @Test
        @DisplayName("is instance of PaintInstruction (sealed hierarchy)")
        void isPaintInstruction() {
            PaintInstruction instr = new PaintInstruction.PaintRect(0, 0, 10, 10, "#000");
            assertInstanceOf(PaintInstruction.PaintRect.class, instr);
        }
    }

    // =========================================================================
    // PaintInstruction.PaintPath
    // =========================================================================

    @Nested
    @DisplayName("PaintInstruction.PaintPath")
    class PaintPathTests {

        private List<PathCommand> triangle() {
            return List.of(
                    new PathCommand.MoveTo(0, 0),
                    new PathCommand.LineTo(10, 0),
                    new PathCommand.LineTo(5, 8.66),
                    PathCommand.ClosePath.INSTANCE
            );
        }

        @Test
        @DisplayName("stores commands and fill")
        void storesFields() {
            var p = new PaintInstruction.PaintPath(triangle(), "#ff0000");
            assertEquals(4, p.commands.size());
            assertEquals("#ff0000", p.fill);
        }

        @Test
        @DisplayName("commands list is immutable")
        void commandsImmutable() {
            var p = new PaintInstruction.PaintPath(triangle(), "#000");
            assertThrows(UnsupportedOperationException.class,
                    () -> p.commands.add(PathCommand.ClosePath.INSTANCE));
        }

        @Test
        @DisplayName("first command is MoveTo")
        void firstCommandIsMoveTo() {
            var p = new PaintInstruction.PaintPath(triangle(), "#000");
            assertInstanceOf(PathCommand.MoveTo.class, p.commands.get(0));
        }

        @Test
        @DisplayName("last command is ClosePath")
        void lastCommandIsClosePath() {
            var cmds = triangle();
            var p = new PaintInstruction.PaintPath(cmds, "#000");
            assertInstanceOf(PathCommand.ClosePath.class, p.commands.get(p.commands.size() - 1));
        }

        @Test
        @DisplayName("equality — same commands and fill equal")
        void equalitySameFields() {
            var a = new PaintInstruction.PaintPath(triangle(), "#aaa");
            var b = new PaintInstruction.PaintPath(triangle(), "#aaa");
            assertEquals(a, b);
            assertEquals(a.hashCode(), b.hashCode());
        }

        @Test
        @DisplayName("equality — different fill not equal")
        void equalityDifferentFill() {
            var a = new PaintInstruction.PaintPath(triangle(), "#000");
            var b = new PaintInstruction.PaintPath(triangle(), "#fff");
            assertNotEquals(a, b);
        }

        @Test
        @DisplayName("null commands throws NullPointerException")
        void nullCommandsThrows() {
            assertThrows(NullPointerException.class,
                    () -> new PaintInstruction.PaintPath(null, "#000"));
        }

        @Test
        @DisplayName("is instance of PaintInstruction (sealed hierarchy)")
        void isPaintInstruction() {
            PaintInstruction instr = new PaintInstruction.PaintPath(triangle(), "#000");
            assertInstanceOf(PaintInstruction.PaintPath.class, instr);
        }

        @Test
        @DisplayName("toString mentions PaintPath and command count")
        void toStringFormat() {
            var p = new PaintInstruction.PaintPath(triangle(), "#000");
            String s = p.toString();
            assertTrue(s.contains("PaintPath"));
        }
    }

    // =========================================================================
    // PaintScene
    // =========================================================================

    @Nested
    @DisplayName("PaintScene")
    class PaintSceneTests {

        @Test
        @DisplayName("stores all fields correctly")
        void storesFields() {
            var instructions = List.of(
                    (PaintInstruction) new PaintInstruction.PaintRect(0, 0, 210, 210, "#fff")
            );
            var scene = new PaintScene(210, 210, "#ffffff", instructions);
            assertEquals(210, scene.width);
            assertEquals(210, scene.height);
            assertEquals("#ffffff", scene.background);
            assertEquals(1, scene.instructions.size());
        }

        @Test
        @DisplayName("instructions list is immutable")
        void instructionsImmutable() {
            var scene = new PaintScene(100, 100, "#fff", List.of());
            assertThrows(UnsupportedOperationException.class,
                    () -> scene.instructions.add(new PaintInstruction.PaintRect(0, 0, 10, 10, "#000")));
        }

        @Test
        @DisplayName("empty instructions list is valid")
        void emptyInstructions() {
            var scene = new PaintScene(50, 50, "#000", List.of());
            assertTrue(scene.instructions.isEmpty());
        }

        @Test
        @DisplayName("equality — same fields equal")
        void equalitySameFields() {
            var instructions = List.<PaintInstruction>of(
                    new PaintInstruction.PaintRect(0, 0, 10, 10, "#000")
            );
            var a = new PaintScene(10, 10, "#fff", instructions);
            var b = new PaintScene(10, 10, "#fff", instructions);
            assertEquals(a, b);
            assertEquals(a.hashCode(), b.hashCode());
        }

        @Test
        @DisplayName("equality — different background not equal")
        void equalityDifferentBackground() {
            var a = new PaintScene(10, 10, "#fff", List.of());
            var b = new PaintScene(10, 10, "#000", List.of());
            assertNotEquals(a, b);
        }

        @Test
        @DisplayName("null background throws NullPointerException")
        void nullBackgroundThrows() {
            assertThrows(NullPointerException.class,
                    () -> new PaintScene(10, 10, null, List.of()));
        }

        @Test
        @DisplayName("metadata is stored and immutable")
        void metadataImmutable() {
            var meta = new java.util.HashMap<String, String>();
            meta.put("k", "v");
            var scene = new PaintScene(10, 10, "#fff", List.of(), meta);
            assertEquals("v", scene.metadata.get("k"));
            assertThrows(UnsupportedOperationException.class, () -> scene.metadata.put("x", "y"));
        }

        @Test
        @DisplayName("toString includes dimensions and background")
        void toStringFormat() {
            var scene = new PaintScene(300, 200, "#aabbcc", List.of());
            String s = scene.toString();
            assertTrue(s.contains("PaintScene"));
            assertTrue(s.contains("300"));
            assertTrue(s.contains("200"));
        }
    }

    // =========================================================================
    // PaintInstructions builder helpers
    // =========================================================================

    @Nested
    @DisplayName("PaintInstructions builder helpers")
    class BuilderHelpersTests {

        @Test
        @DisplayName("paintRect sets defaults when fill is null")
        void paintRectNullFillDefaultsToBlack() {
            var r = PaintInstructions.paintRect(0, 0, 10, 10, null);
            assertEquals("#000000", r.fill);
        }

        @Test
        @DisplayName("paintRect sets defaults when fill is blank")
        void paintRectBlankFillDefaultsToBlack() {
            var r = PaintInstructions.paintRect(0, 0, 10, 10, "  ");
            assertEquals("#000000", r.fill);
        }

        @Test
        @DisplayName("paintRect preserves provided fill")
        void paintRectPreservesFill() {
            var r = PaintInstructions.paintRect(5, 5, 20, 20, "#ff0000");
            assertEquals("#ff0000", r.fill);
            assertEquals(5, r.x);
            assertEquals(5, r.y);
            assertEquals(20, r.width);
            assertEquals(20, r.height);
        }

        @Test
        @DisplayName("paintPath sets defaults when fill is null")
        void paintPathNullFillDefaultsToBlack() {
            var commands = List.of(
                    new PathCommand.MoveTo(0, 0),
                    PathCommand.ClosePath.INSTANCE
            );
            var p = PaintInstructions.paintPath(commands, null);
            assertEquals("#000000", p.fill);
        }

        @Test
        @DisplayName("paintPath preserves provided fill")
        void paintPathPreservesFill() {
            var commands = List.of(
                    new PathCommand.MoveTo(0, 0),
                    PathCommand.ClosePath.INSTANCE
            );
            var p = PaintInstructions.paintPath(commands, "#abcdef");
            assertEquals("#abcdef", p.fill);
        }

        @Test
        @DisplayName("createScene sets defaults when background is null")
        void createSceneNullBackgroundDefaultsToWhite() {
            var scene = PaintInstructions.createScene(100, 100, null, List.of());
            assertEquals("#ffffff", scene.background);
        }

        @Test
        @DisplayName("createScene sets defaults when background is blank")
        void createSceneBlankBackgroundDefaultsToWhite() {
            var scene = PaintInstructions.createScene(100, 100, "  ", List.of());
            assertEquals("#ffffff", scene.background);
        }

        @Test
        @DisplayName("createScene preserves provided background")
        void createScenePreservesBackground() {
            var scene = PaintInstructions.createScene(200, 100, "#112233", List.of());
            assertEquals("#112233", scene.background);
            assertEquals(200, scene.width);
            assertEquals(100, scene.height);
        }

        @Test
        @DisplayName("createScene with metadata stores annotations")
        void createSceneWithMetadata() {
            var meta = Map.of("source", "test");
            var scene = PaintInstructions.createScene(10, 10, "#fff", List.of(), meta);
            assertEquals("test", scene.metadata.get("source"));
        }

        @Test
        @DisplayName("createScene with null instructions uses empty list")
        void createSceneNullInstructionsUsesEmpty() {
            var scene = PaintInstructions.createScene(10, 10, "#fff", null);
            assertNotNull(scene.instructions);
            assertTrue(scene.instructions.isEmpty());
        }

        @Test
        @DisplayName("round-trip: paintRect → PaintScene → check instruction")
        void roundTripPaintRect() {
            var rect = PaintInstructions.paintRect(10, 20, 30, 40, "#123456");
            var scene = PaintInstructions.createScene(100, 100, "#ffffff",
                    List.of(rect));
            assertEquals(1, scene.instructions.size());
            var instr = scene.instructions.get(0);
            assertInstanceOf(PaintInstruction.PaintRect.class, instr);
            var r = (PaintInstruction.PaintRect) instr;
            assertEquals(10, r.x);
            assertEquals(20, r.y);
            assertEquals(30, r.width);
            assertEquals(40, r.height);
            assertEquals("#123456", r.fill);
        }

        @Test
        @DisplayName("round-trip: paintPath (hex) → PaintScene → check commands")
        void roundTripPaintPath() {
            var cmds = List.of(
                    new PathCommand.MoveTo(0, 0),
                    new PathCommand.LineTo(10, 0),
                    new PathCommand.LineTo(5, 8.66),
                    PathCommand.ClosePath.INSTANCE
            );
            var path = PaintInstructions.paintPath(cmds, "#ff0000");
            var scene = PaintInstructions.createScene(50, 50, "#ffffff", List.of(path));
            assertEquals(1, scene.instructions.size());
            var instr = scene.instructions.get(0);
            assertInstanceOf(PaintInstruction.PaintPath.class, instr);
            var p = (PaintInstruction.PaintPath) instr;
            assertEquals(4, p.commands.size());
            assertInstanceOf(PathCommand.MoveTo.class, p.commands.get(0));
            assertInstanceOf(PathCommand.ClosePath.class, p.commands.get(3));
        }
    }

    // =========================================================================
    // Sealed class pattern-matching
    // =========================================================================

    @Nested
    @DisplayName("Sealed class dispatch")
    class SealedClassDispatchTests {

        /**
         * Demonstrate dispatching over PaintInstruction variants.
         * A backend renderer would use a similar pattern.
         */
        private String dispatch(PaintInstruction instr) {
            if (instr instanceof PaintInstruction.PaintRect r) {
                return "rect:" + r.x + "," + r.y;
            } else if (instr instanceof PaintInstruction.PaintPath p) {
                return "path:" + p.commands.size();
            } else {
                throw new AssertionError("Unknown instruction type");
            }
        }

        @Test
        @DisplayName("dispatch works for PaintRect")
        void dispatchRect() {
            var r = new PaintInstruction.PaintRect(5, 10, 20, 20, "#000");
            assertEquals("rect:5,10", dispatch(r));
        }

        @Test
        @DisplayName("dispatch works for PaintPath")
        void dispatchPath() {
            var cmds = List.of(
                    new PathCommand.MoveTo(0, 0),
                    PathCommand.ClosePath.INSTANCE
            );
            var p = new PaintInstruction.PaintPath(cmds, "#000");
            assertEquals("path:2", dispatch(p));
        }

        /**
         * Demonstrate dispatching over PathCommand variants.
         * A path renderer would convert each command to backend-specific calls.
         */
        private String dispatchCmd(PathCommand cmd) {
            if (cmd instanceof PathCommand.MoveTo m) {
                return "M " + m.x + " " + m.y;
            } else if (cmd instanceof PathCommand.LineTo l) {
                return "L " + l.x + " " + l.y;
            } else if (cmd instanceof PathCommand.ClosePath) {
                return "Z";
            } else {
                throw new AssertionError("Unknown command type");
            }
        }

        @Test
        @DisplayName("PathCommand dispatch produces SVG-like strings")
        void pathCommandDispatch() {
            assertEquals("M 0.0 0.0", dispatchCmd(new PathCommand.MoveTo(0, 0)));
            assertEquals("L 10.0 5.0", dispatchCmd(new PathCommand.LineTo(10, 5)));
            assertEquals("Z", dispatchCmd(PathCommand.ClosePath.INSTANCE));
        }
    }

    // =========================================================================
    // Multi-instruction scene
    // =========================================================================

    @Test
    @DisplayName("scene with multiple instructions preserves order")
    void scenePreservesInstructionOrder() {
        var instructions = List.of(
                (PaintInstruction) new PaintInstruction.PaintRect(0, 0, 210, 210, "#ffffff"),
                new PaintInstruction.PaintRect(40, 40, 10, 10, "#000000"),
                new PaintInstruction.PaintRect(50, 40, 10, 10, "#000000")
        );
        var scene = new PaintScene(210, 210, "#ffffff", instructions);
        assertEquals(3, scene.instructions.size());
        // Background is first
        var bg = (PaintInstruction.PaintRect) scene.instructions.get(0);
        assertEquals("#ffffff", bg.fill);
        assertEquals(210, bg.width);
    }

    @Test
    @DisplayName("scene with mixed rect and path instructions")
    void sceneMixedInstructions() {
        var cmds = List.of(
                new PathCommand.MoveTo(0, 0),
                new PathCommand.LineTo(10, 0),
                PathCommand.ClosePath.INSTANCE
        );
        var instructions = List.of(
                (PaintInstruction) new PaintInstruction.PaintRect(0, 0, 100, 100, "#fff"),
                new PaintInstruction.PaintPath(cmds, "#000")
        );
        var scene = new PaintScene(100, 100, "#fff", instructions);
        assertEquals(2, scene.instructions.size());
        assertInstanceOf(PaintInstruction.PaintRect.class, scene.instructions.get(0));
        assertInstanceOf(PaintInstruction.PaintPath.class, scene.instructions.get(1));
    }
}
