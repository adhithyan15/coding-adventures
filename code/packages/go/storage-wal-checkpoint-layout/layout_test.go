package storagewalcheckpointlayout

import (
	"errors"
	"fmt"
	"math"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewaldatalayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-data-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
)

func layouts(width int64, order [3]string) (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	logBlocksPerPage := (storagewalpageslot.MinPageBytes + width + width - 1) / width
	checkpointBlocksPerPage := int64(1 + (40-1)/width)
	lengths := map[string]int64{"log": 3 * logBlocksPerPage, "data": 1, "checkpoint": 2 * checkpointBlocksPerPage}
	geometry := blockdevicecontract.Geometry{BlockCount: lengths["log"] + lengths["data"] + lengths["checkpoint"], BytesPerBlock: width}
	starts := make(map[string]int64, len(order))
	cursor := int64(0)
	for _, name := range order {
		starts[name] = cursor
		cursor += lengths[name]
	}
	log := storagepagelayout.Layout{Geometry: geometry, FirstBlock: starts["log"], PageCount: 3, BlocksPerPage: logBlocksPerPage}
	data := storagepagelayout.Layout{Geometry: geometry, FirstBlock: starts["data"], PageCount: 1, BlocksPerPage: 1}
	checkpoint := storagepagelayout.Layout{Geometry: geometry, FirstBlock: starts["checkpoint"], PageCount: 2, BlocksPerPage: checkpointBlocksPerPage}
	return log, data, checkpoint
}

func TestHDDNANDAndLargeBlockWidths(t *testing.T) {
	for _, width := range []int64{1, 8, 53, 4096, 32768} {
		t.Run(fmt.Sprintf("bytes-%d", width), func(t *testing.T) {
			log, data, checkpoint := layouts(width, [3]string{"log", "data", "checkpoint"})
			if err := Validate(log, data, checkpoint); err != nil {
				t.Fatalf("valid pair refused: %v", err)
			}
			wantBlocks := int64(1 + (40-1)/width)
			if checkpoint.BlocksPerPage != wantBlocks {
				t.Fatalf("checkpoint blocks=%d want=%d", checkpoint.BlocksPerPage, wantBlocks)
			}
			if width == 53 && wantBlocks != 1 {
				t.Fatal("53-byte NAND block should use one block with padding")
			}
		})
	}
}

func TestAllPhysicalIntervalOrderingsAreAccepted(t *testing.T) {
	orders := [][3]string{
		{"log", "data", "checkpoint"}, {"log", "checkpoint", "data"},
		{"data", "log", "checkpoint"}, {"data", "checkpoint", "log"},
		{"checkpoint", "log", "data"}, {"checkpoint", "data", "log"},
	}
	for i, order := range orders {
		t.Run(fmt.Sprintf("ordering-%d", i), func(t *testing.T) {
			log, data, checkpoint := layouts(8, order)
			if err := Validate(log, data, checkpoint); err != nil {
				t.Fatalf("ordering %v rejected: %v", order, err)
			}
		})
	}
}

func TestCheckpointShapeAndOverlap(t *testing.T) {
	log, data, checkpoint := layouts(8, [3]string{"log", "data", "checkpoint"})
	wrongCount := checkpoint
	wrongCount.PageCount = 1
	wrongBlocks := checkpoint
	wrongBlocks.BlocksPerPage = 4
	for _, tc := range []struct {
		name   string
		layout storagepagelayout.Layout
		want   error
	}{
		{"one-checkpoint-page", wrongCount, ErrCheckpointShape},
		{"undersized-checkpoint-page", wrongBlocks, ErrCheckpointShape},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if err := Validate(log, data, tc.layout); !errors.Is(err, tc.want) {
				t.Fatalf("error=%v want=%v", err, tc.want)
			}
		})
	}
	shapeAndOverlap := checkpoint
	shapeAndOverlap.BlocksPerPage = 4
	shapeAndOverlap.FirstBlock = log.FirstBlock
	if err := Validate(log, data, shapeAndOverlap); !errors.Is(err, ErrCheckpointShape) {
		t.Fatalf("checkpoint shape must precede overlap: %v", err)
	}
	for _, tc := range []struct {
		name  string
		first int64
	}{
		{"overlap-log", log.FirstBlock},
		{"overlap-data", data.FirstBlock},
	} {
		t.Run(tc.name, func(t *testing.T) {
			bad := checkpoint
			bad.FirstBlock = tc.first
			if err := Validate(log, data, bad); !errors.Is(err, ErrCheckpointOverlap) {
				t.Fatalf("error=%v", err)
			}
		})
	}
}

func TestValidationErrorOrder(t *testing.T) {
	log, data, checkpoint := layouts(8, [3]string{"log", "data", "checkpoint"})
	badLog := log
	badLog.PageCount = 0
	badCheckpoint := checkpoint
	badCheckpoint.PageCount = 0
	if err := Validate(badLog, data, badCheckpoint); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("log error did not precede checkpoint: %v", err)
	}
	badData := data
	badData.PageCount = 0
	if err := Validate(log, badData, badCheckpoint); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("data error did not precede checkpoint: %v", err)
	}
	badCheckpoint = checkpoint
	badCheckpoint.PageCount = 0
	if err := Validate(log, data, badCheckpoint); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("checkpoint layout error lost: %v", err)
	}
	badCheckpoint.Geometry.BlockCount++
	if err := Validate(log, data, badCheckpoint); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("checkpoint layout refusal must precede its geometry mismatch: %v", err)
	}
	badLog = log
	badLog.Geometry.BlockCount = 0
	badData = data
	badData.PageCount = 0
	if err := Validate(badLog, badData, checkpoint); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("log/data validation order changed: %v", err)
	}
	mismatched := checkpoint
	mismatched.Geometry.BlockCount++
	if err := Validate(log, data, mismatched); !errors.Is(err, storagepageio.ErrGeometryMismatch) {
		t.Fatalf("geometry mismatch=%v", err)
	}
	shapeAndGeometry := mismatched
	shapeAndGeometry.PageCount = 1
	if err := Validate(log, data, shapeAndGeometry); !errors.Is(err, storagepageio.ErrGeometryMismatch) {
		t.Fatalf("geometry mismatch must precede checkpoint shape: %v", err)
	}
	badLogData := data
	badLogData.FirstBlock = log.FirstBlock
	if err := Validate(log, badLogData, checkpoint); !errors.Is(err, storagewaldatalayout.ErrLayoutOverlap) {
		t.Fatalf("existing log/data error lost: %v", err)
	}
}

func TestMaximumPortableAddressAndSignedBounds(t *testing.T) {
	const maxBlocks int64 = 9_007_199_254_740_991
	g := blockdevicecontract.Geometry{BlockCount: maxBlocks, BytesPerBlock: 8}
	log := storagepagelayout.Layout{Geometry: g, FirstBlock: maxBlocks - 32, PageCount: 3, BlocksPerPage: 7}
	data := storagepagelayout.Layout{Geometry: g, FirstBlock: maxBlocks - 11, PageCount: 1, BlocksPerPage: 1}
	checkpoint := storagepagelayout.Layout{Geometry: g, FirstBlock: maxBlocks - 10, PageCount: 2, BlocksPerPage: 5}
	if err := Validate(log, data, checkpoint); err != nil {
		t.Fatalf("maximum bounded layout refused: %v", err)
	}
	for _, first := range []int64{-1, math.MinInt64, math.MaxInt64} {
		bad := checkpoint
		bad.FirstBlock = first
		if err := Validate(log, data, bad); !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
			t.Fatalf("first block %d error=%v", first, err)
		}
	}
}

func TestValidateDoesNotAllocate(t *testing.T) {
	log, data, checkpoint := layouts(8, [3]string{"log", "data", "checkpoint"})
	allocations := testing.AllocsPerRun(1000, func() {
		if err := Validate(log, data, checkpoint); err != nil {
			panic(err)
		}
	})
	if allocations != 0 {
		t.Fatalf("Validate allocations=%v", allocations)
	}
}

func FuzzCheckpointLayoutDecisions(f *testing.F) {
	f.Add([]byte{0})
	f.Add([]byte{1, 2, 3, 4})
	f.Add([]byte{255, 255, 0, 53, 1, 0, 0, 0})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) == 0 || len(raw) > 32 {
			t.Skip()
		}
		log, data, checkpoint := layouts(8, [3]string{"log", "data", "checkpoint"})
		if len(raw) > 1 {
			checkpoint.FirstBlock += int64(raw[1] % 5)
		}
		if len(raw) > 2 {
			checkpoint.PageCount = int64(raw[2] % 4)
		}
		if len(raw) > 3 {
			checkpoint.BlocksPerPage = int64(raw[3] % 8)
		}
		if len(raw) > 4 && raw[4]&1 != 0 {
			checkpoint.Geometry.BytesPerBlock = int64(raw[4])
		}
		err := Validate(log, data, checkpoint)
		if err == nil {
			if checkpoint.PageCount != 2 || checkpoint.BlocksPerPage != int64(1+(40-1)/checkpoint.Geometry.BytesPerBlock) {
				t.Fatal("accepted wrong checkpoint shape")
			}
			if overlaps(checkpoint, log) || overlaps(checkpoint, data) {
				t.Fatal("accepted overlapping checkpoint")
			}
		}
	})
}
