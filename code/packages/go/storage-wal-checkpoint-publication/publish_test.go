package storagewalcheckpointpublication

import (
	"errors"
	"fmt"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointselection "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-selection"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
)

var (
	errWriteInjected = errors.New("injected write refusal")
	errFlushInjected = errors.New("injected barrier refusal")
	errLostAck       = errors.New("simulated lost barrier acknowledgement")
)

type writeEvent struct {
	Block int64
	Bytes []byte
}

type scriptedDevice struct {
	geometry   blockdevicecontract.Geometry
	blocks     map[int64][]byte
	writes     []writeEvent
	flushes    int
	failWrite  int
	failFlush  int
	readBlocks []int64
	readErr    map[int64]error
}

func newScripted(g blockdevicecontract.Geometry) *scriptedDevice {
	return &scriptedDevice{geometry: g, blocks: make(map[int64][]byte), readErr: make(map[int64]error)}
}
func (d *scriptedDevice) Geometry() blockdevicecontract.Geometry { return d.geometry }
func (d *scriptedDevice) ReadBlock(block int64) ([]byte, error) {
	d.readBlocks = append(d.readBlocks, block)
	if err := d.readErr[block]; err != nil {
		return nil, err
	}
	if value, ok := d.blocks[block]; ok {
		return append([]byte(nil), value...), nil
	}
	return make([]byte, int(d.geometry.BytesPerBlock)), nil
}
func (d *scriptedDevice) WriteBlock(block int64, data []byte) error {
	d.writes = append(d.writes, writeEvent{Block: block, Bytes: append([]byte(nil), data...)})
	if d.failWrite != 0 && len(d.writes) == d.failWrite {
		return errWriteInjected
	}
	d.blocks[block] = append([]byte(nil), data...)
	return nil
}
func (d *scriptedDevice) Flush() error {
	d.flushes++
	if d.failFlush != 0 && d.flushes == d.failFlush {
		return errFlushInjected
	}
	return nil
}

func testLayouts(width int64) (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	logBlocksPerPage := int64((storagewalpageslot.MinPageBytes + width + width - 1) / width)
	checkpointBlocksPerPage := int64((storagewalcheckpointrecord.FrameBytes + int(width) - 1) / int(width))
	logBlockCount := int64(3) * logBlocksPerPage
	dataFirst := logBlockCount
	checkpointFirst := dataFirst + 1
	geometry := blockdevicecontract.Geometry{BlockCount: checkpointFirst + 2*checkpointBlocksPerPage, BytesPerBlock: width}
	return storagepagelayout.Layout{Geometry: geometry, FirstBlock: 0, PageCount: 3, BlocksPerPage: logBlocksPerPage},
		storagepagelayout.Layout{Geometry: geometry, FirstBlock: dataFirst, PageCount: 1, BlocksPerPage: 1},
		storagepagelayout.Layout{Geometry: geometry, FirstBlock: checkpointFirst, PageCount: 2, BlocksPerPage: checkpointBlocksPerPage}
}

func readyRecord(epoch, previous uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: epoch, PreviousLSN: previous}
}
func resettingRecord(epoch, previous uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Resetting, Epoch: epoch, PreviousLSN: previous}
}

func pageImage(width int64, record storagewalcheckpointrecord.Record) []byte {
	frame, err := storagewalcheckpointslot.Stage(record)
	if err != nil {
		panic(err)
	}
	pageBytes := ((len(frame) + int(width) - 1) / int(width)) * int(width)
	result := make([]byte, pageBytes)
	copy(result, frame)
	result[storagewalcheckpointslot.GuardOffset] = 0
	return result
}

func writePageDirect(t *testing.T, device blockdevicecontract.Device, layout storagepagelayout.Layout, page int64, bytes []byte) {
	t.Helper()
	pager, err := storagepageio.Open(device, layout)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := pager.WritePage(page, bytes); err != nil {
		t.Fatal(err)
	}
}

func seedSpy(t *testing.T, width int64) (*scriptedDevice, storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	t.Helper()
	log, data, checkpoint := testLayouts(width)
	device := newScripted(log.Geometry)
	installPageDirect(t, device, checkpoint, 0, pageImage(width, readyRecord(2, 9)))
	installPageDirect(t, device, checkpoint, 1, pageImage(width, resettingRecord(1, 3)))
	return device, log, data, checkpoint
}

func installPageDirect(t *testing.T, device *scriptedDevice, layout storagepagelayout.Layout, page int64, bytes []byte) {
	t.Helper()
	span, err := layout.Span(page)
	if err != nil {
		t.Fatal(err)
	}
	width := int(layout.Geometry.BytesPerBlock)
	for ordinal := int64(0); ordinal < span.BlockCount; ordinal++ {
		start := int(ordinal) * width
		device.blocks[span.FirstBlock+ordinal] = append([]byte(nil), bytes[start:start+width]...)
	}
}

func expectedStepWrites(checkpoint storagepagelayout.Layout, destination int64, oldPage []byte, desired storagewalcheckpointrecord.Record) ([][]writeEvent, error) {
	span, err := checkpoint.Span(destination)
	if err != nil {
		return nil, err
	}
	width := int(checkpoint.Geometry.BytesPerBlock)
	guardBlockOffset := storagewalcheckpointslot.GuardOffset / width
	guardByteOffset := storagewalcheckpointslot.GuardOffset % width
	guard := append([]byte(nil), oldPage[guardBlockOffset*width:(guardBlockOffset+1)*width]...)
	guard[guardByteOffset] = storagewalcheckpointslot.StagedGuard
	stageGuard := []writeEvent{{Block: span.FirstBlock + int64(guardBlockOffset), Bytes: append([]byte(nil), guard...)}}
	frame, err := storagewalcheckpointslot.Stage(desired)
	if err != nil {
		return nil, err
	}
	stagedPage := make([]byte, len(oldPage))
	copy(stagedPage, frame)
	var body []writeEvent
	for ordinal := int64(0); ordinal < span.BlockCount; ordinal++ {
		start := int(ordinal) * width
		body = append(body, writeEvent{Block: span.FirstBlock + ordinal, Bytes: append([]byte(nil), stagedPage[start:start+width]...)})
	}
	guard = append([]byte(nil), stagedPage[guardBlockOffset*width:(guardBlockOffset+1)*width]...)
	guard[guardByteOffset] = 0
	publishGuard := []writeEvent{{Block: span.FirstBlock + int64(guardBlockOffset), Bytes: guard}}
	return [][]writeEvent{stageGuard, body, publishGuard}, nil
}

func allWrites(groups ...[]writeEvent) []writeEvent {
	var result []writeEvent
	for _, group := range groups {
		result = append(result, group...)
	}
	return result
}

func TestPublishThreeSteps(t *testing.T) {
	for _, width := range []int64{8, 13} {
		t.Run(fmt.Sprintf("block-width-%d", width), func(t *testing.T) {
			device, log, data, checkpoint := seedSpy(t, width)
			oldDestination, err := readPageMap(device, checkpoint, 1)
			if err != nil {
				t.Fatal(err)
			}
			groups, err := expectedStepWrites(checkpoint, 1, oldDestination, resettingRecord(3, 777))
			if err != nil {
				t.Fatal(err)
			}
			receipt, err := Publish(device, log, data, checkpoint, resettingRecord(3, 777))
			if err != nil {
				t.Fatal(err)
			}
			want := Receipt{Slot: 1, Attempted: PublishGuard, BodyBlocks: checkpoint.BlocksPerPage, Confirmed: true}
			if receipt != want {
				t.Fatalf("receipt=%+v want=%+v", receipt, want)
			}
			if !reflect.DeepEqual(device.writes, allWrites(groups...)) || device.flushes != 3 {
				t.Fatalf("writes/flushes=%+v/%d want=%+v/3", device.writes, device.flushes, allWrites(groups...))
			}
			captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
			if err != nil || captured.Selection != (storagewalcheckpointselection.Selection{Slot: 1, Record: resettingRecord(3, 777)}) {
				t.Fatalf("published capture=(%+v,%v)", captured, err)
			}
			if width == 13 {
				page := captured.Pages[1]
				for _, padding := range page[storagewalcheckpointrecord.FrameBytes:] {
					if padding != 0 {
						t.Fatalf("padded page contains %x", padding)
					}
				}
			}
		})
	}
}

func readPageMap(device blockdevicecontract.Device, layout storagepagelayout.Layout, page int64) ([]byte, error) {
	pager, err := storagepageio.Open(device, layout)
	if err != nil {
		return nil, err
	}
	return pager.ReadPage(page)
}

func TestPublishPreflightAndIdempotence(t *testing.T) {
	device, log, data, checkpoint := seedSpy(t, 8)
	invalid := storagewalcheckpointrecord.Record{Phase: 0, Epoch: 1}
	if receipt, err := Publish(device, log, data, checkpoint, invalid); err != storagewalcheckpointrecord.ErrPhase || receipt != (Receipt{}) {
		t.Fatalf("record preflight=(%+v,%v)", receipt, err)
	}
	if len(device.readBlocks) != 0 || len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("invalid desired record reached device")
	}
	device.readErr[checkpoint.FirstBlock] = errWriteInjected
	if receipt, err := Publish(device, log, data, checkpoint, resettingRecord(3, 9)); err != errWriteInjected || receipt != (Receipt{}) {
		t.Fatalf("capture refusal=(%+v,%v)", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("capture refusal changed device")
	}
	delete(device.readErr, checkpoint.FirstBlock)
	current := readyRecord(2, 9)
	installPageDirect(t, device, checkpoint, 0, pageImage(8, current))
	receipt, err := Publish(device, log, data, checkpoint, current)
	if err != nil || receipt != (Receipt{Slot: 0, Confirmed: true, AlreadyCurrent: true}) {
		t.Fatalf("idempotent result=(%+v,%v)", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("idempotent publication wrote or flushed")
	}

	device, log, data, checkpoint = seedSpy(t, 8)
	for _, desired := range []storagewalcheckpointrecord.Record{
		readyRecord(3, 9), resettingRecord(4, 9),
	} {
		before := len(device.writes)
		receipt, err = Publish(device, log, data, checkpoint, desired)
		if err != ErrCheckpointTransition || receipt != (Receipt{}) {
			t.Fatalf("invalid transition %v => (%+v,%v)", desired, receipt, err)
		}
		if len(device.writes) != before || device.flushes != 0 {
			t.Fatal("transition refusal changed device")
		}
	}

	device, log, data, checkpoint = seedSpy(t, 8)
	installPageDirect(t, device, checkpoint, 0, pageImage(8, resettingRecord(4, 99)))
	installPageDirect(t, device, checkpoint, 1, pageImage(8, readyRecord(3, 99)))
	if receipt, err := Publish(device, log, data, checkpoint, readyRecord(4, 98)); err != ErrCheckpointTransition || receipt != (Receipt{}) {
		t.Fatalf("reset-to-ready changed rank=(%+v,%v)", receipt, err)
	}
	if receipt, err := Publish(device, log, data, checkpoint, readyRecord(4, 99)); err != nil || !receipt.Confirmed || receipt.Slot != 1 {
		t.Fatalf("reset-to-ready publication=(%+v,%v)", receipt, err)
	}
}

func TestPublishWriteRefusalsHaveExactReceipts(t *testing.T) {
	for _, width := range []int64{8, 13} {
		_, _, _, checkpoint := seedSpy(t, width)
		total := int(checkpoint.BlocksPerPage) + 2
		for failedWrite := 1; failedWrite <= total; failedWrite++ {
			t.Run(fmt.Sprintf("width-%d-write-%d", width, failedWrite), func(t *testing.T) {
				device, log, data, checkpoint := seedSpy(t, width)
				device.failWrite = failedWrite
				receipt, err := Publish(device, log, data, checkpoint, resettingRecord(3, 777))
				if err != errWriteInjected {
					t.Fatalf("error=%v", err)
				}
				var want Receipt
				want.Slot = 1
				switch {
				case failedWrite == 1:
					want.Attempted = InvalidateGuard
				case failedWrite <= 1+int(checkpoint.BlocksPerPage):
					want.Attempted = StageBody
					want.BodyBlocks = int64(failedWrite - 2)
				default:
					want.Attempted = PublishGuard
					want.BodyBlocks = checkpoint.BlocksPerPage
				}
				if receipt != want {
					t.Fatalf("receipt=%+v want=%+v", receipt, want)
				}
				wantFlushes := 0
				if failedWrite > 1 {
					wantFlushes = 1
				}
				if failedWrite > 1+int(checkpoint.BlocksPerPage) {
					wantFlushes = 2
				}
				if device.flushes != wantFlushes {
					t.Fatalf("flushes=%d want=%d", device.flushes, wantFlushes)
				}
			})
		}
	}
}

func TestPublishBarrierRefusalsHaveExactReceipts(t *testing.T) {
	for _, width := range []int64{8, 13} {
		for failedFlush := 1; failedFlush <= 3; failedFlush++ {
			t.Run(fmt.Sprintf("width-%d-flush-%d", width, failedFlush), func(t *testing.T) {
				device, log, data, checkpoint := seedSpy(t, width)
				device.failFlush = failedFlush
				receipt, err := Publish(device, log, data, checkpoint, resettingRecord(3, 777))
				if err != errFlushInjected {
					t.Fatalf("error=%v", err)
				}
				want := Receipt{Slot: 1, Attempted: Step(failedFlush)}
				if failedFlush >= 2 {
					want.BodyBlocks = checkpoint.BlocksPerPage
				}
				if receipt != want {
					t.Fatalf("receipt=%+v want=%+v", receipt, want)
				}
				if device.flushes != failedFlush {
					t.Fatalf("flushes=%d want=%d", device.flushes, failedFlush)
				}
			})
		}
	}
}

type barrierCutDevice struct {
	inner *hddblockdevice.HDD
	cuts  []hddwritebackcache.BarrierCut
	seen  int
}

func (d *barrierCutDevice) Geometry() blockdevicecontract.Geometry { return d.inner.Geometry() }
func (d *barrierCutDevice) ReadBlock(block int64) ([]byte, error)  { return d.inner.ReadBlock(block) }
func (d *barrierCutDevice) WriteBlock(block int64, data []byte) error {
	return d.inner.WriteBlock(block, data)
}
func (d *barrierCutDevice) Flush() error {
	if d.seen >= len(d.cuts) {
		return errFlushInjected
	}
	cut := d.cuts[d.seen]
	d.seen++
	receipt, err := d.inner.FlushWithCut(cut)
	if err != nil {
		return err
	}
	if !receipt.Acknowledged {
		return errLostAck
	}
	return nil
}

func seedHDD(t *testing.T) (*hddblockdevice.HDD, storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	t.Helper()
	log, data, checkpoint := testLayouts(8)
	if checkpoint.Geometry != (blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}) {
		t.Fatalf("test HDD geometry=%+v", checkpoint.Geometry)
	}
	device := hddblockdevice.New()
	writePageDirect(t, device, checkpoint, 0, pageImage(8, readyRecord(2, 9)))
	writePageDirect(t, device, checkpoint, 1, pageImage(8, resettingRecord(1, 3)))
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	return device, log, data, checkpoint
}

func durableImage(t *testing.T, device *hddblockdevice.HDD) [hddwritecrashcuts.SectorCount][hddwritecrashcuts.BytesPerSector]byte {
	t.Helper()
	var image [hddwritecrashcuts.SectorCount][hddwritecrashcuts.BytesPerSector]byte
	for block := int64(0); block < hddwritecrashcuts.SectorCount; block++ {
		value, err := device.ReadBlock(block)
		if err != nil {
			t.Fatal(err)
		}
		copy(image[block][:], value)
	}
	return image
}

func applyModelBarrier(image *[hddwritecrashcuts.SectorCount][hddwritecrashcuts.BytesPerSector]byte, writes []writeEvent, cut hddwritebackcache.BarrierCut) {
	if cut.Kind == hddwritebackcache.BeforeFirstSector {
		return
	}
	for ordinal, event := range writes {
		if cut.Kind == hddwritebackcache.AtSector && int64(ordinal) == cut.SectorIndex {
			if cut.MediaCut.Kind == hddwritecrashcuts.DuringMediaWrite {
				prefix := 0
				if cut.MediaCut.WrittenPrefix != nil {
					prefix = *cut.MediaCut.WrittenPrefix
				}
				copy(image[event.Block][:prefix], event.Bytes[:prefix])
			} else {
				copy(image[event.Block][:], event.Bytes)
			}
			return
		}
		copy(image[event.Block][:], event.Bytes)
	}
}

func TestPublishHDDModeledBarrierCuts(t *testing.T) {
	for targetStep := 1; targetStep <= 3; targetStep++ {
		_, _, _, checkpoint := seedHDD(t)
		destPage := pageImage(8, resettingRecord(1, 3))
		groups, err := expectedStepWrites(checkpoint, 1, destPage, resettingRecord(3, 777))
		if err != nil {
			t.Fatal(err)
		}
		stepWrites := groups[targetStep-1]
		cuts := []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BeforeFirstSector}, {Kind: hddwritebackcache.AfterAllMedia}}
		for ordinal := range stepWrites {
			for prefix := 1; prefix < hddwritecrashcuts.BytesPerSector; prefix++ {
				value := prefix
				cuts = append(cuts, hddwritebackcache.BarrierCut{
					Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal),
					MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value},
				})
			}
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})
		}
		for cutIndex, targetCut := range cuts {
			t.Run(fmt.Sprintf("step-%d-cut-%d", targetStep, cutIndex), func(t *testing.T) {
				inner, log, data, checkpoint := seedHDD(t)
				wrapperCuts := []hddwritebackcache.BarrierCut{
					{Kind: hddwritebackcache.BarrierComplete},
					{Kind: hddwritebackcache.BarrierComplete},
					{Kind: hddwritebackcache.BarrierComplete},
				}
				wrapperCuts[targetStep-1] = targetCut
				device := &barrierCutDevice{inner: inner, cuts: wrapperCuts}
				receipt, publishErr := Publish(device, log, data, checkpoint, resettingRecord(3, 777))

				wantImage := durableImage(t, inner)
				// Reconstruct the initial durable image and apply only the exact writes
				// whose barriers precede or equal the selected cut.
				initial, _, _, _ := seedHDD(t)
				wantImage = durableImage(t, initial)
				for step := 0; step < targetStep-1; step++ {
					applyModelBarrier(&wantImage, groups[step], hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
				}
				applyModelBarrier(&wantImage, stepWrites, targetCut)
				gotImage := durableImage(t, inner)
				if gotImage != wantImage {
					t.Fatalf("durable image differs at step %d cut %+v", targetStep, targetCut)
				}
				if targetCut.Kind == hddwritebackcache.BarrierComplete {
					if publishErr != nil || !receipt.Confirmed {
						t.Fatalf("complete path=(%+v,%v)", receipt, publishErr)
					}
					return
				}
				if publishErr != errLostAck || receipt.Confirmed {
					t.Fatalf("cut path=(%+v,%v)", receipt, publishErr)
				}
				restarted, err := inner.Restart()
				if err != nil {
					t.Fatal(err)
				}
				captured, err := storagewalcheckpointcapture.Capture(restarted, log, data, checkpoint)
				if err != nil {
					t.Fatalf("restart capture: %v", err)
				}
				if targetStep < 3 && captured.Selection.Record != readyRecord(2, 9) {
					t.Fatalf("step %d unexpectedly displaced current: %+v", targetStep, captured.Selection)
				}
				if targetStep == 3 {
					guardDurable := false
					if targetCut.Kind == hddwritebackcache.AfterAllMedia || targetCut.Kind == hddwritebackcache.AtSector {
						if targetCut.Kind == hddwritebackcache.AtSector {
							guardPhysical := checkpoint.FirstBlock + checkpoint.BlocksPerPage + int64(storagewalcheckpointslot.GuardOffset/8)
							guardDurable = gotImage[guardPhysical][storagewalcheckpointslot.GuardOffset%8] == 0
						} else {
							guardDurable = true
						}
					}
					if guardDurable {
						if captured.Selection.Record != resettingRecord(3, 777) {
							t.Fatalf("final guard persisted but selection=%+v", captured.Selection)
						}
						retry, retryErr := Publish(restarted, log, data, checkpoint, resettingRecord(3, 777))
						if retryErr != nil || !retry.AlreadyCurrent || !retry.Confirmed || retry.Attempted != NoWrite {
							t.Fatalf("lost-ack retry=(%+v,%v)", retry, retryErr)
						}
					} else if captured.Selection.Record != readyRecord(2, 9) {
						t.Fatalf("partial final guard selected new record: %+v", captured.Selection)
					}
				}
			})
		}
	}
}

func FuzzPublishTransitions(f *testing.F) {
	for _, seed := range [][]byte{{0, 0, 0}, {1, 1, 1}, {2, 9, 4}, {3, 255, 255}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 64 {
			t.Skip()
		}
		width := int64(8)
		if len(input) > 0 && input[0]&1 != 0 {
			width = 13
		}
		device, log, data, checkpoint := seedSpy(t, width)
		current := readyRecord(2, 9)
		kind := byte(0)
		if len(input) > 0 {
			kind = input[0] % 4
		}
		desired := current
		switch kind {
		case 1:
			desired = resettingRecord(3, uint64(byteAt(input, 1)))
		case 2:
			desired = storagewalcheckpointrecord.Record{Phase: 0, Epoch: 3}
		case 3:
			desired = readyRecord(3, 9)
		}
		receipt, err := Publish(device, log, data, checkpoint, desired)
		switch kind {
		case 0:
			if err != nil || !receipt.AlreadyCurrent || !receipt.Confirmed || len(device.writes) != 0 || device.flushes != 0 {
				t.Fatalf("idempotence model mismatch (%+v,%v)", receipt, err)
			}
		case 1:
			if err != nil || receipt.Attempted != PublishGuard || !receipt.Confirmed || receipt.AlreadyCurrent || len(device.writes) != int(checkpoint.BlocksPerPage)+2 || device.flushes != 3 {
				t.Fatalf("success model mismatch (%+v,%v)", receipt, err)
			}
			captured, captureErr := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
			if captureErr != nil || captured.Selection.Slot != 1 || captured.Selection.Record != desired {
				t.Fatalf("model successor mismatch (%+v,%v)", captured.Selection, captureErr)
			}
		case 2:
			if err != storagewalcheckpointrecord.ErrPhase || receipt != (Receipt{}) || len(device.readBlocks) != 0 || len(device.writes) != 0 {
				t.Fatalf("encode refusal model mismatch (%+v,%v)", receipt, err)
			}
		case 3:
			if err != ErrCheckpointTransition || receipt != (Receipt{}) || len(device.writes) != 0 || device.flushes != 0 {
				t.Fatalf("transition refusal model mismatch (%+v,%v)", receipt, err)
			}
		}
	})
}

func byteAt(input []byte, index int) byte {
	if index >= len(input) {
		return 0
	}
	return input[index]
}

func TestPublishHDDCompleteAndIdempotent(t *testing.T) {
	inner, log, data, checkpoint := seedHDD(t)
	device := &barrierCutDevice{inner: inner, cuts: []hddwritebackcache.BarrierCut{
		{Kind: hddwritebackcache.BarrierComplete},
		{Kind: hddwritebackcache.BarrierComplete},
		{Kind: hddwritebackcache.BarrierComplete},
	}}
	desired := resettingRecord(3, 777)
	receipt, err := Publish(device, log, data, checkpoint, desired)
	if err != nil || receipt != (Receipt{Slot: 1, Attempted: PublishGuard, BodyBlocks: checkpoint.BlocksPerPage, Confirmed: true}) {
		t.Fatalf("HDD publication=(%+v,%v)", receipt, err)
	}
	restarted, err := inner.Restart()
	if err != nil {
		t.Fatal(err)
	}
	captured, err := storagewalcheckpointcapture.Capture(restarted, log, data, checkpoint)
	if err != nil || captured.Selection.Record != desired || captured.Selection.Slot != 1 {
		t.Fatalf("HDD restart capture=(%+v,%v)", captured, err)
	}
	retry, err := Publish(restarted, log, data, checkpoint, desired)
	if err != nil || retry != (Receipt{Slot: 1, Confirmed: true, AlreadyCurrent: true}) {
		t.Fatalf("HDD idempotent retry=(%+v,%v)", retry, err)
	}
}
