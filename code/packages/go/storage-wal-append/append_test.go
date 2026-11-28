package storagewalappend

import (
	"errors"
	"fmt"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

var testGeometry = blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
var testLayout = storagepagelayout.Layout{Geometry: testGeometry, FirstBlock: 2, PageCount: 4, BlocksPerPage: 7}

type scriptedDevice struct {
	geometry  blockdevicecontract.Geometry
	blocks    map[int64][]byte
	reads     []int64
	writes    []int64
	flushes   int
	readErr   map[int64]error
	readWidth map[int64]int
	writeErr  map[int64]error
	flushErr  error
}

func newScripted() *scriptedDevice {
	return &scriptedDevice{geometry: testGeometry, blocks: make(map[int64][]byte), readErr: make(map[int64]error), readWidth: make(map[int64]int), writeErr: make(map[int64]error)}
}

func (device *scriptedDevice) Geometry() blockdevicecontract.Geometry { return device.geometry }

func (device *scriptedDevice) ReadBlock(block int64) ([]byte, error) {
	device.reads = append(device.reads, block)
	if err := device.readErr[block]; err != nil {
		return nil, err
	}
	if data, ok := device.blocks[block]; ok {
		if width, ok := device.readWidth[block]; ok {
			return make([]byte, width), nil
		}
		return append([]byte(nil), data...), nil
	}
	if width, ok := device.readWidth[block]; ok {
		return make([]byte, width), nil
	}
	return make([]byte, int(device.geometry.BytesPerBlock)), nil
}

func (device *scriptedDevice) WriteBlock(block int64, data []byte) error {
	device.writes = append(device.writes, block)
	if err := device.writeErr[block]; err != nil {
		return err
	}
	device.blocks[block] = append([]byte(nil), data...)
	return nil
}

func (device *scriptedDevice) Flush() error {
	device.flushes++
	return device.flushErr
}

func TestAppendWritesNextVacantPageAndAcknowledges(t *testing.T) {
	device := newScripted()
	seedPage(t, device, 0, validRecord(1, 10))
	receipt, err := Append(device, testLayout, validRecord(2, 11))
	if err != nil || receipt != (Receipt{Attempted: true, Page: 1, WrittenBlocks: 7, Acknowledged: true}) {
		t.Fatalf("Append=(%+v,%v)", receipt, err)
	}
	if len(device.reads) != 4*7 || device.flushes != 1 {
		t.Fatalf("reads=%d flushes=%d", len(device.reads), device.flushes)
	}
	for index, block := range device.reads {
		if block != 2+int64(index) {
			t.Fatalf("read[%d]=%d", index, block)
		}
	}
	if !equalBlocks(device.writes, sequence(9, 7)) {
		t.Fatalf("writes=%v want blocks 9..15", device.writes)
	}
	page := readPage(t, device, 1)
	got, present, err := storagewalpageslot.DecodePage(page)
	if err != nil || !present || !sameRecord(got, validRecord(2, 11)) {
		t.Fatalf("appended page=(%+v,%v,%v)", got, present, err)
	}
}

func TestAppendPreflightRefusalsAreZeroAndDoNotWrite(t *testing.T) {
	t.Run("open owns invalid layout before nil device", func(t *testing.T) {
		invalid := storagepagelayout.Layout{Geometry: blockdevicecontract.Geometry{}, FirstBlock: -1}
		got, err := Append(nil, invalid, validRecord(1, 1))
		if got != (Receipt{}) || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
			t.Fatalf("Append=(%+v,%v)", got, err)
		}
	})
	t.Run("nil device after valid layout", func(t *testing.T) {
		got, err := Append(nil, testLayout, validRecord(1, 1))
		if got != (Receipt{}) || !errors.Is(err, storagepageio.ErrInvalidDevice) {
			t.Fatalf("Append=(%+v,%v)", got, err)
		}
	})
	t.Run("capacity before reading", func(t *testing.T) {
		// A large sparse device reaches the capacity gate without allocating
		// the reserved media.
		geometry := blockdevicecontract.Geometry{BlockCount: MaxLogPages + 1, BytesPerBlock: 48}
		device := &scriptedDevice{geometry: geometry, blocks: make(map[int64][]byte), readErr: make(map[int64]error), readWidth: make(map[int64]int), writeErr: make(map[int64]error)}
		layout := storagepagelayout.Layout{Geometry: geometry, FirstBlock: 0, PageCount: MaxLogPages + 1, BlocksPerPage: 1}
		got, err := Append(device, layout, validRecord(1, 1))
		if got != (Receipt{}) || !errors.Is(err, ErrLogCapacity) || len(device.reads) != 0 {
			t.Fatalf("Append=(%+v,%v)", got, err)
		}
	})
	t.Run("slot width after open", func(t *testing.T) {
		geometry := blockdevicecontract.Geometry{BlockCount: 64, BytesPerBlock: 1}
		device := &scriptedDevice{geometry: geometry, blocks: make(map[int64][]byte), readErr: make(map[int64]error), writeErr: make(map[int64]error)}
		layout := storagepagelayout.Layout{Geometry: geometry, FirstBlock: 0, PageCount: 1, BlocksPerPage: 32}
		got, err := Append(device, layout, validRecord(1, 1))
		if got != (Receipt{}) || !errors.Is(err, storagewalpageslot.ErrPageSize) || len(device.reads) != 0 {
			t.Fatalf("Append=(%+v,%v), reads=%v", got, err, device.reads)
		}
	})
	t.Run("full", func(t *testing.T) {
		device := newScripted()
		for page := int64(0); page < testLayout.PageCount; page++ {
			seedPage(t, device, page, validRecord(uint64(page+1), uint64(page+1)))
		}
		assertPreflight(t, device, validRecord(5, 9), ErrLogFull)
	})
	t.Run("damaged tail reports page and decoder cause", func(t *testing.T) {
		device := newScripted()
		seedPage(t, device, 0, validRecord(1, 1))
		bad := make([]byte, 56)
		bad[0] = 1 // impossible short slot frame
		seedRawPage(t, device, 1, bad)
		var damaged *DamagedTailError
		got, err := Append(device, testLayout, validRecord(2, 2))
		if got != (Receipt{}) || !errors.As(err, &damaged) || damaged.Page != 1 || !errors.Is(err, ErrDamagedTail) || !errors.Is(err, storagewalpageslot.ErrSlotFrameLength) {
			t.Fatalf("Append=(%+v,%v), damaged=%+v", got, err, damaged)
		}
		assertNoWrites(t, device)
	})
	t.Run("hard scanner refusal after vacancy", func(t *testing.T) {
		device := newScripted()
		seedPage(t, device, 0, validRecord(1, 1))
		seedPage(t, device, 2, validRecord(2, 2))
		assertPreflight(t, device, validRecord(2, 3), storagewalscan.ErrPageAfterTail)
	})
	t.Run("sequence", func(t *testing.T) {
		device := newScripted()
		seedPage(t, device, 0, validRecord(1, 1))
		assertPreflight(t, device, validRecord(3, 2), ErrAppendSequence)
	})
	t.Run("record refusal", func(t *testing.T) {
		device := newScripted()
		bad := validRecord(1, 1)
		bad.Kind = 99
		assertPreflight(t, device, bad, storagewalrecord.ErrKind)
	})
}

func TestAppendReadAndWriteFailuresPreserveReceiptContract(t *testing.T) {
	for failed := int64(0); failed < testLayout.PageCount*testLayout.BlocksPerPage; failed++ {
		t.Run(fmt.Sprintf("read-offset-%d", failed), func(t *testing.T) {
			device := newScripted()
			failure := errors.New("read-fault")
			device.readErr[2+failed] = failure
			got, err := Append(device, testLayout, validRecord(1, 1))
			if got != (Receipt{}) || !errors.Is(err, failure) {
				t.Fatalf("Append=(%+v,%v)", got, err)
			}
			if len(device.reads) != int(failed+1) {
				t.Fatalf("reads stopped at %d, want %d", len(device.reads), failed+1)
			}
			assertNoWrites(t, device)
		})
	}
	t.Run("read width mismatch", func(t *testing.T) {
		device := newScripted()
		device.readWidth[4] = 7
		got, err := Append(device, testLayout, validRecord(1, 1))
		if got != (Receipt{}) || !errors.Is(err, storagepageio.ErrBlockWidthMismatch) || len(device.reads) != 3 {
			t.Fatalf("Append=(%+v,%v), reads=%v", got, err, device.reads)
		}
		assertNoWrites(t, device)
	})
	for failed := int64(0); failed < 7; failed++ {
		t.Run(fmt.Sprintf("write-offset-%d", failed), func(t *testing.T) {
			device := newScripted()
			failure := errors.New("write-fault")
			device.writeErr[2+failed] = failure
			got, err := Append(device, testLayout, validRecord(1, 1))
			want := Receipt{Attempted: true, Page: 0, WrittenBlocks: failed}
			if got != want || !errors.Is(err, failure) {
				t.Fatalf("Append=(%+v,%v) want=%+v", got, err, want)
			}
			if device.flushes != 0 {
				t.Fatalf("flush after incomplete page: %d", device.flushes)
			}
		})
	}
	t.Run("flush error after full write", func(t *testing.T) {
		device := newScripted()
		failure := errors.New("flush-fault")
		device.flushErr = failure
		got, err := Append(device, testLayout, validRecord(1, 1))
		want := Receipt{Attempted: true, Page: 0, WrittenBlocks: 7}
		if got != want || !errors.Is(err, failure) {
			t.Fatalf("Append=(%+v,%v) want=%+v", got, err, want)
		}
		if len(device.writes) != 7 || device.flushes != 1 {
			t.Fatalf("writes=%d flushes=%d", len(device.writes), device.flushes)
		}
	})
}

func TestAppendOnMemoryAndHDDAndEveryHDDBarrierCut(t *testing.T) {
	memory, err := memoryblockdevice.New(testGeometry)
	if err != nil {
		t.Fatal(err)
	}
	if receipt, err := Append(memory, testLayout, validRecord(1, 3)); err != nil || receipt.Acknowledged != true {
		t.Fatalf("memory append=(%+v,%v)", receipt, err)
	}
	memoryRestart, err := memory.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertScannedRecord(t, memoryRestart, 1)

	plainHDD := hddblockdevice.New()
	if receipt, err := Append(plainHDD, testLayout, validRecord(1, 4)); err != nil || receipt.Acknowledged != true {
		t.Fatalf("HDD append=(%+v,%v)", receipt, err)
	}
	hddRestart, err := plainHDD.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertScannedRecord(t, hddRestart, 1)

	for _, cut := range allBarrierCuts() {
		t.Run(fmt.Sprintf("%v-%d-%s", cut.Kind, cut.SectorIndex, cut.MediaCut.Kind), func(t *testing.T) {
			lower := hddblockdevice.New()
			first, firstErr := Append(lower, testLayout, validRecord(1, 5))
			if firstErr != nil || !first.Acknowledged {
				t.Fatalf("seed append=(%+v,%v)", first, firstErr)
			}
			device := &cutHDD{HDD: lower, cut: cut}
			receipt, appendErr := Append(device, testLayout, validRecord(2, 6))
			restarted, restartErr := lower.Restart()
			if restartErr != nil {
				t.Fatal(restartErr)
			}
			if cut.Kind == hddwritebackcache.BarrierComplete {
				if appendErr != nil || !receipt.Acknowledged {
					t.Fatalf("complete=(%+v,%v)", receipt, appendErr)
				}
				assertScannedRecord(t, restarted, 2)
				return
			}
			if !errors.Is(appendErr, errLostAck) || receipt.Acknowledged || receipt.WrittenBlocks != 7 {
				t.Fatalf("lost response=(%+v,%v)", receipt, appendErr)
			}
			wantRecords := 1
			if persistedPageBytes(cut) >= 48 { // the final eight page bytes are zero padding
				wantRecords = 2
			}
			assertScannedRecord(t, restarted, wantRecords)
		})
	}
}

func TestBoundedSeededAppendHistory(t *testing.T) {
	for seed := uint32(1); seed <= 64; seed++ {
		device, err := memoryblockdevice.New(testGeometry)
		if err != nil {
			t.Fatal(err)
		}
		nextLSN := uint64(1)
		state := seed
		for step := 0; step < 4; step++ {
			state = state*1664525 + 1013904223
			if state&3 == 0 { // Rejected sequence must leave the device untouched.
				got, err := Append(device, testLayout, validRecord(nextLSN+1, uint64(seed)))
				if got != (Receipt{}) || !errors.Is(err, ErrAppendSequence) {
					t.Fatalf("seed=%d step=%d refusal=(%+v,%v)", seed, step, got, err)
				}
				continue
			}
			receipt, err := Append(device, testLayout, validRecord(nextLSN, uint64(seed)))
			if err != nil || !receipt.Acknowledged {
				t.Fatalf("seed=%d step=%d append=(%+v,%v)", seed, step, receipt, err)
			}
			nextLSN++
		}
		assertScannedRecord(t, device, int(nextLSN-1))
	}
}

func TestDamagedTailErrorFormattingAndUnwrap(t *testing.T) {
	err := &DamagedTailError{Page: 3, Cause: storagewalpageslot.ErrPadding}
	if got, want := err.Error(), "WAL damaged tail at page 3: "+storagewalpageslot.ErrPadding.Error(); got != want {
		t.Fatalf("Error()=%q want=%q", got, want)
	}
	if !errors.Is(err, ErrDamagedTail) || !errors.Is(err, storagewalpageslot.ErrPadding) {
		t.Fatalf("Unwrap lost identities: %v", err)
	}
	if !errors.Is(&DamagedTailError{Page: 0}, ErrDamagedTail) {
		t.Fatal("nil cause unwrap lost ErrDamagedTail")
	}
}

func FuzzAppendPreflightAndReceipt(f *testing.F) {
	f.Add([]byte{1, 0, 1, 7, 0, 0, 0, 0})
	f.Add([]byte{0xff, 1, 0, 0, 3, 9, 8, 7, 6})
	f.Add([]byte{2, 0, 0, 0, 0})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 256 {
			t.Skip()
		}
		device, err := memoryblockdevice.New(testGeometry)
		if err != nil {
			t.Fatal(err)
		}
		lsn := uint64(1)
		if len(input) > 0 && input[0]&1 != 0 {
			lsn = 2
		}
		record := validRecord(lsn, uint64(len(input)+1))
		receipt, err := Append(device, testLayout, record)
		if lsn == 2 {
			if receipt != (Receipt{}) || !errors.Is(err, ErrAppendSequence) {
				t.Fatalf("bad sequence=(%+v,%v)", receipt, err)
			}
			return
		}
		if err != nil || receipt != (Receipt{Attempted: true, Page: 0, WrittenBlocks: 7, Acknowledged: true}) {
			t.Fatalf("append=(%+v,%v)", receipt, err)
		}
		assertScannedRecord(t, device, 1)
	})
}

type cutHDD struct {
	*hddblockdevice.HDD
	cut hddwritebackcache.BarrierCut
}

func (device *cutHDD) Flush() error {
	receipt, err := device.HDD.FlushWithCut(device.cut)
	if err != nil {
		return err
	}
	if !receipt.Acknowledged {
		return errLostAck
	}
	return nil
}

var errLostAck = errors.New("lost-barrier-response")

func allBarrierCuts() []hddwritebackcache.BarrierCut {
	cuts := []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BeforeFirstSector}, {Kind: hddwritebackcache.AfterAllMedia}, {Kind: hddwritebackcache.BarrierComplete}}
	for ordinal := int64(0); ordinal < 7; ordinal++ {
		cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}})
		for prefix := 1; prefix <= 7; prefix++ {
			value := prefix
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value}})
		}
		cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})
	}
	return cuts
}

func persistedPageBytes(cut hddwritebackcache.BarrierCut) int {
	switch cut.Kind {
	case hddwritebackcache.AfterAllMedia, hddwritebackcache.BarrierComplete:
		return 56
	case hddwritebackcache.BeforeFirstSector:
		return 0
	case hddwritebackcache.AtSector:
		prefix := 0
		switch cut.MediaCut.Kind {
		case hddwritecrashcuts.DuringMediaWrite:
			prefix = *cut.MediaCut.WrittenPrefix
		case hddwritecrashcuts.AfterMediaWriteBeforeResponse, hddwritecrashcuts.Acknowledged:
			prefix = 8
		}
		return int(cut.SectorIndex)*8 + prefix
	default:
		return 0
	}
}

func validRecord(lsn, tx uint64) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: lsn, TxID: tx}
}
func assertPreflight(t *testing.T, device *scriptedDevice, record storagewalrecord.Record, target error) {
	t.Helper()
	beforeReads := len(device.reads)
	got, err := Append(device, testLayout, record)
	if got != (Receipt{}) || !errors.Is(err, target) {
		t.Fatalf("Append=(%+v,%v), want zero/%v", got, err, target)
	}
	if len(device.reads) < beforeReads {
		t.Fatal("read count regressed")
	}
	assertNoWrites(t, device)
}
func assertNoWrites(t *testing.T, device *scriptedDevice) {
	t.Helper()
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatalf("preflight wrote blocks=%v flushes=%d", device.writes, device.flushes)
	}
}
func seedPage(t *testing.T, device *scriptedDevice, page int64, record storagewalrecord.Record) {
	t.Helper()
	encoded, err := storagewalpageslot.EncodePage(56, record)
	if err != nil {
		t.Fatal(err)
	}
	seedRawPage(t, device, page, encoded)
}
func seedRawPage(t *testing.T, device *scriptedDevice, page int64, data []byte) {
	t.Helper()
	start := testLayout.FirstBlock + page*testLayout.BlocksPerPage
	for offset := int64(0); offset < testLayout.BlocksPerPage; offset++ {
		a, b := offset*8, (offset+1)*8
		device.blocks[start+offset] = append([]byte(nil), data[a:b]...)
	}
}
func readPage(t *testing.T, device blockdevicecontract.Device, page int64) []byte {
	t.Helper()
	pager, err := storagepageio.Open(device, testLayout)
	if err != nil {
		t.Fatal(err)
	}
	got, err := pager.ReadPage(page)
	if err != nil {
		t.Fatal(err)
	}
	return got
}
func assertScannedRecord(t *testing.T, device blockdevicecontract.Device, count int) {
	t.Helper()
	pages := make([][]byte, testLayout.PageCount)
	pager, err := storagepageio.Open(device, testLayout)
	if err != nil {
		t.Fatal(err)
	}
	for index := range pages {
		pages[index], err = pager.ReadPage(int64(index))
		if err != nil {
			t.Fatal(err)
		}
	}
	scan, err := storagewalscan.ScanPages(pages)
	if err != nil || len(scan.Records) != count {
		t.Fatalf("scan=(%+v,%v), records=%d", scan, err, count)
	}
}
func sequence(first int64, count int) []int64 {
	result := make([]int64, count)
	for i := range result {
		result[i] = first + int64(i)
	}
	return result
}
func equalBlocks(a, b []int64) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
func sameRecord(got, want storagewalrecord.Record) bool {
	return got.Kind == want.Kind && got.LSN == want.LSN && got.TxID == want.TxID && got.PageID == want.PageID && len(got.Image) == len(want.Image)
}
