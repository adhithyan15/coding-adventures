// Package barcode2d provides the shared 2D barcode abstraction layer.
//
// # What is a 2D barcode?
//
// A 2D barcode encodes information as a 2-dimensional pattern of dark and light
// modules (cells), unlike 1D barcodes which are just a row of vertical bars.
// Common 2D barcodes include QR Code, Data Matrix, Aztec Code, PDF417, and
// MaxiCode. They differ in how they arrange modules and perform error correction,
// but they all produce the same fundamental artifact: a rectangular grid of dark
// and light cells.
//
// # Where this fits in the pipeline
//
//	Input data
//	  → format encoder (qr-code, data-matrix, aztec…)
//	  → ModuleGrid          ← produced by the encoder
//	  → Layout()            ← THIS PACKAGE converts to pixels
//	  → PaintScene          ← consumed by paint-vm
//	  → backend (SVG, Metal, Canvas, terminal…)
//
// All coordinates before Layout() are measured in "module units" — abstract
// grid steps that have no physical size yet. Only Layout() multiplies by
// ModuleSizePx to produce real pixel coordinates. This separation means
// encoders never need to know anything about screen resolution or output format.
//
// # Supported module shapes
//
//   - Square (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
//     Each module becomes a PaintRect.
//
//   - Hex (flat-top hexagons): used by MaxiCode (ISO/IEC 16023). Each module
//     becomes a PaintPath tracing six vertices.
//
// # Annotations
//
// The optional AnnotatedModuleGrid adds per-module role information useful for
// visualizers (highlighting finder patterns, data codewords, etc.). Annotations
// are never required for rendering — Layout() only reads the boolean Modules grid.
package barcode2d

import (
	"errors"
	"fmt"
	"math"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

// Version is the semantic version of this package.
const Version = "0.1.0"

// ============================================================================
// ModuleShape — square vs. hex
// ============================================================================

// ModuleShape describes the geometric shape of every module in a grid.
//
// Think of it like the shape of tiles on a floor:
//   - Square tiles (the overwhelming majority of 2D barcodes)
//   - Hexagonal tiles (MaxiCode, used by UPS and other carriers)
type ModuleShape int

const (
	// ModuleShapeSquare represents square modules, used by QR Code, Data Matrix,
	// Aztec Code, and PDF417. The overwhelmingly common shape.
	ModuleShapeSquare ModuleShape = iota

	// ModuleShapeHex represents flat-top hexagonal modules, used exclusively by
	// MaxiCode (ISO/IEC 16023). MaxiCode is printed on parcels and read by
	// high-speed conveyor-belt scanners — the hex tiling packs more information
	// into the fixed 1 inch × 1 inch symbol footprint.
	ModuleShapeHex
)

// ============================================================================
// ModuleGrid — the universal output of every 2D barcode encoder
// ============================================================================

// ModuleGrid is the universal intermediate representation produced by every 2D
// barcode encoder. It is a 2D boolean grid:
//
//	Modules[row][col] == true   →  dark module (ink / filled)
//	Modules[row][col] == false  →  light module (background / empty)
//
// Row 0 is the top row. Col 0 is the leftmost column. This matches the natural
// reading order used in every 2D barcode standard.
//
// # Immutability
//
// ModuleGrid is designed to be treated as immutable. Use SetModule() to produce
// a new grid with one module changed, rather than mutating in place. This
// makes encoders easy to test and compose. For example, a QR Code encoder
// trying all 8 mask patterns can keep the pre-mask grid and apply each mask to
// a fresh copy without risk of contamination.
//
// # MaxiCode fixed size
//
// MaxiCode grids are always 33 rows × 30 columns with ModuleShapeHex. The
// physical symbol is approximately 1 inch × 1 inch.
type ModuleGrid struct {
	// Cols is the number of columns (width) in the grid.
	Cols uint32
	// Rows is the number of rows (height) in the grid.
	Rows uint32
	// Modules is the 2D boolean grid. Access as Modules[row][col].
	// true = dark module, false = light module.
	Modules [][]bool
	// ModuleShape describes whether modules are square or hexagonal.
	ModuleShape ModuleShape
}

// ============================================================================
// ModuleRole — what a module structurally represents
// ============================================================================

// ModuleRole describes the structural purpose of a module within its symbol.
//
// Think of it like the roles in a printed form: some boxes hold data, others
// are just alignment guides, and some carry error-correction redundancy. These
// roles are generic enough to cover all 2D barcode formats.
type ModuleRole int

const (
	// ModuleRoleFinder marks locator patterns that help scanners detect and
	// orient the symbol. QR Code uses three 7×7 corner finder patterns.
	ModuleRoleFinder ModuleRole = iota

	// ModuleRoleSeparator marks the quiet border strips between finder patterns
	// and the data area. These modules are always light in a valid symbol.
	ModuleRoleSeparator

	// ModuleRoleTiming marks the alternating dark/light calibration strips.
	// They let the scanner measure module size and compensate for skew.
	ModuleRoleTiming

	// ModuleRoleAlignment marks secondary locator patterns in large QR symbols.
	// They help the scanner correct for lens distortion.
	ModuleRoleAlignment

	// ModuleRoleFormat marks modules that encode ECC level, mask pattern, or
	// other symbol-level metadata. The scanner reads these before the data area.
	ModuleRoleFormat

	// ModuleRoleData marks modules that carry one bit of an encoded codeword.
	ModuleRoleData

	// ModuleRoleEcc marks modules that carry one bit of an error correction
	// codeword. These enable the scanner to recover the message even when part
	// of the symbol is damaged or obscured.
	ModuleRoleEcc

	// ModuleRolePadding marks filler bits used when the message is shorter than
	// the symbol's capacity. QR Code pads with alternating 0xEC and 0x11 bytes.
	ModuleRolePadding
)

// ============================================================================
// ModuleAnnotation — per-module role metadata for visualizers
// ============================================================================

// ModuleAnnotation carries per-module role information used by visualizers to
// colour-code the symbol anatomy.
//
// Annotations are entirely optional. Layout() only reads ModuleGrid.Modules
// and never looks at annotations. You would use annotations when building a
// teaching tool that highlights finder patterns in red, data modules in blue,
// and ECC modules in green.
//
// # CodewordIndex and BitIndex
//
// For Data and Ecc modules these fields identify exactly which bit in which
// codeword this module carries:
//   - CodewordIndex — zero-based index into the interleaved codeword stream
//   - BitIndex      — zero-based bit index within that codeword, 0 = MSB
//
// For structural modules (Finder, Timing, etc.) these pointers are nil.
type ModuleAnnotation struct {
	// Role is the structural purpose of this module.
	Role ModuleRole
	// Dark is true when this module is dark (ink/filled).
	Dark bool
	// CodewordIndex identifies which codeword this module belongs to.
	// Only set for Data and Ecc modules.
	CodewordIndex *uint32
	// BitIndex identifies which bit within the codeword this module carries.
	// Only set for Data and Ecc modules.
	BitIndex *uint32
	// Metadata holds arbitrary format-specific annotations.
	// Key "format_role" carries namespaced strings like "qr:dark-module".
	Metadata map[string]string
}

// ============================================================================
// AnnotatedModuleGrid — ModuleGrid with per-module role annotations
// ============================================================================

// AnnotatedModuleGrid is a ModuleGrid extended with per-module role annotations.
//
// The Annotations slice mirrors Modules exactly in dimensions:
//
//	Annotations[row][col] corresponds to Modules[row][col]
//
// A nil annotation means "no annotation for this module". This can happen when
// an encoder only annotates some modules (e.g. only the data region).
type AnnotatedModuleGrid struct {
	Grid        ModuleGrid
	Annotations [][]*ModuleAnnotation
}

// ============================================================================
// Barcode2DLayoutConfig — pixel-level rendering options
// ============================================================================

// Barcode2DLayoutConfig controls how Layout() converts a ModuleGrid into pixels.
//
// All fields have sensible defaults in DefaultBarcode2DLayoutConfig, so you
// only need to set the fields you want to override.
//
// # ModuleSizePx
//
// The pixel size of one module. For square modules this is both width and height.
// For hex modules it is the hexagon's flat-to-flat width (which equals its side
// length for a regular hexagon). Must be > 0.
//
// # QuietZoneModules
//
// The number of module-widths added as a blank margin on every side of the
// symbol. Standards differ: QR Code requires 4, Data Matrix requires 1,
// MaxiCode requires 1. Must be >= 0.
//
// # ModuleShape
//
// Must match ModuleGrid.ModuleShape. If they disagree Layout() returns an error.
// This sanity check prevents accidentally rendering a MaxiCode hex grid with
// square modules.
type Barcode2DLayoutConfig struct {
	// ModuleSizePx is the size of one module in pixels. Must be > 0.
	ModuleSizePx float64
	// QuietZoneModules is the blank margin width on each side, in module units.
	QuietZoneModules uint32
	// Foreground is the hex color string for dark modules, e.g. "#000000".
	Foreground string
	// Background is the hex color string for light modules, e.g. "#ffffff".
	Background string
	// ShowAnnotations enables annotation-aware rendering (reserved for future use).
	ShowAnnotations bool
	// ModuleShape must match ModuleGrid.ModuleShape.
	ModuleShape ModuleShape
}

// DefaultBarcode2DLayoutConfig provides sensible defaults for Layout().
//
//	ModuleSizePx     10        Produces a readable QR at ~210×210 px for v1
//	QuietZoneModules  4        QR Code minimum per ISO/IEC 18004 §6.3.8
//	Foreground        #000000  Black ink
//	Background        #ffffff  White paper
//	ShowAnnotations   false    Off by default; opt-in for visualizers
//	ModuleShape       Square   The overwhelmingly common case
var DefaultBarcode2DLayoutConfig = Barcode2DLayoutConfig{
	ModuleSizePx:     10,
	QuietZoneModules: 4,
	Foreground:       "#000000",
	Background:       "#ffffff",
	ShowAnnotations:  false,
	ModuleShape:      ModuleShapeSquare,
}

// ============================================================================
// Error types
// ============================================================================

// Barcode2DError is the base error type for all barcode-2d errors.
// Use errors.As(err, &Barcode2DError{}) to check if an error originated here.
type Barcode2DError struct {
	// Message is the human-readable error description.
	Message string
}

func (e *Barcode2DError) Error() string {
	return fmt.Sprintf("barcode-2d: %s", e.Message)
}

// InvalidBarcode2DConfigError is returned by Layout() when the configuration
// is invalid. Specific cases:
//   - ModuleSizePx <= 0
//   - QuietZoneModules is unreasonably negative (not possible with uint32, but
//     caught for defensive completeness)
//   - Config.ModuleShape does not match Grid.ModuleShape
type InvalidBarcode2DConfigError struct {
	// Embed the base error so callers can check for either type.
	Barcode2DError
}

// newInvalidConfig creates a wrapped InvalidBarcode2DConfigError with the
// given message.
func newInvalidConfig(msg string) *InvalidBarcode2DConfigError {
	return &InvalidBarcode2DConfigError{Barcode2DError{Message: msg}}
}

// IsInvalidBarcode2DConfigError reports whether err (or any error in its chain)
// is an InvalidBarcode2DConfigError.
func IsInvalidBarcode2DConfigError(err error) bool {
	var target *InvalidBarcode2DConfigError
	return errors.As(err, &target)
}

// ============================================================================
// MakeModuleGrid — create an all-light grid
// ============================================================================

// MakeModuleGrid creates a new ModuleGrid of the given dimensions with every
// module set to false (light / background).
//
// This is the starting point for every 2D barcode encoder. The encoder calls
// MakeModuleGrid(rows, cols, ModuleShapeSquare) and then calls SetModule() to
// paint dark modules one by one as it places finder patterns, timing strips,
// data bits, and error correction bits.
//
// Example — start a 21×21 QR Code v1 grid:
//
//	grid := MakeModuleGrid(21, 21, ModuleShapeSquare)
//	// grid.Modules[0][0] == false  (all light)
//	// grid.Rows == 21
//	// grid.Cols == 21
func MakeModuleGrid(rows, cols uint32, moduleShape ModuleShape) ModuleGrid {
	// Allocate a rows × cols slice of false values.
	// Each row is an independent slice so SetModule can replace individual rows
	// without copying the entire grid (structural sharing / persistent data structure).
	modules := make([][]bool, rows)
	for r := range modules {
		modules[r] = make([]bool, cols)
		// make([]bool, n) zero-initialises to false, so no explicit fill needed.
	}
	return ModuleGrid{
		Cols:        cols,
		Rows:        rows,
		Modules:     modules,
		ModuleShape: moduleShape,
	}
}

// ============================================================================
// SetModule — immutable single-module update
// ============================================================================

// SetModule returns a new ModuleGrid identical to grid except that the module
// at (row, col) is set to dark.
//
// This function is pure and immutable — it never modifies the input grid.
// The original grid remains valid and unchanged. Only the affected row is
// re-allocated; all other rows are shared between old and new grid (structural
// sharing, similar to a persistent tree node update).
//
// # Why immutability matters
//
// Barcode encoders often need to backtrack. For example, QR Code evaluates all
// 8 mask patterns and keeps the one with the best penalty score. Immutable grids
// make this trivial: save the grid before applying a mask, score it, discard if
// worse, keep the old one otherwise. No undo stack needed.
//
// # Out-of-bounds
//
// Returns an error if row or col is outside the grid dimensions. This is a
// programming error in the encoder, not a user-facing error, but returning an
// error is safer than a panic.
//
// Example:
//
//	g := MakeModuleGrid(3, 3, ModuleShapeSquare)
//	g2, err := SetModule(g, 1, 1, true)
//	// g.Modules[1][1] == false   (original unchanged)
//	// g2.Modules[1][1] == true
func SetModule(grid ModuleGrid, row, col uint32, dark bool) (ModuleGrid, error) {
	if row >= grid.Rows {
		return ModuleGrid{}, fmt.Errorf("SetModule: row %d out of range [0, %d]", row, grid.Rows-1)
	}
	if col >= grid.Cols {
		return ModuleGrid{}, fmt.Errorf("SetModule: col %d out of range [0, %d]", col, grid.Cols-1)
	}

	// Allocate a new top-level slice that shares all rows except the changed one.
	// This is O(rows) for the slice header copy but O(cols) for the one changed row.
	newModules := make([][]bool, grid.Rows)
	copy(newModules, grid.Modules)

	// Replace only the affected row with a fresh copy.
	newRow := make([]bool, grid.Cols)
	copy(newRow, grid.Modules[row])
	newRow[col] = dark
	newModules[row] = newRow

	return ModuleGrid{
		Cols:        grid.Cols,
		Rows:        grid.Rows,
		Modules:     newModules,
		ModuleShape: grid.ModuleShape,
	}, nil
}

// ============================================================================
// Layout — ModuleGrid → PaintScene
// ============================================================================

// Layout converts a ModuleGrid into a PaintScene ready for the PaintVM.
//
// This is the only function in the entire 2D barcode stack that knows about
// pixels. Everything above this step works in abstract module units. Everything
// below this step is handled by a paint backend (SVG renderer, terminal, etc.).
//
// # Square modules (the common case)
//
// Each dark module at (row, col) becomes one PaintRect:
//
//	quietZonePx = float64(config.QuietZoneModules) * config.ModuleSizePx
//	x = quietZonePx + float64(col) * config.ModuleSizePx
//	y = quietZonePx + float64(row) * config.ModuleSizePx
//
// Total symbol size including quiet zone on all four sides:
//
//	totalWidth  = (cols + 2 * quietZoneModules) * moduleSizePx
//	totalHeight = (rows + 2 * quietZoneModules) * moduleSizePx
//
// The scene always begins with one background PaintRect covering the full
// symbol. This ensures the quiet zone and light modules are filled even when
// the backend has a transparent default.
//
// # Hex modules (MaxiCode)
//
// Each dark module becomes one PaintPath tracing a flat-top regular hexagon.
// Odd-numbered rows are offset right by half a hexagon width to produce the
// standard interlocking hexagonal tiling:
//
//	Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
//	Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
//	Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
//
// # Validation
//
// Returns InvalidBarcode2DConfigError if:
//   - ModuleSizePx <= 0
//   - Config.ModuleShape does not match Grid.ModuleShape
//
// The config parameter may be nil, in which case DefaultBarcode2DLayoutConfig
// is used.
func Layout(grid ModuleGrid, config *Barcode2DLayoutConfig) (paintinstructions.PaintScene, error) {
	// Merge with defaults. If no config was supplied, start with the defaults.
	cfg := DefaultBarcode2DLayoutConfig
	if config != nil {
		cfg = *config
	}

	// ── Validation ──────────────────────────────────────────────────────────

	// ModuleSizePx must be positive. A zero or negative size would produce a
	// degenerate scene with zero or negative pixel dimensions.
	if cfg.ModuleSizePx <= 0 {
		return paintinstructions.PaintScene{}, newInvalidConfig(
			fmt.Sprintf("ModuleSizePx must be > 0, got %v", cfg.ModuleSizePx),
		)
	}

	// The shape in the config must agree with the shape stored in the grid.
	// Mismatching would silently produce nonsense geometry (e.g. rendering a
	// MaxiCode hex grid as a grid of tiny squares).
	if cfg.ModuleShape != grid.ModuleShape {
		return paintinstructions.PaintScene{}, newInvalidConfig(
			fmt.Sprintf(
				"config.ModuleShape (%v) does not match grid.ModuleShape (%v)",
				cfg.ModuleShape, grid.ModuleShape,
			),
		)
	}

	// Dispatch to the appropriate internal renderer.
	if cfg.ModuleShape == ModuleShapeSquare {
		return layoutSquare(grid, cfg)
	}
	return layoutHex(grid, cfg)
}

// ============================================================================
// layoutSquare — internal helper for square-module grids
// ============================================================================

// layoutSquare renders a square-module ModuleGrid into a PaintScene.
//
// Called only by Layout() after validation. Not exported because callers should
// always go through Layout() to ensure the config is validated and defaults are
// applied.
//
// Algorithm:
//  1. Compute total pixel dimensions including quiet zone on all four sides.
//  2. Emit one background PaintRect covering the entire symbol including the
//     quiet zone. This guarantees light modules and the quiet zone are rendered
//     even when the backend has a transparent default.
//  3. For each dark module, emit one filled PaintRect. Light modules are
//     implicitly covered by the background rect — no explicit light rects are
//     needed. This keeps instruction count proportional to dark module count.
func layoutSquare(grid ModuleGrid, cfg Barcode2DLayoutConfig) (paintinstructions.PaintScene, error) {
	s := cfg.ModuleSizePx
	qz := float64(cfg.QuietZoneModules) * s // quiet zone in pixels

	// Total canvas size: grid + quiet zone on each side.
	//   width  = (cols + 2 * quietZoneModules) * moduleSizePx
	//   height = (rows + 2 * quietZoneModules) * moduleSizePx
	totalW := int(math.Round((float64(grid.Cols)+2*float64(cfg.QuietZoneModules)) * s))
	totalH := int(math.Round((float64(grid.Rows)+2*float64(cfg.QuietZoneModules)) * s))

	instructions := make([]paintinstructions.PaintInstruction, 0, int(grid.Rows*grid.Cols)+1)

	// Background rect covering the entire symbol including quiet zone.
	instructions = append(instructions,
		paintinstructions.PaintRect(0, 0, totalW, totalH, cfg.Background, nil),
	)

	// One PaintRect per dark module.
	for row := uint32(0); row < grid.Rows; row++ {
		for col := uint32(0); col < grid.Cols; col++ {
			if grid.Modules[row][col] {
				// Top-left pixel corner of this module.
				x := int(math.Round(qz + float64(col)*s))
				y := int(math.Round(qz + float64(row)*s))
				w := int(math.Round(s))
				h := int(math.Round(s))
				instructions = append(instructions,
					paintinstructions.PaintRect(x, y, w, h, cfg.Foreground, nil),
				)
			}
		}
	}

	scene := paintinstructions.CreateScene(totalW, totalH, instructions, cfg.Background, nil)
	return scene, nil
}

// ============================================================================
// layoutHex — internal helper for hex-module grids (MaxiCode)
// ============================================================================

// layoutHex renders a hex-module ModuleGrid into a PaintScene.
//
// Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons arranged in
// an offset-row grid. Odd rows shift right by half a hexagon width.
//
// # Flat-top hexagon geometry
//
// A "flat-top" hexagon has two flat edges at the top and bottom (contrast with
// "pointy-top" which has a vertex at the top). MaxiCode and most industrial
// barcode standards use flat-top.
//
//	     ___
//	    /   \    ← two flat edges (top and bottom)
//	   |     |
//	    \___/
//
// For a flat-top hexagon where the flat-to-flat width equals moduleSizePx:
//
//	hexWidth  = moduleSizePx       (flat-to-flat = side length for regular hex)
//	hexHeight = moduleSizePx × (√3 / 2)   (vertical row step)
//	circumR   = moduleSizePx / √3         (center-to-vertex distance)
//
// Vertices at angles 0°, 60°, 120°, 180°, 240°, 300° from center (cx, cy):
//
//	vertex_i = ( cx + circumR×cos(i×60°),  cy + circumR×sin(i×60°) )
//
// # Tiling
//
// Odd rows shift right by hexWidth/2 to interlock with even rows:
//
//	cx = quietZonePx + col×hexWidth + (row%2)×(hexWidth/2)
//	cy = quietZonePx + row×hexHeight
func layoutHex(grid ModuleGrid, cfg Barcode2DLayoutConfig) (paintinstructions.PaintScene, error) {
	s := cfg.ModuleSizePx

	// Derive hex geometry from moduleSizePx.
	// hexWidth  = s         (the flat-to-flat distance, which is also the side length)
	// hexHeight = s × √3/2  (vertical distance between row centers in an offset grid)
	// circumR   = s / √3    (circumscribed circle radius — center to any vertex)
	hexWidth := s
	hexHeight := s * (math.Sqrt(3) / 2)
	circumR := s / math.Sqrt(3)

	qz := float64(cfg.QuietZoneModules) * s // quiet zone in pixels

	// Total canvas width includes an extra hexWidth/2 to accommodate the
	// rightward offset of odd rows. Without this, modules on odd rows near the
	// right edge would clip outside the canvas.
	//
	//   totalWidth  = (cols + 2×quiet) × hexWidth  + hexWidth/2
	//   totalHeight = (rows + 2×quiet) × hexHeight
	totalW := int(math.Round((float64(grid.Cols)+2*float64(cfg.QuietZoneModules))*hexWidth + hexWidth/2))
	totalH := int(math.Round((float64(grid.Rows)+2*float64(cfg.QuietZoneModules)) * hexHeight))

	instructions := make([]paintinstructions.PaintInstruction, 0, int(grid.Rows*grid.Cols)+1)

	// Background rect.
	instructions = append(instructions,
		paintinstructions.PaintRect(0, 0, totalW, totalH, cfg.Background, nil),
	)

	// One PaintPath (a hexagon) per dark module.
	for row := uint32(0); row < grid.Rows; row++ {
		for col := uint32(0); col < grid.Cols; col++ {
			if grid.Modules[row][col] {
				// Pixel-space center of this hexagon.
				// Odd rows shift right by hexWidth/2 to interlock with even rows.
				cx := qz + float64(col)*hexWidth + float64(row%2)*(hexWidth/2)
				cy := qz + float64(row)*hexHeight

				commands := buildFlatTopHexPath(cx, cy, circumR)
				instructions = append(instructions,
					paintinstructions.PaintPath(commands, cfg.Foreground, nil),
				)
			}
		}
	}

	scene := paintinstructions.CreateScene(totalW, totalH, instructions, cfg.Background, nil)
	return scene, nil
}

// ============================================================================
// buildFlatTopHexPath — geometry helper
// ============================================================================

// buildFlatTopHexPath builds the six PathCommands for a flat-top regular hexagon.
//
// The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300°
// from the center (cx, cy) at circumradius R:
//
//	vertex_i = ( cx + R×cos(i×60°),  cy + R×sin(i×60°) )
//
// The path structure is:
//
//	move_to   → vertex 0   (right midpoint of the hexagon)
//	line_to   → vertex 1   (bottom-right)
//	line_to   → vertex 2   (bottom-left)
//	line_to   → vertex 3   (left midpoint)
//	line_to   → vertex 4   (top-left)
//	line_to   → vertex 5   (top-right)
//	close              → back to vertex 0
//
// The circumradius R equals moduleSizePx / √3. The six vertices together
// form a closed filled hexagon when painted by the backend.
func buildFlatTopHexPath(cx, cy, circumR float64) []paintinstructions.PathCommand {
	const degToRad = math.Pi / 180
	commands := make([]paintinstructions.PathCommand, 0, 7) // 1 move + 5 line + 1 close

	// Vertex 0: move_to — lifts the pen to the first vertex.
	angle0 := 0.0 * 60 * degToRad // 0 radians
	commands = append(commands, paintinstructions.PathCommand{
		Kind: "move_to",
		X:    cx + circumR*math.Cos(angle0),
		Y:    cy + circumR*math.Sin(angle0),
	})

	// Vertices 1–5: line_to — draws the remaining five edges.
	for i := 1; i <= 5; i++ {
		angle := float64(i) * 60 * degToRad
		commands = append(commands, paintinstructions.PathCommand{
			Kind: "line_to",
			X:    cx + circumR*math.Cos(angle),
			Y:    cy + circumR*math.Sin(angle),
		})
	}

	// close — connects the last vertex back to vertex 0.
	commands = append(commands, paintinstructions.PathCommand{Kind: "close"})

	return commands
}
