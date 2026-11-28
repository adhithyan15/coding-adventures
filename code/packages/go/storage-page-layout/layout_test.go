package storagepagelayout

import (
	"encoding/json"
	"errors"
	"math"
	"os"
	"path/filepath"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
)

const oracleMaxBlockCount int64 = 9_007_199_254_740_991
const oracleMaxBytesPerBlock int64 = 65_536

func TestValidationPreservesGeometryErrorAndThenChecksLayout(t *testing.T) {
	cases := []struct {
		name   string
		layout Layout
		want   error
	}{
		{"underlying geometry first", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 0, BytesPerBlock: 0}, FirstBlock: -1, PageCount: 0, BlocksPerPage: 0}, blockdevicecontract.ErrInvalidGeometry},
		{"negative first block", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 2}, FirstBlock: -1, PageCount: 1, BlocksPerPage: 1}, ErrInvalidLayout},
		{"first block at end", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 2}, FirstBlock: 8, PageCount: 1, BlocksPerPage: 1}, ErrInvalidLayout},
		{"zero pages", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 2}, FirstBlock: 0, PageCount: 0, BlocksPerPage: 1}, ErrInvalidLayout},
		{"negative page count", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 2}, FirstBlock: 0, PageCount: -1, BlocksPerPage: 1}, ErrInvalidLayout},
		{"zero blocks per page", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 2}, FirstBlock: 0, PageCount: 1, BlocksPerPage: 0}, ErrInvalidLayout},
		{"negative blocks per page", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 2}, FirstBlock: 0, PageCount: 1, BlocksPerPage: -1}, ErrInvalidLayout},
		{"page too wide", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 65_537, BytesPerBlock: 1}, FirstBlock: 0, PageCount: 1, BlocksPerPage: 65_537}, ErrInvalidLayout},
		{"one block short", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 16, BytesPerBlock: 8}, FirstBlock: 5, PageCount: 3, BlocksPerPage: 4}, ErrInvalidLayout},
		{"signed page count maximum cannot fit", Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 1}, FirstBlock: 0, PageCount: math.MaxInt64, BlocksPerPage: 1}, ErrInvalidLayout},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			if err := test.layout.Validate(); !errors.Is(err, test.want) {
				t.Fatalf("Validate()=%v want identity %v", err, test.want)
			}
		})
	}
}

func TestSpanValidatesLayoutBeforePageAndReturnsZeroOnRefusal(t *testing.T) {
	badGeometry := Layout{Geometry: blockdevicecontract.Geometry{}, FirstBlock: -1, PageCount: 0, BlocksPerPage: 0}
	if span, err := badGeometry.Span(math.MinInt64); span != (BlockSpan{}) || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("invalid underlying geometry span=(%+v,%v)", span, err)
	}
	badLayout := Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 4, BytesPerBlock: 8}, FirstBlock: -1, PageCount: 1, BlocksPerPage: 1}
	if span, err := badLayout.Span(math.MaxInt64); span != (BlockSpan{}) || !errors.Is(err, ErrInvalidLayout) {
		t.Fatalf("invalid layout span=(%+v,%v)", span, err)
	}
	valid := Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 4, BytesPerBlock: 8}, FirstBlock: 1, PageCount: 2, BlocksPerPage: 1}
	for _, page := range []int64{math.MinInt64, -1, 2, math.MaxInt64} {
		if span, err := valid.Span(page); span != (BlockSpan{}) || !errors.Is(err, ErrInvalidPage) {
			t.Fatalf("Span(%d)=(%+v,%v), want zero ErrInvalidPage", page, span, err)
		}
	}
}

func TestFiniteLayoutsAgainstOccupiedBlockOracle(t *testing.T) {
	for blockCount := int64(1); blockCount <= 10; blockCount++ {
		for bytesPerBlock := int64(1); bytesPerBlock <= 4; bytesPerBlock++ {
			geometry := blockdevicecontract.Geometry{BlockCount: blockCount, BytesPerBlock: bytesPerBlock}
			for firstBlock := int64(-1); firstBlock <= blockCount; firstBlock++ {
				for pageCount := int64(-1); pageCount <= blockCount+1; pageCount++ {
					for blocksPerPage := int64(-1); blocksPerPage <= blockCount+1; blocksPerPage++ {
						layout := Layout{Geometry: geometry, FirstBlock: firstBlock, PageCount: pageCount, BlocksPerPage: blocksPerPage}
						wantErr := oracleValidate(layout)
						if gotErr := layout.Validate(); !sameError(gotErr, wantErr) {
							t.Fatalf("layout=%+v Validate()=%v want=%v", layout, gotErr, wantErr)
						}
						if wantErr != nil {
							continue
						}
						occupied := map[int64]int64{}
						for page := int64(0); page < pageCount; page++ {
							wantSpan := oracleSpan(layout, page)
							gotSpan, err := layout.Span(page)
							if err != nil || gotSpan != wantSpan {
								t.Fatalf("layout=%+v page=%d span=(%+v,%v) want=%+v", layout, page, gotSpan, err, wantSpan)
							}
							for block := wantSpan.FirstBlock; block < wantSpan.FirstBlock+wantSpan.BlockCount; block++ {
								if previous, exists := occupied[block]; exists {
									t.Fatalf("layout=%+v pages %d and %d overlap at block %d", layout, previous, page, block)
								}
								occupied[block] = page
							}
							if page > 0 {
								previous := oracleSpan(layout, page-1)
								if previous.FirstBlock+previous.BlockCount != gotSpan.FirstBlock {
									t.Fatalf("layout=%+v page=%d is not adjacent to predecessor", layout, page)
								}
							}
						}
					}
				}
			}
		}
	}
}

func TestSignedPortableCeilingsAndCheckedArithmetic(t *testing.T) {
	maxWidth := Layout{
		Geometry:      blockdevicecontract.Geometry{BlockCount: oracleMaxBlockCount, BytesPerBlock: oracleMaxBytesPerBlock},
		FirstBlock:    oracleMaxBlockCount - 1,
		PageCount:     1,
		BlocksPerPage: 1,
	}
	if err := maxWidth.Validate(); err != nil {
		t.Fatal(err)
	}
	span, err := maxWidth.Span(0)
	if err != nil || span != (BlockSpan{FirstBlock: oracleMaxBlockCount - 1, BlockCount: 1, BytesPerPage: oracleMaxBytesPerBlock}) {
		t.Fatalf("maximum-safe span=(%+v,%v)", span, err)
	}
	maxCount := Layout{
		Geometry:      blockdevicecontract.Geometry{BlockCount: oracleMaxBlockCount, BytesPerBlock: 1},
		FirstBlock:    0,
		PageCount:     oracleMaxBlockCount,
		BlocksPerPage: 1,
	}
	if err := maxCount.Validate(); err != nil {
		t.Fatal(err)
	}
	last, err := maxCount.Span(oracleMaxBlockCount - 1)
	if err != nil || last != (BlockSpan{FirstBlock: oracleMaxBlockCount - 1, BlockCount: 1, BytesPerPage: 1}) {
		t.Fatalf("maximum-count last span=(%+v,%v)", last, err)
	}
	for _, layout := range []Layout{
		{Geometry: blockdevicecontract.Geometry{BlockCount: oracleMaxBlockCount, BytesPerBlock: 1}, FirstBlock: 0, PageCount: math.MaxInt64, BlocksPerPage: 2},
		{Geometry: blockdevicecontract.Geometry{BlockCount: oracleMaxBlockCount, BytesPerBlock: 1}, FirstBlock: math.MinInt64, PageCount: math.MaxInt64, BlocksPerPage: math.MaxInt64},
		{Geometry: blockdevicecontract.Geometry{BlockCount: oracleMaxBlockCount, BytesPerBlock: 1}, FirstBlock: 0, PageCount: math.MaxInt64, BlocksPerPage: 1},
	} {
		if err := layout.Validate(); !errors.Is(err, ErrInvalidLayout) {
			t.Fatalf("overflow-risk layout=%+v error=%v, want ErrInvalidLayout", layout, err)
		}
	}
}

func TestChecksAllocateNoBuffers(t *testing.T) {
	valid := Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}, FirstBlock: 4, PageCount: 7, BlocksPerPage: 4}
	invalidPage := int64(7)
	bad := Layout{Geometry: blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}, FirstBlock: 4, PageCount: 7, BlocksPerPage: 4}
	checks := []func() error{
		valid.Validate,
		func() error { _, err := valid.Span(6); return err },
		func() error { _, err := bad.Span(invalidPage); return err },
	}
	for index, check := range checks {
		if allocations := testing.AllocsPerRun(100, func() { _ = check() }); allocations != 0 {
			t.Errorf("check %d allocated %g times", index, allocations)
		}
	}
}

func TestPortableVectors(t *testing.T) {
	path := filepath.Join("testdata", "vectors.json")
	encoded, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var fixture struct {
		Schema string `json:"schema"`
		Cases  []struct {
			Name     string `json:"name"`
			Geometry struct {
				BlockCount    int64 `json:"block_count"`
				BytesPerBlock int64 `json:"bytes_per_block"`
			} `json:"geometry"`
			FirstBlock    int64 `json:"first_block"`
			PageCount     int64 `json:"page_count"`
			BlocksPerPage int64 `json:"blocks_per_page"`
			Page          int64 `json:"page"`
			Span          struct {
				FirstBlock   int64 `json:"first_block"`
				BlockCount   int64 `json:"block_count"`
				BytesPerPage int64 `json:"bytes_per_page"`
			} `json:"span"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(encoded, &fixture); err != nil {
		t.Fatal(err)
	}
	if fixture.Schema != "coding-adventures.storage-page-layout.v1" || len(fixture.Cases) != 4 {
		t.Fatalf("schema=%q case count=%d", fixture.Schema, len(fixture.Cases))
	}
	for _, test := range fixture.Cases {
		t.Run(test.Name, func(t *testing.T) {
			layout := Layout{
				Geometry:      blockdevicecontract.Geometry{BlockCount: test.Geometry.BlockCount, BytesPerBlock: test.Geometry.BytesPerBlock},
				FirstBlock:    test.FirstBlock,
				PageCount:     test.PageCount,
				BlocksPerPage: test.BlocksPerPage,
			}
			got, err := layout.Span(test.Page)
			want := BlockSpan{FirstBlock: test.Span.FirstBlock, BlockCount: test.Span.BlockCount, BytesPerPage: test.Span.BytesPerPage}
			if err != nil || got != want {
				t.Fatalf("span=(%+v,%v) want=%+v", got, err, want)
			}
		})
	}
}

func FuzzLayoutAndPage(f *testing.F) {
	f.Add(int64(32), int64(8), int64(4), int64(4), int64(2), int64(2))
	f.Add(oracleMaxBlockCount, oracleMaxBytesPerBlock, oracleMaxBlockCount-1, int64(1), int64(1), int64(0))
	f.Add(int64(8), int64(1), int64(math.MinInt64), int64(math.MaxInt64), int64(math.MaxInt64), int64(math.MinInt64))
	f.Fuzz(func(t *testing.T, blockCount, bytesPerBlock, firstBlock, pageCount, blocksPerPage, page int64) {
		layout := Layout{
			Geometry:      blockdevicecontract.Geometry{BlockCount: blockCount, BytesPerBlock: bytesPerBlock},
			FirstBlock:    firstBlock,
			PageCount:     pageCount,
			BlocksPerPage: blocksPerPage,
		}
		wantErr := oracleValidate(layout)
		gotErr := layout.Validate()
		if !sameError(gotErr, wantErr) {
			t.Fatalf("Validate(%+v)=%v want=%v", layout, gotErr, wantErr)
		}
		wantSpan, wantSpanErr := oracleSpanChecked(layout, page)
		gotSpan, gotSpanErr := layout.Span(page)
		if !sameError(gotSpanErr, wantSpanErr) || gotSpan != wantSpan {
			t.Fatalf("Span(%d)=(%+v,%v) want=(%+v,%v)", page, gotSpan, gotSpanErr, wantSpan, wantSpanErr)
		}
	})
}

func oracleValidate(layout Layout) error {
	geometry := layout.Geometry
	if geometry.BlockCount < 1 || geometry.BlockCount > oracleMaxBlockCount || geometry.BytesPerBlock < 1 || geometry.BytesPerBlock > oracleMaxBytesPerBlock {
		return blockdevicecontract.ErrInvalidGeometry
	}
	if layout.FirstBlock < 0 || layout.FirstBlock >= geometry.BlockCount || layout.PageCount < 1 || layout.BlocksPerPage < 1 {
		return ErrInvalidLayout
	}
	if layout.BlocksPerPage > MaxPageBytes/geometry.BytesPerBlock {
		return ErrInvalidLayout
	}
	remaining := geometry.BlockCount - layout.FirstBlock
	if layout.PageCount > remaining/layout.BlocksPerPage {
		return ErrInvalidLayout
	}
	return nil
}

func oracleSpan(layout Layout, page int64) BlockSpan {
	return BlockSpan{
		FirstBlock:   layout.FirstBlock + page*layout.BlocksPerPage,
		BlockCount:   layout.BlocksPerPage,
		BytesPerPage: layout.BlocksPerPage * layout.Geometry.BytesPerBlock,
	}
}

func oracleSpanChecked(layout Layout, page int64) (BlockSpan, error) {
	if err := oracleValidate(layout); err != nil {
		return BlockSpan{}, err
	}
	if page < 0 || page >= layout.PageCount {
		return BlockSpan{}, ErrInvalidPage
	}
	return oracleSpan(layout, page), nil
}

func sameError(got, want error) bool {
	if got == nil || want == nil {
		return got == want
	}
	return errors.Is(got, want) && errors.Is(want, got)
}
