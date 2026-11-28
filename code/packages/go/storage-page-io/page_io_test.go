package storagepageio

import (
	"bytes"
	"errors"
	"math/rand"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
)

func TestOpenValidationOrderAndZeroPager(t *testing.T) {
	valid := layoutFor(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}, 4, 4, 2)
	invalidLayout := storagepagelayout.Layout{Geometry: blockdevicecontract.Geometry{}, FirstBlock: -1}
	if pager, err := Open(nil, invalidLayout); pager != (Pager{}) || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("invalid layout before nil device: (%+v,%v)", pager, err)
	}
	if pager, err := Open(nil, valid); pager != (Pager{}) || !errors.Is(err, ErrInvalidDevice) {
		t.Fatalf("nil device: (%+v,%v)", pager, err)
	}
	mismatched := newScriptedDevice(blockdevicecontract.Geometry{BlockCount: 31, BytesPerBlock: 8})
	if pager, err := Open(mismatched, valid); pager != (Pager{}) || !errors.Is(err, ErrGeometryMismatch) {
		t.Fatalf("geometry mismatch: (%+v,%v)", pager, err)
	}
	zero := Pager{}
	if data, err := zero.ReadPage(0); data != nil || !errors.Is(err, ErrInvalidDevice) {
		t.Fatalf("zero ReadPage=(%v,%v)", data, err)
	}
	if completed, err := zero.WritePage(0, nil); completed != 0 || !errors.Is(err, ErrInvalidDevice) {
		t.Fatalf("zero WritePage=(%d,%v)", completed, err)
	}
	if err := zero.Flush(); !errors.Is(err, ErrInvalidDevice) {
		t.Fatalf("zero Flush error=%v", err)
	}
}

func TestScriptedReadOffsetsFirstErrorsWidthsAndDetachment(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 20, BytesPerBlock: 2}
	layout := layoutFor(geometry, 5, 4, 3)
	device := newScriptedDevice(geometry)
	fillScriptBlocks(device)
	pager := mustOpen(t, device, layout)
	page, err := pager.ReadPage(1)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(page, []byte{16, 17, 18, 19, 20, 21}) {
		t.Fatalf("ReadPage=%v", page)
	}
	if !equalInts(device.readCalls, []int64{8, 9, 10}) {
		t.Fatalf("read calls=%v want [8 9 10]", device.readCalls)
	}
	page[0] = 0xff
	again, err := pager.ReadPage(1)
	if err != nil || !bytes.Equal(again, []byte{16, 17, 18, 19, 20, 21}) {
		t.Fatalf("read result was not detached: %v, %v", again, err)
	}

	for failureOffset := int64(0); failureOffset < layout.BlocksPerPage; failureOffset++ {
		device := newScriptedDevice(geometry)
		fillScriptBlocks(device)
		failure := errors.New("scripted_read_failure")
		device.readErrors[layout.FirstBlock+2*layout.BlocksPerPage+failureOffset] = failure
		pager := mustOpen(t, device, layout)
		got, err := pager.ReadPage(2)
		if got != nil || !errors.Is(err, failure) {
			t.Fatalf("read failure offset %d: (%v,%v)", failureOffset, got, err)
		}
		wantCalls := make([]int64, failureOffset+1)
		for index := range wantCalls {
			wantCalls[index] = layout.FirstBlock + 2*layout.BlocksPerPage + int64(index)
		}
		if !equalInts(device.readCalls, wantCalls) {
			t.Fatalf("read failure offset %d calls=%v want=%v", failureOffset, device.readCalls, wantCalls)
		}
	}

	for _, mismatch := range []struct {
		name   string
		length int
	}{
		{"short", 1},
		{"long", 3},
	} {
		t.Run(mismatch.name, func(t *testing.T) {
			device := newScriptedDevice(geometry)
			fillScriptBlocks(device)
			device.readLengths[layout.FirstBlock+1] = mismatch.length
			pager := mustOpen(t, device, layout)
			if got, err := pager.ReadPage(0); got != nil || !errors.Is(err, ErrBlockWidthMismatch) {
				t.Fatalf("width mismatch=(%v,%v)", got, err)
			}
			if !equalInts(device.readCalls, []int64{5, 6}) {
				t.Fatalf("width mismatch should stop at first bad block, calls=%v", device.readCalls)
			}
		})
	}
}

func TestScriptedWriteCallsExactOffsetsAndPrefixCounts(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 20, BytesPerBlock: 2}
	layout := layoutFor(geometry, 5, 4, 3)
	payload := []byte{0, 1, 2, 3, 4, 5}
	device := newScriptedDevice(geometry)
	pager := mustOpen(t, device, layout)
	completed, err := pager.WritePage(1, payload)
	if err != nil || completed != 3 {
		t.Fatalf("WritePage=(%d,%v)", completed, err)
	}
	if len(device.writeCalls) != 3 {
		t.Fatalf("write call count=%d", len(device.writeCalls))
	}
	for index, call := range device.writeCalls {
		if call.block != int64(8+index) || !bytes.Equal(call.data, payload[index*2:index*2+2]) {
			t.Fatalf("write %d=(%d,%v)", index, call.block, call.data)
		}
	}
	payload[0] = 0xff
	if !bytes.Equal(device.blocks[8], []byte{0, 1}) {
		t.Fatalf("device retained caller buffer: %v", device.blocks[8])
	}

	for failureOffset := int64(0); failureOffset < layout.BlocksPerPage; failureOffset++ {
		device := newScriptedDevice(geometry)
		failure := errors.New("scripted_write_failure")
		failingBlock := layout.FirstBlock + layout.BlocksPerPage + failureOffset
		device.writeErrors[failingBlock] = failure
		pager := mustOpen(t, device, layout)
		completed, err := pager.WritePage(1, []byte{10, 11, 12, 13, 14, 15})
		if completed != failureOffset || !errors.Is(err, failure) {
			t.Fatalf("write failure offset %d=(%d,%v)", failureOffset, completed, err)
		}
		if len(device.writeCalls) != int(failureOffset+1) {
			t.Fatalf("write failure offset %d calls=%d", failureOffset, len(device.writeCalls))
		}
	}

	device = newScriptedDevice(geometry)
	pager = mustOpen(t, device, layout)
	if completed, err := pager.WritePage(0, []byte{1}); completed != 0 || !errors.Is(err, ErrInvalidPageData) {
		t.Fatalf("wrong page width=(%d,%v)", completed, err)
	}
	if len(device.writeCalls) != 0 {
		t.Fatalf("wrong width made writes: %+v", device.writeCalls)
	}
	if completed, err := pager.WritePage(4, nil); completed != 0 || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
		t.Fatalf("page error should precede width=(%d,%v)", completed, err)
	}
}

func TestFlushForwardsLowerError(t *testing.T) {
	device := newScriptedDevice(blockdevicecontract.Geometry{BlockCount: 8, BytesPerBlock: 2})
	failure := errors.New("scripted_flush_failure")
	device.flushError = failure
	pager := mustOpen(t, device, layoutFor(device.geometry, 0, 4, 2))
	if err := pager.Flush(); !errors.Is(err, failure) {
		t.Fatalf("Flush error=%v want exact lower identity", err)
	}
	if device.flushCalls != 1 {
		t.Fatalf("flush calls=%d want 1", device.flushCalls)
	}
}

func TestRealMemoryAndHDDPageClientTraceAndRestart(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	layout := layoutFor(geometry, 4, 3, 8)
	memory, err := memoryblockdevice.New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	devices := []blockdevicecontract.Device{memory, hddblockdevice.New()}
	pagers := []Pager{mustOpen(t, devices[0], layout), mustOpen(t, devices[1], layout)}
	zero, err := pagers[0].ReadPage(0)
	if err != nil || !bytes.Equal(zero, make([]byte, 64)) {
		t.Fatalf("initial page=(%v,%v)", zero, err)
	}

	data := make([]byte, 64)
	for index := range data {
		data[index] = byte(index + 1)
	}
	snapshot := append([]byte(nil), data...)
	for index := range pagers {
		completed, err := pagers[index].WritePage(1, data)
		if err != nil || completed != 8 {
			t.Fatalf("backend %d WritePage=(%d,%v)", index, completed, err)
		}
	}
	for index := range data {
		data[index] ^= 0xff
	}
	for _, pager := range pagers {
		read, err := pager.ReadPage(1)
		if err != nil || !bytes.Equal(read, snapshot) {
			t.Fatalf("pending page=(%v,%v) want %v", read, err, snapshot)
		}
	}
	for index, backend := range devices {
		pendingRestart, err := restartBackend(backend)
		if err != nil {
			t.Fatal(err)
		}
		pendingPager := mustOpen(t, pendingRestart, layout)
		if got, err := pendingPager.ReadPage(1); err != nil || !bytes.Equal(got, make([]byte, 64)) {
			t.Fatalf("backend %d pending restart=(%v,%v)", index, got, err)
		}
		if got, err := pagers[index].ReadPage(1); err != nil || !bytes.Equal(got, snapshot) {
			t.Fatalf("backend %d restart changed pending source=(%v,%v)", index, got, err)
		}
	}
	for index := range pagers {
		if err := pagers[index].Flush(); err != nil {
			t.Fatalf("backend %d Flush: %v", index, err)
		}
	}

	for index, backend := range devices {
		restarted, err := restartBackend(backend)
		if err != nil {
			t.Fatal(err)
		}
		restartedPager := mustOpen(t, restarted, layout)
		if got, err := restartedPager.ReadPage(1); err != nil || !bytes.Equal(got, snapshot) {
			t.Fatalf("backend %d restarted page=(%v,%v)", index, got, err)
		}
		if got, err := pagers[index].ReadPage(1); err != nil || !bytes.Equal(got, snapshot) {
			t.Fatalf("backend %d live page changed=(%v,%v)", index, got, err)
		}
		if _, err := restartedPager.WritePage(0, bytes.Repeat([]byte{byte(index + 31)}, 64)); err != nil {
			t.Fatal(err)
		}
		if err := restartedPager.Flush(); err != nil {
			t.Fatal(err)
		}
		if got, err := pagers[index].ReadPage(0); err != nil || !bytes.Equal(got, make([]byte, 64)) {
			t.Fatalf("backend %d write on restart branch changed input=(%v,%v)", index, got, err)
		}
	}
}

func TestEveryHDDBarrierCutOnEightBlockPage(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	layout := layoutFor(geometry, 4, 3, 8)
	cuts := []hddwritebackcache.BarrierCut{
		{Kind: hddwritebackcache.BeforeFirstSector},
		{Kind: hddwritebackcache.AfterAllMedia},
		{Kind: hddwritebackcache.BarrierComplete},
	}
	for ordinal := int64(0); ordinal < 8; ordinal++ {
		cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}})
		for prefix := 1; prefix <= 7; prefix++ {
			value := prefix
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value}})
		}
		cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})
	}
	oldPage := blockBytes64(9)
	newPage := blockBytes64(101)
	for _, cut := range cuts {
		device := hddblockdevice.New()
		pager := mustOpen(t, device, layout)
		if completed, err := pager.WritePage(1, oldPage); err != nil || completed != 8 {
			t.Fatalf("baseline WritePage=(%d,%v)", completed, err)
		}
		if err := pager.Flush(); err != nil {
			t.Fatal(err)
		}
		if completed, err := pager.WritePage(1, newPage); err != nil || completed != 8 {
			t.Fatalf("replacement WritePage=(%d,%v)", completed, err)
		}
		expected, receipt := modelPageBarrierCut(oldPage, newPage, cut)
		gotReceipt, err := device.FlushWithCut(cut)
		if err != nil || gotReceipt != receipt {
			t.Fatalf("cut=%+v receipt=(%+v,%v) want=%+v", cut, gotReceipt, err, receipt)
		}
		if got, err := pager.ReadPage(1); err != nil || !bytes.Equal(got, expected) {
			t.Fatalf("cut=%+v page=(%v,%v) want=%v", cut, got, err, expected)
		}
		restarted, err := device.Restart()
		if err != nil {
			t.Fatal(err)
		}
		restartedPager := mustOpen(t, restarted, layout)
		if got, err := restartedPager.ReadPage(1); err != nil || !bytes.Equal(got, expected) {
			t.Fatalf("cut=%+v restarted=(%v,%v) want=%v", cut, got, err, expected)
		}
	}
}

func TestSeededPageHistories(t *testing.T) {
	const seed = int64(0xDB11B)
	const operations = 20_000
	rng := rand.New(rand.NewSource(seed))
	geometry := blockdevicecontract.Geometry{BlockCount: 12, BytesPerBlock: 2}
	layout := layoutFor(geometry, 2, 4, 2)
	device := newScriptedDevice(geometry)
	pager := mustOpen(t, device, layout)
	var expected [12][2]byte
	for step := 0; step < operations; step++ {
		page := int64(rng.Intn(6) - 1)
		switch rng.Intn(3) {
		case 0:
			read, err := pager.ReadPage(page)
			if page < 0 || page >= layout.PageCount {
				if read != nil || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
					t.Fatalf("step %d bad read=(%v,%v)", step, read, err)
				}
			} else if err != nil || !bytes.Equal(read, modelPage(expected, layout, page)) {
				t.Fatalf("step %d read=(%v,%v)", step, read, err)
			}
		case 1:
			payload := make([]byte, int(layout.BlocksPerPage*geometry.BytesPerBlock))
			_, _ = rng.Read(payload)
			completed, err := pager.WritePage(page, payload)
			if page < 0 || page >= layout.PageCount {
				if completed != 0 || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
					t.Fatalf("step %d invalid write=(%d,%v)", step, completed, err)
				}
			} else {
				if err != nil || completed != layout.BlocksPerPage {
					t.Fatalf("step %d valid write=(%d,%v)", step, completed, err)
				}
				modelWritePage(&expected, layout, page, payload)
			}
		case 2:
			if err := pager.Flush(); err != nil {
				t.Fatalf("step %d flush: %v", step, err)
			}
		}
		for pageID := int64(0); pageID < layout.PageCount; pageID++ {
			got, err := pager.ReadPage(pageID)
			if err != nil || !bytes.Equal(got, modelPage(expected, layout, pageID)) {
				t.Fatalf("step %d page %d=(%v,%v)", step, pageID, got, err)
			}
		}
	}
}

func FuzzPageIOHistories(f *testing.F) {
	f.Add([]byte{1, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 2, 0})
	f.Add([]byte{0xff, 0, 1, 2, 3, 4, 2, 5, 0, 0, 5, 1, 1, 9, 8, 7, 6})
	f.Add([]byte{1, 3, 8, 7, 6, 5, 4, 2, 0, 1, 2, 3, 4})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 1024 {
			t.Skip()
		}
		geometry := blockdevicecontract.Geometry{BlockCount: 12, BytesPerBlock: 2}
		layout := layoutFor(geometry, 2, 4, 2)
		device := newScriptedDevice(geometry)
		pager := mustOpen(t, device, layout)
		var expected [12][2]byte
		for cursor := 0; cursor < len(input); {
			op := input[cursor] % 4
			cursor++
			if cursor >= len(input) && op != 3 {
				return
			}
			page := int64(int8(input[cursor-1])) % 6
			switch op {
			case 0:
				got, err := pager.ReadPage(page)
				if page < 0 || page >= layout.PageCount {
					if got != nil || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
						t.Fatalf("read page %d=(%v,%v)", page, got, err)
					}
				} else if err != nil || !bytes.Equal(got, modelPage(expected, layout, page)) {
					t.Fatalf("read page %d=(%v,%v)", page, got, err)
				}
			case 1, 2:
				length := 4
				if op == 2 {
					length = int(input[cursor-1] % 7)
				}
				if cursor+length > len(input) {
					return
				}
				payload := append([]byte(nil), input[cursor:cursor+length]...)
				cursor += length
				completed, err := pager.WritePage(page, payload)
				if page < 0 || page >= layout.PageCount {
					if completed != 0 || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
						t.Fatalf("invalid write=(%d,%v)", completed, err)
					}
				} else if length != 4 {
					if completed != 0 || !errors.Is(err, ErrInvalidPageData) {
						t.Fatalf("wrong-width write=(%d,%v)", completed, err)
					}
				} else {
					if err != nil || completed != 2 {
						t.Fatalf("write=(%d,%v)", completed, err)
					}
					modelWritePage(&expected, layout, page, payload)
				}
			case 3:
				if err := pager.Flush(); err != nil {
					t.Fatal(err)
				}
			}
			for pageID := int64(0); pageID < layout.PageCount; pageID++ {
				got, err := pager.ReadPage(pageID)
				if err != nil || !bytes.Equal(got, modelPage(expected, layout, pageID)) {
					t.Fatalf("state page %d=(%v,%v)", pageID, got, err)
				}
			}
		}
	})
}

func layoutFor(geometry blockdevicecontract.Geometry, first, pages, blocksPerPage int64) storagepagelayout.Layout {
	return storagepagelayout.Layout{Geometry: geometry, FirstBlock: first, PageCount: pages, BlocksPerPage: blocksPerPage}
}

func mustOpen(t *testing.T, device blockdevicecontract.Device, layout storagepagelayout.Layout) Pager {
	t.Helper()
	pager, err := Open(device, layout)
	if err != nil {
		t.Fatalf("Open(): %v", err)
	}
	return pager
}

func restartBackend(device blockdevicecontract.Device) (blockdevicecontract.Device, error) {
	switch typed := device.(type) {
	case *memoryblockdevice.Memory:
		return typed.Restart()
	case *hddblockdevice.HDD:
		return typed.Restart()
	default:
		return nil, errors.New("unknown_test_backend")
	}
}

type writeCall struct {
	block int64
	data  []byte
}

type scriptedDevice struct {
	geometry    blockdevicecontract.Geometry
	blocks      [][]byte
	readCalls   []int64
	writeCalls  []writeCall
	readErrors  map[int64]error
	writeErrors map[int64]error
	readLengths map[int64]int
	flushError  error
	flushCalls  int
}

func newScriptedDevice(geometry blockdevicecontract.Geometry) *scriptedDevice {
	device := &scriptedDevice{
		geometry:    geometry,
		blocks:      make([][]byte, int(geometry.BlockCount)),
		readErrors:  make(map[int64]error),
		writeErrors: make(map[int64]error),
		readLengths: make(map[int64]int),
	}
	for block := range device.blocks {
		device.blocks[block] = make([]byte, int(geometry.BytesPerBlock))
	}
	return device
}

func (device *scriptedDevice) Geometry() blockdevicecontract.Geometry { return device.geometry }

func (device *scriptedDevice) ReadBlock(block int64) ([]byte, error) {
	device.readCalls = append(device.readCalls, block)
	if err := device.geometry.CheckBlock(block); err != nil {
		return nil, err
	}
	if err := device.readErrors[block]; err != nil {
		return nil, err
	}
	data := append([]byte(nil), device.blocks[block]...)
	if length, exists := device.readLengths[block]; exists {
		switch {
		case length < len(data):
			data = data[:length]
		case length > len(data):
			data = append(data, make([]byte, length-len(data))...)
		}
	}
	return data, nil
}

func (device *scriptedDevice) WriteBlock(block int64, data []byte) error {
	device.writeCalls = append(device.writeCalls, writeCall{block: block, data: append([]byte(nil), data...)})
	if err := device.geometry.CheckWrite(block, data); err != nil {
		return err
	}
	if err := device.writeErrors[block]; err != nil {
		return err
	}
	copy(device.blocks[block], data)
	return nil
}

func (device *scriptedDevice) Flush() error {
	device.flushCalls++
	return device.flushError
}

func fillScriptBlocks(device *scriptedDevice) {
	for block := range device.blocks {
		for offset := range device.blocks[block] {
			device.blocks[block][offset] = byte(block*2 + offset)
		}
	}
}

func equalInts(left, right []int64) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if left[index] != right[index] {
			return false
		}
	}
	return true
}

func modelPageBarrierCut(oldPage, newPage []byte, cut hddwritebackcache.BarrierCut) ([]byte, hddwritebackcache.BarrierReceipt) {
	result := append([]byte(nil), oldPage...)
	receipt := hddwritebackcache.BarrierReceipt{}
	blockCount := len(oldPage) / hddwritecrashcuts.BytesPerSector
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector:
		return result, receipt
	case hddwritebackcache.AfterAllMedia, hddwritebackcache.BarrierComplete:
		copy(result, newPage)
		receipt.PriorCompletedSectors = blockCount
		receipt.Acknowledged = cut.Kind == hddwritebackcache.BarrierComplete
		return result, receipt
	case hddwritebackcache.AtSector:
		receipt.PriorCompletedSectors = int(cut.SectorIndex)
		for prior := 0; prior < int(cut.SectorIndex); prior++ {
			start := prior * hddwritecrashcuts.BytesPerSector
			end := start + hddwritecrashcuts.BytesPerSector
			copy(result[start:end], newPage[start:end])
		}
		persisted := 0
		switch cut.MediaCut.Kind {
		case hddwritecrashcuts.DuringMediaWrite:
			persisted = *cut.MediaCut.WrittenPrefix
		case hddwritecrashcuts.AfterMediaWriteBeforeResponse:
			persisted = 8
		}
		start := int(cut.SectorIndex) * hddwritecrashcuts.BytesPerSector
		copy(result[start:start+persisted], newPage[start:start+persisted])
		receipt.TerminalPrefixBytes = persisted
		return result, receipt
	default:
		panic("unmodeled barrier kind")
	}
}

func blockBytes64(start byte) []byte {
	result := make([]byte, 64)
	for index := range result {
		result[index] = start + byte(index)
	}
	return result
}

func modelPage(image [12][2]byte, layout storagepagelayout.Layout, page int64) []byte {
	result := make([]byte, int(layout.BlocksPerPage*layout.Geometry.BytesPerBlock))
	first := layout.FirstBlock + page*layout.BlocksPerPage
	for offset := int64(0); offset < layout.BlocksPerPage; offset++ {
		copy(result[int(offset*layout.Geometry.BytesPerBlock):], image[first+offset][:])
	}
	return result
}

func modelWritePage(image *[12][2]byte, layout storagepagelayout.Layout, page int64, data []byte) {
	first := layout.FirstBlock + page*layout.BlocksPerPage
	for offset := int64(0); offset < layout.BlocksPerPage; offset++ {
		start := int(offset * layout.Geometry.BytesPerBlock)
		copy(image[first+offset][:], data[start:start+int(layout.Geometry.BytesPerBlock)])
	}
}
