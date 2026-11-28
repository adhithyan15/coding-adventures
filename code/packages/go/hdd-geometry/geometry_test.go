package hddgeometry

import (
	"encoding/json"
	"errors"
	"math"
	"math/big"
	"os"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
)

var (
	geometrySink blockdevicecontract.Geometry
	addressSink  Address
	blockSink    int64
)

type vectorFile struct {
	Vectors []struct {
		Name            string `json:"name"`
		Cylinders       int64  `json:"cylinders"`
		Surfaces        int64  `json:"surfaces"`
		SectorsPerTrack int64  `json:"sectors_per_track"`
		Block           int64  `json:"block"`
		Cylinder        int64  `json:"cylinder"`
		Surface         int64  `json:"surface"`
		Sector          int64  `json:"sector"`
	} `json:"vectors"`
}

func TestPortableJSONVectors(t *testing.T) {
	data, err := os.ReadFile("testdata/portable-vectors.json")
	if err != nil {
		t.Fatal(err)
	}
	var vectors vectorFile
	if err := json.Unmarshal(data, &vectors); err != nil {
		t.Fatal(err)
	}
	if len(vectors.Vectors) != 6 {
		t.Fatalf("vector count=%d", len(vectors.Vectors))
	}
	for _, vector := range vectors.Vectors {
		t.Run(vector.Name, func(t *testing.T) {
			geometry := Geometry{Cylinders: vector.Cylinders, Surfaces: vector.Surfaces, SectorsPerTrack: vector.SectorsPerTrack}
			address, err := geometry.Locate(vector.Block)
			want := Address{Cylinder: vector.Cylinder, Surface: vector.Surface, Sector: vector.Sector}
			if err != nil || address != want {
				t.Fatalf("Locate=%+v err=%v want %+v", address, err, want)
			}
			block, err := geometry.Block(want)
			if err != nil || block != vector.Block {
				t.Fatalf("Block=%d err=%v want %d", block, err, vector.Block)
			}
			blockGeometry, err := geometry.BlockGeometry()
			if err != nil || blockGeometry != (blockdevicecontract.Geometry{BlockCount: vector.Cylinders * vector.Surfaces * vector.SectorsPerTrack, BytesPerBlock: 8}) {
				t.Fatalf("BlockGeometry=%+v err=%v", blockGeometry, err)
			}
		})
	}
}

func TestExhaustiveSmallGeometriesMatchIndependentEnumerator(t *testing.T) {
	checked := 0
	for cylinders := int64(1); cylinders <= 6; cylinders++ {
		for surfaces := int64(1); surfaces <= 5; surfaces++ {
			for sectors := int64(1); sectors <= 9; sectors++ {
				geometry := Geometry{Cylinders: cylinders, Surfaces: surfaces, SectorsPerTrack: sectors}
				blockGeometry, err := geometry.BlockGeometry()
				if err != nil {
					t.Fatalf("small geometry refused: %+v: %v", geometry, err)
				}
				wantCount := cylinders * surfaces * sectors
				if blockGeometry != (blockdevicecontract.Geometry{BlockCount: wantCount, BytesPerBlock: 8}) {
					t.Fatalf("block geometry=%+v want count %d", blockGeometry, wantCount)
				}
				var ordinal int64
				for cylinder := int64(0); cylinder < cylinders; cylinder++ {
					for surface := int64(0); surface < surfaces; surface++ {
						for sector := int64(0); sector < sectors; sector++ {
							want := Address{Cylinder: cylinder, Surface: surface, Sector: sector}
							got, err := geometry.Locate(ordinal)
							if err != nil || got != want {
								t.Fatalf("Locate(%d)=%+v,%v want %+v", ordinal, got, err, want)
							}
							back, err := geometry.Block(want)
							if err != nil || back != ordinal {
								t.Fatalf("Block(%+v)=%d,%v want %d", want, back, err, ordinal)
							}
							ordinal++
							checked++
						}
					}
				}
				if ordinal != wantCount {
					t.Fatalf("enumerated %d blocks, wanted %d", ordinal, wantCount)
				}
			}
		}
	}
	if checked < 10_000 {
		t.Fatalf("too few independent enumerations: %d", checked)
	}
	t.Logf("independently enumerated %d block addresses", checked)
}

func TestInvalidGeometryAndAddressPrecedence(t *testing.T) {
	invalid := []Geometry{
		{},
		{Cylinders: 0, Surfaces: 1, SectorsPerTrack: 1},
		{Cylinders: 1, Surfaces: 0, SectorsPerTrack: 1},
		{Cylinders: 1, Surfaces: 1, SectorsPerTrack: 0},
		{Cylinders: -1, Surfaces: 1, SectorsPerTrack: 1},
		{Cylinders: 1, Surfaces: -1, SectorsPerTrack: 1},
		{Cylinders: 1, Surfaces: 1, SectorsPerTrack: -1},
	}
	for _, geometry := range invalid {
		if got, err := geometry.BlockGeometry(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (blockdevicecontract.Geometry{}) {
			t.Fatalf("invalid BlockGeometry(%+v)=(%+v,%v)", geometry, got, err)
		}
		if got, err := geometry.Locate(-1); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (Address{}) {
			t.Fatalf("invalid Locate(%+v)=(%+v,%v)", geometry, got, err)
		}
		if got, err := geometry.Block(Address{Cylinder: -1}); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != 0 {
			t.Fatalf("invalid Block(%+v)=(%d,%v)", geometry, got, err)
		}
	}

	geometry := Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	for _, block := range []int64{-1, 512, math.MinInt64, math.MaxInt64} {
		if got, err := geometry.Locate(block); !errors.Is(err, blockdevicecontract.ErrInvalidBlock) || got != (Address{}) {
			t.Fatalf("out-of-range block %d=(%+v,%v)", block, got, err)
		}
	}
	for _, address := range []Address{
		{Cylinder: -1}, {Cylinder: 16}, {Surface: -1}, {Surface: 2}, {Sector: -1}, {Sector: 16},
		{Cylinder: math.MaxInt64}, {Surface: math.MinInt64}, {Sector: math.MaxInt64},
	} {
		if got, err := geometry.Block(address); !errors.Is(err, ErrInvalidAddress) || got != 0 {
			t.Fatalf("invalid address %+v=(%d,%v)", address, got, err)
		}
	}
}

func TestSurfaceThenCylinderAddressOrder(t *testing.T) {
	geometry := Geometry{Cylinders: 2, Surfaces: 2, SectorsPerTrack: 2}
	want := []Address{
		{Cylinder: 0, Surface: 0, Sector: 0},
		{Cylinder: 0, Surface: 0, Sector: 1},
		{Cylinder: 0, Surface: 1, Sector: 0},
		{Cylinder: 0, Surface: 1, Sector: 1},
		{Cylinder: 1, Surface: 0, Sector: 0},
		{Cylinder: 1, Surface: 0, Sector: 1},
		{Cylinder: 1, Surface: 1, Sector: 0},
		{Cylinder: 1, Surface: 1, Sector: 1},
	}
	for block, expected := range want {
		address, err := geometry.Locate(int64(block))
		if err != nil || address != expected {
			t.Fatalf("surface then cylinder mapping at block %d: got %+v, %v; want %+v", block, address, err, expected)
		}
		back, err := geometry.Block(expected)
		if err != nil || back != int64(block) {
			t.Fatalf("surface then cylinder inverse at address %+v: %d, %v; want %d", expected, back, err, block)
		}
	}
}

func TestLocateRefusesFirstBlockPastGeometry(t *testing.T) {
	geometry := Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	address, err := geometry.Locate(512)
	if !errors.Is(err, blockdevicecontract.ErrInvalidBlock) || address != (Address{}) {
		t.Fatalf("first block past geometry returned %+v, %v", address, err)
	}
}

func TestPortableProductBoundaryAndFinalBlock(t *testing.T) {
	const maximum int64 = 9_007_199_254_740_991
	valid := Geometry{Cylinders: maximum, Surfaces: 1, SectorsPerTrack: 1}
	blockGeometry, err := valid.BlockGeometry()
	if err != nil || blockGeometry != (blockdevicecontract.Geometry{BlockCount: maximum, BytesPerBlock: 8}) {
		t.Fatalf("maximum portable geometry=(%+v,%v)", blockGeometry, err)
	}
	lastAddress, err := valid.Locate(maximum - 1)
	if err != nil || lastAddress != (Address{Cylinder: maximum - 1}) {
		t.Fatalf("last portable address=(%+v,%v)", lastAddress, err)
	}
	lastBlock, err := valid.Block(lastAddress)
	if err != nil || lastBlock != maximum-1 {
		t.Fatalf("last portable block=(%d,%v)", lastBlock, err)
	}
	if _, err := valid.Locate(maximum); !errors.Is(err, blockdevicecontract.ErrInvalidBlock) {
		t.Fatalf("first block past portable geometry: %v", err)
	}
	invalid := Geometry{Cylinders: maximum, Surfaces: 1, SectorsPerTrack: 2}
	if got, err := invalid.BlockGeometry(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (blockdevicecontract.Geometry{}) {
		t.Fatalf("over-limit product=(%+v,%v)", got, err)
	}
}

func TestWideProductsMatchBigIntegerOracle(t *testing.T) {
	values := []int64{math.MinInt64, -1, 0, 1, 2, 16, 512, 9_007_199_254_740_991, math.MaxInt64}
	limit := big.NewInt(9_007_199_254_740_991)
	checked := 0
	for _, cylinders := range values {
		for _, surfaces := range values {
			for _, sectors := range values {
				geometry := Geometry{Cylinders: cylinders, Surfaces: surfaces, SectorsPerTrack: sectors}
				wantValid := cylinders > 0 && surfaces > 0 && sectors > 0
				var product *big.Int
				if wantValid {
					product = new(big.Int).Mul(big.NewInt(cylinders), big.NewInt(surfaces))
					product.Mul(product, big.NewInt(sectors))
					wantValid = product.Cmp(limit) <= 0
				}
				got, err := geometry.BlockGeometry()
				if wantValid {
					if err != nil || got.BlockCount != product.Int64() || got.BytesPerBlock != 8 {
						t.Fatalf("wide valid %+v=(%+v,%v) want %s", geometry, got, err, product)
					}
				} else if !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (blockdevicecontract.Geometry{}) {
					t.Fatalf("wide invalid %+v=(%+v,%v)", geometry, got, err)
				}
				checked++
			}
		}
	}
	if checked != len(values)*len(values)*len(values) {
		t.Fatalf("checked=%d", checked)
	}
	t.Logf("wide big.Int geometry comparisons: %d", checked)
}

func TestGeometryOperationsAllocateNothing(t *testing.T) {
	geometry := Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	address := Address{Cylinder: 15, Surface: 1, Sector: 15}
	if allocations := testing.AllocsPerRun(1_000, func() {
		geometrySink, _ = geometry.BlockGeometry()
		addressSink, _ = geometry.Locate(511)
		blockSink, _ = geometry.Block(address)
	}); allocations != 0 {
		t.Fatalf("geometry operations allocated %v times", allocations)
	}
}

func FuzzGeometryRoundTrips(f *testing.F) {
	for _, values := range [][4]int64{
		{4, 2, 4, 31}, {16, 2, 16, 511}, {64, 2, 4, 511},
		{1, 1, 1, 0}, {9_007_199_254_740_991, 1, 1, 9_007_199_254_740_990},
		{-1, 2, 4, 0}, {math.MaxInt64, math.MaxInt64, math.MaxInt64, math.MaxInt64},
	} {
		f.Add(values[0], values[1], values[2], values[3])
	}
	f.Fuzz(func(t *testing.T, cylinders, surfaces, sectors, block int64) {
		geometry := Geometry{Cylinders: cylinders, Surfaces: surfaces, SectorsPerTrack: sectors}
		validGeometry := cylinders > 0 && surfaces > 0 && sectors > 0
		var count *big.Int
		if validGeometry {
			count = new(big.Int).Mul(big.NewInt(cylinders), big.NewInt(surfaces))
			count.Mul(count, big.NewInt(sectors))
			validGeometry = count.Cmp(big.NewInt(9_007_199_254_740_991)) <= 0
		}
		blockGeometry, geometryErr := geometry.BlockGeometry()
		if !validGeometry {
			if !errors.Is(geometryErr, blockdevicecontract.ErrInvalidGeometry) || blockGeometry != (blockdevicecontract.Geometry{}) {
				t.Fatalf("geometry oracle mismatch %+v -> %+v, %v", geometry, blockGeometry, geometryErr)
			}
			return
		}
		if geometryErr != nil || blockGeometry.BlockCount != count.Int64() {
			t.Fatalf("valid geometry mismatch %+v -> %+v, %v", geometry, blockGeometry, geometryErr)
		}
		if block < 0 || block >= blockGeometry.BlockCount {
			if address, err := geometry.Locate(block); !errors.Is(err, blockdevicecontract.ErrInvalidBlock) || address != (Address{}) {
				t.Fatalf("invalid block %d -> %+v, %v", block, address, err)
			}
			return
		}
		track := block / sectors
		want := Address{Cylinder: track / surfaces, Surface: track % surfaces, Sector: block % sectors}
		address, err := geometry.Locate(block)
		if err != nil || address != want {
			t.Fatalf("Locate(%d)=%+v,%v want %+v", block, address, err, want)
		}
		back, err := geometry.Block(address)
		if err != nil || back != block {
			t.Fatalf("Block(%+v)=%d,%v want %d", address, back, err, block)
		}
	})
}
