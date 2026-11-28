package storagewalcheckpointadvance

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
	storagewalappend "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-append"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointclear "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-clear"
	storagewalcheckpointpublication "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-publication"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalrecovery "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-recovery"
	storagewalredoapply "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-apply"
)

var (
	errWrite = errors.New("injected write refusal")
	errFlush = errors.New("injected flush refusal")
	errLost  = errors.New("injected lost acknowledgement")
)

type tracedDevice struct {
	inner       blockdevicecontract.Device
	reads       []int64
	writes      []int64
	flushes     int
	failWriteAt int
	failFlushAt int
	cutAt       int
	cut         hddwritebackcache.BarrierCut
}

func (device *tracedDevice) Geometry() blockdevicecontract.Geometry { return device.inner.Geometry() }
func (device *tracedDevice) ReadBlock(block int64) ([]byte, error) {
	device.reads = append(device.reads, block)
	return device.inner.ReadBlock(block)
}
func (device *tracedDevice) WriteBlock(block int64, data []byte) error {
	device.writes = append(device.writes, block)
	if device.failWriteAt != 0 && len(device.writes) == device.failWriteAt {
		return errWrite
	}
	return device.inner.WriteBlock(block, data)
}
func (device *tracedDevice) Flush() error {
	device.flushes++
	if device.failFlushAt != 0 && device.flushes == device.failFlushAt {
		return errFlush
	}
	if device.cutAt != 0 && device.flushes == device.cutAt {
		hdd, ok := device.inner.(*hddblockdevice.HDD)
		if !ok {
			return errors.New("barrier cuts require HDD backend")
		}
		receipt, err := hdd.FlushWithCut(device.cut)
		if err != nil {
			return err
		}
		if !receipt.Acknowledged {
			return errLost
		}
		return nil
	}
	return device.inner.Flush()
}

func layouts() (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	log := storagepagelayout.Layout{Geometry: geometry, FirstBlock: 0, PageCount: 3, BlocksPerPage: 7}
	data := storagepagelayout.Layout{Geometry: geometry, FirstBlock: 21, PageCount: 1, BlocksPerPage: 1}
	checkpoint := storagepagelayout.Layout{Geometry: geometry, FirstBlock: 22, PageCount: 2, BlocksPerPage: 5}
	return log, data, checkpoint
}

func cpRecord(phase storagewalcheckpointrecord.Phase, epoch, previous uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: phase, Epoch: epoch, PreviousLSN: previous}
}

func makePage(t *testing.T, record storagewalcheckpointrecord.Record) []byte {
	t.Helper()
	frame, err := storagewalcheckpointslot.Stage(record)
	if err != nil {
		t.Fatal(err)
	}
	page := make([]byte, 40)
	copy(page, frame)
	page[storagewalcheckpointslot.GuardOffset] = 0
	return page
}

func writePage(t *testing.T, device blockdevicecontract.Device, layout storagepagelayout.Layout, index int64, value []byte) {
	t.Helper()
	pager, err := storagepageio.Open(device, layout)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := pager.WritePage(index, value); err != nil {
		t.Fatal(err)
	}
}

func seedCheckpoint(t *testing.T, device blockdevicecontract.Device, checkpoint storagepagelayout.Layout, slot0, slot1 storagewalcheckpointrecord.Record) {
	t.Helper()
	for index, record := range [2]storagewalcheckpointrecord.Record{slot0, slot1} {
		page := make([]byte, int(checkpoint.BlocksPerPage*checkpoint.Geometry.BytesPerBlock))
		if record != (storagewalcheckpointrecord.Record{}) {
			page = makePage(t, record)
		}
		writePage(t, device, checkpoint, int64(index), page)
	}
}

func logRecords(image []byte) []storagewalrecord.Record {
	return []storagewalrecord.Record{
		{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 7},
		{Kind: storagewalrecord.WritePageImage, LSN: 2, TxID: 7, PageID: 0, Image: append([]byte(nil), image...)},
		{Kind: storagewalrecord.CommitTransaction, LSN: 3, TxID: 7},
	}
}

func seedLog(t *testing.T, device blockdevicecontract.Device, log storagepagelayout.Layout, records []storagewalrecord.Record) {
	t.Helper()
	pager, err := storagepageio.Open(device, log)
	if err != nil {
		t.Fatal(err)
	}
	pageBytes := int(log.BlocksPerPage * log.Geometry.BytesPerBlock)
	for index, record := range records {
		page, err := storagewalpageslot.EncodePage(int64(pageBytes), record)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := pager.WritePage(int64(index), page); err != nil {
			t.Fatal(err)
		}
	}
}

func seedInitialHDD(t *testing.T, resetting bool, payload []byte) (*hddblockdevice.HDD, storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	t.Helper()
	log, data, checkpoint := layouts()
	device := hddblockdevice.New()
	seedLog(t, device, log, logRecords(payload))
	ready := cpRecord(storagewalcheckpointrecord.Ready, 1, 0)
	if resetting {
		seedCheckpoint(t, device, checkpoint, ready, cpRecord(storagewalcheckpointrecord.Resetting, 2, 3))
	} else {
		seedCheckpoint(t, device, checkpoint, ready, storagewalcheckpointrecord.Record{})
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	return device, log, data, checkpoint
}

func blockSnapshot(t *testing.T, device blockdevicecontract.Device) [][]byte {
	t.Helper()
	geometry := device.Geometry()
	result := make([][]byte, geometry.BlockCount)
	for block := int64(0); block < geometry.BlockCount; block++ {
		value, err := device.ReadBlock(block)
		if err != nil {
			t.Fatal(err)
		}
		result[block] = append([]byte(nil), value...)
	}
	return result
}

func TestAdvanceMemoryPublishesResettingClearsAndPublishesReady(t *testing.T) {
	log, data, checkpoint := layouts()
	device, err := memoryblockdevice.New(log.Geometry)
	if err != nil {
		t.Fatal(err)
	}
	image := []byte{91, 82, 73, 64, 55, 46, 37, 28}
	seedLog(t, device, log, logRecords(image))
	seedCheckpoint(t, device, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{})
	trace := &tracedDevice{inner: device}
	report, err := Advance(trace, log, data, checkpoint, 2)
	if err != nil {
		t.Fatal(err)
	}
	wantObserved := cpRecord(storagewalcheckpointrecord.Ready, 1, 0)
	wantReplay := storagewalrecovery.Result{Replay: storagewalredoapply.Receipt{AcknowledgedWrites: 1}, LastLSN: 3}
	if report.Observed != wantObserved || report.Replay != wantReplay {
		t.Fatalf("observed/replay=(%+v,%+v)", report.Observed, report.Replay)
	}
	if !report.Resetting.Confirmed || report.Resetting.Slot != 1 || !report.Clear.Acknowledged || report.Clear.Checkpoint != cpRecord(storagewalcheckpointrecord.Resetting, 2, 3) || report.Clear.BlocksWritten != 21 {
		t.Fatalf("reset/clear receipts=(%+v,%+v)", report.Resetting, report.Clear)
	}
	if !report.Ready.Confirmed || report.Ready.Slot != 0 || report.Ready.AlreadyCurrent {
		t.Fatalf("ready receipt=%+v", report.Ready)
	}
	for block := int64(0); block < 21; block++ {
		value, err := trace.ReadBlock(block)
		if err != nil || !reflect.DeepEqual(value, make([]byte, 8)) {
			t.Fatalf("cleared log block %d=%v err=%v", block, value, err)
		}
	}
	value, err := trace.ReadBlock(data.FirstBlock)
	if err != nil || !reflect.DeepEqual(value, image) {
		t.Fatalf("replayed data=%v err=%v", value, err)
	}
	captured, err := storagewalcheckpointcapture.Capture(trace, log, data, checkpoint)
	if err != nil || captured.Selection.Record != cpRecord(storagewalcheckpointrecord.Ready, 2, 3) || captured.Selection.Slot != 0 {
		t.Fatalf("final checkpoint=(%+v,%v)", captured.Selection, err)
	}
}

func TestAdvanceReadyTargetIsNoWriteAndTargetOrderPrecedesReplay(t *testing.T) {
	log, data, checkpoint := layouts()
	image := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	device := &tracedDevice{inner: memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 2, 5), storagewalcheckpointrecord.Record{}, logRecords(image))}
	report, err := Advance(device, log, data, checkpoint, 2)
	want := Report{Observed: cpRecord(storagewalcheckpointrecord.Ready, 2, 5), Ready: storagewalcheckpointpublication.Receipt{Slot: 0, Confirmed: true, AlreadyCurrent: true}}
	if err != nil || report != want || len(device.writes) != 0 || device.flushes != 0 {
		t.Fatalf("current target=(%+v,%v) writes=%v flushes=%d", report, err, device.writes, device.flushes)
	}

	device = &tracedDevice{inner: memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{}, logRecords(image))}
	readsBefore := len(device.reads)
	report, err = Advance(device, log, data, checkpoint, 3)
	if err != ErrCheckpointOrder || report != (Report{}) || len(device.writes) != 0 || device.flushes != 0 {
		t.Fatalf("skipped epoch=(%+v,%v)", report, err)
	}
	if len(device.reads)-readsBefore != int(2*checkpoint.BlocksPerPage) {
		t.Fatalf("order refusal read log before checkpoint order: reads=%v", device.reads)
	}
}

func memoryDeviceWithCheckpoint(t *testing.T, log, checkpoint storagepagelayout.Layout, slot0, slot1 storagewalcheckpointrecord.Record, records []storagewalrecord.Record) *memoryblockdevice.Memory {
	t.Helper()
	device, err := memoryblockdevice.New(log.Geometry)
	if err != nil {
		t.Fatal(err)
	}
	seedLog(t, device, log, records)
	seedCheckpoint(t, device, checkpoint, slot0, slot1)
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	return device
}

func TestAdvanceSelectedResettingResumesWithoutReplay(t *testing.T) {
	log, data, checkpoint := layouts()
	oldPayload := []byte{10, 20, 30, 40, 50, 60, 70, 80}
	newPayload := []byte{80, 70, 60, 50, 40, 30, 20, 10}
	device, err := memoryblockdevice.New(log.Geometry)
	if err != nil {
		t.Fatal(err)
	}
	seedLog(t, device, log, logRecords(newPayload))
	seedCheckpoint(t, device, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), cpRecord(storagewalcheckpointrecord.Resetting, 2, 3))
	if err := device.WriteBlock(data.FirstBlock, oldPayload); err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	trace := &tracedDevice{inner: device}
	startWrites, startFlush := len(trace.writes), trace.flushes
	report, err := Advance(trace, log, data, checkpoint, 2)
	if err != nil {
		t.Fatal(err)
	}
	if report.Observed != cpRecord(storagewalcheckpointrecord.Resetting, 2, 3) || report.Replay != (storagewalrecovery.Result{}) || report.Resetting != (storagewalcheckpointpublication.Receipt{}) {
		t.Fatalf("resetting resume report=%+v", report)
	}
	value, err := trace.ReadBlock(data.FirstBlock)
	if err != nil || !reflect.DeepEqual(value, oldPayload) {
		t.Fatalf("resetting resume replayed old WAL: %v err=%v", value, err)
	}
	if len(trace.writes)-startWrites != int(log.PageCount*log.BlocksPerPage)+int(checkpoint.BlocksPerPage)+2 || trace.flushes-startFlush != 4 {
		t.Fatalf("resetting path effects writes=%d flushes=%d", len(trace.writes)-startWrites, trace.flushes-startFlush)
	}
}

func TestAdvanceRejectsTargetAndCaptureErrorsBeforeEffects(t *testing.T) {
	log, data, checkpoint := layouts()
	device := &tracedDevice{inner: memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{}, nil)}
	for _, target := range []uint64{0, maxPortableEpoch + 1} {
		beforeReads := len(device.reads)
		report, err := Advance(device, log, data, checkpoint, target)
		if err != ErrTargetEpoch || report != (Report{}) || len(device.reads) != beforeReads || len(device.writes) != 0 {
			t.Fatalf("target %d=(%+v,%v) reads=%v", target, report, err, device.reads)
		}
	}

	badDevice := memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{}, nil)
	if err := badDevice.WriteBlock(checkpoint.FirstBlock, []byte{1, 2, 3, 4, 5, 6, 7, 8}); err != nil {
		t.Fatal(err)
	}
	if err := badDevice.Flush(); err != nil {
		t.Fatal(err)
	}
	trace := &tracedDevice{inner: badDevice}
	report, err := Advance(trace, log, data, checkpoint, 2)
	if err == nil || report != (Report{}) || len(trace.writes) != 0 || trace.flushes != 0 {
		t.Fatalf("capture refusal=(%+v,%v)", report, err)
	}

	resetting := memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), cpRecord(storagewalcheckpointrecord.Resetting, 2, 3), nil)
	resettingTrace := &tracedDevice{inner: resetting}
	readsBefore := len(resettingTrace.reads)
	resettingReport, resettingErr := Advance(resettingTrace, log, data, checkpoint, 3)
	if resettingErr != ErrCheckpointOrder || resettingReport != (Report{}) || len(resettingTrace.writes) != 0 || resettingTrace.flushes != 0 || len(resettingTrace.reads)-readsBefore != int(2*checkpoint.BlocksPerPage) {
		t.Fatalf("wrong Resetting target=(%+v,%v), reads=%v", resettingReport, resettingErr, resettingTrace.reads)
	}
}

func TestAdvanceReplayErrorRetainsObservedAndPartialReplay(t *testing.T) {
	log, data, checkpoint := layouts()
	image := []byte{3, 6, 9, 12, 15, 18, 21, 24}
	device := memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{}, logRecords(image))
	trace := &tracedDevice{inner: device, failWriteAt: 1}
	report, err := Advance(trace, log, data, checkpoint, 2)
	wantReplay := storagewalredoapply.Receipt{Interrupted: true, PageID: 0, RecordLSN: 2, WrittenBlocks: 0}
	if err != errWrite || report.Observed != cpRecord(storagewalcheckpointrecord.Ready, 1, 0) || report.Replay != (storagewalrecovery.Result{Replay: wantReplay}) || report.Resetting != (storagewalcheckpointpublication.Receipt{}) || report.Clear != (storagewalcheckpointclear.Receipt{}) || report.Ready != (storagewalcheckpointpublication.Receipt{}) {
		t.Fatalf("replay refusal=(%+v,%v)", report, err)
	}
}

func TestAdvanceDamagedLogPreservesTypedReplayRefusal(t *testing.T) {
	log, data, checkpoint := layouts()
	device, err := memoryblockdevice.New(log.Geometry)
	if err != nil {
		t.Fatal(err)
	}
	badPage, err := storagewalpageslot.EncodePage(56, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 3})
	if err != nil {
		t.Fatal(err)
	}
	badPage[len(badPage)-1] = 0x01
	writePage(t, device, log, 0, badPage)
	seedCheckpoint(t, device, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{})
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	trace := &tracedDevice{inner: device}
	report, err := Advance(trace, log, data, checkpoint, 2)
	var damaged *storagewalappend.DamagedTailError
	if !errors.As(err, &damaged) || !errors.Is(err, storagewalappend.ErrDamagedTail) || report.Observed != cpRecord(storagewalcheckpointrecord.Ready, 1, 0) || report.Replay != (storagewalrecovery.Result{}) || report.Resetting != (storagewalcheckpointpublication.Receipt{}) || report.Clear != (storagewalcheckpointclear.Receipt{}) || report.Ready != (storagewalcheckpointpublication.Receipt{}) || len(trace.writes) != 0 {
		t.Fatalf("damaged recovery=(%+v,%v) writes=%v", report, err, trace.writes)
	}
}

func TestAdvanceEffectRefusalsRetainThroughTheFailedStep(t *testing.T) {
	log, data, checkpoint := layouts()
	for failFlush := 1; failFlush <= 8; failFlush++ {
		t.Run(fmt.Sprintf("flush-%d", failFlush), func(t *testing.T) {
			inner := memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{}, logRecords([]byte{1, 2, 3, 4, 5, 6, 7, 8}))
			trace := &tracedDevice{inner: inner, failFlushAt: failFlush}
			report, err := Advance(trace, log, data, checkpoint, 2)
			if err != errFlush {
				t.Fatalf("Advance error=%v", err)
			}
			if report.Observed != cpRecord(storagewalcheckpointrecord.Ready, 1, 0) {
				t.Fatalf("observed=%+v", report.Observed)
			}
			if failFlush == 1 {
				wantReplay := storagewalrecovery.Result{Replay: storagewalredoapply.Receipt{Interrupted: true, PageID: 0, RecordLSN: 2, WrittenBlocks: 1}}
				if report.Replay != wantReplay || report.Resetting != (storagewalcheckpointpublication.Receipt{}) || report.Clear != (storagewalcheckpointclear.Receipt{}) || report.Ready != (storagewalcheckpointpublication.Receipt{}) {
					t.Fatalf("replay failure receipts=%+v", report)
				}
			} else if report.Replay != (storagewalrecovery.Result{Replay: storagewalredoapply.Receipt{AcknowledgedWrites: 1}, LastLSN: 3}) {
				t.Fatalf("successful replay=%+v", report.Replay)
			}
			if failFlush >= 2 {
				if failFlush == 2 {
					if report.Resetting.Attempted != storagewalcheckpointpublication.InvalidateGuard || report.Resetting.Confirmed {
						t.Fatalf("resetting guard receipt=%+v", report.Resetting)
					}
				} else if failFlush == 3 {
					if report.Resetting.Attempted != storagewalcheckpointpublication.StageBody || report.Resetting.BodyBlocks != 5 || report.Resetting.Confirmed {
						t.Fatalf("resetting body receipt=%+v", report.Resetting)
					}
				} else if failFlush == 4 {
					if report.Resetting.Attempted != storagewalcheckpointpublication.PublishGuard || report.Resetting.BodyBlocks != 5 || report.Resetting.Confirmed {
						t.Fatalf("resetting final-guard receipt=%+v", report.Resetting)
					}
				} else if failFlush == 5 {
					if !report.Resetting.Confirmed || report.Clear.Acknowledged || report.Clear.BlocksWritten != 21 || report.Ready != (storagewalcheckpointpublication.Receipt{}) {
						t.Fatalf("clear receipt path=%+v", report)
					}
				} else {
					wantStep := storagewalcheckpointpublication.InvalidateGuard
					wantBodyBlocks := int64(0)
					if failFlush == 7 {
						wantStep, wantBodyBlocks = storagewalcheckpointpublication.StageBody, 5
					} else if failFlush == 8 {
						wantStep, wantBodyBlocks = storagewalcheckpointpublication.PublishGuard, 5
					}
					if !report.Resetting.Confirmed || !report.Clear.Acknowledged || report.Ready.Attempted != wantStep || report.Ready.BodyBlocks != wantBodyBlocks || report.Ready.Confirmed {
						t.Fatalf("ready receipt path=%+v", report)
					}
				}
			}
		})
	}
}

func TestAdvanceWriteRefusalsRetainThroughTheFailedStep(t *testing.T) {
	log, data, checkpoint := layouts()
	for _, failedWrite := range []int{1, 2, 3, 8, 9, 29, 30, 31, 36} {
		t.Run(fmt.Sprintf("write-%d", failedWrite), func(t *testing.T) {
			inner := memoryDeviceWithCheckpoint(t, log, checkpoint, cpRecord(storagewalcheckpointrecord.Ready, 1, 0), storagewalcheckpointrecord.Record{}, logRecords([]byte{1, 2, 3, 4, 5, 6, 7, 8}))
			trace := &tracedDevice{inner: inner, failWriteAt: failedWrite}
			report, err := Advance(trace, log, data, checkpoint, 2)
			if err != errWrite {
				t.Fatalf("Advance error=%v", err)
			}
			if report.Observed != cpRecord(storagewalcheckpointrecord.Ready, 1, 0) {
				t.Fatalf("observed=%+v", report.Observed)
			}
			switch failedWrite {
			case 1:
				want := storagewalrecovery.Result{Replay: storagewalredoapply.Receipt{Interrupted: true, PageID: 0, RecordLSN: 2}}
				if report.Replay != want || report.Resetting != (storagewalcheckpointpublication.Receipt{}) {
					t.Fatalf("replay refusal=%+v", report)
				}
			case 2:
				if report.Replay.Replay != (storagewalredoapply.Receipt{AcknowledgedWrites: 1}) || report.Replay.LastLSN != 3 || report.Resetting.Attempted != storagewalcheckpointpublication.InvalidateGuard || report.Resetting.Confirmed {
					t.Fatalf("resetting guard refusal=%+v", report)
				}
			case 3:
				if report.Resetting.Attempted != storagewalcheckpointpublication.StageBody || report.Resetting.BodyBlocks != 0 || report.Resetting.Confirmed {
					t.Fatalf("resetting body refusal=%+v", report.Resetting)
				}
			case 8:
				if report.Resetting.Attempted != storagewalcheckpointpublication.PublishGuard || report.Resetting.BodyBlocks != 5 || report.Resetting.Confirmed {
					t.Fatalf("resetting publication refusal=%+v", report.Resetting)
				}
			case 9, 29:
				wantBlocks := int64(0)
				if failedWrite == 29 {
					wantBlocks = 20
				}
				if !report.Resetting.Confirmed || report.Clear.Checkpoint != cpRecord(storagewalcheckpointrecord.Resetting, 2, 3) || report.Clear.BlocksWritten != wantBlocks || report.Clear.Acknowledged {
					t.Fatalf("clear refusal=%+v", report.Clear)
				}
			case 30, 31, 36:
				wantStep := storagewalcheckpointpublication.InvalidateGuard
				wantBlocks := int64(0)
				if failedWrite == 31 {
					wantStep = storagewalcheckpointpublication.StageBody
				} else if failedWrite == 36 {
					wantStep, wantBlocks = storagewalcheckpointpublication.PublishGuard, 5
				}
				if !report.Resetting.Confirmed || !report.Clear.Acknowledged || report.Ready.Attempted != wantStep || report.Ready.BodyBlocks != wantBlocks || report.Ready.Confirmed {
					t.Fatalf("ready refusal=%+v", report.Ready)
				}
			}
		})
	}
}

func cutsForDirtySectors(sectors int) []hddwritebackcache.BarrierCut {
	result := []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BeforeFirstSector}, {Kind: hddwritebackcache.AfterAllMedia}}
	for ordinal := 0; ordinal < sectors; ordinal++ {
		for prefix := 1; prefix < hddwritecrashcuts.BytesPerSector; prefix++ {
			value := prefix
			result = append(result, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value}})
		}
		result = append(result, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})
	}
	return result
}

func TestAdvanceEveryHDDBarrierCutReopensAndRetriesSameTarget(t *testing.T) {
	log, data, checkpoint := layouts()
	payload := []byte{18, 28, 38, 48, 58, 68, 78, 88}
	baseline, _, _, _ := seedInitialHDD(t, false, payload)
	baselineReport, err := Advance(baseline, log, data, checkpoint, 2)
	if err != nil || !baselineReport.Ready.Confirmed {
		t.Fatalf("baseline advance=(%+v,%v)", baselineReport, err)
	}
	wantFinal := blockSnapshot(t, baseline)
	barriers := []int{1, 1, 5, 1, 21, 1, 5, 1}
	for cutOrdinal, sectorCount := range barriers {
		for cutIndex, cut := range cutsForDirtySectors(sectorCount) {
			t.Run(fmt.Sprintf("barrier-%d-cut-%d", cutOrdinal+1, cutIndex), func(t *testing.T) {
				inner, _, _, _ := seedInitialHDD(t, false, payload)
				device := &tracedDevice{inner: inner, cutAt: cutOrdinal + 1, cut: cut}
				failed, err := Advance(device, log, data, checkpoint, 2)
				if err != errLost {
					t.Fatalf("cut Advance=(%+v,%v)", failed, err)
				}
				restarted, err := inner.Restart()
				if err != nil {
					t.Fatal(err)
				}
				beforeRetry, err := storagewalcheckpointcapture.Capture(restarted, log, data, checkpoint)
				if err != nil {
					t.Fatal(err)
				}
				retryDevice := &tracedDevice{inner: restarted}
				retry, err := Advance(retryDevice, log, data, checkpoint, 2)
				if err != nil || !retry.Ready.Confirmed && !retry.Ready.AlreadyCurrent {
					t.Fatalf("retry=(%+v,%v)", retry, err)
				}
				readyWasAlreadyCurrent := beforeRetry.Selection.Record == cpRecord(storagewalcheckpointrecord.Ready, 2, 3)
				if retry.Ready.AlreadyCurrent != readyWasAlreadyCurrent {
					t.Fatalf("retry idempotence=%+v prior=%+v", retry, beforeRetry.Selection)
				}
				if got := blockSnapshot(t, restarted); !reflect.DeepEqual(got, wantFinal) {
					t.Fatalf("same-target retry diverged from complete HDD image at barrier %d cut %+v", cutOrdinal+1, cut)
				}
				captured, err := storagewalcheckpointcapture.Capture(restarted, log, data, checkpoint)
				if err != nil || captured.Selection.Record != cpRecord(storagewalcheckpointrecord.Ready, 2, 3) {
					t.Fatalf("retry checkpoint=(%+v,%v)", captured.Selection, err)
				}
				for block := int64(0); block < 21; block++ {
					value, err := restarted.ReadBlock(block)
					if err != nil || !reflect.DeepEqual(value, make([]byte, 8)) {
						t.Fatalf("retry log block %d=%v err=%v", block, value, err)
					}
				}
			})
		}
	}
}

func TestAdvanceInterruptedSecondAttemptResumesSameResettingEpoch(t *testing.T) {
	log, data, checkpoint := layouts()
	payload := []byte{14, 24, 34, 44, 54, 64, 74, 84}
	baseline, _, _, _ := seedInitialHDD(t, false, payload)
	if report, err := Advance(baseline, log, data, checkpoint, 2); err != nil || !report.Ready.Confirmed {
		t.Fatalf("baseline=(%+v,%v)", report, err)
	}
	wantFinal := blockSnapshot(t, baseline)

	first, _, _, _ := seedInitialHDD(t, false, payload)
	firstCut := &tracedDevice{inner: first, cutAt: 4, cut: hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AfterAllMedia}}
	firstReport, err := Advance(firstCut, log, data, checkpoint, 2)
	if err != errLost || firstReport.Resetting.Attempted != storagewalcheckpointpublication.PublishGuard || firstReport.Resetting.Confirmed {
		t.Fatalf("first cut=(%+v,%v)", firstReport, err)
	}
	resetting, err := first.Restart()
	if err != nil {
		t.Fatal(err)
	}
	captured, err := storagewalcheckpointcapture.Capture(resetting, log, data, checkpoint)
	if err != nil || captured.Selection.Record != cpRecord(storagewalcheckpointrecord.Resetting, 2, 3) {
		t.Fatalf("first restart checkpoint=(%+v,%v)", captured.Selection, err)
	}

	prefix := 3
	secondCut := &tracedDevice{inner: resetting, cutAt: 1, cut: hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 4, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}}}
	secondReport, err := Advance(secondCut, log, data, checkpoint, 2)
	if err != errLost || secondReport.Observed != cpRecord(storagewalcheckpointrecord.Resetting, 2, 3) || secondReport.Replay != (storagewalrecovery.Result{}) || secondReport.Resetting != (storagewalcheckpointpublication.Receipt{}) || secondReport.Clear.Acknowledged || secondReport.Clear.BlocksWritten != 21 || secondReport.Ready != (storagewalcheckpointpublication.Receipt{}) {
		t.Fatalf("second cut=(%+v,%v)", secondReport, err)
	}

	third, err := resetting.Restart()
	if err != nil {
		t.Fatal(err)
	}
	thirdReport, err := Advance(third, log, data, checkpoint, 2)
	if err != nil || thirdReport.Observed != cpRecord(storagewalcheckpointrecord.Resetting, 2, 3) || thirdReport.Replay != (storagewalrecovery.Result{}) || !thirdReport.Clear.Acknowledged || !thirdReport.Ready.Confirmed {
		t.Fatalf("third attempt=(%+v,%v)", thirdReport, err)
	}
	if got := blockSnapshot(t, third); !reflect.DeepEqual(got, wantFinal) {
		t.Fatal("interrupted second attempt did not converge to the same durable HDD image")
	}
	value, err := third.ReadBlock(data.FirstBlock)
	if err != nil || !reflect.DeepEqual(value, payload) {
		t.Fatalf("preserved replayed data=%v err=%v", value, err)
	}
}

func FuzzAdvanceEpochChoices(f *testing.F) {
	for mode := byte(0); mode < 6; mode++ {
		f.Add([]byte{mode, 1, 2, 3, 4, 5, 6, 7, 8})
	}
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) == 0 || len(input) > 24 {
			t.Skip()
		}
		log, data, checkpoint := layouts()
		device, err := memoryblockdevice.New(log.Geometry)
		if err != nil {
			t.Fatal(err)
		}
		mode := input[0] % 6
		image := make([]byte, 8)
		for index := range image {
			if index+1 < len(input) {
				image[index] = input[index+1]
			}
		}
		slot0 := cpRecord(storagewalcheckpointrecord.Ready, 1, 0)
		slot1 := storagewalcheckpointrecord.Record{}
		var records []storagewalrecord.Record
		target := uint64(2)
		switch mode {
		case 0:
			target = 0
		case 1:
			target = 1
		case 2:
			target = 3
		case 3:
			records = logRecords(image)
		case 4:
			slot1 = cpRecord(storagewalcheckpointrecord.Resetting, 2, 3)
			records = logRecords(image)
		case 5:
			bad := makePage(t, cpRecord(storagewalcheckpointrecord.Ready, 1, 0))
			bad[len(bad)-1] = 1
			pager, openErr := storagepageio.Open(device, checkpoint)
			if openErr != nil {
				t.Fatal(openErr)
			}
			if _, openErr = pager.WritePage(0, bad); openErr != nil {
				t.Fatal(openErr)
			}
			if openErr = pager.Flush(); openErr != nil {
				t.Fatal(openErr)
			}
		}
		if mode != 5 {
			seedLog(t, device, log, records)
			seedCheckpoint(t, device, checkpoint, slot0, slot1)
			if err := device.Flush(); err != nil {
				t.Fatal(err)
			}
		}
		before := blockSnapshot(t, device)
		trace := &tracedDevice{inner: device}
		report, err := Advance(trace, log, data, checkpoint, target)
		switch mode {
		case 0:
			if err != ErrTargetEpoch || report != (Report{}) {
				t.Fatalf("invalid target=(%+v,%v)", report, err)
			}
		case 1:
			if err != nil || !report.Ready.AlreadyCurrent || report.Ready.Slot != 0 || !reflect.DeepEqual(before, blockSnapshot(t, device)) {
				t.Fatalf("same target=(%+v,%v)", report, err)
			}
		case 2:
			if err != ErrCheckpointOrder || report != (Report{}) || !reflect.DeepEqual(before, blockSnapshot(t, device)) {
				t.Fatalf("order refusal=(%+v,%v)", report, err)
			}
		case 3:
			if err != nil || report.Observed != slot0 || report.Replay.LastLSN != 3 || !report.Resetting.Confirmed || !report.Clear.Acknowledged || !report.Ready.Confirmed {
				t.Fatalf("advance=(%+v,%v)", report, err)
			}
		case 4:
			if err != nil || report.Replay != (storagewalrecovery.Result{}) || report.Observed != slot1 || !report.Clear.Acknowledged || !report.Ready.Confirmed {
				t.Fatalf("reset resume=(%+v,%v)", report, err)
			}
		case 5:
			if err == nil || report != (Report{}) {
				t.Fatalf("capture refusal=(%+v,%v)", report, err)
			}
		}
	})
}
