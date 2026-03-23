package filesystem

import "testing"

func TestBlockBitmapAllFreeInitially(t *testing.T) {
	bm := NewBlockBitmap(10)
	if bm.FreeCount() != 10 {
		t.Errorf("expected 10 free, got %d", bm.FreeCount())
	}
	for i := 0; i < 10; i++ {
		free, err := bm.IsFree(i)
		if err != nil {
			t.Fatal(err)
		}
		if !free {
			t.Errorf("block %d should be free", i)
		}
	}
}

func TestBlockBitmapAllocateSequential(t *testing.T) {
	bm := NewBlockBitmap(5)
	if bm.Allocate() != 0 {
		t.Error("first allocation should return 0")
	}
	if bm.Allocate() != 1 {
		t.Error("second allocation should return 1")
	}
	if bm.Allocate() != 2 {
		t.Error("third allocation should return 2")
	}
}

func TestBlockBitmapAllocateMarksUsed(t *testing.T) {
	bm := NewBlockBitmap(5)
	block := bm.Allocate()
	free, _ := bm.IsFree(block)
	if free {
		t.Error("allocated block should not be free")
	}
}

func TestBlockBitmapFree(t *testing.T) {
	bm := NewBlockBitmap(5)
	block := bm.Allocate()
	bm.Free(block)
	free, _ := bm.IsFree(block)
	if !free {
		t.Error("freed block should be free")
	}
}

func TestBlockBitmapFreeBlockReused(t *testing.T) {
	bm := NewBlockBitmap(5)
	bm.Allocate() // 0
	bm.Allocate() // 1
	bm.Free(0)
	if bm.Allocate() != 0 {
		t.Error("freed block 0 should be reused")
	}
}

func TestBlockBitmapExhaustion(t *testing.T) {
	bm := NewBlockBitmap(3)
	bm.Allocate()
	bm.Allocate()
	bm.Allocate()
	if bm.Allocate() != -1 {
		t.Error("should return -1 when full")
	}
	if bm.FreeCount() != 0 {
		t.Errorf("expected 0 free, got %d", bm.FreeCount())
	}
}

func TestBlockBitmapFreeCountChanges(t *testing.T) {
	bm := NewBlockBitmap(10)
	if bm.FreeCount() != 10 {
		t.Error("expected 10")
	}
	bm.Allocate()
	if bm.FreeCount() != 9 {
		t.Error("expected 9")
	}
	bm.Free(0)
	if bm.FreeCount() != 10 {
		t.Error("expected 10")
	}
}

func TestBlockBitmapOutOfRange(t *testing.T) {
	bm := NewBlockBitmap(5)
	err := bm.Free(5)
	if err == nil {
		t.Error("expected error for out-of-range free")
	}
	err = bm.Free(-1)
	if err == nil {
		t.Error("expected error for negative free")
	}
	_, err = bm.IsFree(5)
	if err == nil {
		t.Error("expected error for out-of-range IsFree")
	}
	_, err = bm.IsFree(-1)
	if err == nil {
		t.Error("expected error for negative IsFree")
	}
}

func TestBlockBitmapTotalBlockCount(t *testing.T) {
	bm := NewBlockBitmap(42)
	if bm.TotalBlockCount() != 42 {
		t.Errorf("expected 42, got %d", bm.TotalBlockCount())
	}
}
