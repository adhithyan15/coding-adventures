package barcode2d

import (
	"errors"
	"math"
	"testing"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

// ============================================================================
// Helpers
// ============================================================================

// mustSetModule calls SetModule and fails the test on error.
func mustSetModule(t *testing.T, g ModuleGrid, row, col uint32, dark bool) ModuleGrid {
	t.Helper()
	g2, err := SetModule(g, row, col, dark)
	if err != nil {
		t.Fatalf("SetModule(%d,%d): unexpected error: %v", row, col, err)
	}
	return g2
}

// rectAt returns the PaintRectInstruction at position i in the scene instructions,
// or fails the test if the instruction is not a PaintRectInstruction.
func rectAt(t *testing.T, scene paintinstructions.PaintScene, i int) paintinstructions.PaintRectInstruction {
	t.Helper()
	if i >= len(scene.Instructions) {
		t.Fatalf("instruction index %d out of range (len=%d)", i, len(scene.Instructions))
	}
	r, ok := scene.Instructions[i].(paintinstructions.PaintRectInstruction)
	if !ok {
		t.Fatalf("instruction[%d] is %T, want PaintRectInstruction", i, scene.Instructions[i])
	}
	return r
}

// pathAt returns the PaintPathInstruction at position i in the scene instructions,
// or fails the test if the instruction is not a PaintPathInstruction.
func pathAt(t *testing.T, scene paintinstructions.PaintScene, i int) paintinstructions.PaintPathInstruction {
	t.Helper()
	if i >= len(scene.Instructions) {
		t.Fatalf("instruction index %d out of range (len=%d)", i, len(scene.Instructions))
	}
	p, ok := scene.Instructions[i].(paintinstructions.PaintPathInstruction)
	if !ok {
		t.Fatalf("instruction[%d] is %T, want PaintPathInstruction", i, scene.Instructions[i])
	}
	return p
}

// almostEqual returns true if a and b differ by less than 1e-9.
func almostEqual(a, b float64) bool {
	return math.Abs(a-b) < 1e-9
}

// ============================================================================
// Version
// ============================================================================

func TestVersion(t *testing.T) {
	// Ensure Version is a non-empty string — minimal smoke test so the constant
	// is exercised by at least one test.
	if Version == "" {
		t.Fatal("Version must not be empty")
	}
}

// ============================================================================
// MakeModuleGrid
// ============================================================================

func TestMakeModuleGridCreatesCorrectDimensions(t *testing.T) {
	// A 5×7 grid should have 5 rows and 7 columns.
	g := MakeModuleGrid(5, 7, ModuleShapeSquare)

	if g.Rows != 5 {
		t.Errorf("Rows: got %d, want 5", g.Rows)
	}
	if g.Cols != 7 {
		t.Errorf("Cols: got %d, want 7", g.Cols)
	}
	if len(g.Modules) != 5 {
		t.Errorf("len(Modules): got %d, want 5", len(g.Modules))
	}
	for r, row := range g.Modules {
		if len(row) != 7 {
			t.Errorf("len(Modules[%d]): got %d, want 7", r, len(row))
		}
	}
}

func TestMakeModuleGridAllLight(t *testing.T) {
	// Every module should be false (light) immediately after creation.
	g := MakeModuleGrid(4, 4, ModuleShapeSquare)
	for r := uint32(0); r < g.Rows; r++ {
		for c := uint32(0); c < g.Cols; c++ {
			if g.Modules[r][c] {
				t.Errorf("Modules[%d][%d] should be false (light), got true", r, c)
			}
		}
	}
}

func TestMakeModuleGridSquareShape(t *testing.T) {
	g := MakeModuleGrid(3, 3, ModuleShapeSquare)
	if g.ModuleShape != ModuleShapeSquare {
		t.Errorf("ModuleShape: got %v, want ModuleShapeSquare", g.ModuleShape)
	}
}

func TestMakeModuleGridHexShape(t *testing.T) {
	g := MakeModuleGrid(33, 30, ModuleShapeHex)
	if g.ModuleShape != ModuleShapeHex {
		t.Errorf("ModuleShape: got %v, want ModuleShapeHex", g.ModuleShape)
	}
}

func TestMakeModuleGridZeroRows(t *testing.T) {
	// A 0×5 grid should have an empty modules slice — not a panic.
	g := MakeModuleGrid(0, 5, ModuleShapeSquare)
	if g.Rows != 0 {
		t.Errorf("Rows: got %d, want 0", g.Rows)
	}
	if len(g.Modules) != 0 {
		t.Errorf("len(Modules): got %d, want 0", len(g.Modules))
	}
}

// ============================================================================
// SetModule
// ============================================================================

func TestSetModuleImmutable(t *testing.T) {
	// SetModule must not modify the original grid.
	g := MakeModuleGrid(3, 3, ModuleShapeSquare)
	g2 := mustSetModule(t, g, 1, 1, true)

	// Original must be unchanged.
	if g.Modules[1][1] {
		t.Error("original grid was mutated by SetModule")
	}
	// New grid must have the change.
	if !g2.Modules[1][1] {
		t.Error("new grid does not reflect the SetModule change")
	}
}

func TestSetModuleOtherRowsAreShared(t *testing.T) {
	// Rows that are not touched should be the same slice (structural sharing).
	g := MakeModuleGrid(3, 3, ModuleShapeSquare)
	g2 := mustSetModule(t, g, 1, 1, true)

	// Row 0 and row 2 should be the exact same slice in both grids.
	if &g.Modules[0][0] != &g2.Modules[0][0] {
		t.Error("unmodified rows should share underlying array (structural sharing)")
	}
	if &g.Modules[2][0] != &g2.Modules[2][0] {
		t.Error("unmodified rows should share underlying array (structural sharing)")
	}
}

func TestSetModuleReturnsBothValues(t *testing.T) {
	g := MakeModuleGrid(2, 2, ModuleShapeSquare)
	g2, err := SetModule(g, 0, 0, true)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !g2.Modules[0][0] {
		t.Error("module should be dark after SetModule")
	}
}

func TestSetModuleOutOfBoundsRow(t *testing.T) {
	g := MakeModuleGrid(3, 3, ModuleShapeSquare)
	_, err := SetModule(g, 3, 0, true) // row 3 does not exist in a 3-row grid (0-indexed)
	if err == nil {
		t.Fatal("SetModule should return an error for out-of-bounds row")
	}
}

func TestSetModuleOutOfBoundsCol(t *testing.T) {
	g := MakeModuleGrid(3, 3, ModuleShapeSquare)
	_, err := SetModule(g, 0, 3, true) // col 3 does not exist in a 3-col grid
	if err == nil {
		t.Fatal("SetModule should return an error for out-of-bounds col")
	}
}

func TestSetModulePreservesModuleShape(t *testing.T) {
	g := MakeModuleGrid(5, 5, ModuleShapeHex)
	g2 := mustSetModule(t, g, 2, 2, true)
	if g2.ModuleShape != ModuleShapeHex {
		t.Errorf("SetModule should preserve ModuleShape, got %v", g2.ModuleShape)
	}
}

func TestSetModulePreservesDimensions(t *testing.T) {
	g := MakeModuleGrid(5, 7, ModuleShapeSquare)
	g2 := mustSetModule(t, g, 2, 3, true)
	if g2.Rows != 5 || g2.Cols != 7 {
		t.Errorf("SetModule should preserve dimensions, got %dx%d", g2.Rows, g2.Cols)
	}
}

// ============================================================================
// Layout — validation errors
// ============================================================================

func TestLayoutRejectsZeroModuleSize(t *testing.T) {
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	cfg := DefaultBarcode2DLayoutConfig
	cfg.ModuleSizePx = 0

	_, err := Layout(g, &cfg)
	if err == nil {
		t.Fatal("Layout should return an error for ModuleSizePx=0")
	}
	if !IsInvalidBarcode2DConfigError(err) {
		t.Errorf("error should be InvalidBarcode2DConfigError, got %T", err)
	}
}

func TestLayoutRejectsNegativeModuleSize(t *testing.T) {
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	cfg := DefaultBarcode2DLayoutConfig
	cfg.ModuleSizePx = -1

	_, err := Layout(g, &cfg)
	if err == nil {
		t.Fatal("Layout should return an error for negative ModuleSizePx")
	}
	if !IsInvalidBarcode2DConfigError(err) {
		t.Errorf("error should be InvalidBarcode2DConfigError, got %T", err)
	}
}

func TestLayoutRejectsMismatchedModuleShape(t *testing.T) {
	// Grid is square, but config says hex — should be rejected.
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	cfg := DefaultBarcode2DLayoutConfig
	cfg.ModuleShape = ModuleShapeHex

	_, err := Layout(g, &cfg)
	if err == nil {
		t.Fatal("Layout should return an error when config.ModuleShape != grid.ModuleShape")
	}
	if !IsInvalidBarcode2DConfigError(err) {
		t.Errorf("error should be InvalidBarcode2DConfigError, got %T", err)
	}
}

func TestLayoutRejectsMismatchedModuleShapeHexGrid(t *testing.T) {
	// Grid is hex, but config says square — should also be rejected.
	g := MakeModuleGrid(33, 30, ModuleShapeHex)
	cfg := DefaultBarcode2DLayoutConfig
	cfg.ModuleShape = ModuleShapeSquare

	_, err := Layout(g, &cfg)
	if err == nil {
		t.Fatal("Layout should return an error when hex grid is used with square config")
	}
}

func TestLayoutNilConfigUsesDefaults(t *testing.T) {
	// Passing nil config should use DefaultBarcode2DLayoutConfig without panic.
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	scene, err := Layout(g, nil)
	if err != nil {
		t.Fatalf("Layout with nil config returned error: %v", err)
	}

	// With defaults: moduleSizePx=10, quietZoneModules=4
	// totalWidth = (5 + 2*4) * 10 = 130
	expectedW := (5 + 2*4) * 10
	if scene.Width != expectedW {
		t.Errorf("Width: got %d, want %d", scene.Width, expectedW)
	}
}

// ============================================================================
// Layout — square mode
// ============================================================================

func TestLayoutSquareDimensions(t *testing.T) {
	// A 21×21 grid with moduleSizePx=10, quietZoneModules=4 (QR Code v1 typical).
	g := MakeModuleGrid(21, 21, ModuleShapeSquare)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 4,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	// totalWidth  = (21 + 2*4) * 10 = 290
	// totalHeight = (21 + 2*4) * 10 = 290
	expectedW := (21 + 2*4) * 10
	expectedH := (21 + 2*4) * 10

	if scene.Width != expectedW {
		t.Errorf("Width: got %d, want %d", scene.Width, expectedW)
	}
	if scene.Height != expectedH {
		t.Errorf("Height: got %d, want %d", scene.Height, expectedH)
	}
}

func TestLayoutSquareDifferentDimensions(t *testing.T) {
	// Non-square grid: 10 rows × 15 cols, moduleSizePx=8, quietZoneModules=2
	g := MakeModuleGrid(10, 15, ModuleShapeSquare)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     8,
		QuietZoneModules: 2,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	// totalWidth  = (15 + 2*2) * 8 = 152
	// totalHeight = (10 + 2*2) * 8 = 112
	expectedW := (15 + 2*2) * 8
	expectedH := (10 + 2*2) * 8

	if scene.Width != expectedW {
		t.Errorf("Width: got %d, want %d", scene.Width, expectedW)
	}
	if scene.Height != expectedH {
		t.Errorf("Height: got %d, want %d", scene.Height, expectedH)
	}
}

func TestLayoutSquareBackgroundRectIsFirst(t *testing.T) {
	// The very first instruction must be a full-canvas background PaintRect.
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 2,
		Foreground:       "#000000",
		Background:       "#aabbcc",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	if len(scene.Instructions) == 0 {
		t.Fatal("scene has no instructions")
	}

	bg := rectAt(t, scene, 0)

	// Background rect should cover the entire canvas.
	if bg.X != 0 || bg.Y != 0 {
		t.Errorf("background rect origin: got (%d,%d), want (0,0)", bg.X, bg.Y)
	}
	if bg.Width != scene.Width || bg.Height != scene.Height {
		t.Errorf("background rect size: got %dx%d, want %dx%d", bg.Width, bg.Height, scene.Width, scene.Height)
	}
	if bg.Fill != "#aabbcc" {
		t.Errorf("background fill: got %q, want %q", bg.Fill, "#aabbcc")
	}
}

func TestLayoutSquareAllLightProducesOnlyBackground(t *testing.T) {
	// An all-light grid should produce exactly one instruction: the background rect.
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 2,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	if len(scene.Instructions) != 1 {
		t.Errorf("all-light grid should produce 1 instruction (background), got %d", len(scene.Instructions))
	}
}

func TestLayoutSquareDarkModuleProducesPaintRect(t *testing.T) {
	// A grid with exactly one dark module should produce 2 instructions:
	// background rect + one dark rect.
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	g = mustSetModule(t, g, 2, 3, true) // row=2, col=3

	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 0, // no quiet zone to keep math simple
		Foreground:       "#112233",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	if len(scene.Instructions) != 2 {
		t.Errorf("expected 2 instructions, got %d", len(scene.Instructions))
	}

	dark := rectAt(t, scene, 1)

	// With quietZone=0, moduleSizePx=10: x=col*10=30, y=row*10=20, w=10, h=10
	if dark.X != 30 || dark.Y != 20 || dark.Width != 10 || dark.Height != 10 {
		t.Errorf("dark rect: got x=%d y=%d w=%d h=%d, want x=30 y=20 w=10 h=10",
			dark.X, dark.Y, dark.Width, dark.Height)
	}
	if dark.Fill != "#112233" {
		t.Errorf("dark rect fill: got %q, want %q", dark.Fill, "#112233")
	}
}

func TestLayoutSquareDarkModuleWithQuietZone(t *testing.T) {
	// Dark module at (0,0) with quiet zone of 4 and size 10 should land at (40,40).
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	g = mustSetModule(t, g, 0, 0, true)

	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 4,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	dark := rectAt(t, scene, 1)
	// quietZonePx = 4 * 10 = 40
	if dark.X != 40 || dark.Y != 40 {
		t.Errorf("dark rect at (0,0) with quiet zone 4: got (%d,%d), want (40,40)", dark.X, dark.Y)
	}
}

func TestLayoutSquareMultipleDarkModulesCount(t *testing.T) {
	// 3 dark modules → 4 instructions (1 background + 3 dark rects).
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	g = mustSetModule(t, g, 0, 0, true)
	g = mustSetModule(t, g, 1, 1, true)
	g = mustSetModule(t, g, 4, 4, true)

	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 0,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	if len(scene.Instructions) != 4 {
		t.Errorf("expected 4 instructions, got %d", len(scene.Instructions))
	}
}

func TestLayoutSquareSceneBackground(t *testing.T) {
	// Scene.Background should match the config background.
	g := MakeModuleGrid(3, 3, ModuleShapeSquare)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 0,
		Foreground:       "#000000",
		Background:       "#f0f0f0",
		ModuleShape:      ModuleShapeSquare,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	if scene.Background != "#f0f0f0" {
		t.Errorf("scene.Background: got %q, want %q", scene.Background, "#f0f0f0")
	}
}

// ============================================================================
// Layout — hex mode
// ============================================================================

func TestLayoutHexProducesPaintPathForDarkModule(t *testing.T) {
	// A single dark module in a hex grid should produce 2 instructions:
	// background rect + one PaintPath.
	g := MakeModuleGrid(3, 3, ModuleShapeHex)
	g = mustSetModule(t, g, 0, 0, true)

	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 0,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeHex,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	if len(scene.Instructions) != 2 {
		t.Errorf("expected 2 instructions, got %d", len(scene.Instructions))
	}

	path := pathAt(t, scene, 1)

	// A flat-top hexagon has 7 commands: move_to + 5 line_to + close.
	if len(path.Commands) != 7 {
		t.Errorf("hex path should have 7 commands (move_to + 5 line_to + close), got %d", len(path.Commands))
	}

	// First command must be move_to.
	if path.Commands[0].Kind != "move_to" {
		t.Errorf("first command: got %q, want %q", path.Commands[0].Kind, "move_to")
	}

	// Commands 1–5 must be line_to.
	for i := 1; i <= 5; i++ {
		if path.Commands[i].Kind != "line_to" {
			t.Errorf("command[%d]: got %q, want %q", i, path.Commands[i].Kind, "line_to")
		}
	}

	// Last command must be close.
	if path.Commands[6].Kind != "close" {
		t.Errorf("last command: got %q, want %q", path.Commands[6].Kind, "close")
	}
}

func TestLayoutHexOddRowOffset(t *testing.T) {
	// Odd rows should be offset right by hexWidth/2 compared to even rows.
	// Compare two grids: one dark module at (0,0) and one at (1,0).
	// The (1,0) module center x should be hexWidth/2 larger than (0,0) center x.

	g := MakeModuleGrid(2, 2, ModuleShapeHex)
	g0 := mustSetModule(t, g, 0, 0, true) // row=0 (even)
	g1 := mustSetModule(t, g, 1, 0, true) // row=1 (odd)

	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 0,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeHex,
	}

	scene0, err := Layout(g0, &cfg)
	if err != nil {
		t.Fatalf("Layout g0: %v", err)
	}
	scene1, err := Layout(g1, &cfg)
	if err != nil {
		t.Fatalf("Layout g1: %v", err)
	}

	path0 := pathAt(t, scene0, 1) // move_to is at vertex 0
	path1 := pathAt(t, scene1, 1)

	// Vertex 0 of the hexagon is at angle 0 (positive X) from the center.
	// Its X coordinate is cx + circumR * cos(0) = cx + circumR.
	// The center cx for row=0,col=0 is 0 (no quiet zone), and for row=1,col=0
	// it is hexWidth/2 = 5.
	//
	// So moveTo.X for row=1 should be exactly hexWidth/2 more than for row=0.
	hexWidth := 10.0
	diff := path1.Commands[0].X - path0.Commands[0].X
	if !almostEqual(diff, hexWidth/2) {
		t.Errorf("odd-row x offset: got %v, want %v (hexWidth/2=%v)", diff, hexWidth/2, hexWidth/2)
	}
}

func TestLayoutHexAllLightProducesOnlyBackground(t *testing.T) {
	g := MakeModuleGrid(4, 4, ModuleShapeHex)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 0,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeHex,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	if len(scene.Instructions) != 1 {
		t.Errorf("all-light hex grid should produce 1 instruction, got %d", len(scene.Instructions))
	}
}

func TestLayoutHexDimensions(t *testing.T) {
	// A 33×30 MaxiCode grid with moduleSizePx=10, quietZoneModules=1.
	// hexWidth  = 10
	// hexHeight = 10 * (√3/2) ≈ 8.660
	// totalW = (30 + 2*1) * 10 + 10/2 = 325
	// totalH = (33 + 2*1) * (10 * √3/2) ≈ 303.1...  → round to nearest int

	g := MakeModuleGrid(33, 30, ModuleShapeHex)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 1,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeHex,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	hexHeight := 10.0 * (math.Sqrt(3) / 2)
	expectedW := int(math.Round((float64(30)+2*1)*10 + 10.0/2))
	expectedH := int(math.Round((float64(33)+2*1) * hexHeight))

	if scene.Width != expectedW {
		t.Errorf("Width: got %d, want %d", scene.Width, expectedW)
	}
	if scene.Height != expectedH {
		t.Errorf("Height: got %d, want %d", scene.Height, expectedH)
	}
}

func TestLayoutHexVertexAngles(t *testing.T) {
	// Verify the 6 vertices of the hex path are at correct angles from the center.
	// For row=0, col=0, quietZone=0: center cx = circumR (vertex 0 is at 0°), cy = 0
	// Wait — center is at (cx, cy) and vertex i is at (cx + R*cos(i*60°), cy + R*sin(i*60°)).
	// For row=0, col=0, no quiet zone: cx=0, cy=0, so vertex 0 is at (circumR, 0).

	s := 10.0
	circumR := s / math.Sqrt(3)

	g := MakeModuleGrid(2, 2, ModuleShapeHex)
	g = mustSetModule(t, g, 0, 0, true)

	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     s,
		QuietZoneModules: 0,
		Foreground:       "#000000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeHex,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout returned error: %v", err)
	}

	path := pathAt(t, scene, 1)

	// The first 6 commands (move_to + 5 line_to) give vertex positions.
	const degToRad = math.Pi / 180
	for i := 0; i < 6; i++ {
		angle := float64(i) * 60 * degToRad
		// Center is at (0, 0) for row=0, col=0, quietZone=0.
		wantX := circumR * math.Cos(angle)
		wantY := circumR * math.Sin(angle)

		gotX := path.Commands[i].X
		gotY := path.Commands[i].Y

		if !almostEqual(gotX, wantX) || !almostEqual(gotY, wantY) {
			t.Errorf("vertex %d: got (%.6f, %.6f), want (%.6f, %.6f)",
				i, gotX, gotY, wantX, wantY)
		}
	}
}

func TestLayoutHexPathFill(t *testing.T) {
	// The path's Fill should be the configured foreground color.
	g := MakeModuleGrid(2, 2, ModuleShapeHex)
	g = mustSetModule(t, g, 0, 0, true)

	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:     10,
		QuietZoneModules: 0,
		Foreground:       "#ff0000",
		Background:       "#ffffff",
		ModuleShape:      ModuleShapeHex,
	}

	scene, err := Layout(g, &cfg)
	if err != nil {
		t.Fatalf("Layout: %v", err)
	}

	path := pathAt(t, scene, 1)
	if path.Fill != "#ff0000" {
		t.Errorf("path.Fill: got %q, want %q", path.Fill, "#ff0000")
	}
}

// ============================================================================
// Error type checks
// ============================================================================

func TestInvalidBarcode2DConfigErrorIsBarcode2DError(t *testing.T) {
	// InvalidBarcode2DConfigError should satisfy errors.As for both its own type
	// and the embedded Barcode2DError.
	g := MakeModuleGrid(5, 5, ModuleShapeSquare)
	cfg := Barcode2DLayoutConfig{
		ModuleSizePx:    0, // invalid
		ModuleShape:     ModuleShapeSquare,
		QuietZoneModules: 0,
	}

	_, err := Layout(g, &cfg)
	if err == nil {
		t.Fatal("expected error")
	}

	// Check IsInvalidBarcode2DConfigError helper.
	if !IsInvalidBarcode2DConfigError(err) {
		t.Error("IsInvalidBarcode2DConfigError should return true for InvalidBarcode2DConfigError")
	}

	// Check errors.As for InvalidBarcode2DConfigError directly.
	var cfgErr *InvalidBarcode2DConfigError
	if !errors.As(err, &cfgErr) {
		t.Error("errors.As should find *InvalidBarcode2DConfigError in the error chain")
	}
}

func TestBarcode2DErrorMessage(t *testing.T) {
	// Error message should start with "barcode-2d:".
	e := &Barcode2DError{Message: "test error"}
	got := e.Error()
	want := "barcode-2d: test error"
	if got != want {
		t.Errorf("Error(): got %q, want %q", got, want)
	}
}

// ============================================================================
// buildFlatTopHexPath — unit test
// ============================================================================

func TestBuildFlatTopHexPathCommandCount(t *testing.T) {
	cmds := buildFlatTopHexPath(0, 0, 5)
	// Expected: 1 move_to + 5 line_to + 1 close = 7 total
	if len(cmds) != 7 {
		t.Errorf("expected 7 commands, got %d", len(cmds))
	}
}

func TestBuildFlatTopHexPathKinds(t *testing.T) {
	cmds := buildFlatTopHexPath(10, 20, 5)

	if cmds[0].Kind != "move_to" {
		t.Errorf("cmd[0].Kind: got %q, want %q", cmds[0].Kind, "move_to")
	}
	for i := 1; i <= 5; i++ {
		if cmds[i].Kind != "line_to" {
			t.Errorf("cmd[%d].Kind: got %q, want %q", i, cmds[i].Kind, "line_to")
		}
	}
	if cmds[6].Kind != "close" {
		t.Errorf("cmd[6].Kind: got %q, want %q", cmds[6].Kind, "close")
	}
}

func TestBuildFlatTopHexPathCenter(t *testing.T) {
	// The centroid of the 6 vertices should be very close to (cx, cy).
	cx, cy, R := 50.0, 75.0, 10.0
	cmds := buildFlatTopHexPath(cx, cy, R)

	sumX, sumY := 0.0, 0.0
	for i := 0; i < 6; i++ {
		sumX += cmds[i].X
		sumY += cmds[i].Y
	}

	if !almostEqual(sumX/6, cx) || !almostEqual(sumY/6, cy) {
		t.Errorf("centroid: got (%.4f, %.4f), want (%.4f, %.4f)", sumX/6, sumY/6, cx, cy)
	}
}

func TestBuildFlatTopHexPathAllVerticesAtCircumR(t *testing.T) {
	// Every vertex should be exactly circumR away from the center.
	cx, cy, R := 30.0, 40.0, 8.0
	cmds := buildFlatTopHexPath(cx, cy, R)

	for i := 0; i < 6; i++ {
		dx := cmds[i].X - cx
		dy := cmds[i].Y - cy
		dist := math.Sqrt(dx*dx + dy*dy)
		if !almostEqual(dist, R) {
			t.Errorf("vertex %d distance from center: got %.6f, want %.6f", i, dist, R)
		}
	}
}

// ============================================================================
// ModuleAnnotation and AnnotatedModuleGrid (compile-time / structure checks)
// ============================================================================

func TestModuleAnnotationFields(t *testing.T) {
	// Verify that the struct can be constructed with all fields.
	codewordIdx := uint32(5)
	bitIdx := uint32(3)
	ann := ModuleAnnotation{
		Role:          ModuleRoleData,
		Dark:          true,
		CodewordIndex: &codewordIdx,
		BitIndex:      &bitIdx,
		Metadata:      map[string]string{"format_role": "qr:dark-module"},
	}

	if ann.Role != ModuleRoleData {
		t.Errorf("Role: got %v, want ModuleRoleData", ann.Role)
	}
	if *ann.CodewordIndex != 5 {
		t.Errorf("CodewordIndex: got %d, want 5", *ann.CodewordIndex)
	}
	if *ann.BitIndex != 3 {
		t.Errorf("BitIndex: got %d, want 3", *ann.BitIndex)
	}
}

func TestAnnotatedModuleGridStructure(t *testing.T) {
	// Verify AnnotatedModuleGrid can be constructed with a grid and annotations.
	g := MakeModuleGrid(2, 2, ModuleShapeSquare)
	ann := AnnotatedModuleGrid{
		Grid: g,
		Annotations: [][]*ModuleAnnotation{
			{nil, nil},
			{nil, nil},
		},
	}

	if ann.Grid.Rows != 2 || ann.Grid.Cols != 2 {
		t.Errorf("AnnotatedModuleGrid grid dimensions: got %dx%d", ann.Grid.Rows, ann.Grid.Cols)
	}
}

// ============================================================================
// ModuleRole constants
// ============================================================================

func TestModuleRoleValues(t *testing.T) {
	// All eight roles must be distinct.
	roles := []ModuleRole{
		ModuleRoleFinder,
		ModuleRoleSeparator,
		ModuleRoleTiming,
		ModuleRoleAlignment,
		ModuleRoleFormat,
		ModuleRoleData,
		ModuleRoleEcc,
		ModuleRolePadding,
	}

	seen := make(map[ModuleRole]bool)
	for _, r := range roles {
		if seen[r] {
			t.Errorf("duplicate ModuleRole value: %v", r)
		}
		seen[r] = true
	}

	if len(seen) != 8 {
		t.Errorf("expected 8 distinct ModuleRole values, got %d", len(seen))
	}
}
