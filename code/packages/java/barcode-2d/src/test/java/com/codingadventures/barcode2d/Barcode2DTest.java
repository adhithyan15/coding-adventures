package com.codingadventures.barcode2d;

import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintScene;
import com.codingadventures.paintinstructions.PathCommand;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for the barcode-2d package.
 *
 * <p>Tests cover:
 * <ul>
 *   <li>{@link ModuleShape} — SQUARE and HEX enum values.</li>
 *   <li>{@link ModuleGrid} — construction, immutability, toString.</li>
 *   <li>{@link Barcode2D#makeModuleGrid} — all-light grid creation.</li>
 *   <li>{@link Barcode2D#setModule} — immutable single-module update, bounds checking.</li>
 *   <li>{@link Barcode2DLayoutConfig} — defaults, builder, equality.</li>
 *   <li>{@link Barcode2D#layout} — validation, square layout, hex layout.</li>
 *   <li>{@link Barcode2D#buildFlatTopHexPath} — hex geometry.</li>
 * </ul>
 *
 * <p>Cross-validated against the TypeScript and Kotlin reference implementations.
 */
class Barcode2DTest {

    // =========================================================================
    // Version
    // =========================================================================

    @Test
    @DisplayName("VERSION constant is present")
    void testVersion() {
        assertNotNull(Barcode2D.VERSION);
        assertFalse(Barcode2D.VERSION.isBlank());
        assertEquals("0.1.0", Barcode2D.VERSION);
    }

    // =========================================================================
    // ModuleShape
    // =========================================================================

    @Nested
    @DisplayName("ModuleShape")
    class ModuleShapeTests {

        @Test
        @DisplayName("SQUARE and HEX are the only values")
        void onlyTwoValues() {
            assertEquals(2, ModuleShape.values().length);
        }

        @Test
        @DisplayName("SQUARE name is correct")
        void squareName() {
            assertEquals("SQUARE", ModuleShape.SQUARE.name());
        }

        @Test
        @DisplayName("HEX name is correct")
        void hexName() {
            assertEquals("HEX", ModuleShape.HEX.name());
        }
    }

    // =========================================================================
    // ModuleGrid construction
    // =========================================================================

    @Nested
    @DisplayName("ModuleGrid construction")
    class ModuleGridConstructionTests {

        @Test
        @DisplayName("stores rows and cols correctly")
        void storesRowsCols() {
            var grid = Barcode2D.makeModuleGrid(5, 7);
            assertEquals(5, grid.rows);
            assertEquals(7, grid.cols);
        }

        @Test
        @DisplayName("default moduleShape is SQUARE")
        void defaultModuleShapeIsSquare() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            assertEquals(ModuleShape.SQUARE, grid.moduleShape);
        }

        @Test
        @DisplayName("can create HEX grid")
        void hexGrid() {
            var grid = Barcode2D.makeModuleGrid(33, 30, ModuleShape.HEX);
            assertEquals(ModuleShape.HEX, grid.moduleShape);
            assertEquals(33, grid.rows);
            assertEquals(30, grid.cols);
        }

        @Test
        @DisplayName("all modules are false (light) on creation")
        void allModulesLight() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            for (int r = 0; r < 3; r++) {
                for (int c = 0; c < 3; c++) {
                    assertFalse(grid.modules.get(r).get(c),
                            "module at (" + r + "," + c + ") should be light");
                }
            }
        }

        @Test
        @DisplayName("modules list is immutable")
        void modulesImmutable() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            assertThrows(UnsupportedOperationException.class,
                    () -> grid.modules.add(List.of()));
        }

        @Test
        @DisplayName("inner row list is immutable")
        void innerRowImmutable() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            assertThrows(UnsupportedOperationException.class,
                    () -> grid.modules.get(0).set(0, true));
        }

        @Test
        @DisplayName("null moduleShape throws NullPointerException")
        void nullModuleShapeThrows() {
            var modules = List.of(List.of(false));
            assertThrows(NullPointerException.class,
                    () -> new ModuleGrid(1, 1, modules, null));
        }

        @Test
        @DisplayName("equality — same grid equals")
        void equalityEqual() {
            var g1 = Barcode2D.makeModuleGrid(3, 3);
            var g2 = Barcode2D.makeModuleGrid(3, 3);
            assertEquals(g1, g2);
            assertEquals(g1.hashCode(), g2.hashCode());
        }

        @Test
        @DisplayName("equality — different size not equal")
        void equalityDifferentSize() {
            var g1 = Barcode2D.makeModuleGrid(3, 3);
            var g2 = Barcode2D.makeModuleGrid(4, 4);
            assertNotEquals(g1, g2);
        }

        @Test
        @DisplayName("toString includes rows and cols")
        void toStringFormat() {
            var grid = Barcode2D.makeModuleGrid(5, 7);
            String s = grid.toString();
            assertTrue(s.contains("5"));
            assertTrue(s.contains("7"));
        }
    }

    // =========================================================================
    // setModule
    // =========================================================================

    @Nested
    @DisplayName("setModule")
    class SetModuleTests {

        @Test
        @DisplayName("returns new grid with module set to true")
        void setsDarkModule() {
            var g = Barcode2D.makeModuleGrid(3, 3);
            var g2 = Barcode2D.setModule(g, 1, 1, true);
            assertTrue(g2.modules.get(1).get(1));
        }

        @Test
        @DisplayName("original grid is not modified")
        void originalUnchanged() {
            var g = Barcode2D.makeModuleGrid(3, 3);
            var g2 = Barcode2D.setModule(g, 1, 1, true);
            assertFalse(g.modules.get(1).get(1), "original should still be false");
            assertTrue(g2.modules.get(1).get(1), "new grid should be true");
        }

        @Test
        @DisplayName("sets module at top-left corner")
        void topLeftCorner() {
            var g = Barcode2D.makeModuleGrid(5, 5);
            var g2 = Barcode2D.setModule(g, 0, 0, true);
            assertTrue(g2.modules.get(0).get(0));
        }

        @Test
        @DisplayName("sets module at bottom-right corner")
        void bottomRightCorner() {
            var g = Barcode2D.makeModuleGrid(5, 5);
            var g2 = Barcode2D.setModule(g, 4, 4, true);
            assertTrue(g2.modules.get(4).get(4));
        }

        @Test
        @DisplayName("can set module back to false (light)")
        void setLightModule() {
            var g = Barcode2D.makeModuleGrid(3, 3);
            var g2 = Barcode2D.setModule(g, 1, 1, true);
            var g3 = Barcode2D.setModule(g2, 1, 1, false);
            assertFalse(g3.modules.get(1).get(1));
        }

        @Test
        @DisplayName("preserves moduleShape")
        void preservesModuleShape() {
            var g = Barcode2D.makeModuleGrid(3, 3, ModuleShape.HEX);
            var g2 = Barcode2D.setModule(g, 0, 0, true);
            assertEquals(ModuleShape.HEX, g2.moduleShape);
        }

        @Test
        @DisplayName("row out of bounds throws IndexOutOfBoundsException")
        void rowOutOfBounds() {
            var g = Barcode2D.makeModuleGrid(3, 3);
            assertThrows(IndexOutOfBoundsException.class,
                    () -> Barcode2D.setModule(g, 3, 0, true));
            assertThrows(IndexOutOfBoundsException.class,
                    () -> Barcode2D.setModule(g, -1, 0, true));
        }

        @Test
        @DisplayName("col out of bounds throws IndexOutOfBoundsException")
        void colOutOfBounds() {
            var g = Barcode2D.makeModuleGrid(3, 3);
            assertThrows(IndexOutOfBoundsException.class,
                    () -> Barcode2D.setModule(g, 0, 3, true));
            assertThrows(IndexOutOfBoundsException.class,
                    () -> Barcode2D.setModule(g, 0, -1, true));
        }

        @Test
        @DisplayName("null grid throws NullPointerException")
        void nullGridThrows() {
            assertThrows(NullPointerException.class,
                    () -> Barcode2D.setModule(null, 0, 0, true));
        }

        @Test
        @DisplayName("multiple setModule calls chain correctly")
        void chaining() {
            var g = Barcode2D.makeModuleGrid(3, 3);
            var g2 = Barcode2D.setModule(g, 0, 0, true);
            var g3 = Barcode2D.setModule(g2, 1, 1, true);
            var g4 = Barcode2D.setModule(g3, 2, 2, true);

            assertTrue(g4.modules.get(0).get(0));
            assertTrue(g4.modules.get(1).get(1));
            assertTrue(g4.modules.get(2).get(2));
            assertFalse(g4.modules.get(0).get(1));
        }
    }

    // =========================================================================
    // Barcode2DLayoutConfig
    // =========================================================================

    @Nested
    @DisplayName("Barcode2DLayoutConfig")
    class LayoutConfigTests {

        @Test
        @DisplayName("defaults() returns expected values")
        void defaultValues() {
            var config = Barcode2DLayoutConfig.defaults();
            assertEquals(10, config.moduleSizePx);
            assertEquals(4, config.quietZoneModules);
            assertEquals("#000000", config.foreground);
            assertEquals("#ffffff", config.background);
            assertEquals(ModuleShape.SQUARE, config.moduleShape);
        }

        @Test
        @DisplayName("Builder overrides moduleSizePx")
        void builderModuleSizePx() {
            var config = new Barcode2DLayoutConfig.Builder().moduleSizePx(5).build();
            assertEquals(5, config.moduleSizePx);
            // Other fields stay at defaults
            assertEquals(4, config.quietZoneModules);
        }

        @Test
        @DisplayName("Builder overrides quietZoneModules")
        void builderQuietZone() {
            var config = new Barcode2DLayoutConfig.Builder().quietZoneModules(1).build();
            assertEquals(1, config.quietZoneModules);
        }

        @Test
        @DisplayName("Builder overrides foreground")
        void builderForeground() {
            var config = new Barcode2DLayoutConfig.Builder().foreground("#112233").build();
            assertEquals("#112233", config.foreground);
        }

        @Test
        @DisplayName("Builder overrides background")
        void builderBackground() {
            var config = new Barcode2DLayoutConfig.Builder().background("#aabbcc").build();
            assertEquals("#aabbcc", config.background);
        }

        @Test
        @DisplayName("Builder overrides moduleShape to HEX")
        void builderModuleShapeHex() {
            var config = new Barcode2DLayoutConfig.Builder().moduleShape(ModuleShape.HEX).build();
            assertEquals(ModuleShape.HEX, config.moduleShape);
        }

        @Test
        @DisplayName("equality — same values equal")
        void equalityEqual() {
            var a = Barcode2DLayoutConfig.defaults();
            var b = Barcode2DLayoutConfig.defaults();
            assertEquals(a, b);
            assertEquals(a.hashCode(), b.hashCode());
        }

        @Test
        @DisplayName("equality — different moduleSizePx not equal")
        void equalityDifferent() {
            var a = Barcode2DLayoutConfig.defaults();
            var b = new Barcode2DLayoutConfig.Builder().moduleSizePx(5).build();
            assertNotEquals(a, b);
        }

        @Test
        @DisplayName("null foreground in Builder throws NullPointerException")
        void nullForegroundThrows() {
            assertThrows(NullPointerException.class,
                    () -> new Barcode2DLayoutConfig.Builder().foreground(null));
        }

        @Test
        @DisplayName("toString includes moduleSizePx")
        void toStringFormat() {
            var config = Barcode2DLayoutConfig.defaults();
            assertTrue(config.toString().contains("10"));
            assertTrue(config.toString().contains("SQUARE"));
        }
    }

    // =========================================================================
    // layout() — validation
    // =========================================================================

    @Nested
    @DisplayName("layout() validation")
    class LayoutValidationTests {

        @Test
        @DisplayName("throws on moduleSizePx <= 0")
        void throwsOnZeroModuleSize() {
            var grid = Barcode2D.makeModuleGrid(5, 5);
            var config = new Barcode2DLayoutConfig.Builder().moduleSizePx(0).build();
            assertThrows(InvalidBarcode2DConfigException.class,
                    () -> Barcode2D.layout(grid, config));
        }

        @Test
        @DisplayName("throws on negative moduleSizePx")
        void throwsOnNegativeModuleSize() {
            var grid = Barcode2D.makeModuleGrid(5, 5);
            var config = new Barcode2DLayoutConfig.Builder().moduleSizePx(-1).build();
            assertThrows(InvalidBarcode2DConfigException.class,
                    () -> Barcode2D.layout(grid, config));
        }

        @Test
        @DisplayName("throws on negative quietZoneModules")
        void throwsOnNegativeQuietZone() {
            var grid = Barcode2D.makeModuleGrid(5, 5);
            var config = new Barcode2DLayoutConfig.Builder().quietZoneModules(-1).build();
            assertThrows(InvalidBarcode2DConfigException.class,
                    () -> Barcode2D.layout(grid, config));
        }

        @Test
        @DisplayName("throws when config.moduleShape != grid.moduleShape")
        void throwsOnShapeMismatch() {
            var grid = Barcode2D.makeModuleGrid(5, 5, ModuleShape.SQUARE);
            var config = new Barcode2DLayoutConfig.Builder().moduleShape(ModuleShape.HEX).build();
            assertThrows(InvalidBarcode2DConfigException.class,
                    () -> Barcode2D.layout(grid, config));
        }

        @Test
        @DisplayName("zero quietZone is valid")
        void zeroQuietZoneIsValid() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            var config = new Barcode2DLayoutConfig.Builder().quietZoneModules(0).build();
            assertDoesNotThrow(() -> Barcode2D.layout(grid, config));
        }

        @Test
        @DisplayName("null grid throws NullPointerException")
        void nullGridThrows() {
            assertThrows(NullPointerException.class,
                    () -> Barcode2D.layout(null, Barcode2DLayoutConfig.defaults()));
        }

        @Test
        @DisplayName("null config throws NullPointerException")
        void nullConfigThrows() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            assertThrows(NullPointerException.class,
                    () -> Barcode2D.layout(grid, null));
        }
    }

    // =========================================================================
    // layout() — square modules
    // =========================================================================

    @Nested
    @DisplayName("layout() square modules")
    class LayoutSquareTests {

        @Test
        @DisplayName("all-light grid produces background rect only")
        void allLightProducesBackgroundOnly() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleSizePx(10).quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            // Only the background rect — no dark modules.
            assertEquals(1, scene.instructions.size());
            assertInstanceOf(PaintInstruction.PaintRect.class, scene.instructions.get(0));
        }

        @Test
        @DisplayName("total canvas size includes quiet zone on all sides")
        void canvasSize() {
            var grid = Barcode2D.makeModuleGrid(21, 21);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleSizePx(10).quietZoneModules(4).build();
            var scene = Barcode2D.layout(grid, config);

            // totalWidth  = (21 + 2*4) * 10 = 29 * 10 = 290
            // totalHeight = (21 + 2*4) * 10 = 290
            assertEquals(290, scene.width);
            assertEquals(290, scene.height);
        }

        @Test
        @DisplayName("single dark module produces background + 1 rect")
        void singleDarkModule() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            grid = Barcode2D.setModule(grid, 1, 1, true);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleSizePx(10).quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            // background + 1 dark module
            assertEquals(2, scene.instructions.size());
        }

        @Test
        @DisplayName("dark module position is correctly computed")
        void darkModulePosition() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            grid = Barcode2D.setModule(grid, 1, 2, true);  // row=1, col=2
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleSizePx(10).quietZoneModules(4).build();
            var scene = Barcode2D.layout(grid, config);

            // Expected position of dark module rect:
            // quietZonePx = 4 * 10 = 40
            // x = 40 + 2 * 10 = 60
            // y = 40 + 1 * 10 = 50
            var darkRect = (PaintInstruction.PaintRect) scene.instructions.get(1);
            assertEquals(60, darkRect.x);
            assertEquals(50, darkRect.y);
            assertEquals(10, darkRect.width);
            assertEquals(10, darkRect.height);
        }

        @Test
        @DisplayName("background fill matches config.background")
        void backgroundFill() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            var config = new Barcode2DLayoutConfig.Builder()
                    .background("#ffeecc").build();
            var scene = Barcode2D.layout(grid, config);

            assertEquals("#ffeecc", scene.background);
            var bgRect = (PaintInstruction.PaintRect) scene.instructions.get(0);
            assertEquals("#ffeecc", bgRect.fill);
        }

        @Test
        @DisplayName("foreground fill matches config.foreground")
        void foregroundFill() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            grid = Barcode2D.setModule(grid, 0, 0, true);
            var config = new Barcode2DLayoutConfig.Builder()
                    .foreground("#112233").quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            var darkRect = (PaintInstruction.PaintRect) scene.instructions.get(1);
            assertEquals("#112233", darkRect.fill);
        }

        @Test
        @DisplayName("default layout() uses default config")
        void defaultLayoutUsesDefaults() {
            var grid = Barcode2D.makeModuleGrid(5, 5);
            var scene = Barcode2D.layout(grid);
            // (5 + 2*4) * 10 = 130
            assertEquals(130, scene.width);
            assertEquals(130, scene.height);
            assertEquals("#ffffff", scene.background);
        }

        @Test
        @DisplayName("multiple dark modules — instruction count matches")
        void multipleDarkModules() {
            var grid = Barcode2D.makeModuleGrid(5, 5);
            grid = Barcode2D.setModule(grid, 0, 0, true);
            grid = Barcode2D.setModule(grid, 1, 1, true);
            grid = Barcode2D.setModule(grid, 2, 2, true);
            var config = new Barcode2DLayoutConfig.Builder().quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            // background + 3 dark rects
            assertEquals(4, scene.instructions.size());
        }

        @Test
        @DisplayName("scene instruction order: background first, dark modules after")
        void instructionOrder() {
            var grid = Barcode2D.makeModuleGrid(3, 3);
            grid = Barcode2D.setModule(grid, 0, 0, true);
            var config = new Barcode2DLayoutConfig.Builder()
                    .background("#ffffff").foreground("#000000")
                    .quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            // First instruction is background (white)
            var bg = (PaintInstruction.PaintRect) scene.instructions.get(0);
            assertEquals("#ffffff", bg.fill);
            // Second instruction is the dark module (black)
            var dark = (PaintInstruction.PaintRect) scene.instructions.get(1);
            assertEquals("#000000", dark.fill);
        }
    }

    // =========================================================================
    // layout() — hex modules
    // =========================================================================

    @Nested
    @DisplayName("layout() hex modules")
    class LayoutHexTests {

        @Test
        @DisplayName("all-light hex grid produces background only")
        void allLightHexGrid() {
            var grid = Barcode2D.makeModuleGrid(5, 5, ModuleShape.HEX);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleShape(ModuleShape.HEX).quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);
            assertEquals(1, scene.instructions.size());
        }

        @Test
        @DisplayName("single dark hex module produces background + 1 path")
        void singleDarkHexModule() {
            var grid = Barcode2D.makeModuleGrid(5, 5, ModuleShape.HEX);
            grid = Barcode2D.setModule(grid, 2, 2, true);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleShape(ModuleShape.HEX).quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            assertEquals(2, scene.instructions.size());
            assertInstanceOf(PaintInstruction.PaintPath.class, scene.instructions.get(1));
        }

        @Test
        @DisplayName("hex path has exactly 7 commands (6 vertices + ClosePath)")
        void hexPathHas7Commands() {
            var grid = Barcode2D.makeModuleGrid(3, 3, ModuleShape.HEX);
            grid = Barcode2D.setModule(grid, 0, 0, true);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleShape(ModuleShape.HEX).quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            var path = (PaintInstruction.PaintPath) scene.instructions.get(1);
            assertEquals(7, path.commands.size());
        }

        @Test
        @DisplayName("hex path starts with MoveTo and ends with ClosePath")
        void hexPathStartsAndEnds() {
            var grid = Barcode2D.makeModuleGrid(3, 3, ModuleShape.HEX);
            grid = Barcode2D.setModule(grid, 0, 0, true);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleShape(ModuleShape.HEX).quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            var path = (PaintInstruction.PaintPath) scene.instructions.get(1);
            assertInstanceOf(PathCommand.MoveTo.class, path.commands.get(0));
            assertInstanceOf(PathCommand.ClosePath.class, path.commands.get(6));
        }

        @Test
        @DisplayName("hex path has 1 MoveTo, 5 LineTo, 1 ClosePath")
        void hexPathCommandTypes() {
            var grid = Barcode2D.makeModuleGrid(3, 3, ModuleShape.HEX);
            grid = Barcode2D.setModule(grid, 1, 1, true);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleShape(ModuleShape.HEX).quietZoneModules(0).build();
            var scene = Barcode2D.layout(grid, config);

            var path = (PaintInstruction.PaintPath) scene.instructions.get(1);
            long moveTos = path.commands.stream().filter(c -> c instanceof PathCommand.MoveTo).count();
            long lineTos = path.commands.stream().filter(c -> c instanceof PathCommand.LineTo).count();
            long closes = path.commands.stream().filter(c -> c instanceof PathCommand.ClosePath).count();

            assertEquals(1, moveTos);
            assertEquals(5, lineTos);
            assertEquals(1, closes);
        }
    }

    // =========================================================================
    // buildFlatTopHexPath
    // =========================================================================

    @Nested
    @DisplayName("buildFlatTopHexPath geometry")
    class BuildFlatTopHexPathTests {

        @Test
        @DisplayName("returns exactly 7 commands")
        void returns7Commands() {
            var cmds = Barcode2D.buildFlatTopHexPath(50.0, 50.0, 6.0);
            assertEquals(7, cmds.size());
        }

        @Test
        @DisplayName("first command is MoveTo")
        void firstIsMoveTo() {
            var cmds = Barcode2D.buildFlatTopHexPath(50.0, 50.0, 6.0);
            assertInstanceOf(PathCommand.MoveTo.class, cmds.get(0));
        }

        @Test
        @DisplayName("last command is ClosePath")
        void lastIsClosePath() {
            var cmds = Barcode2D.buildFlatTopHexPath(50.0, 50.0, 6.0);
            assertInstanceOf(PathCommand.ClosePath.class, cmds.get(6));
        }

        @Test
        @DisplayName("commands 1..5 are all LineTo")
        void middleAreLineTo() {
            var cmds = Barcode2D.buildFlatTopHexPath(50.0, 50.0, 6.0);
            for (int i = 1; i <= 5; i++) {
                assertInstanceOf(PathCommand.LineTo.class, cmds.get(i),
                        "command " + i + " should be LineTo");
            }
        }

        @Test
        @DisplayName("vertex 0 is at angle 0 degrees (right midpoint)")
        void vertex0AtAngle0() {
            double cx = 100.0;
            double cy = 100.0;
            double circumR = 10.0;
            var cmds = Barcode2D.buildFlatTopHexPath(cx, cy, circumR);

            var v0 = (PathCommand.MoveTo) cmds.get(0);
            // angle 0°: cos=1, sin=0  → (cx + circumR, cy)
            assertEquals(cx + circumR, v0.x, 1e-9);
            assertEquals(cy, v0.y, 1e-9);
        }

        @Test
        @DisplayName("all six vertices are at distance circumR from centre")
        void allVerticesAtCircumR() {
            double cx = 50.0;
            double cy = 60.0;
            double circumR = 8.0;
            var cmds = Barcode2D.buildFlatTopHexPath(cx, cy, circumR);

            for (int i = 0; i < 6; i++) {
                double vx, vy;
                if (i == 0) {
                    var m = (PathCommand.MoveTo) cmds.get(0);
                    vx = m.x; vy = m.y;
                } else {
                    var l = (PathCommand.LineTo) cmds.get(i);
                    vx = l.x; vy = l.y;
                }
                double dist = Math.sqrt((vx - cx) * (vx - cx) + (vy - cy) * (vy - cy));
                assertEquals(circumR, dist, 1e-9,
                        "vertex " + i + " should be at distance circumR from centre");
            }
        }

        @Test
        @DisplayName("returned list is unmodifiable")
        void returnedListUnmodifiable() {
            var cmds = Barcode2D.buildFlatTopHexPath(50.0, 50.0, 6.0);
            assertThrows(UnsupportedOperationException.class,
                    () -> cmds.add(PathCommand.ClosePath.INSTANCE));
        }
    }

    // =========================================================================
    // Full pipeline integration
    // =========================================================================

    @Nested
    @DisplayName("Full pipeline integration")
    class FullPipelineTests {

        @Test
        @DisplayName("QR-like 21x21 grid — instruction count = background + dark modules")
        void qrLikeGrid() {
            // A minimal QR-like grid with 4 dark modules (corners of finder pattern top-left)
            var grid = Barcode2D.makeModuleGrid(21, 21);
            grid = Barcode2D.setModule(grid, 0, 0, true);
            grid = Barcode2D.setModule(grid, 0, 6, true);
            grid = Barcode2D.setModule(grid, 6, 0, true);
            grid = Barcode2D.setModule(grid, 6, 6, true);

            var scene = Barcode2D.layout(grid);

            // background + 4 dark rects
            assertEquals(5, scene.instructions.size());
            assertEquals(290, scene.width);
            assertEquals(290, scene.height);
        }

        @Test
        @DisplayName("MaxiCode-like 33x30 hex grid — dark modules produce PaintPath instructions")
        void maxiCodeLikeGrid() {
            var grid = Barcode2D.makeModuleGrid(33, 30, ModuleShape.HEX);
            grid = Barcode2D.setModule(grid, 0, 0, true);
            grid = Barcode2D.setModule(grid, 16, 15, true);

            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleShape(ModuleShape.HEX)
                    .moduleSizePx(10)
                    .quietZoneModules(1)
                    .build();
            var scene = Barcode2D.layout(grid, config);

            // background + 2 hex modules
            assertEquals(3, scene.instructions.size());
            assertInstanceOf(PaintInstruction.PaintRect.class, scene.instructions.get(0)); // bg
            assertInstanceOf(PaintInstruction.PaintPath.class, scene.instructions.get(1)); // hex
            assertInstanceOf(PaintInstruction.PaintPath.class, scene.instructions.get(2)); // hex
        }

        @Test
        @DisplayName("scene dimensions are correct for 5x5 grid with quiet zone 2")
        void sceneDimensions5x5() {
            var grid = Barcode2D.makeModuleGrid(5, 5);
            var config = new Barcode2DLayoutConfig.Builder()
                    .moduleSizePx(10).quietZoneModules(2).build();
            var scene = Barcode2D.layout(grid, config);

            // (5 + 2*2) * 10 = 9 * 10 = 90
            assertEquals(90, scene.width);
            assertEquals(90, scene.height);
        }
    }
}
