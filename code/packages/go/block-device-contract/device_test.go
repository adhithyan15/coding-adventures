package blockdevicecontract

import (
	"encoding/binary"
	"errors"
	"math"
	"testing"
)

const modelMaximumBlockCount int64 = 9_007_199_254_740_991
const modelMaximumWidth int64 = 65_536

func TestGeometryExactLimitsAndSignedExtremes(t *testing.T) {
	cases := []struct {
		name string
		g    Geometry
		want error
	}{
		{"minimum", Geometry{1, 1}, nil},
		{"maximum", Geometry{modelMaximumBlockCount, modelMaximumWidth}, nil},
		{"zero blocks", Geometry{0, 1}, ErrInvalidGeometry},
		{"negative blocks", Geometry{-1, 1}, ErrInvalidGeometry},
		{"minimum signed block count", Geometry{math.MinInt64, 1}, ErrInvalidGeometry},
		{"above portable blocks", Geometry{modelMaximumBlockCount + 1, 1}, ErrInvalidGeometry},
		{"maximum signed block count", Geometry{math.MaxInt64, 1}, ErrInvalidGeometry},
		{"zero width", Geometry{1, 0}, ErrInvalidGeometry},
		{"negative width", Geometry{1, -1}, ErrInvalidGeometry},
		{"minimum signed width", Geometry{1, math.MinInt64}, ErrInvalidGeometry},
		{"above maximum width", Geometry{1, modelMaximumWidth + 1}, ErrInvalidGeometry},
		{"maximum signed width", Geometry{1, math.MaxInt64}, ErrInvalidGeometry},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) { assertErrorIdentity(t, tc.g.Validate(), tc.want) })
	}
}

func TestCheckBlockSignedLimitsAndValidationPrecedence(t *testing.T) {
	valid := Geometry{BlockCount: 4, BytesPerBlock: 8}
	for _, tc := range []struct {
		name  string
		g     Geometry
		block int64
		want  error
	}{
		{"first", valid, 0, nil},
		{"last", valid, 3, nil},
		{"negative", valid, -1, ErrInvalidBlock},
		{"minimum signed", valid, math.MinInt64, ErrInvalidBlock},
		{"at count", valid, 4, ErrInvalidBlock},
		{"maximum signed", valid, math.MaxInt64, ErrInvalidBlock},
		{"bad geometry precedes address", Geometry{0, 0}, math.MinInt64, ErrInvalidGeometry},
		{"maximum accepted address", Geometry{modelMaximumBlockCount, modelMaximumWidth}, modelMaximumBlockCount - 1, nil},
		{"one past portable count", Geometry{modelMaximumBlockCount, modelMaximumWidth}, modelMaximumBlockCount, ErrInvalidBlock},
	} {
		t.Run(tc.name, func(t *testing.T) { assertErrorIdentity(t, tc.g.CheckBlock(tc.block), tc.want) })
	}
}

func TestCheckWriteValidationPrecedenceAndExactWidth(t *testing.T) {
	data := []byte{1, 2, 3, 4}
	cases := []struct {
		name  string
		g     Geometry
		block int64
		data  []byte
		want  error
	}{
		{"valid", Geometry{4, 4}, 2, data, nil},
		{"nil accepted only for zero width is impossible", Geometry{4, 1}, 2, nil, ErrInvalidBlockData},
		{"short", Geometry{4, 4}, 2, data[:3], ErrInvalidBlockData},
		{"long", Geometry{4, 3}, 2, data, ErrInvalidBlockData},
		{"geometry before address and width", Geometry{0, 0}, -1, nil, ErrInvalidGeometry},
		{"address before width", Geometry{4, 4}, -1, nil, ErrInvalidBlock},
		{"upper address before width", Geometry{4, 4}, 4, nil, ErrInvalidBlock},
		{"portable width exact", Geometry{1, modelMaximumWidth}, 0, make([]byte, int(modelMaximumWidth)), nil},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) { assertErrorIdentity(t, tc.g.CheckWrite(tc.block, tc.data), tc.want) })
	}
	before := append([]byte(nil), data...)
	if err := (Geometry{4, 4}).CheckWrite(0, data); err != nil {
		t.Fatal(err)
	}
	if string(data) != string(before) {
		t.Fatal("CheckWrite changed caller bytes")
	}
}

func TestChecksDoNotAllocate(t *testing.T) {
	geometry := Geometry{BlockCount: 16, BytesPerBlock: 8}
	data := make([]byte, 8)
	checks := []func() error{
		geometry.Validate,
		func() error { return geometry.CheckBlock(15) },
		func() error { return geometry.CheckBlock(-1) },
		func() error { return geometry.CheckWrite(15, data) },
		func() error { return geometry.CheckWrite(16, nil) },
	}
	for index, check := range checks {
		if allocations := testing.AllocsPerRun(100, func() { _ = check() }); allocations != 0 {
			t.Errorf("check %d allocated %g times", index, allocations)
		}
	}
}

func TestErrorSentinelsAreStable(t *testing.T) {
	if !errors.Is((Geometry{}).Validate(), ErrInvalidGeometry) {
		t.Fatal("invalid geometry error identity lost")
	}
	if !errors.Is((Geometry{1, 1}).CheckBlock(1), ErrInvalidBlock) {
		t.Fatal("invalid block error identity lost")
	}
	if !errors.Is((Geometry{1, 1}).CheckWrite(0, nil), ErrInvalidBlockData) {
		t.Fatal("invalid data error identity lost")
	}
}

func TestIndependentPrecedenceTable(t *testing.T) {
	geometries := []Geometry{{0, 0}, {-1, 8}, {1, 65_537}, {1, 8}, {4, 8}, {modelMaximumBlockCount, modelMaximumWidth}}
	blocks := []int64{math.MinInt64, -1, 0, 1, 3, 4, modelMaximumBlockCount - 1, modelMaximumBlockCount, math.MaxInt64}
	lengths := []int{0, 1, 7, 8, 9}
	for _, geometry := range geometries {
		for _, block := range blocks {
			wantBlock := modelCheckBlock(geometry, block)
			assertErrorIdentity(t, geometry.CheckBlock(block), wantBlock)
			for _, length := range lengths {
				want := modelCheckWrite(geometry, block, length)
				assertErrorIdentity(t, geometry.CheckWrite(block, make([]byte, length)), want)
			}
		}
	}
}

func FuzzGeometryAndAddressChecks(f *testing.F) {
	f.Add(int64(4), int64(8), int64(0), uint8(8))
	f.Add(modelMaximumBlockCount, modelMaximumWidth, modelMaximumBlockCount-1, uint8(0))
	f.Add(int64(-1), int64(65_537), int64(math.MinInt64), uint8(9))
	f.Fuzz(func(t *testing.T, blocks, width, block int64, dataLength uint8) {
		length := int(dataLength % 18)
		geometry := Geometry{BlockCount: blocks, BytesPerBlock: width}
		assertErrorIdentity(t, geometry.Validate(), modelValidate(geometry))
		assertErrorIdentity(t, geometry.CheckBlock(block), modelCheckBlock(geometry, block))
		assertErrorIdentity(t, geometry.CheckWrite(block, make([]byte, length)), modelCheckWrite(geometry, block, length))
	})
}

func modelValidate(g Geometry) error {
	if g.BlockCount < 1 || g.BlockCount > modelMaximumBlockCount || g.BytesPerBlock < 1 || g.BytesPerBlock > modelMaximumWidth {
		return ErrInvalidGeometry
	}
	return nil
}

func modelCheckBlock(g Geometry, block int64) error {
	if modelValidate(g) != nil {
		return ErrInvalidGeometry
	}
	if block < 0 || block >= g.BlockCount {
		return ErrInvalidBlock
	}
	return nil
}

func modelCheckWrite(g Geometry, block int64, dataLength int) error {
	if err := modelCheckBlock(g, block); err != nil {
		return err
	}
	if int64(dataLength) != g.BytesPerBlock {
		return ErrInvalidBlockData
	}
	return nil
}

func assertErrorIdentity(t *testing.T, got, want error) {
	t.Helper()
	if want == nil {
		if got != nil {
			t.Fatalf("error=%v; want nil", got)
		}
		return
	}
	if !errors.Is(got, want) {
		t.Fatalf("error=%v; want identity %v", got, want)
	}
}

func TestFuzzBoundaryEncodingKeepsExactSignedValues(t *testing.T) {
	// A compact deterministic table ensures all signed fields pass unchanged
	// through the pure checks; no derived address multiplication is performed.
	for _, raw := range [][]byte{
		make([]byte, 8),
		{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f},
		{0, 0, 0, 0, 0, 0, 0, 0x80},
	} {
		value := int64(binary.LittleEndian.Uint64(raw))
		got := (Geometry{BlockCount: value, BytesPerBlock: 1}).Validate()
		assertErrorIdentity(t, got, modelValidate(Geometry{BlockCount: value, BytesPerBlock: 1}))
	}
}
