package storagewalcheckpointpublication

import (
	"errors"
	"fmt"
	"reflect"
	"testing"

	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointselection "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-selection"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
)

func initialRecord() storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1, PreviousLSN: 0}
}

func initialDevice(width int64, slot0 string, slot1 string) (*scriptedDevice, storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	log, data, checkpoint := testLayouts(width)
	device := newScripted(log.Geometry)
	for block := int64(0); block < log.Geometry.BlockCount; block++ {
		device.blocks[block] = make([]byte, int(width))
	}
	install := func(ordinal int64, state string) {
		var page []byte
		switch state {
		case "vacant":
			page = make([]byte, int(checkpoint.BlocksPerPage*width))
		case "staged":
			frame, err := storagewalcheckpointslot.Stage(initialRecord())
			if err != nil {
				panic(err)
			}
			page = make([]byte, int(checkpoint.BlocksPerPage*width))
			copy(page, frame)
		case "initial":
			page = pageImage(width, initialRecord())
		case "other":
			page = pageImage(width, readyRecord(2, 0))
		case "resetting":
			page = pageImage(width, resettingRecord(1, 0))
		case "corrupt":
			page = make([]byte, int(checkpoint.BlocksPerPage*width))
			page[0] = 0x7f
		case "nonzero-padding":
			page = make([]byte, int(checkpoint.BlocksPerPage*width))
			page[storagewalcheckpointrecord.FrameBytes] = 0x5a
		default:
			panic("unknown test slot state")
		}
		installInitialPage(device, checkpoint, ordinal, page)
	}
	install(0, slot0)
	install(1, slot1)
	return device, log, data, checkpoint
}

func installInitialPage(device *scriptedDevice, layout storagepagelayout.Layout, id int64, value []byte) {
	span, err := layout.Span(id)
	if err != nil {
		panic(err)
	}
	width := int(layout.Geometry.BytesPerBlock)
	for ordinal := int64(0); ordinal < span.BlockCount; ordinal++ {
		start := int(ordinal) * width
		device.blocks[span.FirstBlock+ordinal] = append([]byte(nil), value[start:start+width]...)
	}
}

func expectedInitReads(log, data, checkpoint storagepagelayout.Layout) []int64 {
	var result []int64
	for page := int64(0); page < checkpoint.PageCount; page++ {
		span, _ := checkpoint.Span(page)
		for ordinal := int64(0); ordinal < span.BlockCount; ordinal++ {
			result = append(result, span.FirstBlock+ordinal)
		}
	}
	for _, layout := range []storagepagelayout.Layout{log, data} {
		end := layout.FirstBlock + layout.PageCount*layout.BlocksPerPage
		for block := layout.FirstBlock; block < end; block++ {
			result = append(result, block)
		}
	}
	return result
}

func TestInitializeEmptyAndPaddedReservation(t *testing.T) {
	for _, width := range []int64{8, 13} {
		t.Run(fmt.Sprintf("width-%d", width), func(t *testing.T) {
			device, log, data, checkpoint := initialDevice(width, "vacant", "vacant")
			before := copyBlocksForInit(device)
			receipt, err := Initialize(device, log, data, checkpoint)
			want := Receipt{Slot: 0, Attempted: PublishGuard, BodyBlocks: checkpoint.BlocksPerPage, Confirmed: true}
			if err != nil || receipt != want {
				t.Fatalf("Initialize=(%+v,%v) want=%+v", receipt, err, want)
			}
			if !reflect.DeepEqual(device.readBlocks, expectedInitReads(log, data, checkpoint)) {
				t.Fatalf("preflight read trace=%v", device.readBlocks)
			}
			if len(device.writes) != int(checkpoint.BlocksPerPage)+2 || device.flushes != 3 {
				t.Fatalf("writes/flushes=%+v/%d", device.writes, device.flushes)
			}
			groups, err := expectedStepWrites(checkpoint, 0, make([]byte, int(checkpoint.BlocksPerPage*width)), initialRecord())
			if err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(device.writes, allWrites(groups...)) {
				t.Fatalf("write steps=%+v want=%+v", device.writes, allWrites(groups...))
			}
			for block, value := range before {
				if block >= checkpoint.FirstBlock && block < checkpoint.FirstBlock+checkpoint.BlocksPerPage {
					continue
				}
				if !reflect.DeepEqual(device.blocks[block], value) {
					t.Fatalf("non-target block %d changed", block)
				}
			}
			captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
			if err != nil || captured.Selection != (storagewalcheckpointselection.Selection{Slot: 0, Record: initialRecord()}) {
				t.Fatalf("initialized capture=(%+v,%v)", captured, err)
			}
		})
	}
}

func copyBlocksForInit(device *scriptedDevice) map[int64][]byte {
	result := make(map[int64][]byte, len(device.blocks))
	for block, value := range device.blocks {
		result[block] = append([]byte(nil), value...)
	}
	return result
}

func TestInitializeAcceptsVacantStagedAndExistingInitialSlotZero(t *testing.T) {
	for _, slot0 := range []string{"vacant", "staged", "initial"} {
		t.Run(slot0, func(t *testing.T) {
			device, log, data, checkpoint := initialDevice(8, slot0, "vacant")
			receipt, err := Initialize(device, log, data, checkpoint)
			if err != nil {
				t.Fatal(err)
			}
			if slot0 == "initial" {
				if receipt != (Receipt{Slot: 0, Confirmed: true, AlreadyCurrent: true}) || len(device.writes) != 0 || device.flushes != 0 {
					t.Fatalf("idempotent=(%+v,%v), writes=%v flush=%d", receipt, err, device.writes, device.flushes)
				}
			} else if !receipt.Confirmed || receipt.Slot != 0 || receipt.Attempted != PublishGuard {
				t.Fatalf("publish staged/vacant=(%+v,%v)", receipt, err)
			}
			captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
			if err != nil || captured.Selection.Record != initialRecord() {
				t.Fatalf("selected=(%+v,%v)", captured.Selection, err)
			}
		})
	}
}

func TestInitializeInspectsBothSlotsBeforeEligibility(t *testing.T) {
	device, log, data, checkpoint := initialDevice(8, "other", "corrupt")
	pages, err := storagewalcheckpointcapture.ReadPages(device, log, data, checkpoint)
	if err != nil {
		t.Fatal(err)
	}
	_, _, expectedErr := storagewalcheckpointslot.Inspect(pages[1][:storagewalcheckpointrecord.FrameBytes])
	if expectedErr == nil {
		t.Fatal("corrupt slot one unexpectedly valid")
	}
	device.readBlocks = nil
	receipt, err := Initialize(device, log, data, checkpoint)
	if err != expectedErr || receipt != (Receipt{}) {
		t.Fatalf("inspection order=(%+v,%v), want err %v", receipt, err, expectedErr)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("slot inspection refusal had effects")
	}
}

func TestInitializeRejectsCheckpointPadding(t *testing.T) {
	device, log, data, checkpoint := initialDevice(13, "vacant", "vacant")
	span, err := checkpoint.Span(0)
	if err != nil {
		t.Fatal(err)
	}
	paddingOffset := storagewalcheckpointrecord.FrameBytes
	block := span.FirstBlock + int64(paddingOffset/int(checkpoint.Geometry.BytesPerBlock))
	byteOffset := paddingOffset % int(checkpoint.Geometry.BytesPerBlock)
	device.blocks[block][byteOffset] = 0x44
	receipt, err := Initialize(device, log, data, checkpoint)
	if !errors.Is(err, storagewalcheckpointcapture.ErrCheckpointPadding) || receipt != (Receipt{}) {
		t.Fatalf("Initialize=(%+v,%v), want checkpoint padding refusal", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("checkpoint padding refusal had effects")
	}
}

func TestInitializeRefusesOccupiedOrMalformedSlots(t *testing.T) {
	cases := []struct {
		slot0, slot1 string
		want         error
	}{
		{"other", "vacant", ErrCheckpointOccupied},
		{"resetting", "vacant", ErrCheckpointOccupied},
		{"vacant", "staged", ErrCheckpointOccupied},
		{"vacant", "initial", ErrCheckpointOccupied},
		{"vacant", "other", ErrCheckpointOccupied},
	}
	for index, tc := range cases {
		t.Run(fmt.Sprintf("case-%d", index), func(t *testing.T) {
			device, log, data, checkpoint := initialDevice(8, tc.slot0, tc.slot1)
			before := copyBlocksForInit(device)
			receipt, err := Initialize(device, log, data, checkpoint)
			if err != tc.want || receipt != (Receipt{}) {
				t.Fatalf("Initialize=(%+v,%v), want %v", receipt, err, tc.want)
			}
			if len(device.writes) != 0 || device.flushes != 0 || !reflect.DeepEqual(device.blocks, before) {
				t.Fatal("slot refusal had effects")
			}
		})
	}
	device, log, data, checkpoint := initialDevice(8, "corrupt", "vacant")
	pages, _ := storagewalcheckpointcapture.ReadPages(device, log, data, checkpoint)
	_, _, expectedErr := storagewalcheckpointslot.Inspect(pages[0][:storagewalcheckpointrecord.FrameBytes])
	device.readBlocks = nil
	if receipt, err := Initialize(device, log, data, checkpoint); err != expectedErr || receipt != (Receipt{}) {
		t.Fatalf("corrupt slot0=(%+v,%v), want %v", receipt, err, expectedErr)
	}
}

func TestInitializeRefusesOccupiedAlternateSlot(t *testing.T) {
	device, log, data, checkpoint := initialDevice(8, "vacant", "staged")
	before := copyBlocksForInit(device)
	receipt, err := Initialize(device, log, data, checkpoint)
	if err != ErrCheckpointOccupied || receipt != (Receipt{}) {
		t.Fatalf("Initialize=(%+v,%v), want occupied refusal", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 || !reflect.DeepEqual(device.blocks, before) {
		t.Fatal("occupied alternate slot refusal had effects")
	}
}

func TestInitializeRefusesNonemptyData(t *testing.T) {
	device, log, data, checkpoint := initialDevice(13, "vacant", "vacant")
	device.blocks[data.FirstBlock+data.BlocksPerPage-1][12] = 0xa7
	before := copyBlocksForInit(device)
	receipt, err := Initialize(device, log, data, checkpoint)
	if err != ErrInitialDataNonempty || receipt != (Receipt{}) {
		t.Fatalf("Initialize=(%+v,%v), want nonempty data refusal", receipt, err)
	}
	if !reflect.DeepEqual(device.readBlocks, expectedInitReads(log, data, checkpoint)) {
		t.Fatalf("data reservation read trace=%v", device.readBlocks)
	}
	if len(device.writes) != 0 || device.flushes != 0 || !reflect.DeepEqual(device.blocks, before) {
		t.Fatal("nonempty data refusal had effects")
	}
}

func TestInitializeIdempotenceStillChecksReservations(t *testing.T) {
	device, log, data, checkpoint := initialDevice(8, "initial", "vacant")
	device.blocks[data.FirstBlock][0] = 0x01
	receipt, err := Initialize(device, log, data, checkpoint)
	if err != ErrInitialDataNonempty || receipt != (Receipt{}) {
		t.Fatalf("Initialize=(%+v,%v), want data refusal before idempotence", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("nonempty data idempotence refusal had effects")
	}
}

func TestInitializeChecksLogThenDataBytesAndErrors(t *testing.T) {
	device, log, data, checkpoint := initialDevice(13, "vacant", "vacant")
	device.blocks[log.FirstBlock+log.PageCount*log.BlocksPerPage-1][12] = 0x9a
	device.blocks[data.FirstBlock][0] = 0xa5
	beforeReads := len(device.readBlocks)
	if receipt, err := Initialize(device, log, data, checkpoint); err != ErrInitialLogNonempty || receipt != (Receipt{}) {
		t.Fatalf("log nonempty=(%+v,%v)", receipt, err)
	}
	wantLogReads := expectedInitReads(log, storagepagelayout.Layout{}, checkpoint)
	if got := device.readBlocks[beforeReads:]; !reflect.DeepEqual(got, wantLogReads) {
		t.Fatalf("log refusal trace=%v want=%v", got, wantLogReads)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("nonempty log refusal had effects")
	}

	device, log, data, checkpoint = initialDevice(13, "vacant", "vacant")
	device.blocks[data.FirstBlock][12] = 0x55
	if receipt, err := Initialize(device, log, data, checkpoint); err != ErrInitialDataNonempty || receipt != (Receipt{}) {
		t.Fatalf("data nonempty=(%+v,%v)", receipt, err)
	}
	wantReads := expectedInitReads(log, data, checkpoint)
	if !reflect.DeepEqual(device.readBlocks, wantReads) {
		t.Fatalf("data read trace=%v want=%v", device.readBlocks, wantReads)
	}

	device, log, data, checkpoint = initialDevice(8, "vacant", "vacant")
	device.readErr[log.FirstBlock+1] = errWriteInjected
	if receipt, err := Initialize(device, log, data, checkpoint); err != errWriteInjected || receipt != (Receipt{}) {
		t.Fatalf("log read refusal=(%+v,%v)", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("read refusal had effects")
	}
	device, log, data, checkpoint = initialDevice(8, "vacant", "vacant")
	device.readErr[data.FirstBlock] = errWriteInjected
	if receipt, err := Initialize(device, log, data, checkpoint); err != errWriteInjected || receipt != (Receipt{}) {
		t.Fatalf("data read refusal=(%+v,%v)", receipt, err)
	}
	device, log, data, checkpoint = initialDevice(8, "vacant", "vacant")
	device.badWidth[log.FirstBlock] = true
	if receipt, err := Initialize(device, log, data, checkpoint); !errors.Is(err, storagepageio.ErrBlockWidthMismatch) || receipt != (Receipt{}) {
		t.Fatalf("log width refusal=(%+v,%v)", receipt, err)
	}
	device, log, data, checkpoint = initialDevice(8, "vacant", "vacant")
	device.badWidth[data.FirstBlock] = true
	if receipt, err := Initialize(device, log, data, checkpoint); !errors.Is(err, storagepageio.ErrBlockWidthMismatch) || receipt != (Receipt{}) {
		t.Fatalf("data width refusal=(%+v,%v)", receipt, err)
	}
}

func TestInitializeWriteAndBarrierReceipts(t *testing.T) {
	_, _, _, checkpoint := initialDevice(13, "vacant", "vacant")
	total := int(checkpoint.BlocksPerPage) + 2
	for failed := 1; failed <= total; failed++ {
		t.Run(fmt.Sprintf("write-%d", failed), func(t *testing.T) {
			device, log, data, checkpoint := initialDevice(13, "vacant", "vacant")
			device.failWrite = failed
			receipt, err := Initialize(device, log, data, checkpoint)
			if err != errWriteInjected {
				t.Fatalf("error=%v", err)
			}
			want := Receipt{Slot: 0}
			switch {
			case failed == 1:
				want.Attempted = InvalidateGuard
			case failed <= 1+int(checkpoint.BlocksPerPage):
				want.Attempted = StageBody
				want.BodyBlocks = int64(failed - 2)
			default:
				want.Attempted = PublishGuard
				want.BodyBlocks = checkpoint.BlocksPerPage
			}
			if receipt != want {
				t.Fatalf("receipt=%+v want=%+v", receipt, want)
			}
		})
	}
	for failed := 1; failed <= 3; failed++ {
		t.Run(fmt.Sprintf("flush-%d", failed), func(t *testing.T) {
			device, log, data, checkpoint := initialDevice(8, "vacant", "vacant")
			device.failFlush = failed
			receipt, err := Initialize(device, log, data, checkpoint)
			if err != errFlushInjected {
				t.Fatalf("error=%v", err)
			}
			want := Receipt{Slot: 0, Attempted: Step(failed)}
			if failed >= 2 {
				want.BodyBlocks = checkpoint.BlocksPerPage
			}
			if receipt != want {
				t.Fatalf("receipt=%+v want=%+v", receipt, want)
			}
		})
	}
}

func initializeCutCases(blocks int) []hddwritebackcache.BarrierCut {
	cuts := []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BeforeFirstSector}, {Kind: hddwritebackcache.AfterAllMedia}}
	for ordinal := 0; ordinal < blocks; ordinal++ {
		for prefix := 1; prefix < hddwritecrashcuts.BytesPerSector; prefix++ {
			value := prefix
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value}})
		}
		cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})
	}
	return cuts
}

func TestInitializeHDDCutsRestartAndRetry(t *testing.T) {
	log, data, checkpoint := testLayouts(8)
	groups, err := expectedStepWrites(checkpoint, 0, make([]byte, int(checkpoint.BlocksPerPage*8)), initialRecord())
	if err != nil {
		t.Fatal(err)
	}
	for targetStep := 1; targetStep <= 3; targetStep++ {
		writeCount := 1
		if targetStep == 2 {
			writeCount = int(checkpoint.BlocksPerPage)
		}
		for cutIndex, cut := range initializeCutCases(writeCount) {
			t.Run(fmt.Sprintf("step-%d-cut-%d", targetStep, cutIndex), func(t *testing.T) {
				inner := hddblockdevice.New()
				before := durableImage(t, inner)
				cuts := []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BarrierComplete}, {Kind: hddwritebackcache.BarrierComplete}, {Kind: hddwritebackcache.BarrierComplete}}
				cuts[targetStep-1] = cut
				device := &barrierCutDevice{inner: inner, cuts: cuts}
				receipt, initErr := Initialize(device, log, data, checkpoint)
				want := before
				for step := 0; step < targetStep-1; step++ {
					applyModelBarrier(&want, groups[step], hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
				}
				applyModelBarrier(&want, groups[targetStep-1], cut)
				if got := durableImage(t, inner); got != want {
					t.Fatalf("durable HDD image differs at step %d cut %+v", targetStep, cut)
				}
				if initErr != errLostAck || receipt.Confirmed {
					t.Fatalf("cut result=(%+v,%v)", receipt, initErr)
				}
				restarted, err := inner.Restart()
				if err != nil {
					t.Fatal(err)
				}
				captured, err := storagewalcheckpointcapture.Capture(restarted, log, data, checkpoint)
				if err != nil && !errors.Is(err, storagewalcheckpointselection.ErrNoPublishedCheckpoint) {
					t.Fatalf("restart capture: %v", err)
				}
				if err == nil && captured.Selection.Record != initialRecord() {
					t.Fatalf("restart selected unexpected record %+v", captured.Selection)
				}
				retry, err := Initialize(restarted, log, data, checkpoint)
				if err != nil || !retry.Confirmed {
					t.Fatalf("Initialize retry=(%+v,%v)", retry, err)
				}
				alreadyCurrent := false
				if targetStep == 3 {
					if cut.Kind == hddwritebackcache.AfterAllMedia || cut.Kind == hddwritebackcache.AtSector && cut.MediaCut.Kind == hddwritecrashcuts.AfterMediaWriteBeforeResponse {
						alreadyCurrent = true
					} else if cut.Kind == hddwritebackcache.AtSector && cut.SectorIndex == 0 {
						guardWithinSector := storagewalcheckpointslot.GuardOffset % int64(hddwritecrashcuts.BytesPerSector)
						alreadyCurrent = cut.MediaCut.Kind == hddwritecrashcuts.DuringMediaWrite && cut.MediaCut.WrittenPrefix != nil && int64(*cut.MediaCut.WrittenPrefix) > guardWithinSector
					}
				}
				wantAttempt := PublishGuard
				if alreadyCurrent {
					wantAttempt = NoWrite
				}
				if retry.AlreadyCurrent != alreadyCurrent || retry.Attempted != wantAttempt {
					t.Fatalf("retry=(%+v,%v), already-current=%t", retry, err, alreadyCurrent)
				}
			})
		}
	}
}

func FuzzInitializeFreshReservations(f *testing.F) {
	f.Add([]byte{0, 0, 0})
	f.Add([]byte{1, 0, 0})
	f.Add([]byte{0, 1, 0})
	f.Add([]byte{0, 2, 0})
	f.Add([]byte{0, 0, 1})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 32 {
			t.Skip()
		}
		width := int64(8)
		if len(input) > 0 && input[0]&0x80 != 0 {
			width = 13
		}
		slot0 := "vacant"
		mode := byteAt(input, 1) % 3
		if mode == 1 {
			slot0 = "staged"
		}
		if mode == 2 {
			slot0 = "initial"
		}
		device, log, data, checkpoint := initialDevice(width, slot0, "vacant")
		region := byteAt(input, 2) % 3
		if region == 1 {
			device.blocks[log.FirstBlock][0] = 1
		}
		if region == 2 {
			device.blocks[data.FirstBlock][0] = 1
		}
		receipt, err := Initialize(device, log, data, checkpoint)
		if region == 1 {
			if err != ErrInitialLogNonempty || receipt != (Receipt{}) || len(device.writes) != 0 {
				t.Fatalf("log preflight model=(%+v,%v)", receipt, err)
			}
			return
		}
		if region == 2 {
			if err != ErrInitialDataNonempty || receipt != (Receipt{}) || len(device.writes) != 0 {
				t.Fatalf("data preflight model=(%+v,%v)", receipt, err)
			}
			return
		}
		if slot0 == "initial" {
			if err != nil || receipt != (Receipt{Slot: 0, Confirmed: true, AlreadyCurrent: true}) || len(device.writes) != 0 {
				t.Fatalf("idempotent model=(%+v,%v)", receipt, err)
			}
			return
		}
		if err != nil || !receipt.Confirmed || receipt.Slot != 0 || receipt.Attempted != PublishGuard || len(device.writes) != int(checkpoint.BlocksPerPage)+2 || device.flushes != 3 {
			t.Fatalf("publication model=(%+v,%v)", receipt, err)
		}
	})
}
