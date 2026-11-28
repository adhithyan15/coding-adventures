package storagewalreopen

import (
	"bytes"
	"errors"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalappend "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-append"
	storagewalcheckpointadvance "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-advance"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointpublication "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-publication"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointselection "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-selection"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalrecovery "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-recovery"
	storagewalredoapply "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-apply"
)

func testLayouts() (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	return storagepagelayout.Layout{
			Geometry: geometry, FirstBlock: 0, PageCount: 3, BlocksPerPage: 7,
		}, storagepagelayout.Layout{
			Geometry: geometry, FirstBlock: 21, PageCount: 1, BlocksPerPage: 1,
		}, storagepagelayout.Layout{
			Geometry: geometry, FirstBlock: 22, PageCount: 2, BlocksPerPage: 5,
		}
}

func testRecord(phase storagewalcheckpointrecord.Phase, epoch, previous uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: phase, Epoch: epoch, PreviousLSN: previous}
}

func testRecords(payload []byte, committed bool) []storagewalrecord.Record {
	records := []storagewalrecord.Record{
		{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 7},
		{Kind: storagewalrecord.WritePageImage, LSN: 2, TxID: 7, PageID: 0, Image: append([]byte(nil), payload...)},
	}
	if committed {
		records = append(records, storagewalrecord.Record{Kind: storagewalrecord.CommitTransaction, LSN: 3, TxID: 7})
	}
	return records
}

func newDevice(t *testing.T, kind string) blockdevicecontract.Device {
	t.Helper()
	log, _, _ := testLayouts()
	if kind == "memory" {
		device, err := memoryblockdevice.New(log.Geometry)
		if err != nil {
			t.Fatal(err)
		}
		return device
	}
	return hddblockdevice.New()
}

func seed(t *testing.T, device blockdevicecontract.Device, records []storagewalrecord.Record, slot0, slot1 storagewalcheckpointrecord.Record) {
	t.Helper()
	log, _, checkpoint := testLayouts()
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
	for index, record := range [2]storagewalcheckpointrecord.Record{slot0, slot1} {
		page := make([]byte, int(checkpoint.BlocksPerPage*checkpoint.Geometry.BytesPerBlock))
		if record != (storagewalcheckpointrecord.Record{}) {
			frame, err := storagewalcheckpointslot.Stage(record)
			if err != nil {
				t.Fatal(err)
			}
			copy(page, frame)
			page[storagewalcheckpointslot.GuardOffset] = 0
		}
		pager, err := storagepageio.Open(device, checkpoint)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := pager.WritePage(int64(index), page); err != nil {
			t.Fatal(err)
		}
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
}

func readBlock(t *testing.T, device blockdevicecontract.Device, block int64) []byte {
	t.Helper()
	value, err := device.ReadBlock(block)
	if err != nil {
		t.Fatal(err)
	}
	return value
}

func TestReopenReadyReplaysCommittedWALOnMemoryAndHDD(t *testing.T) {
	for _, kind := range []string{"memory", "hdd"} {
		t.Run(kind, func(t *testing.T) {
			log, data, checkpoint := testLayouts()
			payload := []byte{91, 82, 73, 64, 55, 46, 37, 28}
			device := newDevice(t, kind)
			ready := testRecord(storagewalcheckpointrecord.Ready, 4, 0)
			seed(t, device, testRecords(payload, true), ready, storagewalcheckpointrecord.Record{})

			result, err := Reopen(device, log, data, checkpoint)
			wantReplay := storagewalrecovery.Result{
				Replay: storagewalredoapply.Receipt{AcknowledgedWrites: 1}, LastLSN: 3,
			}
			if err != nil || result.Observed != ready {
				t.Fatalf("ready checkpoint selection=(%+v,%v)", result, err)
			}
			if got := readBlock(t, device, data.FirstBlock); !bytes.Equal(got, payload) {
				t.Fatalf("replayed data=%v want %v", got, payload)
			}
			if result.Ready != ready || result.Replay != wantReplay || result.Resume != (storagewalcheckpointadvance.Report{}) {
				t.Fatalf("ready reopen receipt=(%+v,%v), want replay=%+v", result, err, wantReplay)
			}
			restarted := restart(t, device)
			if got := readBlock(t, restarted, data.FirstBlock); !bytes.Equal(got, payload) {
				t.Fatalf("replay did not survive restart: %v", got)
			}
		})
	}
}

func TestReopenReadyPreservesUnfinishedBoundaryWithoutApplyingWrites(t *testing.T) {
	log, data, checkpoint := testLayouts()
	device := newDevice(t, "memory")
	ready := testRecord(storagewalcheckpointrecord.Ready, 2, 0)
	seed(t, device, testRecords([]byte{8, 7, 6, 5, 4, 3, 2, 1}, false), ready, storagewalcheckpointrecord.Record{})

	result, err := Reopen(device, log, data, checkpoint)
	if err != nil || result.Observed != ready || result.Ready != ready || result.Replay.LastLSN != 2 || result.Replay.Replay.AcknowledgedWrites != 0 {
		t.Fatalf("unfinished Ready replay=(%+v,%v)", result, err)
	}
	if got := readBlock(t, device, data.FirstBlock); !bytes.Equal(got, make([]byte, 8)) {
		t.Fatalf("unfinished transaction changed data: %v", got)
	}
}

func TestReopenEmptyCheckpointInitializesThenRecoversEmptyLog(t *testing.T) {
	for _, kind := range []string{"memory", "hdd"} {
		t.Run(kind, func(t *testing.T) {
			log, data, checkpoint := testLayouts()
			counted := &countDevice{inner: newDevice(t, kind)}
			result, err := Reopen(counted, log, data, checkpoint)
			if err != storagewalcheckpointselection.ErrNoPublishedCheckpoint || result != (Result{}) || counted.writes != 0 || counted.flushes != 0 {
				t.Fatalf("vacant capture must refuse without effects: result=%+v err=%v writes=%d flushes=%d", result, err, counted.writes, counted.flushes)
			}
			if _, err := storagewalcheckpointpublication.Initialize(counted, log, data, checkpoint); err != nil {
				t.Fatal(err)
			}
			device := counted
			result, err = Reopen(device, log, data, checkpoint)
			initial := testRecord(storagewalcheckpointrecord.Ready, 1, 0)
			if err != nil || result.Observed != initial || result.Ready != initial || result.Replay != (storagewalrecovery.Result{}) {
				t.Fatalf("initialized reopen=(%+v,%v)", result, err)
			}
			captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
			if err != nil || captured.Selection.Record != initial {
				t.Fatalf("initial checkpoint=(%+v,%v)", captured.Selection, err)
			}
		})
	}
}

func TestReopenDoesNotInitializeOverAnUncheckpointedWAL(t *testing.T) {
	log, data, checkpoint := testLayouts()
	inner := newDevice(t, "memory")
	seed(t, inner, testRecords([]byte{1, 3, 5, 7, 9, 11, 13, 15}, true), storagewalcheckpointrecord.Record{}, storagewalcheckpointrecord.Record{})
	device := &countDevice{inner: inner}
	result, err := Reopen(device, log, data, checkpoint)
	if err != storagewalcheckpointselection.ErrNoPublishedCheckpoint || result != (Result{}) {
		t.Fatalf("uncheckpointed WAL initialization=(%+v,%v)", result, err)
	}
	if device.writes != 0 || device.flushes != 0 {
		t.Fatalf("capture refusal wrote or flushed: writes=%d flushes=%d", device.writes, device.flushes)
	}
	if _, err := storagewalcheckpointpublication.Initialize(device, log, data, checkpoint); err != storagewalcheckpointpublication.ErrInitialLogNonempty {
		t.Fatalf("explicit initialization should refuse the uncheckpointed log: %v", err)
	}
	if got := readBlock(t, device, data.FirstBlock); !bytes.Equal(got, make([]byte, 8)) {
		t.Fatalf("initialization refusal changed data: %v", got)
	}
}

func TestReopenResettingContinuesWithoutReplayingOldLog(t *testing.T) {
	for _, kind := range []string{"memory", "hdd"} {
		t.Run(kind, func(t *testing.T) {
			log, data, checkpoint := testLayouts()
			device := newDevice(t, kind)
			resetting := testRecord(storagewalcheckpointrecord.Resetting, 7, 3)
			payload := []byte{90, 80, 70, 60, 50, 40, 30, 20}
			seed(t, device, testRecords(payload, true), resetting, testRecord(storagewalcheckpointrecord.Ready, 6, 0))

			result, err := Reopen(device, log, data, checkpoint)
			ready := testRecord(storagewalcheckpointrecord.Ready, 7, 3)
			if err != nil || result.Observed != resetting || result.Ready != ready {
				t.Fatalf("resetting reopen=(%+v,%v)", result, err)
			}
			if result.Replay != (storagewalrecovery.Result{}) {
				t.Fatalf("Resetting path replayed old log: %+v", result.Replay)
			}
			if result.Resume.Observed != resetting || result.Resume.Replay != (storagewalrecovery.Result{}) ||
				result.Resume.Clear.Checkpoint != resetting || !result.Resume.Clear.Acknowledged ||
				!result.Resume.Ready.Confirmed || result.Resume.Ready.AlreadyCurrent {
				t.Fatalf("resume report=%+v", result.Resume)
			}
			if got := readBlock(t, device, data.FirstBlock); !bytes.Equal(got, make([]byte, 8)) {
				t.Fatalf("old WAL was replayed during Resetting: %v", got)
			}
			for block := log.FirstBlock; block < log.FirstBlock+log.PageCount*log.BlocksPerPage; block++ {
				if got := readBlock(t, device, block); !bytes.Equal(got, make([]byte, 8)) {
					t.Fatalf("log block %d not cleared: %v", block, got)
				}
			}
			restarted := restart(t, device)
			captured, err := storagewalcheckpointcapture.Capture(restarted, log, data, checkpoint)
			if err != nil || captured.Selection.Record != ready {
				t.Fatalf("resumed Ready checkpoint not durable: %+v %v", captured.Selection, err)
			}
		})
	}
}

func TestReopenForwardsCaptureAndReplayRefusals(t *testing.T) {
	log, data, checkpoint := testLayouts()
	badDevice := newDevice(t, "memory")
	badCheckpoint := checkpoint
	badCheckpoint.FirstBlock = 20 // overlaps the data reservation
	result, err := Reopen(badDevice, log, data, badCheckpoint)
	if result != (Result{}) || err == nil {
		t.Fatalf("capture refusal was not exact/zero: %+v %v", result, err)
	}

	device := newDevice(t, "memory")
	ready := testRecord(storagewalcheckpointrecord.Ready, 3, 0)
	seed(t, device, nil, ready, storagewalcheckpointrecord.Record{})
	// A nonzero first byte in the fixed-width WAL slot has no valid frame and is
	// classified as a damaged tail after checkpoint selection succeeds.
	if err := device.WriteBlock(log.FirstBlock, []byte{1, 0, 0, 0, 0, 0, 0, 0}); err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	result, err = Reopen(device, log, data, checkpoint)
	var damaged *storagewalappend.DamagedTailError
	if !errors.As(err, &damaged) || result.Observed != ready || result.Ready != (storagewalcheckpointrecord.Record{}) {
		t.Fatalf("replay refusal=(%+v,%v)", result, err)
	}
}

func TestReopenResettingReturnsExactResumeErrorAndPartialReport(t *testing.T) {
	log, data, checkpoint := testLayouts()
	inner := newDevice(t, "memory")
	resetting := testRecord(storagewalcheckpointrecord.Resetting, 5, 0)
	seed(t, inner, nil, resetting, testRecord(storagewalcheckpointrecord.Ready, 4, 0))
	device := &failFlushDevice{inner: inner, failAt: 1, err: errors.New("flush blocked")}
	result, err := Reopen(device, log, data, checkpoint)
	if err != device.err || result.Observed != resetting || result.Ready != (storagewalcheckpointrecord.Record{}) || result.Resume.Observed != resetting {
		t.Fatalf("resume error/report=(%+v,%v)", result, err)
	}
	if result.Replay != (storagewalrecovery.Result{}) || result.Resume.Ready.Confirmed {
		t.Fatalf("failed resume applied Ready/replay: %+v", result)
	}
	if result.Resume.Clear.Checkpoint != resetting || result.Resume.Clear.BlocksWritten != 21 || result.Resume.Clear.Acknowledged {
		t.Fatalf("failed resume did not preserve the exact lower clear report: %+v", result.Resume.Clear)
	}
}

type failFlushDevice struct {
	inner   blockdevicecontract.Device
	flushes int
	failAt  int
	err     error
}

func (device *failFlushDevice) Geometry() blockdevicecontract.Geometry {
	return device.inner.Geometry()
}
func (device *failFlushDevice) ReadBlock(block int64) ([]byte, error) {
	return device.inner.ReadBlock(block)
}
func (device *failFlushDevice) WriteBlock(block int64, data []byte) error {
	return device.inner.WriteBlock(block, data)
}
func (device *failFlushDevice) Flush() error {
	device.flushes++
	if device.flushes == device.failAt {
		return device.err
	}
	return device.inner.Flush()
}

type countDevice struct {
	inner   blockdevicecontract.Device
	writes  int
	flushes int
}

func (device *countDevice) Geometry() blockdevicecontract.Geometry {
	return device.inner.Geometry()
}
func (device *countDevice) ReadBlock(block int64) ([]byte, error) {
	return device.inner.ReadBlock(block)
}
func (device *countDevice) WriteBlock(block int64, data []byte) error {
	device.writes++
	return device.inner.WriteBlock(block, data)
}
func (device *countDevice) Flush() error {
	device.flushes++
	return device.inner.Flush()
}

func restart(t *testing.T, device blockdevicecontract.Device) blockdevicecontract.Device {
	t.Helper()
	switch current := device.(type) {
	case *memoryblockdevice.Memory:
		next, err := current.Restart()
		if err != nil {
			t.Fatal(err)
		}
		return next
	case *hddblockdevice.HDD:
		next, err := current.Restart()
		if err != nil {
			t.Fatal(err)
		}
		return next
	default:
		t.Fatalf("unsupported restart test device %T", device)
		return nil
	}
}

func TestReopenReadyResultEqualsRecoveryOnFuzzedPayloads(t *testing.T) {
	// Fixed-seed stateful generation makes payload ownership and successful redo
	// reproducible while covering every payload byte position independently.
	for seedValue := int64(0); seedValue < 256; seedValue++ {
		payload := make([]byte, 8)
		for i := range payload {
			payload[i] = byte(seedValue + int64(i)*37)
		}
		log, data, checkpoint := testLayouts()
		device := newDevice(t, "memory")
		ready := testRecord(storagewalcheckpointrecord.Ready, uint64(seedValue+1), 0)
		seed(t, device, testRecords(payload, true), ready, storagewalcheckpointrecord.Record{})
		result, err := Reopen(device, log, data, checkpoint)
		if err != nil || result.Ready != ready || result.Replay.LastLSN != 3 || !bytes.Equal(readBlock(t, device, data.FirstBlock), payload) {
			t.Fatalf("seed %d reopen=(%+v,%v)", seedValue, result, err)
		}
		if !reflect.DeepEqual(result.Resume, storagewalcheckpointadvance.Report{}) {
			t.Fatalf("Ready reopen has resume report: %+v", result.Resume)
		}
	}
}

func FuzzReopenSelectedRestartPath(f *testing.F) {
	for _, mode := range []byte{0, 1, 2, 3, 0xff} {
		f.Add(mode, []byte{0, 1, 2, 3, 4, 5, 6, 7})
	}
	f.Fuzz(func(t *testing.T, mode byte, input []byte) {
		log, data, checkpoint := testLayouts()
		payload := make([]byte, 8)
		copy(payload, input)
		device := newDevice(t, "memory")
		if mode&1 == 0 {
			ready := testRecord(storagewalcheckpointrecord.Ready, uint64(mode>>1)+1, 0)
			committed := mode&2 != 0
			seed(t, device, testRecords(payload, committed), ready, storagewalcheckpointrecord.Record{})
			result, err := Reopen(device, log, data, checkpoint)
			lastLSN, writes := uint64(2), 0
			if committed {
				lastLSN, writes = 3, 1
			}
			if err != nil || result.Observed != ready || result.Ready != ready || result.Replay.LastLSN != lastLSN || result.Replay.Replay.AcknowledgedWrites != writes || result.Resume != (storagewalcheckpointadvance.Report{}) {
				t.Fatalf("Ready mode=%#02x result=(%+v,%v)", mode, result, err)
			}
			wantData := make([]byte, 8)
			if committed {
				wantData = payload
			}
			if got := readBlock(t, device, data.FirstBlock); !bytes.Equal(got, wantData) {
				t.Fatalf("Ready mode=%#02x data=%v want %v", mode, got, wantData)
			}
			return
		}

		resetting := testRecord(storagewalcheckpointrecord.Resetting, uint64(mode>>1)+2, 3)
		oldReady := testRecord(storagewalcheckpointrecord.Ready, resetting.Epoch-1, 0)
		seed(t, device, testRecords(payload, true), resetting, oldReady)
		result, err := Reopen(device, log, data, checkpoint)
		wantReady := testRecord(storagewalcheckpointrecord.Ready, resetting.Epoch, resetting.PreviousLSN)
		if err != nil || result.Observed != resetting || result.Ready != wantReady || result.Replay != (storagewalrecovery.Result{}) || result.Resume.Observed != resetting {
			t.Fatalf("Resetting mode=%#02x result=(%+v,%v)", mode, result, err)
		}
		if got := readBlock(t, device, data.FirstBlock); !bytes.Equal(got, make([]byte, 8)) {
			t.Fatalf("Resetting mode=%#02x replayed old WAL into data: %v", mode, got)
		}
	})
}
