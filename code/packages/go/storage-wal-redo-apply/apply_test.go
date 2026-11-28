package storagewalredoapply

import (
	"errors"
	"fmt"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalredoplan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-plan"
)

var replayGeometry = blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
var replayLayout = storagepagelayout.Layout{Geometry: replayGeometry, FirstBlock: 2, PageCount: 4, BlocksPerPage: 7}
var replayWriteFailure = errors.New("scripted-write-failure")
var replayFlushFailure = errors.New("scripted-flush-failure")
var lostAcknowledgement = errors.New("simulated-lost-barrier-response")

type scriptedDevice struct {
	geometry    blockdevicecontract.Geometry
	blocks      map[int64][]byte
	events      []string
	writes      int
	flushes     int
	failWriteAt int
	failFlushAt int
}

func newScripted(geometry blockdevicecontract.Geometry) *scriptedDevice {
	return &scriptedDevice{geometry: geometry, blocks: make(map[int64][]byte), failWriteAt: -1, failFlushAt: -1}
}
func (device *scriptedDevice) Geometry() blockdevicecontract.Geometry { return device.geometry }
func (device *scriptedDevice) ReadBlock(block int64) ([]byte, error) {
	device.events = append(device.events, fmt.Sprintf("R%d", block))
	if value, ok := device.blocks[block]; ok {
		return append([]byte(nil), value...), nil
	}
	return make([]byte, int(device.geometry.BytesPerBlock)), nil
}
func (device *scriptedDevice) WriteBlock(block int64, data []byte) error {
	device.events = append(device.events, fmt.Sprintf("W%d", block))
	index := device.writes
	device.writes++
	if index == device.failWriteAt {
		return replayWriteFailure
	}
	device.blocks[block] = append([]byte(nil), data...)
	return nil
}
func (device *scriptedDevice) Flush() error {
	device.events = append(device.events, "F")
	index := device.flushes
	device.flushes++
	if index == device.failFlushAt {
		return replayFlushFailure
	}
	return nil
}

type cutHDD struct {
	device     *hddblockdevice.HDD
	cuts       []hddwritebackcache.BarrierCut
	flushIndex int
}

func (device *cutHDD) Geometry() blockdevicecontract.Geometry { return device.device.Geometry() }
func (device *cutHDD) ReadBlock(block int64) ([]byte, error)  { return device.device.ReadBlock(block) }
func (device *cutHDD) WriteBlock(block int64, data []byte) error {
	return device.device.WriteBlock(block, data)
}
func (device *cutHDD) Flush() error {
	cut := hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete}
	if device.flushIndex < len(device.cuts) {
		cut = device.cuts[device.flushIndex]
	}
	device.flushIndex++
	receipt, err := device.device.FlushWithCut(cut)
	if err != nil {
		return err
	}
	if !receipt.Acknowledged {
		return lostAcknowledgement
	}
	return nil
}

func TestReplayWritesOnePageThenBarrierAndReturnsCompleteCount(t *testing.T) {
	records := replayRecords()
	device := newScripted(replayGeometry)
	for block, value := range map[int64]byte{0: 0xa0, 1: 0xa1, 30: 0xbe, 31: 0xbf} {
		device.blocks[block] = bytesOf(value, 8)
	}
	receipt, err := Replay(device, replayLayout, records)
	if err != nil || receipt != (Receipt{AcknowledgedWrites: 3}) {
		t.Fatalf("Replay=(%+v,%v)", receipt, err)
	}
	want := appendWriteBarrierEvents(nil, 2, 7)
	want = appendWriteBarrierEvents(want, 2, 7)
	want = appendWriteBarrierEvents(want, 9, 7)
	if !reflect.DeepEqual(device.events, want) {
		t.Fatalf("events=%v want=%v", device.events, want)
	}
	assertDataImage(t, device)
}

func TestReplayEmptyPlanIsAZeroCommand(t *testing.T) {
	device := newScripted(replayGeometry)
	receipt, err := Replay(device, replayLayout, []storagewalrecord.Record{begin(1, 2), commit(2, 2)})
	if err != nil || receipt != (Receipt{}) || len(device.events) != 0 {
		t.Fatalf("Replay=(%+v,%v), events=%v", receipt, err, device.events)
	}
}

func TestReplayOpenAndPlanErrorsPrecedeMutation(t *testing.T) {
	device := newScripted(replayGeometry)
	badLayout := storagepagelayout.Layout{Geometry: blockdevicecontract.Geometry{}, FirstBlock: -1}
	receipt, err := Replay(device, badLayout, []storagewalrecord.Record{{Kind: 255}})
	if receipt != (Receipt{}) || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || len(device.events) != 0 {
		t.Fatalf("Replay=(%+v,%v), events=%v", receipt, err, device.events)
	}
	receipt, err = Replay(device, replayLayout, []storagewalrecord.Record{{Kind: 255}})
	if receipt != (Receipt{}) || !errors.Is(err, storagewalredoplan.ErrRecordKind) || len(device.events) != 0 {
		t.Fatalf("plan refusal=(%+v,%v), events=%v", receipt, err, device.events)
	}
	badPage := []storagewalrecord.Record{begin(1, 1), write(2, 1, -1, bytesOf(1, 8)), commit(3, 1)}
	receipt, err = Replay(device, replayLayout, badPage)
	if receipt != (Receipt{}) || !errors.Is(err, storagepagelayout.ErrInvalidPage) || len(device.events) != 0 {
		t.Fatalf("page refusal=(%+v,%v), events=%v", receipt, err, device.events)
	}
}

func TestReplayEveryPageWritePrefixStopsBeforeBarrier(t *testing.T) {
	for failAt := 0; failAt < 7; failAt++ {
		t.Run(fmt.Sprintf("block-prefix-%d", failAt), func(t *testing.T) {
			device := newScripted(replayGeometry)
			device.failWriteAt = failAt
			receipt, err := Replay(device, replayLayout, oneWriteRecords(bytesOf(7, 56)))
			want := Receipt{Interrupted: true, PageID: 1, RecordLSN: 2, WrittenBlocks: int64(failAt)}
			if receipt != want || !errors.Is(err, replayWriteFailure) {
				t.Fatalf("Replay=(%+v,%v) want=%+v", receipt, err, want)
			}
			if device.flushes != 0 || len(device.events) != failAt+1 {
				t.Fatalf("write failure requested barrier: events=%v", device.events)
			}
		})
	}
}

func TestReplayWriteAndBarrierFailuresReportEarlierAcknowledgements(t *testing.T) {
	t.Run("second image write fails", func(t *testing.T) {
		device := newScripted(replayGeometry)
		device.failWriteAt = 7
		receipt, err := Replay(device, replayLayout, replayRecords())
		want := Receipt{AcknowledgedWrites: 1, Interrupted: true, PageID: 0, RecordLSN: 5, WrittenBlocks: 0}
		if receipt != want || !errors.Is(err, replayWriteFailure) {
			t.Fatalf("Replay=(%+v,%v) want=%+v", receipt, err, want)
		}
		if device.flushes != 1 || device.writes != 8 {
			t.Fatalf("writes=%d flushes=%d", device.writes, device.flushes)
		}
	})
	t.Run("second barrier fails", func(t *testing.T) {
		pages := []int64{0, 0, 1}
		lsns := []uint64{2, 5, 6}
		for failed := 0; failed < 3; failed++ {
			t.Run(fmt.Sprintf("flush-%d", failed), func(t *testing.T) {
				device := newScripted(replayGeometry)
				device.failFlushAt = failed
				receipt, err := Replay(device, replayLayout, replayRecords())
				want := Receipt{AcknowledgedWrites: failed, Interrupted: true, PageID: pages[failed], RecordLSN: lsns[failed], WrittenBlocks: 7}
				if receipt != want || !errors.Is(err, replayFlushFailure) {
					t.Fatalf("Replay=(%+v,%v) want=%+v", receipt, err, want)
				}
				if device.flushes != failed+1 || device.writes != (failed+1)*7 {
					t.Fatalf("unexpected operations: events=%v", device.events)
				}
			})
		}
	})
}

func TestReplayMemoryComparatorAndTwoCrashHDDConvergence(t *testing.T) {
	records := replayRecords()
	memory, err := memoryblockdevice.New(replayGeometry)
	if err != nil {
		t.Fatal(err)
	}
	if err := seedNeighbors(memory); err != nil {
		t.Fatal(err)
	}
	if receipt, err := Replay(memory, replayLayout, records); err != nil || receipt.AcknowledgedWrites != 3 {
		t.Fatalf("memory replay=(%+v,%v)", receipt, err)
	}
	assertDataImage(t, memory)

	hdd := hddblockdevice.New()
	if err := seedNeighbors(hdd); err != nil {
		t.Fatal(err)
	}
	for cutIndex, firstCut := range allBarrierCuts() {
		t.Run(fmt.Sprintf("first-cut-%02d", cutIndex), func(t *testing.T) {
			device := hddblockdevice.New()
			if err := seedNeighbors(device); err != nil {
				t.Fatal(err)
			}
			first := &cutHDD{device: device, cuts: []hddwritebackcache.BarrierCut{firstCut}}
			firstReceipt, firstErr := Replay(first, replayLayout, records)
			if firstCut.Kind == hddwritebackcache.BarrierComplete {
				if firstErr != nil || firstReceipt != (Receipt{AcknowledgedWrites: 3}) {
					t.Fatalf("complete cut=(%+v,%v)", firstReceipt, firstErr)
				}
				assertDataImage(t, device)
				return
			}
			wantCrash := Receipt{Interrupted: true, PageID: 0, RecordLSN: 2, WrittenBlocks: 7}
			if !errors.Is(firstErr, lostAcknowledgement) || firstReceipt != wantCrash {
				t.Fatalf("first crash=(%+v,%v)", firstReceipt, firstErr)
			}
			restarted, err := device.Restart()
			if err != nil {
				t.Fatal(err)
			}
			second := &cutHDD{device: restarted, cuts: []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.AfterAllMedia}}}
			secondReceipt, secondErr := Replay(second, replayLayout, records)
			if !errors.Is(secondErr, lostAcknowledgement) || secondReceipt != wantCrash {
				t.Fatalf("second crash=(%+v,%v)", secondReceipt, secondErr)
			}
			last, err := restarted.Restart()
			if err != nil {
				t.Fatal(err)
			}
			finalReceipt, finalErr := Replay(last, replayLayout, records)
			if finalErr != nil || finalReceipt != (Receipt{AcknowledgedWrites: 3}) {
				t.Fatalf("final replay=(%+v,%v)", finalReceipt, finalErr)
			}
			assertDataImage(t, last)
		})
	}
}

func TestBoundedSeededReplayFaults(t *testing.T) {
	for seed := uint32(1); seed <= 512; seed++ {
		state := seed*747796405 + 2891336453
		input := make([]byte, 24)
		for index := range input {
			state = state*1664525 + 1013904223
			input[index] = byte(state >> 24)
		}
		runReplayOracle(t, input)
	}
}

func FuzzReplayWritesAndBarriers(f *testing.F) {
	f.Add([]byte{0, 0, 0, 0, 0})
	f.Add([]byte{1, 0, 2, 0, 3, 0, 1})
	f.Add([]byte{0xff, 7, 2, 1, 3, 0})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 128 {
			t.Skip()
		}
		runReplayOracle(t, input)
	})
}

func runReplayOracle(t *testing.T, input []byte) {
	t.Helper()
	records := decodeReplayHistory(input)
	plan, planErr := referencePlan(records)
	device := newScripted(replayGeometry)
	wantBlocks := make(map[int64][]byte, replayGeometry.BlockCount)
	for block := int64(0); block < replayGeometry.BlockCount; block++ {
		initial := bytesOf(byte(block*11+3), 8)
		device.blocks[block] = append([]byte(nil), initial...)
		wantBlocks[block] = append([]byte(nil), initial...)
	}
	if planErr != nil {
		got, err := Replay(device, replayLayout, records)
		if got != (Receipt{}) || !sameFailure(err, planErr) || len(device.events) != 0 {
			t.Fatalf("preflight got=(%+v,%v) events=%v, want error=%v", got, err, device.events, planErr)
		}
		return
	}
	if len(plan) == 0 {
		got, err := Replay(device, replayLayout, records)
		if got != (Receipt{}) || err != nil || len(device.events) != 0 {
			t.Fatalf("empty got=(%+v,%v) events=%v", got, err, device.events)
		}
		return
	}
	mode, failureAt, blockPrefix := 0, 0, 0
	if len(input) > 0 {
		mode = int(input[0] % 3)
		if len(input) > 1 {
			failureAt = int(input[1]) % len(plan)
		}
		if len(input) > 2 {
			blockPrefix = int(input[2]) % int(replayLayout.BlocksPerPage)
		}
	}
	if mode == 1 {
		device.failWriteAt = failureAt*int(replayLayout.BlocksPerPage) + blockPrefix
	}
	if mode == 2 {
		device.failFlushAt = failureAt
	}
	acknowledged := len(plan)
	var wantErr error
	if mode != 0 {
		acknowledged = failureAt
		if mode == 1 {
			wantErr = replayWriteFailure
		} else {
			wantErr = replayFlushFailure
		}
	}
	applyCount := acknowledged
	if mode == 2 {
		applyCount++
	}
	for index := 0; index < applyCount; index++ {
		entry := plan[index]
		first := replayLayout.FirstBlock + entry.Record.PageID*replayLayout.BlocksPerPage
		for offset := int64(0); offset < replayLayout.BlocksPerPage; offset++ {
			start := offset * replayGeometry.BytesPerBlock
			wantBlocks[first+offset] = append([]byte(nil), entry.Record.Image[start:start+replayGeometry.BytesPerBlock]...)
		}
	}
	if mode == 1 {
		current := plan[failureAt]
		first := replayLayout.FirstBlock + current.Record.PageID*replayLayout.BlocksPerPage
		for offset := 0; offset < blockPrefix; offset++ {
			start := offset * int(replayGeometry.BytesPerBlock)
			wantBlocks[first+int64(offset)] = append([]byte(nil), current.Record.Image[start:start+int(replayGeometry.BytesPerBlock)]...)
		}
	}
	got, err := Replay(device, replayLayout, records)
	var want Receipt
	if mode == 0 {
		want.AcknowledgedWrites = len(plan)
	} else {
		current := plan[failureAt]
		want = Receipt{AcknowledgedWrites: failureAt, Interrupted: true, PageID: current.Record.PageID, RecordLSN: current.Record.LSN, WrittenBlocks: int64(blockPrefix)}
		if mode == 2 {
			want.WrittenBlocks = replayLayout.BlocksPerPage
		}
	}
	if got != want || !sameFailure(err, wantErr) {
		t.Fatalf("input=%v got=(%+v,%v) want=(%+v,%v)", input, got, err, want, wantErr)
	}
	for block, expected := range wantBlocks {
		if !equalBytes(device.blocks[block], expected) {
			t.Fatalf("input=%v block %d=%v want=%v", input, block, device.blocks[block], expected)
		}
	}
}

func decodeReplayHistory(input []byte) []storagewalrecord.Record {
	count := len(input)
	if count > 32 {
		count = 32
	}
	records := make([]storagewalrecord.Record, count)
	for index := 0; index < count; index++ {
		value := input[index]
		lsn := uint64(index + 1)
		tx := uint64((value>>3)%3 + 1)
		switch value % 6 {
		case 0:
			records[index] = begin(lsn, tx)
		case 1:
			records[index] = commit(lsn, tx)
		case 2, 3, 4:
			page := int64((value>>5)%6) - 1
			width := 56
			if value&4 != 0 {
				width = 55
			}
			image := make([]byte, width)
			for i := range image {
				image[i] = value + byte(i*13)
			}
			records[index] = write(lsn, tx, page, image)
		case 5:
			records[index] = storagewalrecord.Record{Kind: 255, LSN: lsn, TxID: tx}
		}
	}
	return records
}

func referencePlan(records []storagewalrecord.Record) ([]storagewalredoplan.RedoWrite, error) {
	type txModel struct {
		active  bool
		started bool
		writes  []storagewalrecord.Record
	}
	table := map[uint64]txModel{}
	var output []storagewalredoplan.RedoWrite
	for _, record := range records {
		switch record.Kind {
		case storagewalrecord.BeginTransaction:
			row := table[record.TxID]
			if row.started {
				return nil, storagewalredoplan.ErrTransactionExists
			}
			row.started = true
			row.active = true
			table[record.TxID] = row
		case storagewalrecord.WritePageImage:
			row, ok := table[record.TxID]
			if !ok || !row.active {
				return nil, storagewalredoplan.ErrTransactionInactive
			}
			if record.PageID < 0 || record.PageID >= replayLayout.PageCount {
				return nil, storagepagelayout.ErrInvalidPage
			}
			if len(record.Image) != int(replayLayout.BlocksPerPage*replayGeometry.BytesPerBlock) {
				return nil, storagewalredoplan.ErrImageWidth
			}
			row.writes = append(row.writes, record)
			table[record.TxID] = row
		case storagewalrecord.CommitTransaction:
			row, ok := table[record.TxID]
			if !ok || !row.active {
				return nil, storagewalredoplan.ErrTransactionInactive
			}
			row.active = false
			for _, image := range row.writes {
				owned := image
				owned.Image = append([]byte(nil), image.Image...)
				output = append(output, storagewalredoplan.RedoWrite{Record: owned, CommitLSN: record.LSN})
			}
			row.writes = nil
			table[record.TxID] = row
		default:
			return nil, storagewalredoplan.ErrRecordKind
		}
	}
	return output, nil
}

func sameFailure(got, want error) bool {
	if got == nil || want == nil {
		return got == nil && want == nil
	}
	return errors.Is(got, want) && errors.Is(want, got)
}

func replayRecords() []storagewalrecord.Record {
	return []storagewalrecord.Record{
		begin(1, 10), write(2, 10, 0, bytesOf(0x11, 56)), commit(3, 10),
		begin(4, 20), write(5, 20, 0, bytesOf(0x22, 56)), write(6, 20, 1, bytesOf(0x33, 56)), commit(7, 20),
		begin(8, 30), write(9, 30, 2, bytesOf(0x44, 56)),
	}
}
func oneWriteRecords(image []byte) []storagewalrecord.Record {
	return []storagewalrecord.Record{begin(1, 7), write(2, 7, 1, image), commit(3, 7)}
}
func begin(lsn, tx uint64) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: lsn, TxID: tx}
}
func write(lsn, tx uint64, page int64, image []byte) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: lsn, TxID: tx, PageID: page, Image: append([]byte(nil), image...)}
}
func commit(lsn, tx uint64) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.CommitTransaction, LSN: lsn, TxID: tx}
}
func bytesOf(value byte, width int) []byte {
	result := make([]byte, width)
	for index := range result {
		result[index] = value
	}
	return result
}
func intPointer(value int) *int { return &value }
func seedNeighbors(device blockdevicecontract.Device) error {
	for _, pair := range []struct {
		block int64
		value byte
	}{{0, 0xa0}, {1, 0xa1}, {30, 0xbe}, {31, 0xbf}} {
		if err := device.WriteBlock(pair.block, bytesOf(pair.value, 8)); err != nil {
			return err
		}
	}
	return device.Flush()
}
func assertDataImage(t *testing.T, device blockdevicecontract.Device) {
	t.Helper()
	pager, err := storagepageio.Open(device, replayLayout)
	if err != nil {
		t.Fatal(err)
	}
	for page, value := range map[int64]byte{0: 0x22, 1: 0x33, 2: 0, 3: 0} {
		got, err := pager.ReadPage(page)
		if err != nil || !equalBytes(got, bytesOf(value, 56)) {
			t.Fatalf("page %d=(%v,%v) want %02x", page, got, err, value)
		}
	}
	for block, value := range map[int64]byte{0: 0xa0, 1: 0xa1, 30: 0xbe, 31: 0xbf} {
		got, err := device.ReadBlock(block)
		if err != nil || !equalBytes(got, bytesOf(value, 8)) {
			t.Fatalf("block %d=(%v,%v) want %02x", block, got, err, value)
		}
	}
}

func appendWriteBarrierEvents(events []string, first int64, count int) []string {
	for offset := 0; offset < count; offset++ {
		events = append(events, fmt.Sprintf("W%d", first+int64(offset)))
	}
	return append(events, "F")
}

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
func equalBytes(a, b []byte) bool {
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
