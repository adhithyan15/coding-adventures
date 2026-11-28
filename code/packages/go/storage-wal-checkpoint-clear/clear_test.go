package storagewalcheckpointclear

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
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
)

var (
	errClearWrite = errors.New("clear write refusal")
	errClearFlush = errors.New("clear barrier refusal")
	errClearLost  = errors.New("clear barrier acknowledgement lost")
	errClearRead  = errors.New("checkpoint read refusal")
)

type scriptedDevice struct {
	geometry  blockdevicecontract.Geometry
	blocks    map[int64][]byte
	readErr   map[int64]error
	badWidth  map[int64]bool
	writes    []int64
	inputs    [][]byte
	flushes   int
	failAt    int
	failFlush bool
}

func newScripted(geometry blockdevicecontract.Geometry) *scriptedDevice {
	return &scriptedDevice{geometry: geometry, blocks: make(map[int64][]byte), readErr: make(map[int64]error), badWidth: make(map[int64]bool)}
}
func (d *scriptedDevice) Geometry() blockdevicecontract.Geometry { return d.geometry }
func (d *scriptedDevice) ReadBlock(block int64) ([]byte, error) {
	if err := d.readErr[block]; err != nil {
		return nil, err
	}
	if d.badWidth[block] {
		return []byte{1}, nil
	}
	if value, ok := d.blocks[block]; ok {
		return append([]byte(nil), value...), nil
	}
	return make([]byte, int(d.geometry.BytesPerBlock)), nil
}
func (d *scriptedDevice) WriteBlock(block int64, data []byte) error {
	d.writes = append(d.writes, block)
	d.inputs = append(d.inputs, data)
	if d.failAt != 0 && len(d.writes) == d.failAt {
		return errClearWrite
	}
	d.blocks[block] = append([]byte(nil), data...)
	return nil
}
func (d *scriptedDevice) Flush() error {
	d.flushes++
	if d.failFlush {
		return errClearFlush
	}
	return nil
}

func layouts(width int64) (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	logBPP := (storagewalpageslot.MinPageBytes + width + width - 1) / width
	checkpointBPP := int64((storagewalcheckpointrecord.FrameBytes + int(width) - 1) / int(width))
	logBlocks := int64(3) * logBPP
	dataFirst := logBlocks
	checkpointFirst := dataFirst + 1
	geometry := blockdevicecontract.Geometry{BlockCount: checkpointFirst + 2*checkpointBPP, BytesPerBlock: width}
	return storagepagelayout.Layout{Geometry: geometry, FirstBlock: 0, PageCount: 3, BlocksPerPage: logBPP},
		storagepagelayout.Layout{Geometry: geometry, FirstBlock: dataFirst, PageCount: 1, BlocksPerPage: 1},
		storagepagelayout.Layout{Geometry: geometry, FirstBlock: checkpointFirst, PageCount: 2, BlocksPerPage: checkpointBPP}
}

func record(phase storagewalcheckpointrecord.Phase, epoch, lsn uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: phase, Epoch: epoch, PreviousLSN: lsn}
}

func page(width int64, value storagewalcheckpointrecord.Record) []byte {
	frame, err := storagewalcheckpointslot.Stage(value)
	if err != nil {
		panic(err)
	}
	result := make([]byte, ((len(frame)+int(width)-1)/int(width))*int(width))
	copy(result, frame)
	result[storagewalcheckpointslot.GuardOffset] = 0
	return result
}

func installPage(t *testing.T, device *scriptedDevice, layout storagepagelayout.Layout, id int64, value []byte) {
	t.Helper()
	span, err := layout.Span(id)
	if err != nil {
		t.Fatal(err)
	}
	width := int(layout.Geometry.BytesPerBlock)
	for ordinal := int64(0); ordinal < span.BlockCount; ordinal++ {
		start := int(ordinal) * width
		device.blocks[span.FirstBlock+ordinal] = append([]byte(nil), value[start:start+width]...)
	}
}

func seed(width int64, phase storagewalcheckpointrecord.Phase) (*scriptedDevice, storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	log, data, checkpoint := layouts(width)
	device := newScripted(log.Geometry)
	for block := int64(0); block < log.Geometry.BlockCount; block++ {
		value := make([]byte, int(width))
		for index := range value {
			value[index] = byte((block*17 + int64(index)*3 + 1) & 0xff)
		}
		device.blocks[block] = value
	}
	installPageDirect(device, checkpoint, 0, page(width, record(phase, 5, 91)))
	installPageDirect(device, checkpoint, 1, page(width, record(storagewalcheckpointrecord.Resetting, 4, 80)))
	return device, log, data, checkpoint
}

func installPageDirect(device *scriptedDevice, layout storagepagelayout.Layout, id int64, value []byte) {
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

func copyBlocks(device *scriptedDevice) map[int64][]byte {
	result := make(map[int64][]byte, len(device.blocks))
	for block, value := range device.blocks {
		result[block] = append([]byte(nil), value...)
	}
	return result
}

func TestClearResetsOnlyValidatedLogRange(t *testing.T) {
	for _, width := range []int64{8, 13} {
		t.Run(fmt.Sprintf("width-%d", width), func(t *testing.T) {
			device, log, data, checkpoint := seed(width, storagewalcheckpointrecord.Resetting)
			before := copyBlocks(device)
			receipt, err := Clear(device, log, data, checkpoint)
			want := Receipt{Checkpoint: record(storagewalcheckpointrecord.Resetting, 5, 91), BlocksWritten: log.PageCount * log.BlocksPerPage, Acknowledged: true}
			if err != nil || receipt != want {
				t.Fatalf("Clear=(%+v,%v) want=%+v", receipt, err, want)
			}
			if int64(len(device.writes)) != want.BlocksWritten || device.flushes != 1 {
				t.Fatalf("effects writes=%v flushes=%d", device.writes, device.flushes)
			}
			for index, block := range device.writes {
				if block != log.FirstBlock+int64(index) {
					t.Fatalf("write order=%v", device.writes)
				}
				if !reflect.DeepEqual(device.blocks[block], make([]byte, int(width))) {
					t.Fatalf("block %d was not zeroed", block)
				}
				if index > 0 && &device.inputs[index][0] != &device.inputs[0][0] {
					t.Fatal("zero write buffer was not reused")
				}
			}
			for block, value := range before {
				if block >= log.FirstBlock && block < log.FirstBlock+want.BlocksWritten {
					continue
				}
				if !reflect.DeepEqual(device.blocks[block], value) {
					t.Fatalf("non-log block %d changed", block)
				}
			}
			captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
			if err != nil || captured.Selection.Record != receipt.Checkpoint {
				t.Fatalf("checkpoint after clear=(%+v,%v)", captured.Selection, err)
			}

			device.writes = nil
			device.inputs = nil
			device.flushes = 0
			repeated, err := Clear(device, log, data, checkpoint)
			if err != nil || repeated != want {
				t.Fatalf("repeat=(%+v,%v)", repeated, err)
			}
		})
	}
}

func TestClearRefusesReadyCheckpoint(t *testing.T) {
	device, log, data, checkpoint := seed(8, storagewalcheckpointrecord.Ready)
	before := copyBlocks(device)
	receipt, err := Clear(device, log, data, checkpoint)
	if err != ErrCheckpointNotResetting || receipt != (Receipt{}) {
		t.Fatalf("Ready refusal=(%+v,%v)", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 || !reflect.DeepEqual(device.blocks, before) {
		t.Fatal("Ready refusal had effects")
	}
}

func TestClearCaptureRefusalsPrecedeEffects(t *testing.T) {
	device, log, data, checkpoint := seed(8, storagewalcheckpointrecord.Resetting)
	device.readErr[checkpoint.FirstBlock] = errClearRead
	if receipt, err := Clear(device, log, data, checkpoint); err != errClearRead || receipt != (Receipt{}) {
		t.Fatalf("capture refusal=(%+v,%v)", receipt, err)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatal("capture refusal caused writes")
	}
	delete(device.readErr, checkpoint.FirstBlock)
	device.badWidth[checkpoint.FirstBlock] = true
	if receipt, err := Clear(device, log, data, checkpoint); !errors.Is(err, storagepageio.ErrBlockWidthMismatch) || receipt != (Receipt{}) {
		t.Fatalf("width refusal=(%+v,%v)", receipt, err)
	}
}

func TestClearWriteRefusalReturnsExactPrefix(t *testing.T) {
	for _, width := range []int64{8, 13} {
		_, log, _, _ := seed(width, storagewalcheckpointrecord.Resetting)
		count := log.PageCount * log.BlocksPerPage
		for failed := int64(1); failed <= count; failed++ {
			t.Run(fmt.Sprintf("width-%d-block-%d", width, failed), func(t *testing.T) {
				device, log, data, checkpoint := seed(width, storagewalcheckpointrecord.Resetting)
				device.failAt = int(failed)
				receipt, err := Clear(device, log, data, checkpoint)
				if err != errClearWrite {
					t.Fatalf("error=%v", err)
				}
				want := Receipt{Checkpoint: record(storagewalcheckpointrecord.Resetting, 5, 91), BlocksWritten: failed - 1}
				if receipt != want || device.flushes != 0 {
					t.Fatalf("result=(%+v,%v) flushes=%d want=%+v", receipt, err, device.flushes, want)
				}
			})
		}
	}
}

func TestClearFlushRefusalDoesNotAcknowledge(t *testing.T) {
	device, log, data, checkpoint := seed(8, storagewalcheckpointrecord.Resetting)
	device.failFlush = true
	receipt, err := Clear(device, log, data, checkpoint)
	want := Receipt{Checkpoint: record(storagewalcheckpointrecord.Resetting, 5, 91), BlocksWritten: log.PageCount * log.BlocksPerPage}
	if err != errClearFlush || receipt != want || device.flushes != 1 {
		t.Fatalf("barrier refusal=(%+v,%v), want=%+v", receipt, err, want)
	}
}

type cutDevice struct {
	inner *hddblockdevice.HDD
	cut   hddwritebackcache.BarrierCut
}

func (d *cutDevice) Geometry() blockdevicecontract.Geometry { return d.inner.Geometry() }
func (d *cutDevice) ReadBlock(block int64) ([]byte, error)  { return d.inner.ReadBlock(block) }
func (d *cutDevice) WriteBlock(block int64, data []byte) error {
	return d.inner.WriteBlock(block, data)
}
func (d *cutDevice) Flush() error {
	receipt, err := d.inner.FlushWithCut(d.cut)
	if err != nil {
		return err
	}
	if !receipt.Acknowledged {
		return errClearLost
	}
	return nil
}

func seedHDD(t *testing.T) (*hddblockdevice.HDD, storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	t.Helper()
	log, data, checkpoint := layouts(8)
	if log.Geometry != (blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}) {
		t.Fatalf("HDD test geometry=%+v", log.Geometry)
	}
	device := hddblockdevice.New()
	for block := int64(0); block < log.Geometry.BlockCount; block++ {
		value := make([]byte, 8)
		for offset := range value {
			value[offset] = byte((block*17 + int64(offset)*3 + 1) & 0xff)
		}
		if err := device.WriteBlock(block, value); err != nil {
			t.Fatal(err)
		}
	}
	pager, err := storagepageio.Open(device, checkpoint)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := pager.WritePage(0, page(8, record(storagewalcheckpointrecord.Resetting, 5, 91))); err != nil {
		t.Fatal(err)
	}
	if _, err := pager.WritePage(1, page(8, record(storagewalcheckpointrecord.Resetting, 4, 80))); err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	return device, log, data, checkpoint
}

func hddImage(t *testing.T, device *hddblockdevice.HDD) [hddwritecrashcuts.SectorCount][hddwritecrashcuts.BytesPerSector]byte {
	t.Helper()
	var result [hddwritecrashcuts.SectorCount][hddwritecrashcuts.BytesPerSector]byte
	for block := int64(0); block < hddwritecrashcuts.SectorCount; block++ {
		value, err := device.ReadBlock(block)
		if err != nil {
			t.Fatal(err)
		}
		copy(result[block][:], value)
	}
	return result
}

func TestClearHDDInterruptedPrefixesAndRestart(t *testing.T) {
	log, _, _ := layouts(8)
	cutCases := []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BeforeFirstSector}, {Kind: hddwritebackcache.AfterAllMedia}}
	blockCount := log.PageCount * log.BlocksPerPage
	for ordinal := int64(0); ordinal < blockCount; ordinal++ {
		for prefix := 1; prefix < hddwritecrashcuts.BytesPerSector; prefix++ {
			value := prefix
			cutCases = append(cutCases, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value}})
		}
		cutCases = append(cutCases, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})
	}
	for cutIndex, cut := range cutCases {
		t.Run(fmt.Sprintf("cut-%d", cutIndex), func(t *testing.T) {
			inner, log, data, checkpoint := seedHDD(t)
			before := hddImage(t, inner)
			device := &cutDevice{inner: inner, cut: cut}
			receipt, err := Clear(device, log, data, checkpoint)
			if err != errClearLost || receipt.Acknowledged || receipt.BlocksWritten != log.PageCount*log.BlocksPerPage {
				t.Fatalf("cut=(%+v,%v)", receipt, err)
			}
			want := before
			if cut.Kind == hddwritebackcache.AfterAllMedia {
				for block := log.FirstBlock; block < log.FirstBlock+receipt.BlocksWritten; block++ {
					want[block] = [hddwritecrashcuts.BytesPerSector]byte{}
				}
			} else if cut.Kind == hddwritebackcache.AtSector {
				for block := log.FirstBlock; block < log.FirstBlock+cut.SectorIndex; block++ {
					want[block] = [hddwritecrashcuts.BytesPerSector]byte{}
				}
				if cut.MediaCut.Kind == hddwritecrashcuts.DuringMediaWrite {
					for offset := 0; offset < *cut.MediaCut.WrittenPrefix; offset++ {
						want[log.FirstBlock+cut.SectorIndex][offset] = 0
					}
				} else {
					want[log.FirstBlock+cut.SectorIndex] = [hddwritecrashcuts.BytesPerSector]byte{}
				}
			}
			if got := hddImage(t, inner); got != want {
				t.Fatalf("durable image differs for %+v", cut)
			}
			restarted, err := inner.Restart()
			if err != nil {
				t.Fatal(err)
			}
			captured, err := storagewalcheckpointcapture.Capture(restarted, log, data, checkpoint)
			if err != nil || captured.Selection.Record != record(storagewalcheckpointrecord.Resetting, 5, 91) {
				t.Fatalf("restart checkpoint=(%+v,%v)", captured.Selection, err)
			}
			retry, err := Clear(restarted, log, data, checkpoint)
			if err != nil || !retry.Acknowledged || retry.BlocksWritten != receipt.BlocksWritten {
				t.Fatalf("restart retry=(%+v,%v)", retry, err)
			}
		})
	}
}

func FuzzClearResettingGateAndWritePrefix(f *testing.F) {
	for _, seedBytes := range [][]byte{{0, 0}, {1, 0}, {2, 3}, {3, 19}} {
		f.Add(seedBytes)
	}
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 64 {
			t.Skip()
		}
		width := int64(8)
		if len(input) > 0 && input[0]&0x80 != 0 {
			width = 13
		}
		phase := storagewalcheckpointrecord.Resetting
		if len(input) > 0 && input[0]&1 != 0 {
			phase = storagewalcheckpointrecord.Ready
		}
		device, log, data, checkpoint := seed(width, phase)
		count := int(log.PageCount * log.BlocksPerPage)
		failAt := 0
		if len(input) > 1 && input[1] != 0 {
			failAt = int(input[1])%count + 1
		}
		device.failAt = failAt
		receipt, err := Clear(device, log, data, checkpoint)
		if phase == storagewalcheckpointrecord.Ready {
			if err != ErrCheckpointNotResetting || receipt != (Receipt{}) || len(device.writes) != 0 || device.flushes != 0 {
				t.Fatalf("Ready model mismatch (%+v,%v)", receipt, err)
			}
			return
		}
		if failAt != 0 {
			want := Receipt{Checkpoint: record(phase, 5, 91), BlocksWritten: int64(failAt - 1)}
			if err != errClearWrite || receipt != want || device.flushes != 0 {
				t.Fatalf("write-prefix model mismatch (%+v,%v)", receipt, err)
			}
			return
		}
		want := Receipt{Checkpoint: record(phase, 5, 91), BlocksWritten: int64(count), Acknowledged: true}
		if err != nil || receipt != want || device.flushes != 1 {
			t.Fatalf("success model mismatch (%+v,%v)", receipt, err)
		}
	})
}
