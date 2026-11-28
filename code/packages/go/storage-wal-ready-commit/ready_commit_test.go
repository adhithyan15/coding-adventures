package storagewalreadycommit

import (
	"bytes"
	"errors"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
	storagewalcommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-commit"
	storagewalpreparecommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-prepare-commit"
)

var errInjected = errors.New("injected device failure")

type observedDevice struct {
	base      blockdevicecontract.Device
	reads     int
	writes    int
	flushes   int
	failRead  bool
	failWrite int
	failFlush int
}

func (d *observedDevice) Geometry() blockdevicecontract.Geometry { return d.base.Geometry() }
func (d *observedDevice) ReadBlock(block int64) ([]byte, error) {
	d.reads++
	if d.failRead {
		return nil, errInjected
	}
	return d.base.ReadBlock(block)
}
func (d *observedDevice) WriteBlock(block int64, data []byte) error {
	d.writes++
	if d.failWrite != 0 && d.writes == d.failWrite {
		return errInjected
	}
	return d.base.WriteBlock(block, data)
}
func (d *observedDevice) Flush() error {
	d.flushes++
	if d.failFlush != 0 && d.flushes == d.failFlush {
		return errInjected
	}
	return d.base.Flush()
}

type cutHDD struct {
	base *hddblockdevice.HDD
	cut  hddwritebackcache.BarrierCut
	used bool
}

func (d *cutHDD) Geometry() blockdevicecontract.Geometry    { return d.base.Geometry() }
func (d *cutHDD) ReadBlock(block int64) ([]byte, error)     { return d.base.ReadBlock(block) }
func (d *cutHDD) WriteBlock(block int64, data []byte) error { return d.base.WriteBlock(block, data) }
func (d *cutHDD) Flush() error {
	if d.used {
		return d.base.Flush()
	}
	d.used = true
	receipt, err := d.base.FlushWithCut(d.cut)
	if err != nil {
		return err
	}
	if !receipt.Acknowledged {
		return errInjected
	}
	return nil
}

func testLayouts(geometry blockdevicecontract.Geometry) (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	return storagepagelayout.Layout{Geometry: geometry, FirstBlock: 0, PageCount: 3, BlocksPerPage: 7},
		storagepagelayout.Layout{Geometry: geometry, FirstBlock: 21, PageCount: 1, BlocksPerPage: 1},
		storagepagelayout.Layout{Geometry: geometry, FirstBlock: 22, PageCount: 2, BlocksPerPage: 5}
}

func installCheckpoint(t *testing.T, device blockdevicecontract.Device, record storagewalcheckpointrecord.Record) (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	t.Helper()
	log, data, checkpoint := testLayouts(device.Geometry())
	frame, err := storagewalcheckpointslot.Stage(record)
	if err != nil {
		t.Fatalf("encode checkpoint: %v", err)
	}
	frame[storagewalcheckpointslot.GuardOffset] = 0
	pager, err := storagepageio.Open(device, checkpoint)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := pager.WritePage(0, frame); err != nil {
		t.Fatal(err)
	}
	if err := pager.Flush(); err != nil {
		t.Fatal(err)
	}
	return log, data, checkpoint
}

func oneImage() []storagewalpreparecommit.PageImage {
	image := make([]byte, 8)
	for index := range image {
		image[index] = byte(index*7 + 3)
	}
	return []storagewalpreparecommit.PageImage{{PageID: 0, Image: image}}
}

func TestInvalidEpochRefusesBeforeCheckpointReads(t *testing.T) {
	base, err := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
	if err != nil {
		t.Fatal(err)
	}
	device := &observedDevice{base: base, failRead: true}
	for _, epoch := range []uint64{0, maxPortableEpoch + 1} {
		got, err := Commit(device, storagepagelayout.Layout{}, storagepagelayout.Layout{}, storagepagelayout.Layout{}, epoch, 7, nil)
		if !errors.Is(err, ErrInvalidEpoch) || !reflect.DeepEqual(got, Result{}) {
			t.Fatalf("epoch %d result=%+v err=%v", epoch, got, err)
		}
	}
	if device.reads != 0 || device.writes != 0 || device.flushes != 0 {
		t.Fatalf("invalid epoch touched device: reads/writes/flushes=%d/%d/%d", device.reads, device.writes, device.flushes)
	}
}

func TestResettingPrecedesEpochMismatchAndHasNoEffects(t *testing.T) {
	base, err := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
	if err != nil {
		t.Fatal(err)
	}
	log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Resetting, Epoch: 2})
	device := &observedDevice{base: base}
	got, err := Commit(device, log, data, checkpoint, 1, 9, oneImage())
	if !errors.Is(err, ErrCheckpointResetting) || !reflect.DeepEqual(got, Result{}) {
		t.Fatalf("Resetting with mismatched epoch result=%+v err=%v", got, err)
	}
	if device.writes != 0 || device.flushes != 0 {
		t.Fatalf("Resetting refusal wrote/flushed: %d/%d", device.writes, device.flushes)
	}
}

func TestCheckpointCaptureFailureIsPreservedWithoutEffects(t *testing.T) {
	base, err := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
	if err != nil {
		t.Fatal(err)
	}
	log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1})
	device := &observedDevice{base: base, failRead: true}
	got, err := Commit(device, log, data, checkpoint, 1, 9, oneImage())
	if !errors.Is(err, errInjected) || !reflect.DeepEqual(got, Result{}) {
		t.Fatalf("capture refusal result=%+v err=%v", got, err)
	}
	if device.writes != 0 || device.flushes != 0 {
		t.Fatalf("capture refusal wrote/flushed: %d/%d", device.writes, device.flushes)
	}
}

func TestReadyEpochMismatchHasNoEffects(t *testing.T) {
	base, err := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
	if err != nil {
		t.Fatal(err)
	}
	log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1})
	device := &observedDevice{base: base}
	got, err := Commit(device, log, data, checkpoint, 2, 9, oneImage())
	if !errors.Is(err, ErrCheckpointEpoch) || !reflect.DeepEqual(got, Result{}) {
		t.Fatalf("mismatched Ready result=%+v err=%v", got, err)
	}
	if device.writes != 0 || device.flushes != 0 {
		t.Fatalf("epoch refusal wrote/flushed: %d/%d", device.writes, device.flushes)
	}
}

func TestMatchingReadyEpochReturnsExactCommitReceipt(t *testing.T) {
	base, err := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
	if err != nil {
		t.Fatal(err)
	}
	log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1})
	baseline, err := base.Restart()
	if err != nil {
		t.Fatal(err)
	}
	device := &observedDevice{base: base}
	images := oneImage()
	result, err := Commit(device, log, data, checkpoint, 1, 9, images)
	if err != nil {
		t.Fatal(err)
	}
	wantCommit, err := storagewalcommit.Commit(baseline, log, data, 9, images)
	if err != nil {
		t.Fatal(err)
	}
	wantCheckpoint := storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1}
	if result.Checkpoint != wantCheckpoint || !reflect.DeepEqual(result.Commit, wantCommit) {
		t.Fatalf("result=%+v want checkpoint=%+v commit=%+v", result, wantCheckpoint, wantCommit)
	}
	originalImage := append([]byte(nil), images[0].Image...)
	images[0].Image[0] ^= 0xff
	if !bytes.Equal(result.Commit.Log.Committed[1].Image, originalImage) {
		t.Fatal("commit receipt aliases the caller's image")
	}
	if device.writes == 0 || device.flushes == 0 {
		t.Fatalf("admitted commit did not reach lower writes/barriers: %d/%d", device.writes, device.flushes)
	}
}

func TestCommitFailurePreservesSelectedCheckpointAndLowerReceipt(t *testing.T) {
	base, err := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
	if err != nil {
		t.Fatal(err)
	}
	log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1})
	baseline, err := base.Restart()
	if err != nil {
		t.Fatal(err)
	}
	device := &observedDevice{base: base, failWrite: 1}
	wantDevice := &observedDevice{base: baseline, failWrite: 1}
	images := oneImage()
	got, gotErr := Commit(device, log, data, checkpoint, 1, 11, images)
	want, wantErr := storagewalcommit.Commit(wantDevice, log, data, 11, images)
	if !errors.Is(gotErr, errInjected) || !errors.Is(wantErr, errInjected) || !reflect.DeepEqual(got.Commit, want) {
		t.Fatalf("wrapper=(%+v,%v) lower=(%+v,%v)", got, gotErr, want, wantErr)
	}
	if got.Checkpoint != (storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1}) {
		t.Fatalf("failure lost selected checkpoint: %+v", got)
	}
}

func TestActualHDDAndMemoryBackendsShareReadySemantics(t *testing.T) {
	for _, tc := range []struct {
		name string
		new  func() blockdevicecontract.Device
	}{
		{name: "memory", new: func() blockdevicecontract.Device {
			device, _ := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
			return device
		}},
		{name: "hdd", new: func() blockdevicecontract.Device { return hddblockdevice.New() }},
	} {
		t.Run(tc.name, func(t *testing.T) {
			device := tc.new()
			log, data, checkpoint := installCheckpoint(t, device, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1})
			result, err := Commit(device, log, data, checkpoint, 1, 23, oneImage())
			if err != nil {
				t.Fatal(err)
			}
			if result.Checkpoint.Phase != storagewalcheckpointrecord.Ready || result.Checkpoint.Epoch != 1 || result.Commit.Log.AcknowledgedRecords != 3 || result.Commit.Log.LastLSN != 3 || result.Commit.Data.AcknowledgedWrites != 1 {
				t.Fatalf("receipt=%+v", result)
			}
		})
	}
}

func TestActualHDDLostBarrierAcknowledgementReturnsExactLowerReceipt(t *testing.T) {
	makeDevice := func(t *testing.T) (*hddblockdevice.HDD, storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
		t.Helper()
		base := hddblockdevice.New()
		log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1})
		return base, log, data, checkpoint
	}
	base, log, data, checkpoint := makeDevice(t)
	wantBase, _, _, _ := makeDevice(t)
	cut := hddwritebackcache.BarrierCut{
		Kind:        hddwritebackcache.AtSector,
		SectorIndex: 0,
		MediaCut:    hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse},
	}
	device := &cutHDD{base: base, cut: cut}
	wantDevice := &cutHDD{base: wantBase, cut: cut}
	images := oneImage()
	got, gotErr := Commit(device, log, data, checkpoint, 1, 29, images)
	want, wantErr := storagewalcommit.Commit(wantDevice, log, data, 29, images)
	if !errors.Is(gotErr, errInjected) || !errors.Is(wantErr, errInjected) || !reflect.DeepEqual(got.Commit, want) {
		t.Fatalf("wrapper=(%+v,%v) lower=(%+v,%v)", got, gotErr, want, wantErr)
	}
	if got.Checkpoint != (storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: 1}) || got.Commit.Log.LastAppend.Acknowledged {
		t.Fatalf("lost-ack result=%+v", got)
	}
	actualRestart, err := base.Restart()
	if err != nil {
		t.Fatal(err)
	}
	wantRestart, err := wantBase.Restart()
	if err != nil {
		t.Fatal(err)
	}
	for block := int64(0); block < 32; block++ {
		actual, actualErr := actualRestart.ReadBlock(block)
		expected, expectedErr := wantRestart.ReadBlock(block)
		if actualErr != nil || expectedErr != nil || !reflect.DeepEqual(actual, expected) {
			t.Fatalf("restarted block %d differs: %v/%v", block, actualErr, expectedErr)
		}
	}
}

func TestCommitModelGuidedFiniteInputs(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	for phase := storagewalcheckpointrecord.Resetting; phase <= storagewalcheckpointrecord.Ready; phase++ {
		for selectedEpoch := uint64(1); selectedEpoch <= 8; selectedEpoch++ {
			for requestedEpoch := uint64(1); requestedEpoch <= 8; requestedEpoch++ {
				base, err := memoryblockdevice.New(geometry)
				if err != nil {
					t.Fatal(err)
				}
				log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: phase, Epoch: selectedEpoch})
				device := &observedDevice{base: base}
				got, err := Commit(device, log, data, checkpoint, requestedEpoch, 17, nil)
				wantErr := error(nil)
				switch {
				case phase == storagewalcheckpointrecord.Resetting:
					wantErr = ErrCheckpointResetting
				case selectedEpoch != requestedEpoch:
					wantErr = ErrCheckpointEpoch
				}
				if wantErr != nil {
					if !errors.Is(err, wantErr) || !reflect.DeepEqual(got, Result{}) || device.writes != 0 || device.flushes != 0 {
						t.Fatalf("phase=%d selected=%d requested=%d got=%+v err=%v effects=%d/%d want=%v", phase, selectedEpoch, requestedEpoch, got, err, device.writes, device.flushes, wantErr)
					}
				} else if err != nil || got.Checkpoint != (storagewalcheckpointrecord.Record{Phase: phase, Epoch: selectedEpoch}) {
					t.Fatalf("phase=%d selected=%d requested=%d got=%+v err=%v", phase, selectedEpoch, requestedEpoch, got, err)
				}
			}
		}
	}
}

func FuzzReadyCommitEpochDecision(f *testing.F) {
	for _, seed := range []uint64{0, 1, 2, 7, 255, maxPortableEpoch, ^uint64(0)} {
		f.Add(seed, seed^0x5a5a)
	}
	f.Fuzz(func(t *testing.T, selected, requested uint64) {
		if selected == 0 || selected > maxPortableEpoch || requested == 0 || requested > maxPortableEpoch {
			t.Skip()
		}
		base, err := memoryblockdevice.New(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8})
		if err != nil {
			t.Fatal(err)
		}
		log, data, checkpoint := installCheckpoint(t, base, storagewalcheckpointrecord.Record{Phase: storagewalcheckpointrecord.Ready, Epoch: selected})
		device := &observedDevice{base: base}
		got, err := Commit(device, log, data, checkpoint, requested, 7, nil)
		if selected != requested {
			if !errors.Is(err, ErrCheckpointEpoch) || !reflect.DeepEqual(got, Result{}) || device.writes != 0 || device.flushes != 0 {
				t.Fatalf("selected=%d requested=%d got=%+v err=%v effects=%d/%d", selected, requested, got, err, device.writes, device.flushes)
			}
		} else if err != nil || got.Checkpoint.Epoch != selected || got.Checkpoint.Phase != storagewalcheckpointrecord.Ready {
			t.Fatalf("selected=%d requested=%d got=%+v err=%v", selected, requested, got, err)
		}
	})
}

func TestRefusalLabelsRemainDistinct(t *testing.T) {
	if ErrCheckpointResetting == ErrCheckpointEpoch || ErrInvalidEpoch == ErrCheckpointResetting || ErrInvalidEpoch == ErrCheckpointEpoch {
		t.Fatalf("typed refusal identities collapsed: %v %v %v", ErrInvalidEpoch, ErrCheckpointResetting, ErrCheckpointEpoch)
	}
}
