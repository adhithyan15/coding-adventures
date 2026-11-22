package trig

// ============================================================================
// Bit-exact conformance against the shared TRIG-REF-1 vectors (PHY00a 6)
// ============================================================================
//
// code/specs/fixtures/trig-reference-v1 holds every vector as one row
//
//	op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// where x and y are IEEE binary64 *bit patterns* (16 lowercase hex digits), never
// decimals. This test decodes x with math.Float64frombits, calls the public
// function, encodes the result with math.Float64bits and compares the two
// uint64 values (PHY00a 6.5). Printed floats are never compared: they hide the
// last digits. The only relaxation is NaN, which matches any NaN (VER04p "same":
// the payload and sign of a NaN are not portable).
//
// Three kinds of check live here:
//
//  1. fixture integrity: the files on disk are the ones the manifest hashes;
//  2. bit-for-bit match: every row of every group, with the failing rows
//     counted per op so a failure says which function is wrong;
//  3. accuracy against `cr`, the correctly rounded value: sin and cos within
//     3 representable steps, radians and degrees within 1. `tan` is the one
//     named no-claim op (PHY00a 3.8); any other op that carries a `cr` but has no
//     limit fails, and so does an op this file does not know.
//
// Go note: Go has no flag that forbids fused multiply-add, and on arm64 it may
// fuse `x*y + z`. These tests run on whatever machine runs `go test`, so on arm64
// they check that the explicit float64() conversions in trig.go really do keep
// the arithmetic unfused; on amd64 they run the same vectors against the default
// code generation.

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"testing"
)

const fixtureDir = "../../../specs/fixtures/trig-reference-v1"

var fixtureGroups = []string{"edge", "sweep-binade", "sweep-cw", "sweep-prng"}

// qNaNBits is the canonical NaN spelling in the fixture: it matches any NaN.
const qNaNBits = 0x7ff8000000000000

type row struct {
	op, x, y, cr, flags string
}

func fromHex(t testing.TB, s string) float64 {
	t.Helper()
	if len(s) != 16 {
		t.Fatalf("bad hex field %q", s)
	}
	u, err := strconv.ParseUint(s, 16, 64)
	if err != nil {
		t.Fatalf("bad hex field %q: %v", s, err)
	}
	return math.Float64frombits(u)
}

func loadGroup(t testing.TB, group string) []row {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(fixtureDir, group+".tsv"))
	if err != nil {
		t.Fatalf("read %s: %v", group, err)
	}
	text := string(data)
	if !strings.HasSuffix(text, "\n") {
		t.Fatalf("%s: no final newline", group)
	}
	var rows []row
	for _, line := range strings.Split(strings.TrimSuffix(text, "\n"), "\n") {
		f := strings.Split(line, "\t")
		if len(f) != 5 {
			t.Fatalf("%s: want 5 fields, got %d in %q", group, len(f), line)
		}
		rows = append(rows, row{f[0], f[1], f[2], f[3], f[4]})
	}
	return rows
}

// call runs the public function named by op. An unknown op is an error, never a
// silent skip: a new op added to the corpus must fail here until it is handled.
func call(op, x string, t testing.TB) (v float64, err error) {
	if op == "atan2" {
		parts := strings.Split(x, ",")
		if len(parts) != 2 {
			return 0, fmt.Errorf("atan2 input %q", x)
		}
		return Atan2(fromHex(t, parts[0]), fromHex(t, parts[1])), nil
	}
	in := fromHex(t, x)
	switch op {
	case "sin":
		return Sin(in), nil
	case "cos":
		return Cos(in), nil
	case "tan":
		return Tan(in), nil
	case "radians":
		return Radians(in), nil
	case "degrees":
		return Degrees(in), nil
	case "sqrt":
		return Sqrt(in), nil
	case "atan":
		return Atan(in), nil
	}
	return 0, fmt.Errorf("unknown op %q in fixture", op)
}

// panics reports whether call panics (Sqrt of a negative panics in this port).
func panics(op, x string, t testing.TB) (did bool) {
	defer func() {
		if recover() != nil {
			did = true
		}
	}()
	_, _ = call(op, x, t)
	return false
}

// matches applies the VER04p "same" relation to one row.
func matches(t testing.TB, r row) bool {
	if r.y == "error" {
		return panics(r.op, r.x, t)
	}
	got, err := call(r.op, r.x, t)
	if err != nil {
		t.Fatalf("%v", err)
	}
	want, perr := strconv.ParseUint(r.y, 16, 64)
	if perr != nil {
		t.Fatalf("bad y %q", r.y)
	}
	if want == qNaNBits {
		return math.IsNaN(got)
	}
	return math.Float64bits(got) == want
}

// ----------------------------------------------------------------------------
// 1. Fixture integrity
// ----------------------------------------------------------------------------

type manifestFile struct {
	Algorithm string            `json:"algorithm"`
	Constants map[string]string `json:"constants"`
	Groups    []struct {
		File   string `json:"file"`
		Name   string `json:"name"`
		Rows   int    `json:"rows"`
		SHA256 string `json:"sha256"`
	} `json:"groups"`
}

func loadManifest(t testing.TB) manifestFile {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(fixtureDir, "manifest.json"))
	if err != nil {
		t.Fatalf("read manifest: %v", err)
	}
	var m manifestFile
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("parse manifest: %v", err)
	}
	return m
}

func TestManifestHashesAndCountsMatchTheFiles(t *testing.T) {
	m := loadManifest(t)
	if m.Algorithm != "TRIG-REF-1" {
		t.Fatalf("algorithm = %q", m.Algorithm)
	}
	for _, group := range fixtureGroups {
		data, err := os.ReadFile(filepath.Join(fixtureDir, group+".tsv"))
		if err != nil {
			t.Fatal(err)
		}
		sum := sha256.Sum256(data)
		found := false
		for _, g := range m.Groups {
			if g.Name != group {
				continue
			}
			found = true
			if g.SHA256 != hex.EncodeToString(sum[:]) {
				t.Errorf("%s: sha256 mismatch", group)
			}
			if n := strings.Count(string(data), "\n"); n != g.Rows {
				t.Errorf("%s: %d rows, manifest says %d", group, n, g.Rows)
			}
		}
		if !found {
			t.Errorf("manifest has no group %s", group)
		}
	}
}

func TestRowSyntaxIsClosed(t *testing.T) {
	isHex16 := func(s string) bool {
		if len(s) != 16 {
			return false
		}
		for _, c := range s {
			if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
				return false
			}
		}
		return true
	}
	known := map[string]bool{"sin": true, "cos": true, "tan": true, "radians": true,
		"degrees": true, "sqrt": true, "atan": true, "atan2": true}
	for _, group := range fixtureGroups {
		for _, r := range loadGroup(t, group) {
			if !known[r.op] {
				t.Errorf("%s: unknown op %q", group, r.op)
			}
			inputs := []string{r.x}
			if r.op == "atan2" {
				inputs = strings.Split(r.x, ",")
				if len(inputs) != 2 {
					t.Errorf("%s: atan2 input %q", group, r.x)
				}
			}
			for _, in := range inputs {
				if !isHex16(in) {
					t.Errorf("%s: x field %q", group, in)
				}
			}
			if r.y != "error" && !isHex16(r.y) {
				t.Errorf("%s: y field %q", group, r.y)
			}
			if r.cr != "-" && !isHex16(r.cr) {
				t.Errorf("%s: cr field %q", group, r.cr)
			}
		}
	}
}

// ----------------------------------------------------------------------------
// 2. Bit-for-bit match, per group, failing rows counted per op
// ----------------------------------------------------------------------------

func checkGroup(t *testing.T, group string) {
	failing := map[string]int{}
	var first []string
	rows := loadGroup(t, group)
	if len(rows) == 0 {
		t.Fatalf("%s has no rows", group)
	}
	for _, r := range rows {
		if !matches(t, r) {
			failing[r.op]++
			if len(first) < 8 {
				first = append(first, fmt.Sprintf("%s %s want %s", r.op, r.x, r.y))
			}
		}
	}
	if len(failing) > 0 {
		ops := make([]string, 0, len(failing))
		for op := range failing {
			ops = append(ops, op)
		}
		sort.Strings(ops)
		var summary []string
		for _, op := range ops {
			summary = append(summary, fmt.Sprintf("%s=%d", op, failing[op]))
		}
		t.Errorf("%s: %d rows checked, failing per op: %s; first: %v",
			group, len(rows), strings.Join(summary, " "), first)
	}
}

func TestEdgeVectorsMatchBitForBit(t *testing.T)        { checkGroup(t, "edge") }
func TestSweepBinadeVectorsMatchBitForBit(t *testing.T) { checkGroup(t, "sweep-binade") }
func TestSweepCwVectorsMatchBitForBit(t *testing.T)     { checkGroup(t, "sweep-cw") }
func TestSweepPrngVectorsMatchBitForBit(t *testing.T)   { checkGroup(t, "sweep-prng") }

// ----------------------------------------------------------------------------
// 3. Accuracy against the correctly rounded column
// ----------------------------------------------------------------------------

// ordered maps a double's bits onto a monotone integer line, so that the
// distance between two doubles is the number of representable steps between them.
func ordered(bits uint64) int64 {
	mag := int64(bits & 0x7fffffffffffffff)
	if bits>>63 == 1 {
		return -mag
	}
	return mag
}

func TestAccuracyAgainstTheCorrectlyRoundedColumn(t *testing.T) {
	limit := map[string]int64{"sin": 3, "cos": 3, "radians": 1, "degrees": 1}
	// PHY00a 3.8: NO accuracy claim for tan (sin / cos plus a division and a
	// pinned pole sentinel), although tan rows carry a cr. A tan gate is a
	// separate follow-up. Naming it here, rather than skipping "everything
	// without a limit", is what makes the next op that gains a cr fail loudly.
	const noClaim = "tan"
	worst := map[string]int64{}
	for _, group := range fixtureGroups {
		for _, r := range loadGroup(t, group) {
			if r.cr == "-" || r.op == noClaim {
				continue
			}
			if _, ok := limit[r.op]; !ok {
				t.Fatalf("op %q has a cr value but no accuracy limit", r.op)
			}
			got, err := call(r.op, r.x, t)
			if err != nil {
				t.Fatal(err)
			}
			cr, perr := strconv.ParseUint(r.cr, 16, 64)
			if perr != nil {
				t.Fatalf("bad cr %q", r.cr)
			}
			steps := ordered(math.Float64bits(got)) - ordered(cr)
			if steps < 0 {
				steps = -steps
			}
			if steps > worst[r.op] {
				worst[r.op] = steps
			}
		}
	}
	for op, lim := range limit {
		if worst[op] > lim {
			t.Errorf("%s: worst %d steps from cr, limit %d", op, worst[op], lim)
		}
	}
}

func TestAnUnknownOpFails(t *testing.T) {
	if _, err := call("sinh", "0000000000000000", t); err == nil {
		t.Fatal("an unknown op must be an error")
	}
}

// ----------------------------------------------------------------------------
// Constants: the manifest's table equals the bits the package embeds
// ----------------------------------------------------------------------------

func manifestBits(t *testing.T, m manifestFile, name string) uint64 {
	t.Helper()
	s, ok := m.Constants[name]
	if !ok {
		t.Fatalf("manifest has no constant %s", name)
	}
	u, err := strconv.ParseUint(s, 16, 64)
	if err != nil {
		t.Fatal(err)
	}
	return u
}

func TestConstantsMatchTheManifest(t *testing.T) {
	m := loadManifest(t)
	want := map[string]float64{
		"PI": PI, "TWO_PI": TwoPI, "HALF_PI": halfPI, "INV_PIO2": invPio2,
		"PIO2_1": pio2P1, "PIO2_2": pio2P2, "PIO2_3": pio2P3, "PIO2_4": pio2P4,
		"PIO2_5": pio2P5, "MAGIC": magic, "LIMIT": limit, "TINY": tiny,
		"RAD_PER_DEG": radPerDeg, "DEG_PER_RAD": degPerRad,
	}
	for n := 1; n <= 9; n++ {
		want[fmt.Sprintf("S%d", n)] = sinCoef[n-1]
	}
	for n := 2; n <= 9; n++ {
		want[fmt.Sprintf("C%d", n)] = cosCoef[n-2]
	}
	for name, v := range want {
		if got := math.Float64bits(v); got != manifestBits(t, m, name) {
			t.Errorf("%s = %016x, manifest %016x", name, got, manifestBits(t, m, name))
		}
	}
}

// The decimal literals the source still writes parse to the pinned bit patterns.
func TestDecimalLiteralsHaveThePinnedBits(t *testing.T) {
	lit := func(f float64) uint64 { return math.Float64bits(f) }
	for _, c := range []struct {
		name string
		got  uint64
		want uint64
	}{
		{"1e-15 (tan pole, sqrt tolerance)", lit(1e-15), 0x3CD203AF9EE75616},
		{"1e308 (tan sentinel)", lit(1.0e308), 0x7FE1CCF385EBC8A0},
		{"1e-17 (atan cut-off)", lit(1e-17), 0x3C670EF54646D497},
		{"1e-300 (sqrt floor)", lit(1e-300), 0x01A56E1FC2F8F359},
		{"2^-27 (atan threshold)", lit(7.450580596923828e-9), 0x3E40000000000000},
	} {
		if c.got != c.want {
			t.Errorf("%s: %016x, want %016x", c.name, c.got, c.want)
		}
	}
}

func TestDegreeConstantsEqualTheDivisions(t *testing.T) {
	// Computed at run time from variables, so the division is rounded once.
	pi, oneEighty := float64(PI), 180.0
	if radPerDeg != pi/oneEighty || degPerRad != oneEighty/pi {
		t.Error("RAD_PER_DEG / DEG_PER_RAD differ from PI/180 and 180/PI")
	}
}

// ----------------------------------------------------------------------------
// Contract points of PHY00a 3.3 asserted directly
// ----------------------------------------------------------------------------

func bitsEq(a float64, want uint64) bool { return math.Float64bits(a) == want }

func TestSinOfNegativeZeroIsNegativeZero(t *testing.T) {
	if !bitsEq(Sin(math.Copysign(0, -1)), 0x8000000000000000) || !bitsEq(Sin(0), 0) {
		t.Error("sin must keep the sign of zero")
	}
}

func TestCosOfZeroIsOne(t *testing.T) {
	if Cos(0) != 1 || Cos(math.Copysign(0, -1)) != 1 {
		t.Error("cos(+-0) must be 1")
	}
}

func TestTinyArgumentsAreReturnedUnchanged(t *testing.T) {
	below := math.Float64frombits(0x3e3fffffffffffff)
	if Sin(below) != below || Cos(below) != 1 {
		t.Error("below 2^-27 sin(x) = x and cos(x) = 1")
	}
	if !bitsEq(Sin(math.SmallestNonzeroFloat64), 1) {
		t.Error("sin(min subnormal) must be the min subnormal")
	}
}

func TestOutsideTheDomainIsNaN(t *testing.T) {
	for _, x := range []float64{
		math.NaN(), math.Inf(1), math.Inf(-1), 1e20,
		math.Float64frombits(0x41c0000000000001), math.MaxFloat64,
	} {
		if !math.IsNaN(Sin(x)) || !math.IsNaN(Cos(x)) || !math.IsNaN(Tan(x)) {
			t.Errorf("sin/cos/tan(%v) must be NaN", x)
		}
	}
}

func TestTheLastInDomainValueIsFinite(t *testing.T) {
	if !bitsEq(Sin(536870912.0), 0x3FD4E67C0E2622DF) || !bitsEq(Cos(536870912.0), 0xBFEE3EDD2DFEEF90) {
		t.Error("sin/cos(2^29) bits")
	}
}

func TestRadiansAndDegreesAreSingleMultiplies(t *testing.T) {
	if Radians(180) != PI || Degrees(PI) != 180 {
		t.Error("radians(180) = PI and degrees(PI) = 180 exactly")
	}
	if !math.IsInf(Degrees(1.7e308), 1) || !math.IsNaN(Radians(math.NaN())) {
		t.Error("degrees overflows to +Inf, NaN propagates")
	}
	if !bitsEq(Radians(math.Copysign(0, -1)), 0x8000000000000000) ||
		!bitsEq(Degrees(math.Copysign(0, -1)), 0x8000000000000000) {
		t.Error("zero keeps its sign")
	}
}

func TestTanPoleSentinel(t *testing.T) {
	if Tan(halfPI) != 1e308 || Tan(-halfPI) != -1e308 {
		t.Error("tan at the poles is the +-1e308 sentinel")
	}
}

func TestAtan2OfNaNIsNaN(t *testing.T) {
	nan := math.NaN()
	for _, c := range [][2]float64{{nan, 1}, {1, nan}, {-1, nan}, {nan, nan}, {nan, -1}, {nan, 0}} {
		if !math.IsNaN(Atan2(c[0], c[1])) {
			t.Errorf("Atan2(%v, %v) must be NaN", c[0], c[1])
		}
	}
}

func TestAtan2PinnedConventions(t *testing.T) {
	if Atan2(math.Copysign(0, -1), -1) != PI || Atan2(0, 0) != 0 {
		t.Error("atan2(-0, -1) = +PI and atan2(0, 0) = 0")
	}
	if !math.IsNaN(Atan2(math.Inf(1), math.Inf(1))) || Atan2(1e300, 1e-300) != halfPI {
		t.Error("atan2 inf/inf is NaN and an overflowing y/x gives HALF_PI")
	}
}

func TestAtanKeepsTheSignOfZero(t *testing.T) {
	if !bitsEq(Atan(math.Copysign(0, -1)), 0x8000000000000000) {
		t.Error("atan(-0) = -0")
	}
}

func TestNegativeQuadrantArithmetic(t *testing.T) {
	// k = -1 and k = -5 map to quadrant 3: "& 3" is the floor modulus.
	if Sin(-halfPI) != -Sin(halfPI) {
		t.Error("sin is odd at -pi/2")
	}
	if math.IsNaN(Sin(-5*halfPI)) || math.Abs(Cos(-5*halfPI)) > 1e-15 {
		t.Error("quadrant of k = -5")
	}
}

func TestSqrtOfNegativePanics(t *testing.T) {
	for _, x := range []float64{-1, math.Inf(-1)} {
		func() {
			defer func() {
				if recover() == nil {
					t.Errorf("Sqrt(%v) must panic", x)
				}
			}()
			Sqrt(x)
		}()
	}
}
