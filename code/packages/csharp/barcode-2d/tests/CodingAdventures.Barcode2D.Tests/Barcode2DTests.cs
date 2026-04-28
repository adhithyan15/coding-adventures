namespace CodingAdventures.Barcode2D.Tests;

// Barcode2DTests.cs — Comprehensive tests for the Barcode2D library
// ==================================================================
//
// Each section corresponds to a major type or method in Barcode2D.cs.
// Tests are arranged from fundamentals (ModuleGrid creation and mutation)
// through layout (square then hex) to error cases.
//
// Reading guide:
//   Section 1 — VERSION constant
//   Section 2 — ModuleGrid.Create
//   Section 3 — ModuleGrid.SetModule (immutability, bounds)
//   Section 4 — Barcode2DLayoutConfig defaults and record equality
//   Section 5 — ModuleAnnotation and AnnotatedModuleGrid
//   Section 6 — Barcode2D.Layout validation errors
//   Section 7 — Barcode2D.LayoutSquare pixel geometry
//   Section 8 — Barcode2D.LayoutHex pixel geometry
//   Section 9 — Barcode2D.Layout dispatch
//   Section 10 — Edge cases: empty grid, zero quiet zone, large grid

using CodingAdventures.PaintInstructions;

public sealed class Barcode2DTests
{
    // =========================================================================
    // Section 1 — VERSION
    // =========================================================================

    [Fact]
    public void ExposesExpectedVersion()
    {
        Assert.Equal("0.1.0", Barcode2D.VERSION);
    }

    // =========================================================================
    // Section 2 — ModuleGrid.Create
    // =========================================================================

    [Fact]
    public void CreateReturnsCorrectDimensions()
    {
        var grid = ModuleGrid.Create(5, 7);
        Assert.Equal(5, grid.Rows);
        Assert.Equal(7, grid.Cols);
    }

    [Fact]
    public void CreateDefaultsToSquareShape()
    {
        var grid = ModuleGrid.Create(3, 3);
        Assert.Equal(ModuleShape.Square, grid.ModuleShape);
    }

    [Fact]
    public void CreateCanProduceHexShape()
    {
        var grid = ModuleGrid.Create(33, 30, ModuleShape.Hex);
        Assert.Equal(ModuleShape.Hex, grid.ModuleShape);
    }

    [Fact]
    public void CreateInitialisesAllModulesToLight()
    {
        var grid = ModuleGrid.Create(4, 6);
        for (int r = 0; r < grid.Rows; r++)
            for (int c = 0; c < grid.Cols; c++)
                Assert.False(grid.Modules[r][c],
                    $"Expected module[{r},{c}] to be light (false)");
    }

    [Fact]
    public void CreateProducesCorrectModuleCount()
    {
        var grid = ModuleGrid.Create(10, 15);
        Assert.Equal(10, grid.Modules.Count);
        foreach (var row in grid.Modules)
            Assert.Equal(15, row.Count);
    }

    [Fact]
    public void CreateSingleCellGrid()
    {
        var grid = ModuleGrid.Create(1, 1);
        Assert.Equal(1, grid.Rows);
        Assert.Equal(1, grid.Cols);
        Assert.False(grid.Modules[0][0]);
    }

    // =========================================================================
    // Section 3 — ModuleGrid.SetModule (immutability and bounds checks)
    // =========================================================================

    [Fact]
    public void SetModuleReturnsDifferentObject()
    {
        var g1 = ModuleGrid.Create(3, 3);
        var g2 = g1.SetModule(1, 1, true);
        Assert.False(ReferenceEquals(g1, g2));
    }

    [Fact]
    public void SetModuleDoesNotMutateOriginal()
    {
        var g1 = ModuleGrid.Create(3, 3);
        var g2 = g1.SetModule(1, 1, true);

        // Original must still be light everywhere.
        for (int r = 0; r < g1.Rows; r++)
            for (int c = 0; c < g1.Cols; c++)
                Assert.False(g1.Modules[r][c],
                    $"Original mutated at [{r},{c}]");

        // New grid has the changed module dark.
        Assert.True(g2.Modules[1][1]);
    }

    [Fact]
    public void SetModulePreservesOtherModules()
    {
        var g1 = ModuleGrid.Create(3, 3);
        // Set two modules dark in sequence.
        var g2 = g1.SetModule(0, 0, true);
        var g3 = g2.SetModule(2, 2, true);

        Assert.True(g3.Modules[0][0]);
        Assert.True(g3.Modules[2][2]);
        Assert.False(g3.Modules[1][1]); // untouched
    }

    [Fact]
    public void SetModulePreservesShape()
    {
        var g1 = ModuleGrid.Create(3, 3, ModuleShape.Hex);
        var g2 = g1.SetModule(0, 0, true);
        Assert.Equal(ModuleShape.Hex, g2.ModuleShape);
    }

    [Fact]
    public void SetModulePreservesDimensions()
    {
        var g1 = ModuleGrid.Create(5, 7);
        var g2 = g1.SetModule(2, 3, true);
        Assert.Equal(5, g2.Rows);
        Assert.Equal(7, g2.Cols);
    }

    [Fact]
    public void SetModuleCanSetDarkThenLight()
    {
        // Set dark, then toggle back to light.
        var g = ModuleGrid.Create(3, 3)
            .SetModule(1, 1, true)
            .SetModule(1, 1, false);
        Assert.False(g.Modules[1][1]);
    }

    [Fact]
    public void SetModuleThrowsOnNegativeRow()
    {
        var g = ModuleGrid.Create(3, 3);
        Assert.Throws<ArgumentOutOfRangeException>(() => g.SetModule(-1, 0, true));
    }

    [Fact]
    public void SetModuleThrowsOnRowAtBound()
    {
        var g = ModuleGrid.Create(3, 3);
        Assert.Throws<ArgumentOutOfRangeException>(() => g.SetModule(3, 0, true));
    }

    [Fact]
    public void SetModuleThrowsOnNegativeCol()
    {
        var g = ModuleGrid.Create(3, 3);
        Assert.Throws<ArgumentOutOfRangeException>(() => g.SetModule(0, -1, true));
    }

    [Fact]
    public void SetModuleThrowsOnColAtBound()
    {
        var g = ModuleGrid.Create(3, 3);
        Assert.Throws<ArgumentOutOfRangeException>(() => g.SetModule(0, 3, true));
    }

    [Fact]
    public void SetModuleSharesUnchangedRows()
    {
        // Row 0 in g2 must be the same reference as row 0 in g1 (structural sharing).
        var g1 = ModuleGrid.Create(3, 3);
        var g2 = g1.SetModule(1, 0, true); // only row 1 changes
        Assert.True(ReferenceEquals(g1.Modules[0], g2.Modules[0]));
        Assert.True(ReferenceEquals(g1.Modules[2], g2.Modules[2]));
        // Row 1 is a new allocation.
        Assert.False(ReferenceEquals(g1.Modules[1], g2.Modules[1]));
    }

    // =========================================================================
    // Section 4 — Barcode2DLayoutConfig defaults and record equality
    // =========================================================================

    [Fact]
    public void DefaultConfigHasExpectedValues()
    {
        var d = Barcode2DLayoutConfig.Default;
        Assert.Equal(10.0,           d.ModuleSizePx);
        Assert.Equal(4,              d.QuietZoneModules);
        Assert.Equal("#000000",      d.Foreground);
        Assert.Equal("#ffffff",      d.Background);
        Assert.Equal(ModuleShape.Square, d.ModuleShape);
    }

    [Fact]
    public void ConfigRecordEqualityWorks()
    {
        var a = new Barcode2DLayoutConfig(10, 4, "#000000", "#ffffff", ModuleShape.Square);
        var b = new Barcode2DLayoutConfig(10, 4, "#000000", "#ffffff", ModuleShape.Square);
        Assert.Equal(a, b);
    }

    [Fact]
    public void ConfigWithExpressionCreatesNewRecord()
    {
        var d = Barcode2DLayoutConfig.Default;
        var custom = d with { ModuleSizePx = 20 };
        Assert.Equal(20.0, custom.ModuleSizePx);
        // Original unchanged.
        Assert.Equal(10.0, d.ModuleSizePx);
    }

    // =========================================================================
    // Section 5 — ModuleAnnotation and AnnotatedModuleGrid
    // =========================================================================

    [Fact]
    public void ModuleAnnotationStoresRole()
    {
        var ann = new ModuleAnnotation(ModuleRole.Finder, Dark: true);
        Assert.Equal(ModuleRole.Finder, ann.Role);
        Assert.True(ann.Dark);
    }

    [Fact]
    public void ModuleAnnotationOptionalFieldsDefaultNull()
    {
        var ann = new ModuleAnnotation(ModuleRole.Data, Dark: false);
        Assert.Null(ann.CodewordIndex);
        Assert.Null(ann.BitIndex);
        Assert.Null(ann.Metadata);
    }

    [Fact]
    public void ModuleAnnotationStoresCodewordAndBitIndex()
    {
        var ann = new ModuleAnnotation(ModuleRole.Ecc, Dark: true, CodewordIndex: 5, BitIndex: 3);
        Assert.Equal(5, ann.CodewordIndex);
        Assert.Equal(3, ann.BitIndex);
    }

    [Fact]
    public void ModuleAnnotationStoresMetadata()
    {
        var meta = new Dictionary<string, string> { { "format_role", "qr:dark-module" } };
        var ann = new ModuleAnnotation(ModuleRole.Format, Dark: true, Metadata: meta);
        Assert.Equal("qr:dark-module", ann.Metadata!["format_role"]);
    }

    [Fact]
    public void AnnotatedModuleGridWrapsGrid()
    {
        var grid = ModuleGrid.Create(2, 2);
        var annotations = new IReadOnlyList<ModuleAnnotation?>[]
        {
            new ModuleAnnotation?[] { null, null },
            new ModuleAnnotation?[] { new ModuleAnnotation(ModuleRole.Finder, true), null },
        };
        var annotated = new AnnotatedModuleGrid(grid, annotations);

        Assert.Equal(2, annotated.Rows);
        Assert.Equal(2, annotated.Cols);
        Assert.Null(annotated.Annotations[0][0]);
        Assert.Equal(ModuleRole.Finder, annotated.Annotations[1][0]!.Role);
    }

    // =========================================================================
    // Section 6 — Barcode2D.Layout validation errors
    // =========================================================================

    [Fact]
    public void LayoutThrowsWhenModuleSizePxIsZero()
    {
        var grid = ModuleGrid.Create(3, 3);
        var cfg  = Barcode2DLayoutConfig.Default with { ModuleSizePx = 0 };
        Assert.Throws<InvalidBarcode2DConfigException>(() => Barcode2D.Layout(grid, cfg));
    }

    [Fact]
    public void LayoutThrowsWhenModuleSizePxIsNegative()
    {
        var grid = ModuleGrid.Create(3, 3);
        var cfg  = Barcode2DLayoutConfig.Default with { ModuleSizePx = -5 };
        Assert.Throws<InvalidBarcode2DConfigException>(() => Barcode2D.Layout(grid, cfg));
    }

    [Fact]
    public void LayoutThrowsWhenQuietZoneIsNegative()
    {
        var grid = ModuleGrid.Create(3, 3);
        var cfg  = Barcode2DLayoutConfig.Default with { QuietZoneModules = -1 };
        Assert.Throws<InvalidBarcode2DConfigException>(() => Barcode2D.Layout(grid, cfg));
    }

    [Fact]
    public void LayoutThrowsWhenShapeMismatchSquareVsHex()
    {
        // Grid is Square but config says Hex.
        var grid = ModuleGrid.Create(3, 3, ModuleShape.Square);
        var cfg  = Barcode2DLayoutConfig.Default with { ModuleShape = ModuleShape.Hex };
        Assert.Throws<InvalidBarcode2DConfigException>(() => Barcode2D.Layout(grid, cfg));
    }

    [Fact]
    public void LayoutThrowsWhenShapeMismatchHexVsSquare()
    {
        // Grid is Hex but config says Square.
        var grid = ModuleGrid.Create(33, 30, ModuleShape.Hex);
        var cfg  = Barcode2DLayoutConfig.Default with { ModuleShape = ModuleShape.Square };
        Assert.Throws<InvalidBarcode2DConfigException>(() => Barcode2D.Layout(grid, cfg));
    }

    [Fact]
    public void LayoutSquareThrowsOnInvalidConfig()
    {
        var grid = ModuleGrid.Create(3, 3);
        var cfg  = Barcode2DLayoutConfig.Default with { ModuleSizePx = 0 };
        Assert.Throws<InvalidBarcode2DConfigException>(() => Barcode2D.LayoutSquare(grid, cfg));
    }

    [Fact]
    public void LayoutHexThrowsOnInvalidConfig()
    {
        var grid = ModuleGrid.Create(3, 3, ModuleShape.Hex);
        var cfg  = new Barcode2DLayoutConfig(0, 4, "#000000", "#ffffff", ModuleShape.Hex);
        Assert.Throws<InvalidBarcode2DConfigException>(() => Barcode2D.LayoutHex(grid, cfg));
    }

    // =========================================================================
    // Section 7 — Barcode2D.LayoutSquare pixel geometry
    // =========================================================================
    //
    // For a 3×3 grid with moduleSizePx=10 and quietZoneModules=4:
    //   totalWidth  = (3 + 2×4) × 10 = 110
    //   totalHeight = (3 + 2×4) × 10 = 110
    //   quietZonePx = 4 × 10 = 40
    //   module[0][0]: x=40, y=40, w=10, h=10
    //   module[0][1]: x=50, y=40
    //   module[1][0]: x=40, y=50

    [Fact]
    public void LayoutSquareSceneHasCorrectDimensions()
    {
        var grid = ModuleGrid.Create(3, 3);
        var cfg  = Barcode2DLayoutConfig.Default with { ModuleSizePx = 10, QuietZoneModules = 4 };
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        Assert.Equal(110.0, scene.Width);
        Assert.Equal(110.0, scene.Height);
    }

    [Fact]
    public void LayoutSquareSceneHasBackgroundColor()
    {
        var grid = ModuleGrid.Create(3, 3);
        var cfg  = Barcode2DLayoutConfig.Default with { Background = "#ff0000" };
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        Assert.Equal("#ff0000", scene.Background);
    }

    [Fact]
    public void LayoutSquareAllLightGridHasOnlyBackgroundInstruction()
    {
        // An all-light grid should produce exactly 1 instruction: the background rect.
        var grid = ModuleGrid.Create(3, 3);
        var scene = Barcode2D.LayoutSquare(grid, Barcode2DLayoutConfig.Default);

        Assert.Single(scene.Instructions);
        Assert.IsType<PaintRect>(scene.Instructions[0]);
    }

    [Fact]
    public void LayoutSquareFirstInstructionIsBackgroundRect()
    {
        var grid  = ModuleGrid.Create(3, 3);
        var cfg   = Barcode2DLayoutConfig.Default with { ModuleSizePx = 10, QuietZoneModules = 4 };
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        var bg = Assert.IsType<PaintRect>(scene.Instructions[0]);
        Assert.Equal(0.0, bg.X);
        Assert.Equal(0.0, bg.Y);
        Assert.Equal(110.0, bg.Width);
        Assert.Equal(110.0, bg.Height);
        Assert.Equal("#ffffff", bg.Fill);
    }

    [Fact]
    public void LayoutSquareOneDarkModuleProducesTwoInstructions()
    {
        var grid  = ModuleGrid.Create(3, 3).SetModule(0, 0, true);
        var scene = Barcode2D.LayoutSquare(grid, Barcode2DLayoutConfig.Default);

        // Background + 1 module rect.
        Assert.Equal(2, scene.Instructions.Count);
    }

    [Fact]
    public void LayoutSquareDarkModuleCoordinatesAreCorrect()
    {
        // module[0][0] with moduleSizePx=10, quietZoneModules=4:
        //   x = 4*10 = 40, y = 4*10 = 40, w = 10, h = 10
        var grid  = ModuleGrid.Create(3, 3).SetModule(0, 0, true);
        var cfg   = Barcode2DLayoutConfig.Default with { ModuleSizePx = 10, QuietZoneModules = 4 };
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        var rect = Assert.IsType<PaintRect>(scene.Instructions[1]);
        Assert.Equal(40.0, rect.X);
        Assert.Equal(40.0, rect.Y);
        Assert.Equal(10.0, rect.Width);
        Assert.Equal(10.0, rect.Height);
        Assert.Equal("#000000", rect.Fill);
    }

    [Fact]
    public void LayoutSquareDarkModuleRowOffsetIsCorrect()
    {
        // module[2][0]: y = 4*10 + 2*10 = 60
        var grid  = ModuleGrid.Create(3, 3).SetModule(2, 0, true);
        var cfg   = Barcode2DLayoutConfig.Default with { ModuleSizePx = 10, QuietZoneModules = 4 };
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        var rect = Assert.IsType<PaintRect>(scene.Instructions[1]);
        Assert.Equal(40.0, rect.X); // col 0 → 40
        Assert.Equal(60.0, rect.Y); // row 2 → 60
    }

    [Fact]
    public void LayoutSquareDarkModuleColOffsetIsCorrect()
    {
        // module[0][2]: x = 4*10 + 2*10 = 60
        var grid  = ModuleGrid.Create(3, 3).SetModule(0, 2, true);
        var cfg   = Barcode2DLayoutConfig.Default with { ModuleSizePx = 10, QuietZoneModules = 4 };
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        var rect = Assert.IsType<PaintRect>(scene.Instructions[1]);
        Assert.Equal(60.0, rect.X);
        Assert.Equal(40.0, rect.Y);
    }

    [Fact]
    public void LayoutSquareDarkModuleUsesCustomModuleSize()
    {
        // moduleSizePx=5, quietZoneModules=2:
        //   module[0][0]: x = 2*5 = 10, y = 10, w = 5, h = 5
        var grid  = ModuleGrid.Create(3, 3).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(5, 2, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        var rect = Assert.IsType<PaintRect>(scene.Instructions[1]);
        Assert.Equal(10.0, rect.X);
        Assert.Equal(10.0, rect.Y);
        Assert.Equal(5.0,  rect.Width);
        Assert.Equal(5.0,  rect.Height);
    }

    [Fact]
    public void LayoutSquareCountMatchesDarkModules()
    {
        // Place 5 dark modules in a 5×5 grid.
        var grid = ModuleGrid.Create(5, 5)
            .SetModule(0, 0, true)
            .SetModule(1, 1, true)
            .SetModule(2, 2, true)
            .SetModule(3, 3, true)
            .SetModule(4, 4, true);
        var scene = Barcode2D.LayoutSquare(grid, Barcode2DLayoutConfig.Default);

        // 1 background + 5 dark rects = 6
        Assert.Equal(6, scene.Instructions.Count);
    }

    [Fact]
    public void LayoutSquareZeroQuietZoneIsAllowed()
    {
        // quietZoneModules=0: modules start at pixel (0,0).
        var grid  = ModuleGrid.Create(3, 3).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        var rect = Assert.IsType<PaintRect>(scene.Instructions[1]);
        Assert.Equal(0.0, rect.X);
        Assert.Equal(0.0, rect.Y);
    }

    [Fact]
    public void LayoutSquareTotalSizeWithZeroQuietZone()
    {
        var grid  = ModuleGrid.Create(21, 21);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        Assert.Equal(210.0, scene.Width);
        Assert.Equal(210.0, scene.Height);
    }

    [Fact]
    public void LayoutSquareUsesNullConfigDefault()
    {
        // Passing null for config should use Barcode2DLayoutConfig.Default.
        var grid  = ModuleGrid.Create(3, 3);
        var scene = Barcode2D.LayoutSquare(grid); // no config arg

        // Default: (3+8)*10 = 110
        Assert.Equal(110.0, scene.Width);
    }

    [Fact]
    public void LayoutSquareDarkModuleUsesCustomForeground()
    {
        var grid  = ModuleGrid.Create(3, 3).SetModule(0, 0, true);
        var cfg   = Barcode2DLayoutConfig.Default with { Foreground = "#ff0000" };
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        var rect = Assert.IsType<PaintRect>(scene.Instructions[1]);
        Assert.Equal("#ff0000", rect.Fill);
    }

    // =========================================================================
    // Section 8 — Barcode2D.LayoutHex pixel geometry
    // =========================================================================
    //
    // A MaxiCode-style 33×30 hex grid. We also test small grids for geometry.
    //
    // For moduleSizePx=10, quietZoneModules=0, row=0, col=0:
    //   hexWidth  = 10
    //   hexHeight = 10 * (√3/2) ≈ 8.660
    //   circumR   = 10 / √3     ≈ 5.774
    //   cx = 0 + 0*10 + (0%2)*5 = 0
    //   cy = 0 + 0*8.660        = 0

    [Fact]
    public void LayoutHexSceneHasPositiveDimensions()
    {
        var grid = ModuleGrid.Create(33, 30, ModuleShape.Hex);
        var cfg  = new Barcode2DLayoutConfig(10, 1, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        Assert.True(scene.Width  > 0);
        Assert.True(scene.Height > 0);
    }

    [Fact]
    public void LayoutHexSceneWidthIncludesHalfHexExtra()
    {
        // totalWidth = (cols + 2*quiet) * hexWidth + hexWidth/2
        // For cols=4, quiet=0, sz=10:
        //   = 4*10 + 5 = 45
        var grid  = ModuleGrid.Create(2, 4, ModuleShape.Hex);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        Assert.Equal(45.0, scene.Width);
    }

    [Fact]
    public void LayoutHexSceneHeightIsCorrect()
    {
        // totalHeight = (rows + 2*quiet) * hexHeight
        // For rows=2, quiet=0, sz=10:
        //   hexHeight = 10 * √3/2 ≈ 8.6602...
        //   totalHeight = 2 * 8.6602 ≈ 17.3205
        var grid  = ModuleGrid.Create(2, 4, ModuleShape.Hex);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        double expected = 2 * 10 * (Math.Sqrt(3.0) / 2.0);
        Assert.Equal(expected, scene.Height, precision: 9);
    }

    [Fact]
    public void LayoutHexAllLightGridHasOnlyBackgroundInstruction()
    {
        var grid = ModuleGrid.Create(5, 5, ModuleShape.Hex);
        var cfg  = new Barcode2DLayoutConfig(10, 1, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        Assert.Single(scene.Instructions);
        Assert.IsType<PaintRect>(scene.Instructions[0]);
    }

    [Fact]
    public void LayoutHexOneDarkModuleProducesTwoInstructions()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        // Background + 1 PaintPath.
        Assert.Equal(2, scene.Instructions.Count);
    }

    [Fact]
    public void LayoutHexDarkModuleProducesPaintPath()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(1, 1, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        Assert.IsType<PaintPath>(scene.Instructions[1]);
    }

    [Fact]
    public void LayoutHexPathHasSevenCommands()
    {
        // A flat-top hex path: MoveTo + 5×LineTo + ClosePath = 7 commands.
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        var path = Assert.IsType<PaintPath>(scene.Instructions[1]);
        Assert.Equal(7, path.Commands.Count);
    }

    [Fact]
    public void LayoutHexPathStartsWithMoveTo()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        var path = Assert.IsType<PaintPath>(scene.Instructions[1]);
        Assert.IsType<MoveToCommand>(path.Commands[0]);
    }

    [Fact]
    public void LayoutHexPathEndsWithClose()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        var path = Assert.IsType<PaintPath>(scene.Instructions[1]);
        Assert.IsType<ClosePathCommand>(path.Commands[6]);
    }

    [Fact]
    public void LayoutHexPathHasFiveLineToCommands()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        var path = Assert.IsType<PaintPath>(scene.Instructions[1]);
        int lineToCount = path.Commands.Count(c => c is LineToCommand);
        Assert.Equal(5, lineToCount);
    }

    [Fact]
    public void LayoutHexPathUsesCorrectForegroundFill()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#123456", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        var path = Assert.IsType<PaintPath>(scene.Instructions[1]);
        Assert.Equal("#123456", path.Fill);
    }

    [Fact]
    public void LayoutHexOddRowOffsetIsApplied()
    {
        // Row 1, col 0 should have cx offset by hexWidth/2 vs row 0, col 0.
        var cfg = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);

        var gridEvenRow = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var sceneEven   = Barcode2D.LayoutHex(gridEvenRow, cfg);
        var pathEven    = Assert.IsType<PaintPath>(sceneEven.Instructions[1]);
        var moveEven    = Assert.IsType<MoveToCommand>(pathEven.Commands[0]);

        var gridOddRow  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(1, 0, true);
        var sceneOdd    = Barcode2D.LayoutHex(gridOddRow, cfg);
        var pathOdd     = Assert.IsType<PaintPath>(sceneOdd.Instructions[1]);
        var moveOdd     = Assert.IsType<MoveToCommand>(pathOdd.Commands[0]);

        // Odd row has cx 5 px further right than even row (hexWidth/2 = 5).
        Assert.True(moveOdd.X > moveEven.X,
            "Odd row centre should be further right than even row");
        Assert.InRange(moveOdd.X - moveEven.X - 5.0, -0.001, 0.001);
    }

    [Fact]
    public void LayoutHexCountMatchesDarkModules()
    {
        var grid = ModuleGrid.Create(5, 5, ModuleShape.Hex)
            .SetModule(0, 0, true)
            .SetModule(1, 1, true)
            .SetModule(2, 2, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        // 1 background + 3 paths
        Assert.Equal(4, scene.Instructions.Count);
    }

    [Fact]
    public void LayoutHexUsesNullConfigDefaultWithHexGrid()
    {
        // LayoutHex with null config should supply a Hex default config.
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid); // no config

        Assert.Single(scene.Instructions); // all light → background only
    }

    // =========================================================================
    // Section 9 — Barcode2D.Layout dispatch
    // =========================================================================

    [Fact]
    public void LayoutDispatchesToSquareForSquareGrid()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Square);
        var cfg   = Barcode2DLayoutConfig.Default;
        var scene = Barcode2D.Layout(grid, cfg);

        // Square grid with all light → background only.
        Assert.Single(scene.Instructions);
        Assert.IsType<PaintRect>(scene.Instructions[0]);
    }

    [Fact]
    public void LayoutDispatchesToHexForHexGrid()
    {
        var grid  = ModuleGrid.Create(3, 3, ModuleShape.Hex).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.Layout(grid, cfg);

        // Should produce a PaintPath for the dark module.
        Assert.Equal(2, scene.Instructions.Count);
        Assert.IsType<PaintPath>(scene.Instructions[1]);
    }

    [Fact]
    public void LayoutUsesDefaultConfigWhenNull()
    {
        var grid  = ModuleGrid.Create(21, 21);
        var scene = Barcode2D.Layout(grid);

        // Default: (21+8)*10 = 290
        Assert.Equal(290.0, scene.Width);
    }

    // =========================================================================
    // Section 10 — Edge cases
    // =========================================================================

    [Fact]
    public void LayoutSquareOneByOneGridAllLight()
    {
        var grid  = ModuleGrid.Create(1, 1);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        Assert.Equal(10.0, scene.Width);
        Assert.Equal(10.0, scene.Height);
        Assert.Single(scene.Instructions); // only background
    }

    [Fact]
    public void LayoutSquareOneByOneGridAllDark()
    {
        var grid  = ModuleGrid.Create(1, 1).SetModule(0, 0, true);
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        Assert.Equal(2, scene.Instructions.Count);
        var rect = Assert.IsType<PaintRect>(scene.Instructions[1]);
        Assert.Equal(0.0, rect.X);
        Assert.Equal(0.0, rect.Y);
        Assert.Equal(10.0, rect.Width);
        Assert.Equal(10.0, rect.Height);
    }

    [Fact]
    public void LayoutSquareLargeQrV40Grid()
    {
        // QR Code version 40 = 177×177 modules.
        var grid  = ModuleGrid.Create(177, 177);
        var cfg   = Barcode2DLayoutConfig.Default; // 10px modules, 4 quiet
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        // totalWidth = (177 + 8) * 10 = 1850
        Assert.Equal(1850.0, scene.Width);
        Assert.Equal(1850.0, scene.Height);
    }

    [Fact]
    public void LayoutHexMaxiCodeFixedSize()
    {
        // MaxiCode is always 33×30 with hex modules.
        var grid  = ModuleGrid.Create(33, 30, ModuleShape.Hex);
        var cfg   = new Barcode2DLayoutConfig(10, 1, "#000000", "#ffffff", ModuleShape.Hex);
        var scene = Barcode2D.LayoutHex(grid, cfg);

        // Width: (30+2)*10 + 5 = 325
        // Height: (33+2)*10*(√3/2)
        double expectedWidth  = (30 + 2) * 10.0 + 5.0;
        double expectedHeight = (33 + 2) * 10.0 * (Math.Sqrt(3.0) / 2.0);

        Assert.Equal(expectedWidth,  scene.Width,  precision: 9);
        Assert.Equal(expectedHeight, scene.Height, precision: 9);
    }

    [Fact]
    public void LayoutSquareNonSquareGrid()
    {
        // 10 rows × 30 cols (PDF417-style wide grid).
        var grid  = ModuleGrid.Create(10, 30);
        var cfg   = new Barcode2DLayoutConfig(5, 2, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        // totalWidth  = (30 + 4) * 5 = 170
        // totalHeight = (10 + 4) * 5 = 70
        Assert.Equal(170.0, scene.Width);
        Assert.Equal(70.0,  scene.Height);
    }

    [Fact]
    public void ChainedSetModuleBuildsCorrectGrid()
    {
        // Build a checkerboard in a 4×4 grid using chained SetModule.
        var grid = ModuleGrid.Create(4, 4);
        for (int r = 0; r < 4; r++)
            for (int c = 0; c < 4; c++)
                if ((r + c) % 2 == 0)
                    grid = grid.SetModule(r, c, true);

        // 8 dark modules out of 16.
        int darkCount = 0;
        for (int r = 0; r < 4; r++)
            for (int c = 0; c < 4; c++)
                if (grid.Modules[r][c]) darkCount++;
        Assert.Equal(8, darkCount);

        // LayoutSquare should produce 1 background + 8 module rects.
        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);
        Assert.Equal(9, scene.Instructions.Count);
    }

    [Fact]
    public void LayoutSquareAllDarkGrid()
    {
        int sz = 5;
        var grid = ModuleGrid.Create(sz, sz);
        for (int r = 0; r < sz; r++)
            for (int c = 0; c < sz; c++)
                grid = grid.SetModule(r, c, true);

        var cfg   = new Barcode2DLayoutConfig(10, 0, "#000000", "#ffffff", ModuleShape.Square);
        var scene = Barcode2D.LayoutSquare(grid, cfg);

        // 1 background + 25 rects
        Assert.Equal(26, scene.Instructions.Count);
    }
}
