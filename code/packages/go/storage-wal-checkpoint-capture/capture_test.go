package storagewalcheckpointcapture

import (
	"bytes"
	"errors"
	"fmt"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointlayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-layout"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointselection "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-selection"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
)

var errReadInjected = errors.New("read failure")

type spyDevice struct {
	geometry blockdevicecontract.Geometry
	blocks   map[int64][]byte
	reads    []int64
	writes   []int64
	flushes  int
	readErr  map[int64]error
	badWidth map[int64]bool
}

func newSpy(g blockdevicecontract.Geometry) *spyDevice {
	return &spyDevice{geometry: g, blocks: make(map[int64][]byte), readErr: make(map[int64]error), badWidth: make(map[int64]bool)}
}
func (d *spyDevice) Geometry() blockdevicecontract.Geometry { return d.geometry }
func (d *spyDevice) ReadBlock(block int64) ([]byte, error) {
	d.reads = append(d.reads, block)
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
func (d *spyDevice) WriteBlock(block int64, data []byte) error {
	d.writes = append(d.writes, block)
	d.blocks[block] = append([]byte(nil), data...)
	return nil
}
func (d *spyDevice) Flush() error { d.flushes++; return nil }

func layouts(width int64) (storagepagelayout.Layout, storagepagelayout.Layout, storagepagelayout.Layout) {
	logBPP := (storagewalpageslot.MinPageBytes + width + width - 1) / width
	checkpointBPP := int64(1 + (storagewalcheckpointrecord.FrameBytes-1)/int(width))
	logLength := 3 * logBPP
	checkpointLength := 2 * checkpointBPP
	g := blockdevicecontract.Geometry{BlockCount: logLength + 1 + checkpointLength, BytesPerBlock: width}
	return storagepagelayout.Layout{Geometry: g, FirstBlock: 0, PageCount: 3, BlocksPerPage: logBPP},
		storagepagelayout.Layout{Geometry: g, FirstBlock: logLength, PageCount: 1, BlocksPerPage: 1},
		storagepagelayout.Layout{Geometry: g, FirstBlock: logLength + 1, PageCount: 2, BlocksPerPage: checkpointBPP}
}
func rec(phase storagewalcheckpointrecord.Phase, epoch, previous uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: phase, Epoch: epoch, PreviousLSN: previous}
}
func page(width int64, frame []byte, padding byte) []byte {
	result := make([]byte, int(width))
	copy(result, frame)
	if width > int64(len(frame)) {
		result[len(frame)] = padding
	}
	return result
}
func installPage(d *spyDevice, layout storagepagelayout.Layout, pageIndex int64, value []byte) {
	span, err := layout.Span(pageIndex)
	if err != nil {
		panic(err)
	}
	for block := int64(0); block < span.BlockCount; block++ {
		start := int(block * layout.Geometry.BytesPerBlock)
		end := start + int(layout.Geometry.BytesPerBlock)
		d.blocks[span.FirstBlock+block] = append([]byte(nil), value[start:end]...)
	}
}
func checkpointPage(width int64, record storagewalcheckpointrecord.Record, padding byte) []byte {
	frame, err := storagewalcheckpointrecord.Encode(record)
	if err != nil {
		panic(err)
	}
	result := make([]byte, int(1+(len(frame)-1)/int(width))*int(width))
	copy(result, frame)
	if len(result) > len(frame) {
		result[len(frame)] = padding
	}
	return result
}
func installSlot(d *spyDevice, checkpoint storagepagelayout.Layout, ordinal int64, record storagewalcheckpointrecord.Record, padding byte) {
	installPage(d, checkpoint, ordinal, checkpointPage(checkpoint.Geometry.BytesPerBlock, record, padding))
}

func TestCaptureReadOrderAndSelect(t *testing.T) {
	for _, width := range []int64{8, 16, 53} {
		t.Run(fmt.Sprintf("width-%d", width), func(t *testing.T) {
			log, data, checkpoint := layouts(width)
			device := newSpy(log.Geometry)
			old := rec(storagewalcheckpointrecord.Resetting, 4, 11)
			newer := rec(storagewalcheckpointrecord.Ready, 5, 12)
			installSlot(device, checkpoint, 0, old, 0)
			installSlot(device, checkpoint, 1, newer, 0)
			snapshot, err := Capture(device, log, data, checkpoint)
			if err != nil || snapshot.Selection != (storagewalcheckpointselection.Selection{Slot: 1, Record: newer}) {
				t.Fatalf("Capture=(%+v,%v)", snapshot, err)
			}
			first, _ := checkpoint.Span(0)
			second, _ := checkpoint.Span(1)
			wantReads := make([]int64, 0, int(first.BlockCount+second.BlockCount))
			for block := int64(0); block < first.BlockCount; block++ {
				wantReads = append(wantReads, first.FirstBlock+block)
			}
			for block := int64(0); block < second.BlockCount; block++ {
				wantReads = append(wantReads, second.FirstBlock+block)
			}
			if !reflect.DeepEqual(device.reads, wantReads) {
				t.Fatalf("read order=%v want=%v", device.reads, wantReads)
			}
			if len(device.writes) != 0 || device.flushes != 0 {
				t.Fatalf("capture performed writes/flushes %v/%d", device.writes, device.flushes)
			}
			before := append([]byte(nil), device.blocks[first.FirstBlock]...)
			snapshot.Pages[0][0] ^= 0xff
			if !reflect.DeepEqual(device.blocks[first.FirstBlock], before) {
				t.Fatal("snapshot page aliases device storage")
			}
		})
	}
}

func TestCaptureStopsAtEveryReadRefusal(t *testing.T) {
	log, data, checkpoint := layouts(16)
	first, _ := checkpoint.Span(0)
	second, _ := checkpoint.Span(1)
	for failBlock := first.FirstBlock; failBlock < second.FirstBlock+second.BlockCount; failBlock++ {
		t.Run(fmt.Sprintf("block-%d", failBlock), func(t *testing.T) {
			device := newSpy(log.Geometry)
			device.readErr[failBlock] = errReadInjected
			snapshot, err := Capture(device, log, data, checkpoint)
			if err != errReadInjected || !reflect.DeepEqual(snapshot, Snapshot{}) {
				t.Fatalf("result=(%+v,%v)", snapshot, err)
			}
			if device.reads[len(device.reads)-1] != failBlock || int64(len(device.reads)) != failBlock-first.FirstBlock+1 {
				t.Fatalf("read stopped incorrectly: %v", device.reads)
			}
			if len(device.writes) != 0 || device.flushes != 0 {
				t.Fatal("read refusal caused media effects")
			}
		})
	}
	device := newSpy(log.Geometry)
	device.badWidth[first.FirstBlock] = true
	snapshot, err := Capture(device, log, data, checkpoint)
	if !reflect.DeepEqual(snapshot, Snapshot{}) || !errors.Is(err, storagepageio.ErrBlockWidthMismatch) || len(device.reads) != 1 {
		t.Fatalf("width refusal=(%+v,%v), reads=%v", snapshot, err, device.reads)
	}
}

func TestCapturePaddingPrecedesFrameSelection(t *testing.T) {
	log, data, checkpoint := layouts(16)
	device := newSpy(log.Geometry)
	invalidFrame := make([]byte, storagewalcheckpointrecord.FrameBytes)
	pageBytes := checkpointPage(16, rec(storagewalcheckpointrecord.Ready, 1, 0), 0)
	copy(pageBytes, invalidFrame)
	pageBytes[40] = 0x7f
	installPage(device, checkpoint, 0, pageBytes)
	snapshot, err := Capture(device, log, data, checkpoint)
	if !reflect.DeepEqual(snapshot, Snapshot{}) || !errors.Is(err, ErrCheckpointPadding) {
		t.Fatalf("padding/frame precedence=(%+v,%v)", snapshot, err)
	}
	device = newSpy(log.Geometry)
	staged := checkpointPage(16, rec(storagewalcheckpointrecord.Resetting, 2, 0), 0)
	staged[18] = 0xa5
	staged[40] = 1
	installPage(device, checkpoint, 0, staged)
	if snapshot, err = Capture(device, log, data, checkpoint); !reflect.DeepEqual(snapshot, Snapshot{}) || !errors.Is(err, ErrCheckpointPadding) {
		t.Fatalf("staged padding=(%+v,%v)", snapshot, err)
	}
}

func TestCaptureLayoutAndDevicePrecedence(t *testing.T) {
	log, data, checkpoint := layouts(8)
	bad := checkpoint
	bad.PageCount = 1
	if snapshot, err := Capture(nil, log, data, bad); !reflect.DeepEqual(snapshot, Snapshot{}) || !errors.Is(err, storagewalcheckpointlayout.ErrCheckpointShape) {
		t.Fatalf("layout-before-device=(%+v,%v)", snapshot, err)
	}
	if snapshot, err := Capture(nil, log, data, checkpoint); !reflect.DeepEqual(snapshot, Snapshot{}) || !errors.Is(err, storagepageio.ErrInvalidDevice) {
		t.Fatalf("nil device=(%+v,%v)", snapshot, err)
	}
	other := newSpy(log.Geometry)
	other.geometry.BlockCount++
	if snapshot, err := Capture(other, log, data, checkpoint); !reflect.DeepEqual(snapshot, Snapshot{}) || !errors.Is(err, storagepageio.ErrGeometryMismatch) {
		t.Fatalf("device geometry=(%+v,%v)", snapshot, err)
	}
	device := newSpy(log.Geometry)
	installSlot(device, checkpoint, 0, rec(storagewalcheckpointrecord.Resetting, 3, 1), 0)
	device.blocks[checkpoint.FirstBlock+int64(storagewalcheckpointslot.GuardOffset)/log.Geometry.BytesPerBlock][storagewalcheckpointslot.GuardOffset%int(log.Geometry.BytesPerBlock)] = storagewalcheckpointslot.StagedGuard
	device.blocks[checkpoint.FirstBlock+checkpoint.BlocksPerPage] = make([]byte, int(log.Geometry.BytesPerBlock))
	selection, err := Capture(device, log, data, checkpoint)
	if !errors.Is(err, storagewalcheckpointselection.ErrNoPublishedCheckpoint) || !reflect.DeepEqual(selection, Snapshot{}) {
		t.Fatalf("selection refusal=(%+v,%v)", selection, err)
	}
}

func TestCaptureMemoryAndHDDAfterRestart(t *testing.T) {
	log, data, checkpoint := layouts(8)
	for _, backend := range []string{"memory", "hdd"} {
		t.Run(backend, func(t *testing.T) {
			var device blockdevicecontract.Device
			var restart func() (blockdevicecontract.Device, error)
			if backend == "memory" {
				memory, err := memoryblockdevice.New(log.Geometry)
				if err != nil {
					t.Fatal(err)
				}
				device = memory
				restart = func() (blockdevicecontract.Device, error) { fresh, err := memory.Restart(); return fresh, err }
			} else {
				hdd := hddblockdevice.New()
				device = hdd
				restart = func() (blockdevicecontract.Device, error) { fresh, err := hdd.Restart(); return fresh, err }
			}
			pager, err := storagepageio.Open(device, checkpoint)
			if err != nil {
				t.Fatal(err)
			}
			for pageIndex, record := range []storagewalcheckpointrecord.Record{rec(storagewalcheckpointrecord.Resetting, 8, 31), rec(storagewalcheckpointrecord.Ready, 8, 31)} {
				frame, err := storagewalcheckpointrecord.Encode(record)
				if err != nil {
					t.Fatal(err)
				}
				page := make([]byte, 40)
				copy(page, frame)
				if _, err := pager.WritePage(int64(pageIndex), page); err != nil {
					t.Fatal(err)
				}
			}
			if err := pager.Flush(); err != nil {
				t.Fatal(err)
			}
			for turn := 0; turn < 2; turn++ {
				snapshot, err := Capture(device, log, data, checkpoint)
				if err != nil || snapshot.Selection.Slot != 1 || snapshot.Selection.Record.Phase != storagewalcheckpointrecord.Ready {
					t.Fatalf("Capture=(%+v,%v)", snapshot, err)
				}
				device, err = restart()
				if err != nil {
					t.Fatal(err)
				}
			}
		})
	}
}

func FuzzCaptureCheckpointPages(f *testing.F) {
	log, data, checkpoint := layouts(8)
	valid0, _ := storagewalcheckpointrecord.Encode(rec(storagewalcheckpointrecord.Resetting, 1, 0))
	valid1, _ := storagewalcheckpointrecord.Encode(rec(storagewalcheckpointrecord.Ready, 2, 0))
	seed := append(append([]byte(nil), valid0...), valid1...)
	f.Add(seed)
	f.Add([]byte{})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 128 {
			t.Skip()
		}
		device := newSpy(log.Geometry)
		for slot := 0; slot < 2; slot++ {
			page := make([]byte, int(checkpoint.BlocksPerPage*checkpoint.Geometry.BytesPerBlock))
			start := slot * len(page)
			if start < len(raw) {
				end := start + len(page)
				if end > len(raw) {
					end = len(raw)
				}
				copy(page, raw[start:end])
			}
			installPage(device, checkpoint, int64(slot), page)
		}
		beforeReads := len(device.reads)
		snapshot, err := Capture(device, log, data, checkpoint)
		if err == nil {
			if snapshot.Selection.Slot > 1 || len(snapshot.Pages[0]) != 40 || len(snapshot.Pages[1]) != 40 {
				t.Fatalf("invalid successful snapshot=%+v", snapshot)
			}
		}
		if len(device.reads) < beforeReads || len(device.writes) != 0 || device.flushes != 0 {
			t.Fatal("capture violated read-only behavior")
		}
	})
}

func cloneSpy(source *spyDevice) *spyDevice {
	result := newSpy(source.geometry)
	for block, value := range source.blocks {
		result.blocks[block] = append([]byte(nil), value...)
	}
	for block, err := range source.readErr {
		result.readErr[block] = err
	}
	for block, bad := range source.badWidth {
		result.badWidth[block] = bad
	}
	return result
}

func TestReadPagesAdmitsRawPrefixes(t *testing.T) {
	log, data, checkpoint := layouts(8)
	t.Run("vacant", func(t *testing.T) {
		device := newSpy(log.Geometry)
		pages, err := ReadPages(device, log, data, checkpoint)
		if err != nil || !reflect.DeepEqual(pages, [2][]byte{make([]byte, 40), make([]byte, 40)}) {
			t.Fatalf("ReadPages vacant=(%x,%v)", pages, err)
		}
		if _, err := Capture(device, log, data, checkpoint); !errors.Is(err, storagewalcheckpointselection.ErrNoPublishedCheckpoint) {
			t.Fatalf("Capture vacant error=%v", err)
		}
	})
	t.Run("staged", func(t *testing.T) {
		device := newSpy(log.Geometry)
		frame, err := storagewalcheckpointslot.Stage(rec(storagewalcheckpointrecord.Resetting, 3, 2))
		if err != nil {
			t.Fatal(err)
		}
		installPage(device, checkpoint, 0, frame)
		want := [2][]byte{append([]byte(nil), frame...), make([]byte, 40)}
		pages, err := ReadPages(device, log, data, checkpoint)
		if err != nil || !reflect.DeepEqual(pages, want) {
			t.Fatalf("ReadPages staged=(%x,%v)", pages, err)
		}
		if _, err := Capture(cloneSpy(device), log, data, checkpoint); !errors.Is(err, storagewalcheckpointselection.ErrNoPublishedCheckpoint) {
			t.Fatalf("Capture staged error=%v", err)
		}
	})
	t.Run("corrupt-frame", func(t *testing.T) {
		device := newSpy(log.Geometry)
		corrupt := make([]byte, 40)
		corrupt[0] = 0x7f
		installPage(device, checkpoint, 1, corrupt)
		pages, err := ReadPages(device, log, data, checkpoint)
		want := [2][]byte{make([]byte, 40), corrupt}
		if err != nil || !reflect.DeepEqual(pages, want) {
			t.Fatalf("ReadPages corrupt=(%x,%v)", pages, err)
		}
		if _, err := Capture(cloneSpy(device), log, data, checkpoint); err == nil {
			t.Fatal("Capture accepted corrupt frame")
		}
	})
}

func TestReadPagesTraceAndOwnershipMatchCapture(t *testing.T) {
	for _, width := range []int64{8, 13, 53} {
		t.Run(fmt.Sprintf("width-%d", width), func(t *testing.T) {
			log, data, checkpoint := layouts(width)
			base := newSpy(log.Geometry)
			installSlot(base, checkpoint, 0, rec(storagewalcheckpointrecord.Resetting, 4, 11), 0)
			installSlot(base, checkpoint, 1, rec(storagewalcheckpointrecord.Ready, 5, 12), 0)
			rawDevice, captureDevice := cloneSpy(base), cloneSpy(base)
			pages, err := ReadPages(rawDevice, log, data, checkpoint)
			if err != nil {
				t.Fatal(err)
			}
			snapshot, err := Capture(captureDevice, log, data, checkpoint)
			if err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(pages, snapshot.Pages) {
				t.Fatal("raw/capture page pair differs")
			}
			if !reflect.DeepEqual(rawDevice.reads, captureDevice.reads) {
				t.Fatalf("trace raw=%v capture=%v", rawDevice.reads, captureDevice.reads)
			}
			original := append([]byte(nil), rawDevice.blocks[checkpoint.FirstBlock]...)
			pages[0][0] ^= 0xff
			if !reflect.DeepEqual(rawDevice.blocks[checkpoint.FirstBlock], original) {
				t.Fatal("ReadPages result aliases device block")
			}
		})
	}
}

func TestReadPagesRejectsNonzeroPadding(t *testing.T) {
	log, data, checkpoint := layouts(13)
	for _, ordinal := range []int64{0, 1} {
		t.Run(fmt.Sprintf("page-%d", ordinal), func(t *testing.T) {
			device := newSpy(log.Geometry)
			pageBytes := make([]byte, int(checkpoint.BlocksPerPage*checkpoint.Geometry.BytesPerBlock))
			pageBytes[storagewalcheckpointrecord.FrameBytes] = 0x5a
			installPage(device, checkpoint, ordinal, pageBytes)
			pages, err := ReadPages(device, log, data, checkpoint)
			if !errors.Is(err, ErrCheckpointPadding) || !reflect.DeepEqual(pages, [2][]byte{}) {
				t.Fatalf("padding=(%x,%v)", pages, err)
			}
			captureDevice := cloneSpy(device)
			if snapshot, captureErr := Capture(captureDevice, log, data, checkpoint); !errors.Is(captureErr, ErrCheckpointPadding) || !reflect.DeepEqual(snapshot, Snapshot{}) {
				t.Fatalf("Capture padding=(%+v,%v)", snapshot, captureErr)
			}
			if !reflect.DeepEqual(device.reads, captureDevice.reads) {
				t.Fatalf("read traces differ: %v / %v", device.reads, captureDevice.reads)
			}
		})
	}
}

func TestReadPagesRefusalParity(t *testing.T) {
	log, data, checkpoint := layouts(16)
	first, _ := checkpoint.Span(0)
	second, _ := checkpoint.Span(1)
	for failBlock := first.FirstBlock; failBlock < second.FirstBlock+second.BlockCount; failBlock++ {
		t.Run(fmt.Sprintf("read-%d", failBlock), func(t *testing.T) {
			base := newSpy(log.Geometry)
			base.readErr[failBlock] = errReadInjected
			rawDevice, captureDevice := cloneSpy(base), cloneSpy(base)
			pages, rawErr := ReadPages(rawDevice, log, data, checkpoint)
			snapshot, captureErr := Capture(captureDevice, log, data, checkpoint)
			if rawErr != errReadInjected || captureErr != rawErr || !reflect.DeepEqual(pages, [2][]byte{}) || !reflect.DeepEqual(snapshot, Snapshot{}) {
				t.Fatalf("read refusal raw=(%x,%v) capture=(%+v,%v)", pages, rawErr, snapshot, captureErr)
			}
			if !reflect.DeepEqual(rawDevice.reads, captureDevice.reads) {
				t.Fatalf("trace mismatch %v / %v", rawDevice.reads, captureDevice.reads)
			}
		})
	}
	base := newSpy(log.Geometry)
	base.badWidth[first.FirstBlock] = true
	rawDevice, captureDevice := cloneSpy(base), cloneSpy(base)
	pages, rawErr := ReadPages(rawDevice, log, data, checkpoint)
	snapshot, captureErr := Capture(captureDevice, log, data, checkpoint)
	if !errors.Is(rawErr, storagepageio.ErrBlockWidthMismatch) || captureErr != rawErr || !reflect.DeepEqual(pages, [2][]byte{}) || !reflect.DeepEqual(snapshot, Snapshot{}) {
		t.Fatalf("width refusal raw=(%x,%v) capture=(%+v,%v)", pages, rawErr, snapshot, captureErr)
	}
	bad := checkpoint
	bad.PageCount = 1
	if pages, err := ReadPages(nil, log, data, bad); !errors.Is(err, storagewalcheckpointlayout.ErrCheckpointShape) || !reflect.DeepEqual(pages, [2][]byte{}) {
		t.Fatalf("layout precedence=(%x,%v)", pages, err)
	}
	if snapshot, err := Capture(nil, log, data, bad); !errors.Is(err, storagewalcheckpointlayout.ErrCheckpointShape) || !reflect.DeepEqual(snapshot, Snapshot{}) {
		t.Fatalf("Capture layout precedence=(%+v,%v)", snapshot, err)
	}
	if pages, err := ReadPages(nil, log, data, checkpoint); !errors.Is(err, storagepageio.ErrInvalidDevice) || !reflect.DeepEqual(pages, [2][]byte{}) {
		t.Fatalf("nil device=(%x,%v)", pages, err)
	}
	if snapshot, err := Capture(nil, log, data, checkpoint); !errors.Is(err, storagepageio.ErrInvalidDevice) || !reflect.DeepEqual(snapshot, Snapshot{}) {
		t.Fatalf("Capture nil device=(%+v,%v)", snapshot, err)
	}
	other := newSpy(log.Geometry)
	other.geometry.BlockCount++
	if pages, err := ReadPages(other, log, data, checkpoint); !errors.Is(err, storagepageio.ErrGeometryMismatch) || !reflect.DeepEqual(pages, [2][]byte{}) {
		t.Fatalf("device geometry=(%x,%v)", pages, err)
	}
	other = newSpy(log.Geometry)
	other.geometry.BlockCount++
	if snapshot, err := Capture(other, log, data, checkpoint); !errors.Is(err, storagepageio.ErrGeometryMismatch) || !reflect.DeepEqual(snapshot, Snapshot{}) {
		t.Fatalf("Capture device geometry=(%+v,%v)", snapshot, err)
	}
}

func FuzzReadPagesRawPrefixes(f *testing.F) {
	f.Add([]byte{})
	f.Add(make([]byte, 80))
	f.Add(bytes.Repeat([]byte{0xa5}, 80))
	for _, phase := range []storagewalcheckpointrecord.Phase{storagewalcheckpointrecord.Resetting, storagewalcheckpointrecord.Ready} {
		frame, _ := storagewalcheckpointrecord.Encode(rec(phase, 2, 1))
		f.Add(append(append([]byte(nil), frame...), make([]byte, 40)...))
	}
	log, data, checkpoint := layouts(8)
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 80 {
			t.Skip()
		}
		device := newSpy(log.Geometry)
		var want [2][]byte
		for ordinal := 0; ordinal < 2; ordinal++ {
			want[ordinal] = make([]byte, 40)
			start := ordinal * 40
			if start < len(input) {
				end := start + 40
				if end > len(input) {
					end = len(input)
				}
				copy(want[ordinal], input[start:end])
			}
			installPage(device, checkpoint, int64(ordinal), want[ordinal])
		}
		pages, err := ReadPages(device, log, data, checkpoint)
		if err != nil || !reflect.DeepEqual(pages, want) {
			t.Fatalf("raw-prefix model=(%x,%v), want=%x", pages, err, want)
		}
	})
}
