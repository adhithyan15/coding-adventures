// Package pdf417 tests exercise the PDF417 encoder against ISO/IEC 15438:2015.
//
// Coverage targets:
//   - Encode / EncodeBytes public API.
//   - Symbol dimension selection (auto vs explicit columns).
//   - Row indicator presence and symbol grid dimensions.
//   - ECC level selection (auto and explicit).
//   - Error handling (oversized input, invalid options).
//   - Determinism.
//   - EncodeToScene wrapper.
package pdf417

import (
	"strings"
	"testing"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
)

// ============================================================================
// Helpers
// ============================================================================

func defaultOpts() Options {
	return Options{ECCLevel: ECCLevelAuto}
}

// ============================================================================
// Version
// ============================================================================

func TestVersion(t *testing.T) {
	if Version == "" {
		t.Fatal("Version must not be empty")
	}
	if !strings.HasPrefix(Version, "0.") {
		t.Fatalf("unexpected Version %q", Version)
	}
}

// ============================================================================
// Encode (string convenience wrapper)
// ============================================================================

func TestEncodeReturnsGrid(t *testing.T) {
	grid := Encode("A")
	if grid == nil {
		t.Fatal("Encode returned nil")
	}
	if grid.Rows < minRows {
		t.Fatalf("rows %d < minRows %d", grid.Rows, minRows)
	}
	if grid.Cols < 1 {
		t.Fatalf("cols %d < 1", grid.Cols)
	}
}

func TestEncodeEmptyString(t *testing.T) {
	grid := Encode("")
	if grid == nil {
		t.Fatal("Encode(\"\") returned nil")
	}
}

// ============================================================================
// EncodeBytes — basic correctness
// ============================================================================

func TestEncodeBytesReturnsGrid(t *testing.T) {
	grid, err := EncodeBytes([]byte("Hello, PDF417!"), defaultOpts())
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid == nil {
		t.Fatal("nil grid")
	}
}

func TestEncodeBytesGridDimensions(t *testing.T) {
	data := []byte("AAMVA driver license data")
	grid, err := EncodeBytes(data, defaultOpts())
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// PDF417 symbol dimensions: rows in [3, 90], cols in [1, 30].
	if grid.Rows < minRows || grid.Rows > maxRows {
		t.Errorf("rows %d out of [%d, %d]", grid.Rows, minRows, maxRows)
	}
	if grid.Cols < 1 {
		t.Errorf("cols %d < 1", grid.Cols)
	}
}

func TestEncodeBytesModuleCount(t *testing.T) {
	grid, err := EncodeBytes([]byte("X"), defaultOpts())
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(grid.Modules) == 0 {
		t.Fatal("modules slice is empty")
	}
	for r, row := range grid.Modules {
		if uint32(len(row)) != grid.Cols {
			t.Errorf("row %d: len=%d want %d", r, len(row), grid.Cols)
		}
	}
}

// ============================================================================
// ECC level
// ============================================================================

func TestAutoECCLevel(t *testing.T) {
	// Two symbols should encode the same data; auto ECC must produce one.
	_, err := EncodeBytes([]byte("test"), Options{ECCLevel: ECCLevelAuto})
	if err != nil {
		t.Fatalf("auto ECC failed: %v", err)
	}
}

func TestExplicitECCLevels(t *testing.T) {
	data := []byte("test data for ecc levels")
	for level := 0; level <= 8; level++ {
		_, err := EncodeBytes(data, Options{ECCLevel: level})
		if err != nil {
			t.Errorf("ECC level %d: unexpected error: %v", level, err)
		}
	}
}

func TestHigherECCMakesSymbolLarger(t *testing.T) {
	data := []byte("data that fits in many symbol sizes")
	low, err := EncodeBytes(data, Options{ECCLevel: 0})
	if err != nil {
		t.Fatalf("ECC 0: %v", err)
	}
	high, err := EncodeBytes(data, Options{ECCLevel: 5})
	if err != nil {
		t.Fatalf("ECC 5: %v", err)
	}
	// Higher ECC produces more codewords → larger symbol area.
	lowArea := low.Rows * low.Cols
	highArea := high.Rows * high.Cols
	if highArea < lowArea {
		t.Errorf("ECC 5 area (%d) < ECC 0 area (%d); expected ≥", highArea, lowArea)
	}
}

// ============================================================================
// Explicit column count
// ============================================================================

func TestExplicitColumns(t *testing.T) {
	for _, cols := range []int{3, 5, 10, 20} {
		opts := Options{ECCLevel: ECCLevelAuto, Columns: cols}
		grid, err := EncodeBytes([]byte("column test data with some length"), opts)
		if err != nil {
			t.Errorf("cols=%d: unexpected error: %v", cols, err)
			continue
		}
		// The actual data-column count in the grid depends on start/stop bars
		// and row indicators — just check rows/cols are non-zero.
		if grid.Rows == 0 || grid.Cols == 0 {
			t.Errorf("cols=%d: zero-dim grid (%d×%d)", cols, grid.Rows, grid.Cols)
		}
	}
}

// ============================================================================
// Determinism
// ============================================================================

func TestDeterminism(t *testing.T) {
	data := []byte("determinism check")
	g1, _ := EncodeBytes(data, defaultOpts())
	g2, _ := EncodeBytes(data, defaultOpts())
	if g1.Rows != g2.Rows || g1.Cols != g2.Cols {
		t.Fatalf("non-deterministic dimensions: (%d×%d) vs (%d×%d)",
			g1.Rows, g1.Cols, g2.Rows, g2.Cols)
	}
	for r := range g1.Modules {
		for c := range g1.Modules[r] {
			if g1.Modules[r][c] != g2.Modules[r][c] {
				t.Fatalf("non-deterministic module at (%d,%d)", r, c)
			}
		}
	}
}

// ============================================================================
// Different inputs produce different grids
// ============================================================================

func TestDifferentInputsDifferentGrid(t *testing.T) {
	g1, _ := EncodeBytes([]byte("AAAA"), defaultOpts())
	g2, _ := EncodeBytes([]byte("ZZZZ"), defaultOpts())
	same := g1.Rows == g2.Rows && g1.Cols == g2.Cols
	if same {
		for r := range g1.Modules {
			for c := range g1.Modules[r] {
				if g1.Modules[r][c] != g2.Modules[r][c] {
					return // found a difference — good
				}
			}
		}
		t.Error("different inputs produced identical grids")
	}
	// different dimensions → clearly different, OK
}

// ============================================================================
// Larger data grows symbol
// ============================================================================

func TestLargerDataGrowsSymbol(t *testing.T) {
	small, _ := EncodeBytes([]byte("A"), defaultOpts())
	large, _ := EncodeBytes([]byte(strings.Repeat("A", 200)), defaultOpts())
	smallArea := small.Rows * small.Cols
	largeArea := large.Rows * large.Cols
	if largeArea <= smallArea {
		t.Errorf("large area (%d) not > small area (%d)", largeArea, smallArea)
	}
}

// ============================================================================
// EncodeToScene
// ============================================================================

func TestEncodeToScene(t *testing.T) {
	scene, err := EncodeToScene([]byte("scene test"), defaultOpts(), nil)
	if err != nil {
		t.Fatalf("EncodeToScene: %v", err)
	}
	if scene.Width <= 0 || scene.Height <= 0 {
		t.Errorf("scene dimensions (%v×%v) must be positive", scene.Width, scene.Height)
	}
}

func TestEncodeToSceneCustomConfig(t *testing.T) {
	cfg := &barcode2d.Barcode2DLayoutConfig{
		ModuleSizePx:     4.0,
		QuietZoneModules: 2,
	}
	scene, err := EncodeToScene([]byte("custom config"), defaultOpts(), cfg)
	if err != nil {
		t.Fatalf("EncodeToScene: %v", err)
	}
	if scene.Width <= 0 {
		t.Error("expected positive width")
	}
}

// ============================================================================
// Module grid shape (every row must have Cols modules)
// ============================================================================

func TestGridShape(t *testing.T) {
	grid, err := EncodeBytes([]byte("shape check"), defaultOpts())
	if err != nil {
		t.Fatalf("%v", err)
	}
	if uint32(len(grid.Modules)) != grid.Rows {
		t.Errorf("len(modules)=%d != Rows=%d", len(grid.Modules), grid.Rows)
	}
	for i, row := range grid.Modules {
		if uint32(len(row)) != grid.Cols {
			t.Errorf("row %d: len=%d != Cols=%d", i, len(row), grid.Cols)
		}
	}
}
