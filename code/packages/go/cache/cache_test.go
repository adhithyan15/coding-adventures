package cache

import (
	"math"
	"strings"
	"testing"
)

// ── Helper Factories ────────────────────────────────────────────────────

func mustConfig(t *testing.T, name string, totalSize, lineSize, assoc, latency int, wp string) CacheConfig {
	t.Helper()
	cfg, err := NewCacheConfig(name, totalSize, lineSize, assoc, latency, wp)
	if err != nil {
		t.Fatalf("NewCacheConfig failed: %v", err)
	}
	return cfg
}

func makeL1D(t *testing.T) *Cache {
	t.Helper()
	return NewCache(mustConfig(t, "L1D", 256, 64, 2, 1, "write-back"))
}

func makeL2(t *testing.T) *Cache {
	t.Helper()
	return NewCache(mustConfig(t, "L2", 1024, 64, 4, 10, "write-back"))
}

func makeL3(t *testing.T) *Cache {
	t.Helper()
	return NewCache(mustConfig(t, "L3", 4096, 64, 8, 30, "write-back"))
}

// ── CacheStats Tests ────────────────────────────────────────────────────

func TestStatsInitialState(t *testing.T) {
	var s CacheStats
	if s.Reads != 0 || s.Writes != 0 || s.Hits != 0 || s.Misses != 0 {
		t.Fatal("expected all zeros")
	}
	if s.TotalAccesses() != 0 {
		t.Fatal("expected 0 total accesses")
	}
}

func TestStatsRecordReadHit(t *testing.T) {
	var s CacheStats
	s.RecordRead(true)
	if s.Reads != 1 || s.Hits != 1 || s.Misses != 0 {
		t.Fatalf("unexpected: reads=%d hits=%d misses=%d", s.Reads, s.Hits, s.Misses)
	}
}

func TestStatsRecordReadMiss(t *testing.T) {
	var s CacheStats
	s.RecordRead(false)
	if s.Reads != 1 || s.Hits != 0 || s.Misses != 1 {
		t.Fatal("unexpected counts")
	}
}

func TestStatsRecordWriteHit(t *testing.T) {
	var s CacheStats
	s.RecordWrite(true)
	if s.Writes != 1 || s.Hits != 1 {
		t.Fatal("unexpected counts")
	}
}

func TestStatsRecordWriteMiss(t *testing.T) {
	var s CacheStats
	s.RecordWrite(false)
	if s.Writes != 1 || s.Misses != 1 {
		t.Fatal("unexpected counts")
	}
}

func TestStatsEvictionClean(t *testing.T) {
	var s CacheStats
	s.RecordEviction(false)
	if s.Evictions != 1 || s.Writebacks != 0 {
		t.Fatal("unexpected")
	}
}

func TestStatsEvictionDirty(t *testing.T) {
	var s CacheStats
	s.RecordEviction(true)
	if s.Evictions != 1 || s.Writebacks != 1 {
		t.Fatal("unexpected")
	}
}

func TestStatsHitRateNoAccesses(t *testing.T) {
	var s CacheStats
	if s.HitRate() != 0.0 || s.MissRate() != 0.0 {
		t.Fatal("expected 0.0")
	}
}

func TestStatsHitRateAllHits(t *testing.T) {
	var s CacheStats
	for range 10 {
		s.RecordRead(true)
	}
	if s.HitRate() != 1.0 || s.MissRate() != 0.0 {
		t.Fatal("expected 100% hit rate")
	}
}

func TestStatsHitRateFiftyPercent(t *testing.T) {
	var s CacheStats
	s.RecordRead(true)
	s.RecordRead(false)
	if math.Abs(s.HitRate()-0.5) > 1e-10 {
		t.Fatalf("expected 0.5, got %f", s.HitRate())
	}
}

func TestStatsHitRatePlusMissRateEqualsOne(t *testing.T) {
	var s CacheStats
	s.RecordRead(true)
	s.RecordRead(true)
	s.RecordRead(false)
	if math.Abs(s.HitRate()+s.MissRate()-1.0) > 1e-10 {
		t.Fatal("expected hit_rate + miss_rate = 1.0")
	}
}

func TestStatsReset(t *testing.T) {
	var s CacheStats
	s.RecordRead(true)
	s.RecordWrite(false)
	s.RecordEviction(true)
	s.Reset()
	if s.Reads != 0 || s.Writes != 0 || s.Hits != 0 || s.Misses != 0 || s.Evictions != 0 || s.Writebacks != 0 {
		t.Fatal("expected all zeros after reset")
	}
}

func TestStatsString(t *testing.T) {
	var s CacheStats
	s.RecordRead(true)
	s.RecordRead(false)
	str := s.String()
	if !strings.Contains(str, "accesses=2") || !strings.Contains(str, "hits=1") {
		t.Fatalf("unexpected string: %s", str)
	}
}

// ── CacheLine Tests ─────────────────────────────────────────────────────

func TestCacheLineDefaultInvalid(t *testing.T) {
	cl := NewCacheLine(64)
	if cl.Valid || cl.Dirty || cl.Tag != 0 || cl.LastAccess != 0 {
		t.Fatal("expected default invalid state")
	}
	if cl.LineSize() != 64 {
		t.Fatal("expected line size 64")
	}
}

func TestCacheLineCustomSize(t *testing.T) {
	cl := NewCacheLine(32)
	if cl.LineSize() != 32 {
		t.Fatal("expected line size 32")
	}
}

func TestCacheLineDataZeros(t *testing.T) {
	cl := NewCacheLine(8)
	for _, b := range cl.Data {
		if b != 0 {
			t.Fatal("expected all zeros")
		}
	}
}

func TestCacheLineFill(t *testing.T) {
	cl := NewCacheLine(8)
	data := []int{1, 2, 3, 4, 5, 6, 7, 8}
	cl.Fill(42, data, 100)
	if !cl.Valid || cl.Dirty || cl.Tag != 42 || cl.LastAccess != 100 {
		t.Fatal("fill state incorrect")
	}
}

func TestCacheLineFillSetsData(t *testing.T) {
	cl := NewCacheLine(4)
	cl.Fill(7, []int{0xAA, 0xBB, 0xCC, 0xDD}, 0)
	if cl.Data[0] != 0xAA || cl.Data[3] != 0xDD {
		t.Fatal("data mismatch")
	}
}

func TestCacheLineFillClearsDirty(t *testing.T) {
	cl := NewCacheLine(4)
	cl.Dirty = true
	cl.Fill(1, []int{0, 0, 0, 0}, 0)
	if cl.Dirty {
		t.Fatal("fill should clear dirty")
	}
}

func TestCacheLineFillDefensiveCopy(t *testing.T) {
	cl := NewCacheLine(4)
	original := []int{1, 2, 3, 4}
	cl.Fill(1, original, 0)
	original[0] = 99
	if cl.Data[0] != 1 {
		t.Fatal("fill should make defensive copy")
	}
}

func TestCacheLineTouch(t *testing.T) {
	cl := NewCacheLine(64)
	cl.Fill(1, make([]int, 64), 10)
	cl.Touch(50)
	if cl.LastAccess != 50 {
		t.Fatalf("expected 50, got %d", cl.LastAccess)
	}
}

func TestCacheLineInvalidate(t *testing.T) {
	cl := NewCacheLine(4)
	cl.Fill(5, []int{1, 2, 3, 4}, 10)
	cl.Dirty = true
	cl.Invalidate()
	if cl.Valid || cl.Dirty {
		t.Fatal("invalidate should clear valid and dirty")
	}
}

func TestCacheLineInvalidateKeepsData(t *testing.T) {
	cl := NewCacheLine(4)
	cl.Fill(5, []int{0xAA, 0xBB, 0xCC, 0xDD}, 0)
	cl.Invalidate()
	if cl.Data[0] != 0xAA {
		t.Fatal("invalidate should not zero data")
	}
}

func TestCacheLineString(t *testing.T) {
	cl := NewCacheLine(4)
	if !strings.Contains(cl.String(), "--") {
		t.Fatal("invalid line should show --")
	}
	cl.Fill(0xFF, []int{0, 0, 0, 0}, 0)
	s := cl.String()
	if !strings.Contains(s, "V-") {
		t.Fatalf("valid clean line should show V-, got: %s", s)
	}
	cl.Dirty = true
	if !strings.Contains(cl.String(), "VD") {
		t.Fatal("valid dirty line should show VD")
	}
}

// ── CacheConfig Tests ───────────────────────────────────────────────────

func TestCacheConfigValid(t *testing.T) {
	cfg, err := NewCacheConfig("L1D", 65536, 64, 4, 1, "write-back")
	if err != nil {
		t.Fatal(err)
	}
	if cfg.NumSets() != 256 || cfg.NumLines() != 1024 {
		t.Fatal("unexpected num_sets or num_lines")
	}
}

func TestCacheConfigInvalidTotalSize(t *testing.T) {
	_, err := NewCacheConfig("bad", 0, 64, 4, 1, "write-back")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestCacheConfigInvalidLineSize(t *testing.T) {
	_, err := NewCacheConfig("bad", 256, 48, 1, 1, "write-back")
	if err == nil {
		t.Fatal("expected error for non-power-of-2 line size")
	}
}

func TestCacheConfigInvalidAssociativity(t *testing.T) {
	_, err := NewCacheConfig("bad", 256, 64, 0, 1, "write-back")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestCacheConfigInvalidAlignment(t *testing.T) {
	_, err := NewCacheConfig("bad", 100, 64, 4, 1, "write-back")
	if err == nil {
		t.Fatal("expected error for misaligned size")
	}
}

func TestCacheConfigInvalidWritePolicy(t *testing.T) {
	_, err := NewCacheConfig("bad", 256, 64, 1, 1, "write-around")
	if err == nil {
		t.Fatal("expected error for invalid write policy")
	}
}

func TestCacheConfigNegativeLatency(t *testing.T) {
	_, err := NewCacheConfig("bad", 256, 64, 1, -1, "write-back")
	if err == nil {
		t.Fatal("expected error for negative latency")
	}
}

func TestCacheConfigWriteThrough(t *testing.T) {
	cfg, err := NewCacheConfig("L1D", 256, 64, 1, 1, "write-through")
	if err != nil {
		t.Fatal(err)
	}
	if cfg.WritePolicy != "write-through" {
		t.Fatal("expected write-through")
	}
}

// ── CacheSet Tests ──────────────────────────────────────────────────────

func TestSetLookupMissOnEmpty(t *testing.T) {
	cs := NewCacheSet(4, 64)
	hit, way := cs.Lookup(42)
	if hit || way != -1 {
		t.Fatal("expected miss on empty set")
	}
}

func TestSetLookupHitAfterFill(t *testing.T) {
	cs := NewCacheSet(4, 8)
	cs.Lines[0].Fill(42, make([]int, 8), 0)
	hit, way := cs.Lookup(42)
	if !hit || way != 0 {
		t.Fatal("expected hit at way 0")
	}
}

func TestSetLookupMissWrongTag(t *testing.T) {
	cs := NewCacheSet(4, 8)
	cs.Lines[0].Fill(42, make([]int, 8), 0)
	hit, _ := cs.Lookup(99)
	if hit {
		t.Fatal("expected miss for wrong tag")
	}
}

func TestSetLookupFindsCorrectWay(t *testing.T) {
	cs := NewCacheSet(4, 8)
	cs.Lines[0].Fill(10, make([]int, 8), 0)
	cs.Lines[1].Fill(20, make([]int, 8), 0)
	cs.Lines[2].Fill(30, make([]int, 8), 0)
	hit, way := cs.Lookup(20)
	if !hit || way != 1 {
		t.Fatal("expected hit at way 1")
	}
}

func TestSetAccessHitUpdatesLRU(t *testing.T) {
	cs := NewCacheSet(2, 8)
	cs.Lines[0].Fill(10, make([]int, 8), 5)
	hit, line := cs.Access(10, 100)
	if !hit || line.LastAccess != 100 {
		t.Fatal("expected hit with updated LRU")
	}
}

func TestSetAccessMissReturnsLRUVictim(t *testing.T) {
	cs := NewCacheSet(2, 8)
	cs.Lines[0].Fill(10, make([]int, 8), 1)
	cs.Lines[1].Fill(20, make([]int, 8), 5)
	hit, victim := cs.Access(99, 10)
	if hit || victim.Tag != 10 {
		t.Fatal("expected miss returning LRU victim (tag=10)")
	}
}

func TestSetAllocateIntoEmptySlot(t *testing.T) {
	cs := NewCacheSet(4, 8)
	evicted := cs.Allocate(42, []int{0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA}, 10)
	if evicted != nil {
		t.Fatal("expected no eviction")
	}
	hit, _ := cs.Lookup(42)
	if !hit {
		t.Fatal("expected hit after allocate")
	}
}

func TestSetAllocateEvictsLRU(t *testing.T) {
	cs := NewCacheSet(2, 8)
	cs.Allocate(10, make([]int, 8), 1)
	cs.Allocate(20, make([]int, 8), 2)
	evicted := cs.Allocate(30, make([]int, 8), 3)
	if evicted != nil {
		t.Fatal("expected nil (clean eviction)")
	}
	hit10, _ := cs.Lookup(10)
	hit30, _ := cs.Lookup(30)
	if hit10 || !hit30 {
		t.Fatal("expected tag=10 evicted, tag=30 present")
	}
}

func TestSetAllocateReturnsDirtyEviction(t *testing.T) {
	cs := NewCacheSet(2, 8)
	cs.Allocate(10, []int{0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA}, 1)
	cs.Lines[0].Dirty = true
	cs.Allocate(20, make([]int, 8), 2)
	evicted := cs.Allocate(30, make([]int, 8), 3)
	if evicted == nil || !evicted.Dirty || evicted.Tag != 10 {
		t.Fatal("expected dirty eviction of tag=10")
	}
}

func TestSetFindLRUPrefersInvalid(t *testing.T) {
	cs := NewCacheSet(4, 8)
	cs.Lines[0].Fill(1, make([]int, 8), 100)
	lru := cs.FindLRU()
	if lru == 0 {
		t.Fatal("should prefer invalid line over valid one")
	}
}

func TestSetFindLRUPicksOldest(t *testing.T) {
	cs := NewCacheSet(4, 8)
	cs.Lines[0].Fill(1, make([]int, 8), 10)
	cs.Lines[1].Fill(2, make([]int, 8), 5) // oldest
	cs.Lines[2].Fill(3, make([]int, 8), 20)
	cs.Lines[3].Fill(4, make([]int, 8), 15)
	if cs.FindLRU() != 1 {
		t.Fatal("expected way 1 (cycle=5) as LRU")
	}
}

func TestDirectMappedConflict(t *testing.T) {
	cs := NewCacheSet(1, 8)
	cs.Allocate(10, make([]int, 8), 1)
	cs.Allocate(20, make([]int, 8), 2)
	hit10, _ := cs.Lookup(10)
	hit20, _ := cs.Lookup(20)
	if hit10 || !hit20 {
		t.Fatal("expected conflict miss")
	}
}

// ── Cache (Single Level) Tests ──────────────────────────────────────────

func TestAddressDecompositionZero(t *testing.T) {
	cfg := mustConfig(t, "test", 1024, 64, 4, 1, "write-back")
	c := NewCache(cfg)
	tag, setIdx, offset := c.DecomposeAddress(0)
	if tag != 0 || setIdx != 0 || offset != 0 {
		t.Fatal("expected all zeros for address 0")
	}
}

func TestAddressDecompositionOffset(t *testing.T) {
	cfg := mustConfig(t, "test", 1024, 64, 4, 1, "write-back")
	c := NewCache(cfg)
	_, setIdx, offset := c.DecomposeAddress(0x1F)
	if offset != 31 || setIdx != 0 {
		t.Fatal("offset extraction failed")
	}
}

func TestAddressDecompositionSetIndex(t *testing.T) {
	cfg := mustConfig(t, "test", 1024, 64, 4, 1, "write-back")
	c := NewCache(cfg)
	_, setIdx, offset := c.DecomposeAddress(0x40)
	if offset != 0 || setIdx != 1 {
		t.Fatal("set index extraction failed")
	}
	_, setIdx2, _ := c.DecomposeAddress(0x80)
	if setIdx2 != 2 {
		t.Fatal("expected set 2")
	}
	_, setIdx3, _ := c.DecomposeAddress(0xC0)
	if setIdx3 != 3 {
		t.Fatal("expected set 3")
	}
}

func TestAddressDecompositionTag(t *testing.T) {
	cfg := mustConfig(t, "test", 1024, 64, 4, 1, "write-back")
	c := NewCache(cfg)
	tag, setIdx, offset := c.DecomposeAddress(0x100)
	if offset != 0 || setIdx != 0 || tag != 1 {
		t.Fatal("tag extraction failed")
	}
}

func TestAddressDecompositionKnown(t *testing.T) {
	cfg := mustConfig(t, "test", 1024, 64, 4, 1, "write-back")
	c := NewCache(cfg)
	tag, setIdx, offset := c.DecomposeAddress(0x1A2B3C4D)
	if offset != 0x0D {
		t.Fatalf("expected offset 0x0D, got 0x%X", offset)
	}
	if setIdx != (0x1A2B3C4D>>6)&0x3 {
		t.Fatal("set index mismatch")
	}
	if tag != (0x1A2B3C4D >> 8) {
		t.Fatal("tag mismatch")
	}
}

func TestFirstReadIsMiss(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 3, "write-back")
	c := NewCache(cfg)
	access := c.Read(0x100, 0)
	if access.Hit || access.Cycles != 3 {
		t.Fatal("expected miss with 3 cycles")
	}
}

func TestSecondReadIsHit(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 3, "write-back")
	c := NewCache(cfg)
	c.Read(0x100, 0)
	access := c.Read(0x100, 1)
	if !access.Hit || access.Cycles != 3 {
		t.Fatal("expected hit with 3 cycles")
	}
}

func TestReadSameLineDifferentOffset(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 3, "write-back")
	c := NewCache(cfg)
	c.Read(0x100, 0)
	access := c.Read(0x110, 1)
	if !access.Hit {
		t.Fatal("expected hit for same cache line")
	}
}

func TestReadMissUpdatesStats(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 3, "write-back")
	c := NewCache(cfg)
	c.Read(0x100, 0)
	if c.Stats.Reads != 1 || c.Stats.Misses != 1 || c.Stats.Hits != 0 {
		t.Fatal("stats mismatch after miss")
	}
}

func TestReadHitUpdatesStats(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 3, "write-back")
	c := NewCache(cfg)
	c.Read(0x100, 0)
	c.Read(0x100, 1)
	if c.Stats.Reads != 2 || c.Stats.Hits != 1 || c.Stats.Misses != 1 {
		t.Fatal("stats mismatch after hit")
	}
}

func TestWriteMissAllocatesLine(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 1, "write-back")
	c := NewCache(cfg)
	access := c.Write(0x100, []int{0xAB}, 0)
	if access.Hit {
		t.Fatal("expected write miss")
	}
	readAccess := c.Read(0x100, 1)
	if !readAccess.Hit {
		t.Fatal("expected read hit after write-allocate")
	}
}

func TestWriteHitMarksDirtyInWriteBack(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 1, "write-back")
	c := NewCache(cfg)
	c.Read(0x100, 0)
	c.Write(0x100, []int{0xAB}, 1)
	tag, setIdx, _ := c.DecomposeAddress(0x100)
	hit, way := c.Sets[setIdx].Lookup(tag)
	if !hit || way == -1 || !c.Sets[setIdx].Lines[way].Dirty {
		t.Fatal("expected dirty line after write-back write")
	}
}

func TestWriteThroughDoesNotMarkDirty(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 1, "write-through")
	c := NewCache(cfg)
	c.Read(0x100, 0)
	c.Write(0x100, []int{0xAB}, 1)
	tag, setIdx, _ := c.DecomposeAddress(0x100)
	hit, way := c.Sets[setIdx].Lookup(tag)
	if !hit || way == -1 || c.Sets[setIdx].Lines[way].Dirty {
		t.Fatal("expected clean line after write-through write")
	}
}

func TestWriteStoresData(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 1, "write-back")
	c := NewCache(cfg)
	c.Write(0x100, []int{0xDE, 0xAD}, 0)
	tag, setIdx, offset := c.DecomposeAddress(0x100)
	_, way := c.Sets[setIdx].Lookup(tag)
	line := c.Sets[setIdx].Lines[way]
	if line.Data[offset] != 0xDE || line.Data[offset+1] != 0xAD {
		t.Fatal("data not stored correctly")
	}
}

func TestWriteUpdatesStats(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 1, "write-back")
	c := NewCache(cfg)
	c.Write(0x100, nil, 0) // miss
	c.Write(0x100, nil, 1) // hit
	if c.Stats.Writes != 2 || c.Stats.Misses != 1 || c.Stats.Hits != 1 {
		t.Fatal("stats mismatch")
	}
}

func TestDirtyEvictionReturnsEvictedLine(t *testing.T) {
	cfg := mustConfig(t, "test", 64, 64, 1, 1, "write-back")
	c := NewCache(cfg)
	c.Write(0, []int{0xFF}, 0)
	access := c.Read(64, 1)
	if access.Hit || access.Evicted == nil || !access.Evicted.Dirty {
		t.Fatal("expected dirty eviction")
	}
}

func TestEvictionStatsTracked(t *testing.T) {
	cfg := mustConfig(t, "test", 64, 64, 1, 1, "write-back")
	c := NewCache(cfg)
	c.Write(0, []int{0xFF}, 0)
	c.Read(64, 1)
	if c.Stats.Evictions < 1 || c.Stats.Writebacks < 1 {
		t.Fatal("expected evictions and writebacks")
	}
}

func TestCacheInvalidate(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 1, "write-back")
	c := NewCache(cfg)
	c.Read(0x100, 0)
	c.Read(0x100, 1)
	if c.Stats.Hits != 1 {
		t.Fatal("expected 1 hit before invalidate")
	}
	c.Invalidate()
	access := c.Read(0x100, 2)
	if access.Hit {
		t.Fatal("expected miss after invalidate")
	}
}

func TestSingleSetCache(t *testing.T) {
	cfg := mustConfig(t, "tiny", 128, 64, 2, 1, "write-back")
	c := NewCache(cfg)
	c.Read(0, 0)
	c.Read(64, 1)
	if !c.Read(0, 2).Hit || !c.Read(64, 3).Hit {
		t.Fatal("expected both addresses cached")
	}
}

func TestDirectMappedConflictEviction(t *testing.T) {
	cfg := mustConfig(t, "dm", 256, 64, 1, 1, "write-back")
	c := NewCache(cfg)
	c.Read(0x000, 0)
	c.Read(0x100, 1)
	c.Read(0x000, 2)
	c.Read(0x100, 3)
	if c.Stats.Hits != 0 || c.Stats.Misses != 4 {
		t.Fatal("expected 0 hits, 4 misses (thrashing)")
	}
}

func TestFillLineDirectly(t *testing.T) {
	cfg := mustConfig(t, "test", 256, 64, 2, 1, "write-back")
	c := NewCache(cfg)
	data := make([]int, 64)
	for i := range data {
		data[i] = 0xAB
	}
	c.FillLine(0x100, data, 0)
	if !c.Read(0x100, 1).Hit {
		t.Fatal("expected hit after fill_line")
	}
}

func TestCacheString(t *testing.T) {
	cfg := mustConfig(t, "L1D", 65536, 64, 4, 1, "write-back")
	c := NewCache(cfg)
	s := c.String()
	if !strings.Contains(s, "L1D") || !strings.Contains(s, "64KB") || !strings.Contains(s, "4-way") {
		t.Fatalf("unexpected string: %s", s)
	}
}

// ── Hierarchy Tests ─────────────────────────────────────────────────────

func TestHierarchyFirstReadGoesToMemory(t *testing.T) {
	h := NewCacheHierarchy(nil, makeL1D(t), makeL2(t), nil, 100)
	result := h.Read(0x1000, false, 0)
	if result.ServedBy != "memory" {
		t.Fatal("expected memory")
	}
	if result.TotalCycles != 1+10+100 {
		t.Fatalf("expected %d cycles, got %d", 1+10+100, result.TotalCycles)
	}
}

func TestHierarchySecondReadHitsL1(t *testing.T) {
	h := NewCacheHierarchy(nil, makeL1D(t), makeL2(t), nil, 100)
	h.Read(0x1000, false, 0)
	result := h.Read(0x1000, false, 1)
	if result.ServedBy != "L1D" || result.TotalCycles != 1 {
		t.Fatal("expected L1D hit with 1 cycle")
	}
}

func TestHierarchyL1MissL2Hit(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(nil, l1d, l2, nil, 100)
	l2.FillLine(0x1000, make([]int, 64), 0)
	result := h.Read(0x1000, false, 1)
	if result.ServedBy != "L2" || result.TotalCycles != 11 {
		t.Fatalf("expected L2 with 11 cycles, got %s with %d", result.ServedBy, result.TotalCycles)
	}
}

func TestHierarchyL1L2MissL3Hit(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	l3 := makeL3(t)
	h := NewCacheHierarchy(nil, l1d, l2, l3, 100)
	l3.FillLine(0x2000, make([]int, 64), 0)
	result := h.Read(0x2000, false, 1)
	if result.ServedBy != "L3" || result.TotalCycles != 41 {
		t.Fatalf("expected L3 with 41 cycles, got %s with %d", result.ServedBy, result.TotalCycles)
	}
}

func TestHierarchyAllMiss(t *testing.T) {
	h := NewCacheHierarchy(nil, makeL1D(t), makeL2(t), makeL3(t), 100)
	result := h.Read(0x3000, false, 0)
	if result.ServedBy != "memory" || result.TotalCycles != 141 {
		t.Fatalf("expected memory with 141 cycles, got %s with %d", result.ServedBy, result.TotalCycles)
	}
}

func TestHierarchyInclusiveFillAfterL2Hit(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(nil, l1d, l2, nil, 100)
	l2.FillLine(0x1000, make([]int, 64), 0)
	h.Read(0x1000, false, 1)
	result := h.Read(0x1000, false, 2)
	if result.ServedBy != "L1D" {
		t.Fatal("expected L1D after inclusive fill")
	}
}

func TestHierarchyInclusiveFillAfterMemory(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(nil, l1d, l2, nil, 100)
	h.Read(0x5000, false, 0)
	result := h.Read(0x5000, false, 1)
	if result.ServedBy != "L1D" {
		t.Fatal("expected L1D after inclusive fill from memory")
	}
}

func TestHarvardInstructionRead(t *testing.T) {
	l1iCfg := mustConfig(t, "L1I", 256, 64, 2, 1, "write-back")
	l1i := NewCache(l1iCfg)
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(l1i, l1d, l2, nil, 100)
	l1i.FillLine(0x1000, make([]int, 64), 0)
	result := h.Read(0x1000, true, 1)
	if result.ServedBy != "L1I" || result.TotalCycles != 1 {
		t.Fatal("expected L1I hit")
	}
}

func TestHarvardDataReadDoesNotUseL1I(t *testing.T) {
	l1iCfg := mustConfig(t, "L1I", 256, 64, 2, 1, "write-back")
	l1i := NewCache(l1iCfg)
	l1d := makeL1D(t)
	h := NewCacheHierarchy(l1i, l1d, nil, nil, 100)
	l1i.FillLine(0x1000, make([]int, 64), 0)
	result := h.Read(0x1000, false, 1)
	if result.ServedBy != "memory" {
		t.Fatal("data read should not use L1I")
	}
}

func TestHierarchyWriteHitL1(t *testing.T) {
	l1d := makeL1D(t)
	h := NewCacheHierarchy(nil, l1d, nil, nil, 100)
	h.Read(0x1000, false, 0)
	result := h.Write(0x1000, []int{0xAB}, 1)
	if result.ServedBy != "L1D" || result.TotalCycles != 1 {
		t.Fatal("expected L1D write hit")
	}
}

func TestHierarchyWriteMissGoesToMemory(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(nil, l1d, l2, nil, 100)
	result := h.Write(0x2000, []int{0xFF}, 0)
	if result.ServedBy != "memory" {
		t.Fatal("expected memory")
	}
}

func TestHierarchyWriteMissL2Hit(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(nil, l1d, l2, nil, 100)
	l2.FillLine(0x1000, make([]int, 64), 0)
	result := h.Write(0x1000, []int{0xAB}, 1)
	if result.ServedBy != "L2" {
		t.Fatalf("expected L2, got %s", result.ServedBy)
	}
}

func TestNoCacheReadGoesToMemory(t *testing.T) {
	h := NewCacheHierarchy(nil, nil, nil, nil, 200)
	result := h.Read(0x1000, false, 0)
	if result.ServedBy != "memory" || result.TotalCycles != 200 {
		t.Fatal("expected straight to memory")
	}
}

func TestNoCacheWriteGoesToMemory(t *testing.T) {
	h := NewCacheHierarchy(nil, nil, nil, nil, 200)
	result := h.Write(0x1000, []int{0xAB}, 0)
	if result.ServedBy != "memory" || result.TotalCycles != 200 {
		t.Fatal("expected straight to memory")
	}
}

func TestHierarchyInvalidateAll(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(nil, l1d, l2, nil, 100)
	h.Read(0x1000, false, 0)
	h.Read(0x1000, false, 1)
	h.InvalidateAll()
	result := h.Read(0x1000, false, 2)
	if result.ServedBy != "memory" {
		t.Fatal("expected memory after invalidate")
	}
}

func TestHierarchyResetStats(t *testing.T) {
	l1d := makeL1D(t)
	l2 := makeL2(t)
	h := NewCacheHierarchy(nil, l1d, l2, nil, 100)
	h.Read(0x1000, false, 0)
	h.ResetStats()
	if l1d.Stats.TotalAccesses() != 0 || l2.Stats.TotalAccesses() != 0 {
		t.Fatal("expected zero stats after reset")
	}
}

func TestHierarchyString(t *testing.T) {
	h := NewCacheHierarchy(nil, makeL1D(t), makeL2(t), makeL3(t), 100)
	s := h.String()
	if !strings.Contains(s, "L1D") || !strings.Contains(s, "L2") ||
		!strings.Contains(s, "L3") || !strings.Contains(s, "mem=100cyc") {
		t.Fatalf("unexpected string: %s", s)
	}
}

func TestHierarchyHitAtLevel(t *testing.T) {
	h := NewCacheHierarchy(nil, makeL1D(t), makeL2(t), nil, 100)
	result := h.Read(0x1000, false, 0)
	if result.HitAtLevel != 2 {
		t.Fatalf("expected hit_at_level=2 (memory), got %d", result.HitAtLevel)
	}
	result = h.Read(0x1000, false, 1)
	if result.HitAtLevel != 0 {
		t.Fatalf("expected hit_at_level=0 (L1D), got %d", result.HitAtLevel)
	}
}
