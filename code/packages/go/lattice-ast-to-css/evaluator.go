package latticeasttocss

// evaluator.go — compile-time evaluation of Lattice expressions.
//
// # What Gets Evaluated?
//
// Lattice expressions appear in three places:
//
//  1. @if conditions:  @if $theme == dark { ... }
//  2. @for bounds:     @for $i from 1 through $count { ... }
//  3. @return values:  @return $multiplier * 8px;
//
// Because Lattice compiles to CSS (there is no runtime), ALL expressions are
// evaluated at compile time. This is similar to constant folding in a
// conventional compiler.
//
// # Value Types
//
// The evaluator works with nine value types that mirror CSS/Lattice semantics:
//
//	LatticeNumber     — 42, 3.14, 0 (no unit)
//	LatticeDimension  — 16px, 2em, 1.5rem (number + CSS unit)
//	LatticePercentage — 50%, 100% (number + %)
//	LatticeString     — "hello", 'world' (quoted strings)
//	LatticeIdent      — red, bold, dark (unquoted identifiers)
//	LatticeColor      — #4a90d9, #fff (hex colors)
//	LatticeBool       — true, false
//	LatticeNull       — null (falsy, like Sass null)
//	LatticeList       — red, green, blue (comma-separated, for @each)
//
// # Operator Precedence (tightest to loosest)
//
//  1. Unary minus:      -$x
//  2. Multiplication:   $a * $b
//  3. Addition:         $a + $b, $a - $b
//  4. Comparison:       ==, !=, >, >=, <=
//  5. Logical AND:      $a and $b
//  6. Logical OR:       $a or $b
//
// The grammar encodes this precedence via nested rules (or_expr → and_expr →
// comparison → additive → multiplicative → unary → primary), so the evaluator
// just recurses without needing its own precedence climbing.
//
// # Arithmetic Rules
//
// Addition and subtraction:
//   Number ± Number → Number
//   Dimension ± Dimension (same unit) → Dimension
//   Percentage ± Percentage → Percentage
//   Anything else → TypeErrorInExpression
//
// Multiplication:
//   Number × Number → Number
//   Number × Dimension → Dimension   (scaling: 2 * 8px = 16px)
//   Dimension × Number → Dimension   (commutative)
//   Number × Percentage → Percentage
//   Percentage × Number → Percentage
//   Anything else → TypeErrorInExpression

import (
	"fmt"
	"math"
	"strconv"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// LatticeValue Interface
// ============================================================================

// LatticeValue is the interface implemented by all Lattice runtime values.
//
// Like json-value's JsonValue, this uses the sealed-interface pattern: the
// unexported marker method latticeValue() restricts the set of implementing
// types to those defined in this package.
type LatticeValue interface {
	latticeValue() // marker — forces implementations to be in this package
	String() string
	Truthy() bool
}

// ============================================================================
// Concrete Value Types
// ============================================================================

// LatticeNumber is a pure number without a unit.
//
// Examples: 42, 3.14, 0, -1
// Maps to the CSS NUMBER token.
type LatticeNumber struct {
	Value float64
}

func (v LatticeNumber) latticeValue() {}
func (v LatticeNumber) Truthy() bool  { return v.Value != 0 }
func (v LatticeNumber) String() string {
	// Emit integers without a decimal point: "42" not "42.000000"
	if v.Value == float64(int64(v.Value)) {
		return strconv.FormatInt(int64(v.Value), 10)
	}
	return strconv.FormatFloat(v.Value, 'f', -1, 64)
}

// LatticeDimension is a number with a CSS unit.
//
// Examples: 16px, 2em, 1.5rem, 100vh, 300ms
// Maps to the CSS DIMENSION token. Arithmetic is only valid between
// dimensions with the same unit; mixed-unit math requires calc().
type LatticeDimension struct {
	Value float64
	Unit  string
}

func (v LatticeDimension) latticeValue() {}
func (v LatticeDimension) Truthy() bool  { return true } // dimensions are always truthy
func (v LatticeDimension) String() string {
	if v.Value == float64(int64(v.Value)) {
		return fmt.Sprintf("%d%s", int64(v.Value), v.Unit)
	}
	return fmt.Sprintf("%s%s", strconv.FormatFloat(v.Value, 'f', -1, 64), v.Unit)
}

// LatticePercentage is a percentage value.
//
// Examples: 50%, 100%, 33.33%
// Maps to the CSS PERCENTAGE token.
type LatticePercentage struct {
	Value float64
}

func (v LatticePercentage) latticeValue() {}
func (v LatticePercentage) Truthy() bool  { return true }
func (v LatticePercentage) String() string {
	if v.Value == float64(int64(v.Value)) {
		return fmt.Sprintf("%d%%", int64(v.Value))
	}
	return fmt.Sprintf("%s%%", strconv.FormatFloat(v.Value, 'f', -1, 64))
}

// LatticeString is a quoted string value.
//
// The quote characters are not stored — they are added back when emitting CSS.
// Examples: "hello", 'world'
type LatticeString struct {
	Value string
}

func (v LatticeString) latticeValue() {}
func (v LatticeString) Truthy() bool  { return true }
func (v LatticeString) String() string {
	return fmt.Sprintf("%q", v.Value)
}

// LatticeIdent is an unquoted CSS identifier.
//
// Examples: red, bold, dark, sans-serif, transparent
// CSS color keywords and other idents are treated as opaque — no arithmetic.
type LatticeIdent struct {
	Value string
}

func (v LatticeIdent) latticeValue() {}
func (v LatticeIdent) Truthy() bool  { return true }
func (v LatticeIdent) String() string { return v.Value }

// LatticeColor is a hex color value.
//
// Examples: #4a90d9, #fff, #00000080
// Stored with the # prefix, exactly as written in the source.
type LatticeColor struct {
	Value string
}

func (v LatticeColor) latticeValue() {}
func (v LatticeColor) Truthy() bool  { return true }
func (v LatticeColor) String() string { return v.Value }

// LatticeBool is a boolean value — true or false.
//
// Lattice boolean literals are matched by the grammar: "true" and "false"
// as IDENT tokens. LatticeBool is the evaluated form of these literals.
type LatticeBool struct {
	Value bool
}

func (v LatticeBool) latticeValue() {}
func (v LatticeBool) Truthy() bool  { return v.Value }
func (v LatticeBool) String() string {
	if v.Value {
		return "true"
	}
	return "false"
}

// LatticeNull is the null value.
//
// null is falsy and stringifies to empty string (matching Sass semantics).
// Used for optional parameters and missing values.
type LatticeNull struct{}

func (v LatticeNull) latticeValue() {}
func (v LatticeNull) Truthy() bool  { return false }
func (v LatticeNull) String() string { return "" }

// LatticeList is a comma-separated list of values.
//
// Used in @each directives and multi-value declarations.
// Example: red, green, blue
type LatticeList struct {
	Items []LatticeValue
}

func (v LatticeList) latticeValue() {}
func (v LatticeList) Truthy() bool  { return len(v.Items) > 0 }
func (v LatticeList) String() string {
	parts := make([]string, len(v.Items))
	for i, item := range v.Items {
		parts[i] = item.String()
	}
	return strings.Join(parts, ", ")
}

// LatticeMap is an ordered key-value map — Lattice v2 value type.
//
// Maps are written as parenthesized key-value pairs in Lattice:
//
//	$theme: (
//	    primary: #4a90d9,
//	    secondary: #7b68ee,
//	);
//
// Items is a slice of (key, value) pairs preserving insertion order.
// Access is through built-in functions: map-get, map-keys, map-values, etc.
type LatticeMap struct {
	Items []MapEntry
}

// MapEntry is a single key-value pair in a LatticeMap.
type MapEntry struct {
	Key   string
	Value LatticeValue
}

func (v LatticeMap) latticeValue() {}
func (v LatticeMap) Truthy() bool  { return true }
func (v LatticeMap) String() string {
	parts := make([]string, len(v.Items))
	for i, entry := range v.Items {
		parts[i] = fmt.Sprintf("%s: %s", entry.Key, entry.Value.String())
	}
	return "(" + strings.Join(parts, ", ") + ")"
}

// MapGet looks up a value by key. Returns (value, true) or (nil, false).
func (v LatticeMap) MapGet(key string) (LatticeValue, bool) {
	for _, entry := range v.Items {
		if entry.Key == key {
			return entry.Value, true
		}
	}
	return nil, false
}

// MapKeys returns all keys in insertion order.
func (v LatticeMap) MapKeys() []string {
	keys := make([]string, len(v.Items))
	for i, entry := range v.Items {
		keys[i] = entry.Key
	}
	return keys
}

// MapValues returns all values in insertion order.
func (v LatticeMap) MapValues() []LatticeValue {
	vals := make([]LatticeValue, len(v.Items))
	for i, entry := range v.Items {
		vals[i] = entry.Value
	}
	return vals
}

// MapHasKey returns true if the key exists.
func (v LatticeMap) MapHasKey(key string) bool {
	for _, entry := range v.Items {
		if entry.Key == key {
			return true
		}
	}
	return false
}

// ============================================================================
// Color Conversion Helpers
// ============================================================================

// colorToRGB parses a hex color string to (r, g, b, a) components.
// r, g, b are 0-255, a is 0.0-1.0.
// Supports #RGB, #RRGGBB, and #RRGGBBAA formats.
func colorToRGB(hex string) (int, int, int, float64) {
	h := strings.TrimPrefix(hex, "#")
	switch len(h) {
	case 3:
		r, _ := strconv.ParseInt(string(h[0])+string(h[0]), 16, 0)
		g, _ := strconv.ParseInt(string(h[1])+string(h[1]), 16, 0)
		b, _ := strconv.ParseInt(string(h[2])+string(h[2]), 16, 0)
		return int(r), int(g), int(b), 1.0
	case 6:
		r, _ := strconv.ParseInt(h[0:2], 16, 0)
		g, _ := strconv.ParseInt(h[2:4], 16, 0)
		b, _ := strconv.ParseInt(h[4:6], 16, 0)
		return int(r), int(g), int(b), 1.0
	case 8:
		r, _ := strconv.ParseInt(h[0:2], 16, 0)
		g, _ := strconv.ParseInt(h[2:4], 16, 0)
		b, _ := strconv.ParseInt(h[4:6], 16, 0)
		a, _ := strconv.ParseInt(h[6:8], 16, 0)
		return int(r), int(g), int(b), float64(a) / 255.0
	}
	return 0, 0, 0, 1.0
}

// colorToHSL converts a hex color to (h, s, l, a).
// h is 0-360, s/l are 0-100, a is 0-1.
func colorToHSL(hex string) (float64, float64, float64, float64) {
	r, g, b, a := colorToRGB(hex)
	rf, gf, bf := float64(r)/255.0, float64(g)/255.0, float64(b)/255.0
	mx := math.Max(rf, math.Max(gf, bf))
	mn := math.Min(rf, math.Min(gf, bf))
	light := (mx + mn) / 2.0

	if mx == mn {
		return 0.0, 0.0, light * 100.0, a
	}

	d := mx - mn
	var sat float64
	if light > 0.5 {
		sat = d / (2.0 - mx - mn)
	} else {
		sat = d / (mx + mn)
	}

	var hue float64
	switch {
	case mx == rf:
		hue = (gf - bf) / d
		if gf < bf {
			hue += 6.0
		}
	case mx == gf:
		hue = (bf-rf)/d + 2.0
	default:
		hue = (rf-gf)/d + 4.0
	}
	hue *= 60.0

	return hue, sat * 100.0, light * 100.0, a
}

// colorFromRGB creates a hex color string from RGBA components.
// r, g, b are 0-255, a is 0-1.
func colorFromRGB(r, g, b int, a float64) string {
	r = clampInt(r, 0, 255)
	g = clampInt(g, 0, 255)
	b = clampInt(b, 0, 255)
	a = clampFloat(a, 0.0, 1.0)
	if a >= 1.0 {
		return fmt.Sprintf("#%02x%02x%02x", r, g, b)
	}
	return fmt.Sprintf("rgba(%d, %d, %d, %s)", r, g, b,
		strconv.FormatFloat(a, 'f', -1, 64))
}

// colorFromHSL creates a hex color from HSLA components.
// h is 0-360, s/l are 0-100, a is 0-1.
func colorFromHSL(h, s, l, a float64) string {
	h = math.Mod(h, 360.0)
	if h < 0 {
		h += 360.0
	}
	s = clampFloat(s, 0, 100) / 100.0
	l = clampFloat(l, 0, 100) / 100.0

	if s == 0.0 {
		v := int(math.Round(l * 255))
		return colorFromRGB(v, v, v, a)
	}

	var q float64
	if l < 0.5 {
		q = l * (1 + s)
	} else {
		q = l + s - l*s
	}
	p := 2*l - q

	hNorm := h / 360.0
	ri := int(math.Round(hueToRGB(p, q, hNorm+1.0/3.0) * 255))
	gi := int(math.Round(hueToRGB(p, q, hNorm) * 255))
	bi := int(math.Round(hueToRGB(p, q, hNorm-1.0/3.0) * 255))

	return colorFromRGB(ri, gi, bi, a)
}

func hueToRGB(p, q, t float64) float64 {
	if t < 0 {
		t += 1
	}
	if t > 1 {
		t -= 1
	}
	if t < 1.0/6.0 {
		return p + (q-p)*6*t
	}
	if t < 1.0/2.0 {
		return q
	}
	if t < 2.0/3.0 {
		return p + (q-p)*(2.0/3.0-t)*6
	}
	return p
}

func clampInt(v, lo, hi int) int {
	if v < lo {
		return lo
	}
	if v > hi {
		return hi
	}
	return v
}

func clampFloat(v, lo, hi float64) float64 {
	if v < lo {
		return lo
	}
	if v > hi {
		return hi
	}
	return v
}

// ============================================================================
// Built-in Function Registry — Lattice v2
// ============================================================================

// BuiltinFunc is the signature for built-in Lattice functions.
// Takes evaluated arguments and scope, returns a LatticeValue.
type BuiltinFunc func(args []LatticeValue, scope *ScopeChain) LatticeValue

// builtinFunctions is the registry of all Lattice v2 built-in functions.
var builtinFunctions = map[string]BuiltinFunc{
	// Map functions
	"map-get":     builtinMapGet,
	"map-keys":    builtinMapKeys,
	"map-values":  builtinMapValues,
	"map-has-key": builtinMapHasKey,
	"map-merge":   builtinMapMerge,
	"map-remove":  builtinMapRemove,
	// Color functions
	"lighten":    builtinLighten,
	"darken":     builtinDarken,
	"saturate":   builtinSaturateFn,
	"desaturate": builtinDesaturate,
	"adjust-hue": builtinAdjustHue,
	"complement": builtinComplement,
	"mix":        builtinMix,
	"rgba":       builtinRGBA,
	"red":        builtinRed,
	"green":      builtinGreen,
	"blue":       builtinBlue,
	"hue":        builtinHue,
	"saturation": builtinSaturation,
	"lightness":  builtinLightness,
	// List functions
	"nth":    builtinNth,
	"length": builtinLength,
	"join":   builtinJoin,
	"append": builtinAppend,
	"index":  builtinIndex,
	// Type functions
	"type-of":    builtinTypeOf,
	"unit":       builtinUnit,
	"unitless":   builtinUnitless,
	"comparable": builtinComparable,
	// Math functions
	"math.div":   builtinMathDiv,
	"math.floor": builtinMathFloor,
	"math.ceil":  builtinMathCeil,
	"math.round": builtinMathRound,
	"math.abs":   builtinMathAbs,
	"math.min":   builtinMathMin,
	"math.max":   builtinMathMax,
}

// IsBuiltinFunction returns true if funcName is a registered built-in.
func IsBuiltinFunction(funcName string) bool {
	_, ok := builtinFunctions[funcName]
	return ok
}

// CallBuiltinFunction calls the named built-in with evaluated args.
func CallBuiltinFunction(funcName string, args []LatticeValue, scope *ScopeChain) LatticeValue {
	fn, ok := builtinFunctions[funcName]
	if !ok {
		return LatticeNull{}
	}
	return fn(args, scope)
}

// typeNameOf returns the Lattice type name for a value.
func typeNameOf(v LatticeValue) string {
	switch v.(type) {
	case LatticeNumber, LatticeDimension, LatticePercentage:
		return "number"
	case LatticeString, LatticeIdent:
		return "string"
	case LatticeColor:
		return "color"
	case LatticeBool:
		return "bool"
	case LatticeNull:
		return "null"
	case LatticeList:
		return "list"
	case LatticeMap:
		return "map"
	}
	return "unknown"
}

// getNumericValue extracts the float64 from a numeric LatticeValue.
func getNumericValue(v LatticeValue) float64 {
	switch val := v.(type) {
	case LatticeNumber:
		return val.Value
	case LatticeDimension:
		return val.Value
	case LatticePercentage:
		return val.Value
	}
	panic(NewTypeErrorInExpression("use",
		fmt.Sprintf("Expected a number, got %s", typeNameOf(v)), "", 0, 0))
}

// ensureColor validates a value is a LatticeColor and returns it.
func ensureColor(v LatticeValue) LatticeColor {
	if c, ok := v.(LatticeColor); ok {
		return c
	}
	panic(NewTypeErrorInExpression("use",
		fmt.Sprintf("Expected a color, got %s", typeNameOf(v)), "", 0, 0))
}

// ensureAmount extracts a 0-100 percentage from a value.
func ensureAmount(v LatticeValue) float64 {
	val := getNumericValue(v)
	if val < 0 || val > 100 {
		panic(NewRangeError("Amount must be between 0% and 100%", 0, 0))
	}
	return val
}

// --- Map Functions ---

func builtinMapGet(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "map-get requires 2 arguments", "", 0, 0))
	}
	m, ok := args[0].(LatticeMap)
	if !ok {
		panic(NewTypeErrorInExpression("use",
			fmt.Sprintf("Expected a map, got %s", typeNameOf(args[0])), "", 0, 0))
	}
	key := strings.Trim(args[1].String(), "\"")
	if v, found := m.MapGet(key); found {
		return v
	}
	return LatticeNull{}
}

func builtinMapKeys(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "map-keys requires 1 argument", "", 0, 0))
	}
	m, ok := args[0].(LatticeMap)
	if !ok {
		panic(NewTypeErrorInExpression("use",
			fmt.Sprintf("Expected a map, got %s", typeNameOf(args[0])), "", 0, 0))
	}
	items := make([]LatticeValue, len(m.Items))
	for i, k := range m.MapKeys() {
		items[i] = LatticeIdent{Value: k}
	}
	return LatticeList{Items: items}
}

func builtinMapValues(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "map-values requires 1 argument", "", 0, 0))
	}
	m, ok := args[0].(LatticeMap)
	if !ok {
		panic(NewTypeErrorInExpression("use",
			fmt.Sprintf("Expected a map, got %s", typeNameOf(args[0])), "", 0, 0))
	}
	return LatticeList{Items: m.MapValues()}
}

func builtinMapHasKey(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "map-has-key requires 2 arguments", "", 0, 0))
	}
	m, ok := args[0].(LatticeMap)
	if !ok {
		panic(NewTypeErrorInExpression("use",
			fmt.Sprintf("Expected a map, got %s", typeNameOf(args[0])), "", 0, 0))
	}
	key := strings.Trim(args[1].String(), "\"")
	return LatticeBool{Value: m.MapHasKey(key)}
}

func builtinMapMerge(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "map-merge requires 2 arguments", "", 0, 0))
	}
	m1, ok1 := args[0].(LatticeMap)
	m2, ok2 := args[1].(LatticeMap)
	if !ok1 || !ok2 {
		panic(NewTypeErrorInExpression("use", "Expected maps", "", 0, 0))
	}
	// Merge: m2 overwrites m1
	merged := make(map[string]LatticeValue)
	order := make([]string, 0)
	for _, e := range m1.Items {
		merged[e.Key] = e.Value
		order = append(order, e.Key)
	}
	for _, e := range m2.Items {
		if _, exists := merged[e.Key]; !exists {
			order = append(order, e.Key)
		}
		merged[e.Key] = e.Value
	}
	items := make([]MapEntry, len(order))
	for i, k := range order {
		items[i] = MapEntry{Key: k, Value: merged[k]}
	}
	return LatticeMap{Items: items}
}

func builtinMapRemove(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "map-remove requires at least 1 argument", "", 0, 0))
	}
	m, ok := args[0].(LatticeMap)
	if !ok {
		panic(NewTypeErrorInExpression("use",
			fmt.Sprintf("Expected a map, got %s", typeNameOf(args[0])), "", 0, 0))
	}
	toRemove := make(map[string]bool)
	for _, a := range args[1:] {
		toRemove[strings.Trim(a.String(), "\"")] = true
	}
	var items []MapEntry
	for _, e := range m.Items {
		if !toRemove[e.Key] {
			items = append(items, e)
		}
	}
	return LatticeMap{Items: items}
}

// --- Color Functions ---

func builtinLighten(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	amount := ensureAmount(args[1])
	h, s, l, a := colorToHSL(c.Value)
	l = math.Min(100.0, l+amount)
	return LatticeColor{Value: colorFromHSL(h, s, l, a)}
}

func builtinDarken(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	amount := ensureAmount(args[1])
	h, s, l, a := colorToHSL(c.Value)
	l = math.Max(0.0, l-amount)
	return LatticeColor{Value: colorFromHSL(h, s, l, a)}
}

func builtinSaturateFn(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	amount := ensureAmount(args[1])
	h, s, l, a := colorToHSL(c.Value)
	s = math.Min(100.0, s+amount)
	return LatticeColor{Value: colorFromHSL(h, s, l, a)}
}

func builtinDesaturate(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	amount := ensureAmount(args[1])
	h, s, l, a := colorToHSL(c.Value)
	s = math.Max(0.0, s-amount)
	return LatticeColor{Value: colorFromHSL(h, s, l, a)}
}

func builtinAdjustHue(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	degrees := getNumericValue(args[1])
	h, s, l, a := colorToHSL(c.Value)
	h = math.Mod(h+degrees, 360.0)
	return LatticeColor{Value: colorFromHSL(h, s, l, a)}
}

func builtinComplement(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	h, s, l, a := colorToHSL(c.Value)
	h = math.Mod(h+180.0, 360.0)
	return LatticeColor{Value: colorFromHSL(h, s, l, a)}
}

func builtinMix(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c1 := ensureColor(args[0])
	c2 := ensureColor(args[1])
	weight := 50.0
	if len(args) >= 3 {
		weight = getNumericValue(args[2])
	}
	w := weight / 100.0
	r1, g1, b1, a1 := colorToRGB(c1.Value)
	r2, g2, b2, a2 := colorToRGB(c2.Value)
	r := int(math.Round(float64(r1)*w + float64(r2)*(1-w)))
	g := int(math.Round(float64(g1)*w + float64(g2)*(1-w)))
	b := int(math.Round(float64(b1)*w + float64(b2)*(1-w)))
	a := a1*w + a2*(1-w)
	return LatticeColor{Value: colorFromRGB(r, g, b, a)}
}

func builtinRGBA(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 2 {
		if c, ok := args[0].(LatticeColor); ok {
			alpha := getNumericValue(args[1])
			r, g, b, _ := colorToRGB(c.Value)
			return LatticeColor{Value: colorFromRGB(r, g, b, alpha)}
		}
	}
	if len(args) == 4 {
		r := int(math.Round(getNumericValue(args[0])))
		g := int(math.Round(getNumericValue(args[1])))
		b := int(math.Round(getNumericValue(args[2])))
		a := getNumericValue(args[3])
		return LatticeColor{Value: colorFromRGB(r, g, b, a)}
	}
	return LatticeNull{}
}

func builtinRed(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	r, _, _, _ := colorToRGB(c.Value)
	return LatticeNumber{Value: float64(r)}
}

func builtinGreen(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	_, g, _, _ := colorToRGB(c.Value)
	return LatticeNumber{Value: float64(g)}
}

func builtinBlue(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	_, _, b, _ := colorToRGB(c.Value)
	return LatticeNumber{Value: float64(b)}
}

func builtinHue(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	h, _, _, _ := colorToHSL(c.Value)
	return LatticeDimension{Value: math.Round(h), Unit: "deg"}
}

func builtinSaturation(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	_, s, _, _ := colorToHSL(c.Value)
	return LatticePercentage{Value: math.Round(s)}
}

func builtinLightness(args []LatticeValue, scope *ScopeChain) LatticeValue {
	c := ensureColor(args[0])
	_, _, l, _ := colorToHSL(c.Value)
	return LatticePercentage{Value: math.Round(l)}
}

// --- List Functions ---

func builtinNth(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "nth requires 2 arguments", "", 0, 0))
	}
	n := int(getNumericValue(args[1]))
	if n < 1 {
		panic(NewRangeError("List index must be 1 or greater", 0, 0))
	}
	if lst, ok := args[0].(LatticeList); ok {
		if n > len(lst.Items) {
			panic(NewRangeError(
				fmt.Sprintf("Index %d out of bounds for list of length %d", n, len(lst.Items)), 0, 0))
		}
		return lst.Items[n-1]
	}
	if n == 1 {
		return args[0]
	}
	panic(NewRangeError(fmt.Sprintf("Index %d out of bounds for list of length 1", n), 0, 0))
}

func builtinLength(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "length requires 1 argument", "", 0, 0))
	}
	switch v := args[0].(type) {
	case LatticeList:
		return LatticeNumber{Value: float64(len(v.Items))}
	case LatticeMap:
		return LatticeNumber{Value: float64(len(v.Items))}
	}
	return LatticeNumber{Value: 1.0}
}

func builtinJoin(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "join requires at least 2 arguments", "", 0, 0))
	}
	var items1, items2 []LatticeValue
	if l, ok := args[0].(LatticeList); ok {
		items1 = l.Items
	} else {
		items1 = []LatticeValue{args[0]}
	}
	if l, ok := args[1].(LatticeList); ok {
		items2 = l.Items
	} else {
		items2 = []LatticeValue{args[1]}
	}
	combined := make([]LatticeValue, 0, len(items1)+len(items2))
	combined = append(combined, items1...)
	combined = append(combined, items2...)
	return LatticeList{Items: combined}
}

func builtinAppend(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "append requires at least 2 arguments", "", 0, 0))
	}
	var items []LatticeValue
	if l, ok := args[0].(LatticeList); ok {
		items = make([]LatticeValue, len(l.Items))
		copy(items, l.Items)
	} else {
		items = []LatticeValue{args[0]}
	}
	items = append(items, args[1])
	return LatticeList{Items: items}
}

func builtinIndex(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "index requires 2 arguments", "", 0, 0))
	}
	var items []LatticeValue
	if l, ok := args[0].(LatticeList); ok {
		items = l.Items
	} else {
		items = []LatticeValue{args[0]}
	}
	target := args[1].String()
	for i, item := range items {
		if item.String() == target {
			return LatticeNumber{Value: float64(i + 1)}
		}
	}
	return LatticeNull{}
}

// --- Type Functions ---

func builtinTypeOf(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "type-of requires 1 argument", "", 0, 0))
	}
	return LatticeString{Value: typeNameOf(args[0])}
}

func builtinUnit(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "unit requires 1 argument", "", 0, 0))
	}
	switch v := args[0].(type) {
	case LatticeDimension:
		return LatticeString{Value: v.Unit}
	case LatticePercentage:
		return LatticeString{Value: "%"}
	case LatticeNumber:
		return LatticeString{Value: ""}
	}
	panic(NewTypeErrorInExpression("use",
		fmt.Sprintf("Expected a number, got %s", typeNameOf(args[0])), "", 0, 0))
}

func builtinUnitless(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "unitless requires 1 argument", "", 0, 0))
	}
	_, isNum := args[0].(LatticeNumber)
	return LatticeBool{Value: isNum}
}

func builtinComparable(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "comparable requires 2 arguments", "", 0, 0))
	}
	a, b := args[0], args[1]
	switch av := a.(type) {
	case LatticeNumber:
		switch b.(type) {
		case LatticeNumber, LatticeDimension, LatticePercentage:
			return LatticeBool{Value: true}
		}
	case LatticeDimension:
		if bv, ok := b.(LatticeDimension); ok {
			return LatticeBool{Value: av.Unit == bv.Unit}
		}
		if _, ok := b.(LatticeNumber); ok {
			return LatticeBool{Value: true}
		}
	case LatticePercentage:
		switch b.(type) {
		case LatticePercentage, LatticeNumber:
			return LatticeBool{Value: true}
		}
	}
	return LatticeBool{Value: false}
}

// --- Math Functions ---

func builtinMathDiv(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) < 2 {
		panic(NewTypeErrorInExpression("call", "math.div requires 2 arguments", "", 0, 0))
	}
	bVal := getNumericValue(args[1])
	if bVal == 0 {
		panic(NewZeroDivisionInExpressionError(0, 0))
	}
	aVal := getNumericValue(args[0])
	if ad, ok := args[0].(LatticeDimension); ok {
		if _, ok2 := args[1].(LatticeNumber); ok2 {
			return LatticeDimension{Value: aVal / bVal, Unit: ad.Unit}
		}
		if bd, ok2 := args[1].(LatticeDimension); ok2 && ad.Unit == bd.Unit {
			return LatticeNumber{Value: aVal / bVal}
		}
	}
	if _, ok := args[0].(LatticePercentage); ok {
		if _, ok2 := args[1].(LatticeNumber); ok2 {
			return LatticePercentage{Value: aVal / bVal}
		}
	}
	return LatticeNumber{Value: aVal / bVal}
}

func builtinMathFloor(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "math.floor requires 1 argument", "", 0, 0))
	}
	val := getNumericValue(args[0])
	result := math.Floor(val)
	switch v := args[0].(type) {
	case LatticeDimension:
		return LatticeDimension{Value: result, Unit: v.Unit}
	case LatticePercentage:
		return LatticePercentage{Value: result}
	}
	return LatticeNumber{Value: result}
}

func builtinMathCeil(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "math.ceil requires 1 argument", "", 0, 0))
	}
	val := getNumericValue(args[0])
	result := math.Ceil(val)
	switch v := args[0].(type) {
	case LatticeDimension:
		return LatticeDimension{Value: result, Unit: v.Unit}
	case LatticePercentage:
		return LatticePercentage{Value: result}
	}
	return LatticeNumber{Value: result}
}

func builtinMathRound(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "math.round requires 1 argument", "", 0, 0))
	}
	val := getNumericValue(args[0])
	result := math.Round(val)
	switch v := args[0].(type) {
	case LatticeDimension:
		return LatticeDimension{Value: result, Unit: v.Unit}
	case LatticePercentage:
		return LatticePercentage{Value: result}
	}
	return LatticeNumber{Value: result}
}

func builtinMathAbs(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "math.abs requires 1 argument", "", 0, 0))
	}
	val := getNumericValue(args[0])
	result := math.Abs(val)
	switch v := args[0].(type) {
	case LatticeDimension:
		return LatticeDimension{Value: result, Unit: v.Unit}
	case LatticePercentage:
		return LatticePercentage{Value: result}
	}
	return LatticeNumber{Value: result}
}

func builtinMathMin(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "math.min requires at least 1 argument", "", 0, 0))
	}
	best := args[0]
	bestVal := getNumericValue(best)
	for _, arg := range args[1:] {
		val := getNumericValue(arg)
		if val < bestVal {
			best = arg
			bestVal = val
		}
	}
	return best
}

func builtinMathMax(args []LatticeValue, scope *ScopeChain) LatticeValue {
	if len(args) == 0 {
		panic(NewTypeErrorInExpression("call", "math.max requires at least 1 argument", "", 0, 0))
	}
	best := args[0]
	bestVal := getNumericValue(best)
	for _, arg := range args[1:] {
		val := getNumericValue(arg)
		if val > bestVal {
			best = arg
			bestVal = val
		}
	}
	return best
}

// ============================================================================
// Token → Value Conversion
// ============================================================================

// tokenTypeName returns the string name of a token's type.
//
// Grammar-driven tokens store the human-readable name in Token.TypeName
// (e.g., "VARIABLE", "IDENT"). Hand-written lexer tokens use Token.Type (int).
// This function abstracts both.
func tokenTypeName(tok lexer.Token) string {
	if tok.TypeName != "" {
		return tok.TypeName
	}
	return tok.Type.String()
}

// tokenToValue converts a lexer Token to a LatticeValue.
//
// This bridges the gap between the parser's token world and the evaluator's
// value world. The parser gives us tokens; arithmetic needs typed values.
//
// Mapping:
//
//	NUMBER     → LatticeNumber
//	DIMENSION  → LatticeDimension (splits "16px" into 16 + "px")
//	PERCENTAGE → LatticePercentage (strips the "%")
//	STRING     → LatticeString
//	HASH       → LatticeColor
//	IDENT      → LatticeIdent (or LatticeBool/LatticeNull for keywords)
//	other      → LatticeIdent (fallback)
func tokenToValue(tok lexer.Token) LatticeValue {
	typeName := tokenTypeName(tok)
	val := tok.Value

	switch typeName {
	case "NUMBER":
		f, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return LatticeIdent{Value: val}
		}
		return LatticeNumber{Value: f}

	case "DIMENSION":
		// Split "16px" into numeric part and unit part.
		// The number may be negative: "-10px" → -10, "px".
		i := 0
		if i < len(val) && val[i] == '-' {
			i++
		}
		for i < len(val) && (val[i] == '.' || (val[i] >= '0' && val[i] <= '9')) {
			i++
		}
		// Handle scientific notation: 1e+2px
		if i < len(val) && (val[i] == 'e' || val[i] == 'E') {
			i++
			if i < len(val) && (val[i] == '+' || val[i] == '-') {
				i++
			}
			for i < len(val) && val[i] >= '0' && val[i] <= '9' {
				i++
			}
		}
		numStr := val[:i]
		unit := val[i:]
		f, err := strconv.ParseFloat(numStr, 64)
		if err != nil {
			return LatticeIdent{Value: val}
		}
		return LatticeDimension{Value: f, Unit: unit}

	case "PERCENTAGE":
		// "50%" → LatticePercentage(50)
		numStr := strings.TrimSuffix(val, "%")
		f, err := strconv.ParseFloat(numStr, 64)
		if err != nil {
			return LatticeIdent{Value: val}
		}
		return LatticePercentage{Value: f}

	case "STRING":
		// The lexer already strips quotes; val is the bare string content.
		return LatticeString{Value: val}

	case "HASH":
		return LatticeColor{Value: val}

	case "IDENT":
		// Special IDENT values that become typed booleans or null
		switch val {
		case "true":
			return LatticeBool{Value: true}
		case "false":
			return LatticeBool{Value: false}
		case "null":
			return LatticeNull{}
		}
		return LatticeIdent{Value: val}
	}

	// Fallback: treat any unrecognized token as an identifier.
	return LatticeIdent{Value: val}
}

// valueToCSSText converts a LatticeValue to its CSS text representation.
//
// Used when substituting evaluated values back into CSS output. Each value
// type's String() method returns the correct CSS representation.
func valueToCSSText(val LatticeValue) string {
	return val.String()
}

// ============================================================================
// Expression Evaluator
// ============================================================================

// ExpressionEvaluator evaluates Lattice expression AST nodes at compile time.
//
// The evaluator is a recursive AST walker. It dispatches on the rule_name of
// each node to the appropriate handler. Leaf tokens are converted to
// LatticeValue via tokenToValue.
//
// The grammar's nesting already encodes operator precedence, so we just
// recurse — no precedence climbing or Pratt parsing is needed.
//
// Usage:
//
//	eval := NewExpressionEvaluator(scope)
//	result := eval.Evaluate(expressionNode)
//	// result is a LatticeValue, e.g., LatticeBool{true}
type ExpressionEvaluator struct {
	scope *ScopeChain
}

// NewExpressionEvaluator creates a new evaluator with the given scope.
// The scope is used to look up $variable values during evaluation.
func NewExpressionEvaluator(scope *ScopeChain) *ExpressionEvaluator {
	return &ExpressionEvaluator{scope: scope}
}

// Evaluate walks an expression AST node and returns the computed LatticeValue.
//
// This is the main entry point. Pass any node from the expression sub-grammar
// (lattice_expression, lattice_or_expr, ..., lattice_primary) or a raw Token.
func (e *ExpressionEvaluator) Evaluate(node interface{}) LatticeValue {
	// Raw token (leaf node) — convert directly to a value
	if tok, ok := node.(lexer.Token); ok {
		return tokenToValue(tok)
	}

	ast, ok := node.(*parser.ASTNode)
	if !ok || ast == nil {
		return LatticeNull{}
	}

	// Dispatch to the handler for this rule
	switch ast.RuleName {
	case "lattice_expression":
		return e.evalExpression(ast)
	case "lattice_or_expr":
		return e.evalOr(ast)
	case "lattice_and_expr":
		return e.evalAnd(ast)
	case "lattice_comparison":
		return e.evalComparison(ast)
	case "comparison_op":
		// Handled by the parent lattice_comparison rule; shouldn't be called directly
		if len(ast.Children) > 0 {
			return tokenToValue(ast.Children[0].(lexer.Token))
		}
		return LatticeNull{}
	case "lattice_additive":
		return e.evalAdditive(ast)
	case "lattice_multiplicative":
		return e.evalMultiplicative(ast)
	case "lattice_unary":
		return e.evalUnary(ast)
	case "lattice_primary":
		return e.evalPrimary(ast)
	case "value_list":
		return e.evalValueList(ast)
	}

	// For single-child wrapper rules, unwrap and recurse
	if len(ast.Children) == 1 {
		return e.Evaluate(ast.Children[0])
	}

	// Default: try to evaluate the first meaningful child
	for _, child := range ast.Children {
		switch child.(type) {
		case *parser.ASTNode, lexer.Token:
			return e.Evaluate(child)
		}
	}

	return LatticeNull{}
}

// evalValueList handles value_list nodes produced by variable substitution.
// When expand_variable_declaration substitutes `$i + 1`, the evaluator receives
// a value_list AST node whose children are [NUMBER(2), PLUS, NUMBER(1)].
// If arithmetic operators are present we delegate to the additive handler;
// otherwise we simply evaluate the first child.
func (e *ExpressionEvaluator) evalValueList(node *parser.ASTNode) LatticeValue {
	if len(node.Children) == 0 {
		return LatticeNull{}
	}
	if len(node.Children) <= 1 {
		return e.Evaluate(node.Children[0])
	}
	hasOps := false
	for _, c := range node.Children {
		if tok, ok := c.(lexer.Token); ok {
			if tok.Value == "+" || tok.Value == "-" || tok.Value == "*" {
				hasOps = true
				break
			}
		}
	}
	if hasOps {
		return e.evalAdditive(node)
	}
	return e.Evaluate(node.Children[0])
}

// evalExpression handles: lattice_expression = lattice_or_expr ;
func (e *ExpressionEvaluator) evalExpression(node *parser.ASTNode) LatticeValue {
	if len(node.Children) > 0 {
		return e.Evaluate(node.Children[0])
	}
	return LatticeNull{}
}

// evalOr handles: lattice_or_expr = lattice_and_expr { "or" lattice_and_expr } ;
//
// Uses short-circuit evaluation: returns the first truthy operand, or the last
// operand if none are truthy. This matches JavaScript's || semantics.
func (e *ExpressionEvaluator) evalOr(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		// Skip the "or" IDENT token
		if tok, ok := node.Children[i].(lexer.Token); ok && tok.Value == "or" {
			i++
			continue
		}
		if result.Truthy() {
			return result
		}
		result = e.Evaluate(node.Children[i])
		i++
	}
	return result
}

// evalAnd handles: lattice_and_expr = lattice_comparison { "and" lattice_comparison } ;
//
// Short-circuit: returns the first falsy operand, or the last if all are truthy.
func (e *ExpressionEvaluator) evalAnd(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		if tok, ok := node.Children[i].(lexer.Token); ok && tok.Value == "and" {
			i++
			continue
		}
		if !result.Truthy() {
			return result
		}
		result = e.Evaluate(node.Children[i])
		i++
	}
	return result
}

// evalComparison handles: lattice_comparison = lattice_additive [ comparison_op lattice_additive ] ;
func (e *ExpressionEvaluator) evalComparison(node *parser.ASTNode) LatticeValue {
	left := e.Evaluate(node.Children[0])
	if len(node.Children) == 1 {
		return left
	}

	// Find comparison_op and the right operand
	var opNode *parser.ASTNode
	var rightNode interface{}
	for i, child := range node.Children[1:] {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "comparison_op" {
			opNode = ast
			if i+2 < len(node.Children) {
				rightNode = node.Children[i+2]
			}
			break
		}
	}

	if opNode == nil || rightNode == nil {
		return left
	}

	right := e.Evaluate(rightNode)

	// The comparison_op node has a single token child
	if len(opNode.Children) == 0 {
		return LatticeBool{Value: false}
	}
	opTok, ok := opNode.Children[0].(lexer.Token)
	if !ok {
		return LatticeBool{Value: false}
	}

	return e.compare(left, right, tokenTypeName(opTok))
}

// compare performs a comparison between two values.
//
// For numeric types (same type), compares by value.
// For equality comparisons of non-numeric types, compares by string representation.
// For ordering comparisons of non-numeric types, returns false.
func (e *ExpressionEvaluator) compare(left, right LatticeValue, opType string) LatticeBool {
	// Numeric comparison for compatible types
	switch l := left.(type) {
	case LatticeNumber:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeBool{Value: numCompare(l.Value, r.Value, opType)}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeDimension); ok {
			if l.Unit == r.Unit {
				return LatticeBool{Value: numCompare(l.Value, r.Value, opType)}
			}
			// Different units: only equality/inequality make sense
			switch opType {
			case "EQUALS_EQUALS":
				return LatticeBool{Value: false}
			case "NOT_EQUALS":
				return LatticeBool{Value: true}
			default:
				return LatticeBool{Value: false}
			}
		}
	case LatticePercentage:
		if r, ok := right.(LatticePercentage); ok {
			return LatticeBool{Value: numCompare(l.Value, r.Value, opType)}
		}
	}

	// Fallback: string equality for mixed/non-numeric types
	switch opType {
	case "EQUALS_EQUALS":
		return LatticeBool{Value: left.String() == right.String()}
	case "NOT_EQUALS":
		return LatticeBool{Value: left.String() != right.String()}
	}
	return LatticeBool{Value: false}
}

// numCompare compares two float64 values using the given operator type name.
func numCompare(lv, rv float64, opType string) bool {
	switch opType {
	case "EQUALS_EQUALS":
		return lv == rv
	case "NOT_EQUALS":
		return lv != rv
	case "GREATER":
		return lv > rv
	case "GREATER_EQUALS":
		return lv >= rv
	case "LESS_EQUALS":
		return lv <= rv
	case "LESS":
		return lv < rv
	}
	return false
}

// evalAdditive handles:
//
//	lattice_additive = lattice_multiplicative { ( PLUS | MINUS ) lattice_multiplicative } ;
func (e *ExpressionEvaluator) evalAdditive(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		tok, ok := node.Children[i].(lexer.Token)
		if !ok {
			i++
			continue
		}
		op := tok.Value
		if op != "+" && op != "-" {
			i++
			continue
		}
		i++
		if i >= len(node.Children) {
			break
		}
		right := e.Evaluate(node.Children[i])
		if op == "+" {
			result = e.add(result, right)
		} else {
			result = e.subtract(result, right)
		}
		i++
	}
	return result
}

// add performs addition. Compatible types only.
func (e *ExpressionEvaluator) add(left, right LatticeValue) LatticeValue {
	switch l := left.(type) {
	case LatticeNumber:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeNumber{Value: l.Value + r.Value}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeDimension); ok {
			if l.Unit == r.Unit {
				return LatticeDimension{Value: l.Value + r.Value, Unit: l.Unit}
			}
			panic(NewTypeErrorInExpression("add", left.String(), right.String(), 0, 0))
		}
	case LatticePercentage:
		if r, ok := right.(LatticePercentage); ok {
			return LatticePercentage{Value: l.Value + r.Value}
		}
	case LatticeString:
		if r, ok := right.(LatticeString); ok {
			return LatticeString{Value: l.Value + r.Value}
		}
	}
	panic(NewTypeErrorInExpression("add", left.String(), right.String(), 0, 0))
}

// subtract performs subtraction. Mirrors add.
func (e *ExpressionEvaluator) subtract(left, right LatticeValue) LatticeValue {
	switch l := left.(type) {
	case LatticeNumber:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeNumber{Value: l.Value - r.Value}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeDimension); ok {
			if l.Unit == r.Unit {
				return LatticeDimension{Value: l.Value - r.Value, Unit: l.Unit}
			}
			panic(NewTypeErrorInExpression("subtract", left.String(), right.String(), 0, 0))
		}
	case LatticePercentage:
		if r, ok := right.(LatticePercentage); ok {
			return LatticePercentage{Value: l.Value - r.Value}
		}
	}
	panic(NewTypeErrorInExpression("subtract", left.String(), right.String(), 0, 0))
}

// evalMultiplicative handles:
//
//	lattice_multiplicative = lattice_unary { ( STAR | SLASH ) lattice_unary } ;
func (e *ExpressionEvaluator) evalMultiplicative(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		tok, ok := node.Children[i].(lexer.Token)
		if !ok || (tok.Value != "*" && tok.Value != "/") {
			i++
			continue
		}
		op := tok.Value
		i++
		if i >= len(node.Children) {
			break
		}
		right := e.Evaluate(node.Children[i])
		if op == "*" {
			result = e.multiply(result, right)
		} else {
			result = e.divide(result, right)
		}
		i++
	}
	return result
}

// multiply performs multiplication.
//
// Supported combinations:
//   Number × Number → Number
//   Number × Dimension → Dimension   (scales the dimension)
//   Dimension × Number → Dimension   (commutative)
//   Number × Percentage → Percentage
//   Percentage × Number → Percentage
func (e *ExpressionEvaluator) multiply(left, right LatticeValue) LatticeValue {
	switch l := left.(type) {
	case LatticeNumber:
		switch r := right.(type) {
		case LatticeNumber:
			return LatticeNumber{Value: l.Value * r.Value}
		case LatticeDimension:
			return LatticeDimension{Value: l.Value * r.Value, Unit: r.Unit}
		case LatticePercentage:
			return LatticePercentage{Value: l.Value * r.Value}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeDimension{Value: l.Value * r.Value, Unit: l.Unit}
		}
	case LatticePercentage:
		if r, ok := right.(LatticeNumber); ok {
			return LatticePercentage{Value: l.Value * r.Value}
		}
	}
	panic(NewTypeErrorInExpression("multiply", left.String(), right.String(), 0, 0))
}

// divide performs division with zero-division guard.
//
//	Number ÷ Number → Number
//	Dimension ÷ Number → Dimension (same unit)
//	Dimension ÷ Dimension (same unit) → Number (unitless ratio)
//	Percentage ÷ Number → Percentage
func (e *ExpressionEvaluator) divide(left, right LatticeValue) LatticeValue {
	switch l := left.(type) {
	case LatticeNumber:
		if r, ok := right.(LatticeNumber); ok {
			if r.Value == 0 {
				panic(NewZeroDivisionInExpressionError(0, 0))
			}
			return LatticeNumber{Value: l.Value / r.Value}
		}
	case LatticeDimension:
		switch r := right.(type) {
		case LatticeNumber:
			if r.Value == 0 {
				panic(NewZeroDivisionInExpressionError(0, 0))
			}
			return LatticeDimension{Value: l.Value / r.Value, Unit: l.Unit}
		case LatticeDimension:
			if l.Unit == r.Unit {
				if r.Value == 0 {
					panic(NewZeroDivisionInExpressionError(0, 0))
				}
				return LatticeNumber{Value: l.Value / r.Value}
			}
		}
	case LatticePercentage:
		if r, ok := right.(LatticeNumber); ok {
			if r.Value == 0 {
				panic(NewZeroDivisionInExpressionError(0, 0))
			}
			return LatticePercentage{Value: l.Value / r.Value}
		}
	}
	panic(NewTypeErrorInExpression("divide", left.String(), right.String(), 0, 0))
}

// evalUnary handles: lattice_unary = MINUS lattice_unary | lattice_primary ;
func (e *ExpressionEvaluator) evalUnary(node *parser.ASTNode) LatticeValue {
	if len(node.Children) < 2 {
		if len(node.Children) == 1 {
			return e.Evaluate(node.Children[0])
		}
		return LatticeNull{}
	}

	// Check if first child is a MINUS token
	if tok, ok := node.Children[0].(lexer.Token); ok && tok.Value == "-" {
		operand := e.Evaluate(node.Children[1])
		return e.negate(operand)
	}

	return e.Evaluate(node.Children[0])
}

// negate negates a numeric value.
func (e *ExpressionEvaluator) negate(val LatticeValue) LatticeValue {
	switch v := val.(type) {
	case LatticeNumber:
		return LatticeNumber{Value: -v.Value}
	case LatticeDimension:
		return LatticeDimension{Value: -v.Value, Unit: v.Unit}
	case LatticePercentage:
		return LatticePercentage{Value: -v.Value}
	}
	panic(NewTypeErrorInExpression("negate", val.String(), "", 0, 0))
}

// evalPrimary handles:
//
//	lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
//	                | STRING | IDENT | HASH
//	                | "true" | "false" | "null"
//	                | function_call
//	                | LPAREN lattice_expression RPAREN ;
func (e *ExpressionEvaluator) evalPrimary(node *parser.ASTNode) LatticeValue {
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			typeName := tokenTypeName(c)

			if typeName == "LPAREN" || typeName == "RPAREN" {
				continue // skip parens
			}

			if typeName == "VARIABLE" {
				// Look up the variable in the current scope
				val, ok := e.scope.Get(c.Value)
				if !ok {
					// Return an ident for now; transformer will raise the error
					return LatticeIdent{Value: c.Value}
				}
				if lv, ok := val.(LatticeValue); ok {
					return lv
				}
				// If it's an AST node (value_list), extract the value
				if ast, ok := val.(*parser.ASTNode); ok {
					return e.extractValueFromAST(ast)
				}
				// Raw token
				if tok, ok := val.(lexer.Token); ok {
					return tokenToValue(tok)
				}
				return LatticeIdent{Value: c.Value}
			}

			return tokenToValue(c)

		case *parser.ASTNode:
			// Recurse into sub-expressions (e.g., LPAREN lattice_expression RPAREN)
			if c.RuleName == "lattice_expression" {
				return e.Evaluate(c)
			}
			// function_call or other rule
			return e.Evaluate(c)
		}
	}
	return LatticeNull{}
}

// extractValueFromAST extracts a LatticeValue from an AST node.
//
// When a variable is bound to a value_list node (from the parser), we need to
// extract the actual value. A value_list like "dark" contains a single value
// node wrapping an IDENT token.
//
// For multi-token value_lists (e.g., "Helvetica, sans-serif"), we take the
// first token's value — which is sufficient for expression evaluation.
func (e *ExpressionEvaluator) extractValueFromAST(node *parser.ASTNode) LatticeValue {
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			return tokenToValue(c)
		case *parser.ASTNode:
			result := e.extractValueFromAST(c)
			if _, isNull := result.(LatticeNull); !isNull {
				return result
			}
		}
	}
	return LatticeNull{}
}
