package storagewalcommit

import (
	"errors"
	"fmt"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpreparecommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-prepare-commit"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalredoapply "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-apply"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

var errInjected = errors.New("injected device error")

type recordingDevice struct {
	base      blockdevicecontract.Device
	writes    []int64
	flushes   int
	failWrite int64
	failFlush int
	loseFlush bool
}

func (device *recordingDevice) Geometry() blockdevicecontract.Geometry { return device.base.Geometry() }
func (device *recordingDevice) ReadBlock(block int64) ([]byte, error) {
	return device.base.ReadBlock(block)
}
func (device *recordingDevice) WriteBlock(block int64, data []byte) error {
	device.writes = append(device.writes, block)
	if block == device.failWrite {
		return errInjected
	}
	return device.base.WriteBlock(block, data)
}
func (device *recordingDevice) Flush() error {
	device.flushes++
	if device.flushes == device.failFlush {
		if device.loseFlush {
			if err := device.base.Flush(); err != nil {
				return err
			}
		}
		return errInjected
	}
	return device.base.Flush()
}

func newRecording(base blockdevicecontract.Device) *recordingDevice {
	return &recordingDevice{base: base, failWrite: -1}
}
func geometry() blockdevicecontract.Geometry {
	return blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
}
func layouts() (storagepagelayout.Layout, storagepagelayout.Layout) {
	g := geometry()
	return storagepagelayout.Layout{Geometry: g, FirstBlock: 0, PageCount: 4, BlocksPerPage: 7}, storagepagelayout.Layout{Geometry: g, FirstBlock: 28, PageCount: 4, BlocksPerPage: 1}
}
func wideLayouts() (storagepagelayout.Layout, storagepagelayout.Layout, blockdevicecontract.Geometry) {
	g := blockdevicecontract.Geometry{BlockCount: 64, BytesPerBlock: 8}
	return storagepagelayout.Layout{Geometry: g, FirstBlock: 0, PageCount: 8, BlocksPerPage: 7}, storagepagelayout.Layout{Geometry: g, FirstBlock: 56, PageCount: 8, BlocksPerPage: 1}, g
}
func imageSet() []storagewalpreparecommit.PageImage {
	return []storagewalpreparecommit.PageImage{
		{PageID: 1, Image: []byte{1, 3, 5, 7, 9, 11, 13, 15}},
		{PageID: 2, Image: []byte{2, 4, 6, 8, 10, 12, 14, 16}},
	}
}
func requireNoDataEffects(t *testing.T, device blockdevicecontract.Device, data storagepagelayout.Layout) {
	t.Helper()
	pager, err := storagepageio.Open(device, data)
	if err != nil {
		t.Fatal(err)
	}
	for page := int64(0); page < data.PageCount; page++ {
		got, err := pager.ReadPage(page)
		if err != nil {
			t.Fatal(err)
		}
		if !reflect.DeepEqual(got, make([]byte, int(data.BlocksPerPage*data.Geometry.BytesPerBlock))) {
			t.Fatalf("data page %d changed: %v", page, got)
		}
	}
}

func TestCommitPublishesBeforeOrderedReplayAndPreservesBothReceipts(t *testing.T) {
	log, data := layouts()
	base, err := memoryblockdevice.New(geometry())
	if err != nil {
		t.Fatal(err)
	}
	device := newRecording(base)
	images := imageSet()
	result, err := Commit(device, log, data, 33, images)
	if err != nil {
		t.Fatal(err)
	}
	if result.Log.AcknowledgedRecords != 4 || result.Log.LastLSN != 4 || !result.Log.LastAppend.Acknowledged || len(result.Log.Committed) != 4 {
		t.Fatalf("log receipt=%+v", result.Log)
	}
	if result.Data.AcknowledgedWrites != 2 || result.Data.Interrupted || result.Data.PageID != 0 || result.Data.RecordLSN != 0 {
		t.Fatalf("data receipt=%+v", result.Data)
	}
	if len(device.writes) != 30 || device.flushes != 6 {
		t.Fatalf("write/barrier counts=%d/%d", len(device.writes), device.flushes)
	}
	for i := int64(0); i < 28; i++ {
		if device.writes[i] != i {
			t.Fatalf("log write[%d]=%d", i, device.writes[i])
		}
	}
	if device.writes[28] != 29 || device.writes[29] != 30 {
		t.Fatalf("data writes=%v", device.writes[28:])
	}
	pager, err := storagepageio.Open(device, data)
	if err != nil {
		t.Fatal(err)
	}
	for _, want := range images {
		got, err := pager.ReadPage(want.PageID)
		if err != nil || !reflect.DeepEqual(got, want.Image) {
			t.Fatalf("page %d=%v err=%v", want.PageID, got, err)
		}
	}
	images[0].Image[0] = 99
	if result.Log.Committed[1].Image[0] != 1 {
		t.Fatal("log result aliases caller images")
	}
}

func TestCommitZeroImageTransactionHasNoDataEffects(t *testing.T) {
	log, data := layouts()
	base, _ := memoryblockdevice.New(geometry())
	device := newRecording(base)
	result, err := Commit(device, log, data, 44, nil)
	if err != nil {
		t.Fatal(err)
	}
	if result.Log.AcknowledgedRecords != 2 || len(result.Log.Committed) != 2 || result.Data != (storagewalredoapply.Receipt{}) {
		t.Fatalf("zero-image result=%+v", result)
	}
	if len(device.writes) != 14 || device.flushes != 2 {
		t.Fatalf("effects=%d writes %d flushes", len(device.writes), device.flushes)
	}
	requireNoDataEffects(t, device, data)
}

func TestCommitLogFailureReturnsLogReceiptBeforeAnyDataWrite(t *testing.T) {
	log, data := layouts()
	base, _ := memoryblockdevice.New(geometry())
	device := newRecording(base)
	device.failWrite = 3
	result, err := Commit(device, log, data, 55, imageSet())
	if err != errInjected || result.Log.AcknowledgedRecords != 0 || result.Log.LastLSN != 1 || result.Log.LastAppend.WrittenBlocks != 3 || result.Log.Committed != nil || result.Data != (storagewalredoapply.Receipt{}) {
		t.Fatalf("log interruption result=%+v err=%v", result, err)
	}
	if len(device.writes) != 4 || device.flushes != 0 {
		t.Fatalf("continued after log failure: writes=%v flushes=%d", device.writes, device.flushes)
	}
	requireNoDataEffects(t, base, data)
}

func TestCommitDataFailureRetainsAcknowledgedLogAndReplayReceipt(t *testing.T) {
	log, data := layouts()
	base, _ := memoryblockdevice.New(geometry())
	device := newRecording(base)
	device.failWrite = 29
	result, err := Commit(device, log, data, 66, imageSet())
	if err != errInjected || result.Log.AcknowledgedRecords != 4 || len(result.Log.Committed) != 4 || !result.Log.LastAppend.Acknowledged {
		t.Fatalf("log result lost after data failure: %+v err=%v", result.Log, err)
	}
	if result.Data != (storagewalredoapply.Receipt{Interrupted: true, PageID: 1, RecordLSN: 2, WrittenBlocks: 0}) {
		t.Fatalf("data interruption receipt=%+v", result.Data)
	}
	if len(device.writes) != 29 || device.flushes != 4 {
		t.Fatalf("effects after data failure writes=%d flushes=%d", len(device.writes), device.flushes)
	}
	pager, _ := storagepageio.Open(base, log)
	pages := make([][]byte, log.PageCount)
	for i := range pages {
		pages[i], err = pager.ReadPage(int64(i))
		if err != nil {
			t.Fatal(err)
		}
	}
	scan, err := storagewalscan.ScanPages(pages)
	if err != nil || scan.Tail != storagewalscan.Full || len(scan.Records) != 4 {
		t.Fatalf("committed WAL scan=%+v err=%v", scan, err)
	}
}

func TestCommitLostLogAcknowledgementDoesNotReplayData(t *testing.T) {
	log, data := layouts()
	base, _ := memoryblockdevice.New(geometry())
	device := newRecording(base)
	device.failFlush, device.loseFlush = 4, true
	result, err := Commit(device, log, data, 77, imageSet())
	if err != errInjected || result.Log.AcknowledgedRecords != 3 || result.Log.LastLSN != 4 || result.Log.LastAppend.Acknowledged || result.Log.Committed != nil || result.Data != (storagewalredoapply.Receipt{}) {
		t.Fatalf("lost log ack result=%+v err=%v", result, err)
	}
	if len(device.writes) != 28 || device.flushes != 4 {
		t.Fatalf("lost ack effects writes=%d flushes=%d", len(device.writes), device.flushes)
	}
	requireNoDataEffects(t, base, data)
	restarted, err := base.Restart()
	if err != nil {
		t.Fatal(err)
	}
	pager, _ := storagepageio.Open(restarted, log)
	pages := make([][]byte, log.PageCount)
	for i := range pages {
		pages[i], err = pager.ReadPage(int64(i))
		if err != nil {
			t.Fatal(err)
		}
	}
	scan, err := storagewalscan.ScanPages(pages)
	if err != nil || scan.Tail != storagewalscan.Full || len(scan.Records) != 4 || scan.Records[3].Kind != storagewalrecord.CommitTransaction {
		t.Fatalf("durable commit after lost response: scan=%+v err=%v", scan, err)
	}
}

func TestCommitMemoryAndHDDConvergeAndRestart(t *testing.T) {
	log, data := layouts()
	for name, makeDevice := range map[string]func() blockdevicecontract.Device{
		"memory": func() blockdevicecontract.Device { value, _ := memoryblockdevice.New(geometry()); return value },
		"hdd":    func() blockdevicecontract.Device { return hddblockdevice.New() },
	} {
		t.Run(name, func(t *testing.T) {
			dev := makeDevice()
			if dev.Geometry() != geometry() {
				t.Skipf("backend geometry differs: %+v", dev.Geometry())
			}
			result, err := Commit(dev, log, data, 88, imageSet())
			if err != nil || result.Data.AcknowledgedWrites != 2 {
				t.Fatalf("Commit=(%+v,%v)", result, err)
			}
			if name == "memory" {
				fresh, err := dev.(*memoryblockdevice.Memory).Restart()
				if err != nil {
					t.Fatal(err)
				}
				pager, _ := storagepageio.Open(fresh, data)
				for _, image := range imageSet() {
					got, err := pager.ReadPage(image.PageID)
					if err != nil || !reflect.DeepEqual(got, image.Image) {
						t.Fatalf("restart page=%v err=%v", got, err)
					}
				}
			}
		})
	}
}

func FuzzCommitHistories(f *testing.F) {
	for _, seed := range [][]byte{{0}, {1, 1}, {2, 0, 2}, {3, 1, 0, 1}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) == 0 || len(commands) > 24 {
			t.Skip()
		}
		log, data := layouts()
		base, _ := memoryblockdevice.New(geometry())
		device := newRecording(base)
		count := 1 + int(commands[0]%2)
		images := make([]storagewalpreparecommit.PageImage, count)
		for i := range images {
			page := int64(i + 1)
			if len(commands) > i+1 && commands[i+1]&1 != 0 {
				page = 1
			}
			value := make([]byte, 8)
			for j := range value {
				value[j] = byte(i*31 + j)
			}
			images[i] = storagewalpreparecommit.PageImage{PageID: page, Image: value}
		}
		if len(commands) > 2 && commands[2]&3 == 1 {
			device.failWrite = 29
		}
		if len(commands) > 2 && commands[2]&3 == 2 {
			device.failFlush = 4
		}
		_, err := Commit(device, log, data, 99, images)
		if err == nil && device.failWrite >= 0 {
			t.Fatal("data-write fault was not observed")
		}
	})
}

func TestCommitRepeatedPageImagesReplayInOriginalRecordOrder(t *testing.T) {
	log, data := layouts()
	base, _ := memoryblockdevice.New(geometry())
	device := newRecording(base)
	images := []storagewalpreparecommit.PageImage{
		{PageID: 2, Image: []byte{1, 1, 1, 1, 1, 1, 1, 1}},
		{PageID: 2, Image: []byte{2, 2, 2, 2, 2, 2, 2, 2}},
	}
	result, err := Commit(device, log, data, 100, images)
	if err != nil || result.Data.AcknowledgedWrites != 2 || result.Log.Committed[1].LSN != 2 || result.Log.Committed[2].LSN != 3 || result.Log.Committed[3].LSN != 4 {
		t.Fatalf("repeated page commit=%+v err=%v", result, err)
	}
	pager, _ := storagepageio.Open(device, data)
	got, err := pager.ReadPage(2)
	if err != nil || !reflect.DeepEqual(got, images[1].Image) {
		t.Fatalf("final repeated page=%v err=%v", got, err)
	}
}

func TestCommitAfterEarlierTransactionReplaysOnlyNewImages(t *testing.T) {
	log, data, g := wideLayouts()
	device, err := memoryblockdevice.New(g)
	if err != nil {
		t.Fatal(err)
	}
	first := []storagewalpreparecommit.PageImage{{PageID: 1, Image: []byte{1, 1, 1, 1, 1, 1, 1, 1}}}
	second := []storagewalpreparecommit.PageImage{{PageID: 2, Image: []byte{2, 2, 2, 2, 2, 2, 2, 2}}}
	firstResult, err := Commit(device, log, data, 101, first)
	if err != nil || firstResult.Log.Committed[0].LSN != 1 || firstResult.Data.AcknowledgedWrites != 1 {
		t.Fatalf("first transaction=%+v err=%v", firstResult, err)
	}
	secondResult, err := Commit(device, log, data, 102, second)
	if err != nil || secondResult.Log.Committed[0].LSN != 4 || secondResult.Log.Committed[1].LSN != 5 || secondResult.Log.Committed[2].LSN != 6 || secondResult.Data.AcknowledgedWrites != 1 {
		t.Fatalf("second transaction=%+v err=%v", secondResult, err)
	}
	pager, _ := storagepageio.Open(device, data)
	for page, want := range map[int64][]byte{1: first[0].Image, 2: second[0].Image} {
		got, err := pager.ReadPage(page)
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("page %d=%v err=%v", page, got, err)
		}
	}
}

func ExampleCommit() {
	fmt.Println("WAL commit before data replay")
	// Output: WAL commit before data replay
}
