package storagewaldatalayout

import (
	"errors"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalreservation "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation"
)

func profile(bytesPerBlock, blocks int64) blockdevicecontract.Geometry {
	return blockdevicecontract.Geometry{BlockCount: blocks, BytesPerBlock: bytesPerBlock}
}

func layout(g blockdevicecontract.Geometry, first, pages, blocksPerPage int64) storagepagelayout.Layout {
	return storagepagelayout.Layout{Geometry: g, FirstBlock: first, PageCount: pages, BlocksPerPage: blocksPerPage}
}

func TestValidateExamplesAndBoundaries(t *testing.T) {
	// 32-sector HDD: four 56-byte log slots and four 8-byte data pages.
	hdd := profile(8, 32)
	if err := Validate(layout(hdd, 0, 4, 7), layout(hdd, 28, 4, 1)); err != nil {
		t.Fatalf("HDD layout: %v", err)
	}
	// 53-byte NAND blocks need two blocks per slot to fit one complete data block.
	nand := profile(53, 10)
	if err := Validate(layout(nand, 0, 3, 2), layout(nand, 6, 1, 1)); err != nil {
		t.Fatalf("NAND layout: %v", err)
	}
	if err := Validate(layout(nand, 0, 3, 1), layout(nand, 3, 1, 1)); !errors.Is(err, ErrImageDoesNotFit) {
		t.Fatalf("single-block NAND slot=%v", err)
	}
	max := profile(1, 262_097)
	if err := Validate(layout(max, 0, 3, 65_536), layout(max, 196_608, 1, 65_488)); err != nil {
		t.Fatalf("largest fitting image: %v", err)
	}
	if err := Validate(layout(max, 0, 3, 65_536), layout(max, 196_608, 1, 65_489)); !errors.Is(err, ErrImageDoesNotFit) {
		t.Fatalf("largest plus one=%v", err)
	}
	capacityGeometry := profile(48, 2*storagewalreservation.MaxLogPages+1)
	if err := Validate(layout(capacityGeometry, 0, storagewalreservation.MaxLogPages, 2), layout(capacityGeometry, 2*storagewalreservation.MaxLogPages, 1, 1)); err != nil {
		t.Fatalf("maximum log slots: %v", err)
	}
}

func TestValidatePreservesOrderedRefusals(t *testing.T) {
	g := profile(8, 64)
	goodLog := layout(g, 0, 4, 7)
	goodData := layout(g, 28, 4, 1)
	badLog := goodLog
	badLog.PageCount = 0
	badData := goodData
	badData.FirstBlock = -1
	if err := Validate(badLog, badData); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("log before data: %v", err)
	}
	if err := Validate(goodLog, badData); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("data layout: %v", err)
	}
	otherGeometry := profile(9, 64)
	if err := Validate(goodLog, layout(otherGeometry, 28, 4, 1)); !errors.Is(err, storagepageio.ErrGeometryMismatch) {
		t.Fatalf("geometry mismatch: %v", err)
	}
	if err := Validate(layout(g, 0, 2, 7), layout(g, 14, 1, 1)); !errors.Is(err, ErrLogTooSmall) {
		t.Fatalf("minimum log slots: %v", err)
	}
	capacityGeometry := profile(48, storagewalreservation.MaxLogPages+2)
	if err := Validate(layout(capacityGeometry, 0, storagewalreservation.MaxLogPages+1, 1), layout(capacityGeometry, storagewalreservation.MaxLogPages+1, 1, 1)); !errors.Is(err, storagewalreservation.ErrLogCapacity) {
		t.Fatalf("capacity before other size/overlap checks: %v", err)
	}
	// Two half-open ranges touching at block 28 are accepted; sharing one block is not.
	if err := Validate(goodLog, layout(g, 27, 4, 1)); !errors.Is(err, ErrLayoutOverlap) {
		t.Fatalf("overlap: %v", err)
	}
	slotGeometry := profile(1, 142)
	if err := Validate(layout(slotGeometry, 0, 3, 47), layout(slotGeometry, 141, 1, 1)); !errors.Is(err, storagewalpageslot.ErrPageSize) {
		t.Fatalf("invalid slot width: %v", err)
	}
	if storagewalreservation.ErrLogCapacity.Error() == "" {
		t.Fatal("shared capacity refusal is empty")
	}
}

func TestValidateMatchesFixedSlotEncoderBoundary(t *testing.T) {
	for _, dataBytes := range []int64{1, 47, 48, 49, 79, 80, 81, 65_487, 65_488, 65_489} {
		logBytes := int64(128)
		if dataBytes > 80 {
			logBytes = 65_536
		}
		g := profile(1, 3*logBytes+dataBytes)
		logLayout := layout(g, 0, 3, logBytes)
		dataLayout := layout(g, 3*logBytes, 1, dataBytes)
		got := Validate(logLayout, dataLayout)
		record := storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: 1, TxID: 1, PageID: 0, Image: make([]byte, int(dataBytes))}
		_, encodeErr := storagewalpageslot.EncodePage(logBytes, record)
		if (got == nil) != (encodeErr == nil) {
			t.Fatalf("data=%d log=%d Validate=%v encoder=%v", dataBytes, logBytes, got, encodeErr)
		}
		if got != nil && !errors.Is(got, ErrImageDoesNotFit) {
			t.Fatalf("data=%d unexpected validation error %v", dataBytes, got)
		}
	}
}

func TestValidateDenseIntervalModel(t *testing.T) {
	comparisons := 0
	accepted := 0
	for _, blockBytes := range []int64{8, 16, 24, 53} {
		for _, blockCount := range []int64{8, 16, 24, 32} {
			g := profile(blockBytes, blockCount)
			var layouts []storagepagelayout.Layout
			for first := int64(0); first < blockCount; first++ {
				for pages := int64(1); pages <= blockCount; pages++ {
					for blocksPerPage := int64(1); blocksPerPage <= blockCount; blocksPerPage++ {
						candidate := layout(g, first, pages, blocksPerPage)
						if candidate.Validate() == nil {
							layouts = append(layouts, candidate)
						}
					}
				}
			}
			for _, log := range layouts {
				for _, data := range layouts {
					comparisons++
					want := modelError(log, data)
					got := Validate(log, data)
					if !sameError(got, want) {
						t.Fatalf("g=%+v log=%+v data=%+v: got %v want %v", g, log, data, got, want)
					}
					if got == nil {
						accepted++
					}
				}
			}
		}
	}
	t.Logf("model decisions=%d accepted=%d", comparisons, accepted)
}

func modelError(log, data storagepagelayout.Layout) error {
	if log.Geometry != data.Geometry {
		return storagepageio.ErrGeometryMismatch
	}
	if log.PageCount > storagewalreservation.MaxLogPages {
		return storagewalreservation.ErrLogCapacity
	}
	if log.PageCount < 3 {
		return ErrLogTooSmall
	}
	logEnd := log.FirstBlock + log.PageCount*log.BlocksPerPage
	dataEnd := data.FirstBlock + data.PageCount*data.BlocksPerPage
	if log.FirstBlock < dataEnd && data.FirstBlock < logEnd {
		return ErrLayoutOverlap
	}
	logBytes := log.BlocksPerPage * log.Geometry.BytesPerBlock
	if logBytes < storagewalpageslot.MinPageBytes || logBytes > storagewalpageslot.MaxPageBytes {
		return storagewalpageslot.ErrPageSize
	}
	dataBytes := data.BlocksPerPage * data.Geometry.BytesPerBlock
	if dataBytes+storagewalpageslot.MinPageBytes > logBytes {
		return ErrImageDoesNotFit
	}
	return nil
}

func sameError(got, want error) bool {
	if got == nil || want == nil {
		return got == want
	}
	return errors.Is(got, want)
}

func TestValidatePortableSignedEndpoint(t *testing.T) {
	const maximumSafeInteger int64 = 9_007_199_254_740_991
	g := profile(48, maximumSafeInteger)
	log := layout(g, 0, 3, 2)
	data := layout(g, maximumSafeInteger-1, 1, 1)
	if err := Validate(log, data); err != nil {
		t.Fatalf("portable endpoint rejected: %v", err)
	}
	data.FirstBlock = maximumSafeInteger
	if err := Validate(log, data); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("out-of-range signed endpoint=%v", err)
	}
}

func TestValidateDoesNotAllocate(t *testing.T) {
	g := profile(8, 32)
	log, data := layout(g, 0, 4, 7), layout(g, 28, 4, 1)
	const maximumSafeInteger int64 = 9_007_199_254_740_991
	large := profile(1, maximumSafeInteger)
	largeLog := layout(large, 0, 3, 65_536)
	largeData := layout(large, maximumSafeInteger-65_488, 1, 65_488)
	if err := Validate(largeLog, largeData); err != nil {
		t.Fatalf("largest portable extent: %v", err)
	}
	for _, pair := range [][2]storagepagelayout.Layout{{log, data}, {largeLog, largeData}} {
		if allocations := testing.AllocsPerRun(1000, func() { _ = Validate(pair[0], pair[1]) }); allocations != 0 {
			t.Fatalf("Validate allocated %v times", allocations)
		}
	}
}

func FuzzValidateLayouts(f *testing.F) {
	for _, seed := range [][8]int64{{8, 32, 0, 4, 7, 28, 4, 1}, {53, 10, 0, 3, 2, 6, 1, 1}, {1, 3, 0, 3, 48, 3, 1, 48}, {65_536, 4, 0, 3, 1, 3, 1, 65_488}} {
		f.Add(seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], seed[6], seed[7])
	}
	f.Fuzz(func(t *testing.T, bytesPerBlock, blockCount, logFirst, logPages, logBlocks, dataFirst, dataPages, dataBlocks int64) {
		if bytesPerBlock < 1 || bytesPerBlock > 65_536 || blockCount < 1 || blockCount > 100_000 || logFirst < 0 || logFirst > blockCount || logPages < 1 || logPages > 1025 || logBlocks < 1 || logBlocks > blockCount || dataFirst < 0 || dataFirst > blockCount || dataPages < 1 || dataPages > blockCount || dataBlocks < 1 || dataBlocks > blockCount {
			t.Skip()
		}
		g := profile(bytesPerBlock, blockCount)
		log := layout(g, logFirst, logPages, logBlocks)
		data := layout(g, dataFirst, dataPages, dataBlocks)
		got := Validate(log, data)
		// Independent arithmetic model for the admitted small geometry.
		if log.Validate() != nil || data.Validate() != nil {
			if got == nil {
				t.Fatalf("accepted invalid layout: log=%+v data=%+v", log, data)
			}
			return
		}
		endLog, endData := logFirst+logPages*logBlocks, dataFirst+dataPages*dataBlocks
		overlap := logFirst < endData && dataFirst < endLog
		wantOK := log.Geometry == data.Geometry && !overlap && logPages >= 3 && logPages <= storagewalreservation.MaxLogPages && logBlocks*bytesPerBlock >= storagewalpageslot.MinPageBytes && logBlocks*bytesPerBlock <= storagewalpageslot.MaxPageBytes && dataBlocks*bytesPerBlock <= logBlocks*bytesPerBlock-storagewalpageslot.MinPageBytes
		if (got == nil) != wantOK {
			t.Fatalf("got %v wantOK=%v", got, wantOK)
		}
	})
}
