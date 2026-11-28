package storagewalpublishcommit

import (
	"errors"
	"fmt"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalappend "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-append"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalpreparecommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-prepare-commit"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

var errInjected = errors.New("injected device failure")

type tracedDevice struct {
	geometry blockdevicecontract.Geometry
	blocks   map[int64][]byte
	reads    []int64
	writes   []int64
	flushes  int
	readErr  map[int64]error
	writeErr map[int64]error
	flushErr map[int]error
}

func newTraced(g blockdevicecontract.Geometry) *tracedDevice {
	return &tracedDevice{geometry: g, blocks: make(map[int64][]byte), readErr: make(map[int64]error), writeErr: make(map[int64]error), flushErr: make(map[int]error)}
}
func (d *tracedDevice) Geometry() blockdevicecontract.Geometry { return d.geometry }
func (d *tracedDevice) ReadBlock(block int64) ([]byte, error) {
	d.reads = append(d.reads, block)
	if err := d.readErr[block]; err != nil {
		return nil, err
	}
	if value, ok := d.blocks[block]; ok {
		return append([]byte(nil), value...), nil
	}
	return make([]byte, int(d.geometry.BytesPerBlock)), nil
}
func (d *tracedDevice) WriteBlock(block int64, data []byte) error {
	d.writes = append(d.writes, block)
	if err := d.writeErr[block]; err != nil {
		return err
	}
	d.blocks[block] = append([]byte(nil), data...)
	return nil
}
func (d *tracedDevice) Flush() error { d.flushes++; return d.flushErr[d.flushes] }
func geometry() blockdevicecontract.Geometry {
	return blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
}
func layouts() (storagepagelayout.Layout, storagepagelayout.Layout) {
	g := geometry()
	return storagepagelayout.Layout{Geometry: g, FirstBlock: 0, PageCount: 4, BlocksPerPage: 7}, storagepagelayout.Layout{Geometry: g, FirstBlock: 28, PageCount: 4, BlocksPerPage: 1}
}
func record(kind storagewalrecord.Kind, lsn, tx uint64, page int64, image []byte) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: kind, LSN: lsn, TxID: tx, PageID: page, Image: image}
}
func seedPage(t *testing.T, d *tracedDevice, page int64, r storagewalrecord.Record) {
	t.Helper()
	encoded, err := storagewalpageslot.EncodePage(56, r)
	if err != nil {
		t.Fatal(err)
	}
	for part := int64(0); part < 7; part++ {
		d.blocks[page*7+part] = append([]byte(nil), encoded[part*8:(part+1)*8]...)
	}
}
func seedPrefix(t *testing.T, d *tracedDevice, rows ...storagewalrecord.Record) {
	t.Helper()
	for i, row := range rows {
		seedPage(t, d, int64(i), row)
	}
}
func request() []storagewalpreparecommit.PageImage {
	return []storagewalpreparecommit.PageImage{{PageID: 1, Image: []byte{1, 3, 5, 7, 9, 11, 13, 15}}}
}
func assertFailureHasNoCommit(t *testing.T, result Result, err error) {
	t.Helper()
	if err == nil {
		t.Fatal("expected failure")
	}
	if result.Committed != nil {
		t.Fatalf("failed publication exposed commit=%+v", result.Committed)
	}
}

func TestPublishAcknowledgesOrderedTransactionAndProtectsDataRegion(t *testing.T) {
	log, data := layouts()
	device := newTraced(geometry())
	device.blocks[28] = []byte{8, 8, 8, 8, 8, 8, 8, 8}
	device.blocks[29] = []byte{9, 9, 9, 9, 9, 9, 9, 9}
	images := request()
	before := append([]byte(nil), images[0].Image...)
	result, err := Publish(device, log, data, 7, images)
	if err != nil {
		t.Fatal(err)
	}
	if result.AcknowledgedRecords != 3 || result.LastLSN != 3 || !result.LastAppend.Acknowledged || result.LastAppend.Page != 2 || result.LastAppend.WrittenBlocks != 7 || len(result.Committed) != 3 {
		t.Fatalf("result=%+v", result)
	}
	if result.Committed[0].Kind != storagewalrecord.BeginTransaction || result.Committed[0].LSN != 1 || result.Committed[1].Kind != storagewalrecord.WritePageImage || result.Committed[1].LSN != 2 || result.Committed[2].Kind != storagewalrecord.CommitTransaction || result.Committed[2].LSN != 3 {
		t.Fatalf("committed order=%+v", result.Committed)
	}
	if string(result.Committed[1].Image) != string(before) {
		t.Fatalf("committed image=%v", result.Committed[1].Image)
	}
	images[0].Image[0] = 88
	if result.Committed[1].Image[0] != before[0] {
		t.Fatal("committed image aliases caller")
	}
	if len(device.writes) != 21 || device.flushes != 3 {
		t.Fatalf("writes=%v flushes=%d", device.writes, device.flushes)
	}
	for i, block := range device.writes {
		if block != int64(i) {
			t.Fatalf("write[%d]=%d", i, block)
		}
	}
	if string(device.blocks[28]) != string([]byte{8, 8, 8, 8, 8, 8, 8, 8}) || string(device.blocks[29]) != string([]byte{9, 9, 9, 9, 9, 9, 9, 9}) {
		t.Fatal("data reservation was modified")
	}
	for block := int64(0); block < 28; block++ {
		if block >= 21 && device.blocks[block] != nil {
			t.Fatalf("unused log tail block %d changed", block)
		}
	}
}

func TestPublishSparsePrefixAndExactCapacity(t *testing.T) {
	log, data := layouts()
	device := newTraced(geometry())
	prefix := []storagewalrecord.Record{record(storagewalrecord.BeginTransaction, 1, 80, 0, nil)}
	seedPrefix(t, device, prefix...)
	result, err := Publish(device, log, data, 81, request())
	if err != nil || result.AcknowledgedRecords != 3 || result.LastLSN != 4 || result.Committed[0].LSN != 2 {
		t.Fatalf("sparse publication=(%+v,%v)", result, err)
	}
	short := log
	short.PageCount = 3
	other := newTraced(geometry())
	seedPrefix(t, other, prefix...)
	result, err = Publish(other, short, data, 81, request())
	if !reflect.DeepEqual(result, Result{}) || !errors.Is(err, storagewalpreparecommit.ErrInsufficientSlots) || len(other.writes) != 0 || other.flushes != 0 {
		t.Fatalf("exact-capacity refusal=(%+v,%v) writes=%v", result, err, other.writes)
	}
}

func TestPublishPreflightRefusalsAreZeroAndDoNotWrite(t *testing.T) {
	log, data := layouts()
	g := geometry()
	cases := []struct {
		name      string
		device    *tracedDevice
		log, data storagepagelayout.Layout
		tx        uint64
		images    []storagewalpreparecommit.PageImage
		want      error
	}{
		{name: "layout", device: newTraced(g), log: storagepagelayout.Layout{Geometry: g, FirstBlock: 0, PageCount: 0, BlocksPerPage: 1}, data: data, want: storagepagelayout.ErrInvalidLayout},
		{name: "capture geometry", device: newTraced(blockdevicecontract.Geometry{BlockCount: 64, BytesPerBlock: 8}), log: log, data: data, want: storagepageio.ErrGeometryMismatch},
		{name: "invalid transaction id", device: newTraced(g), log: log, data: data, tx: 0, images: request(), want: storagewalrecord.ErrTxID},
		{name: "invalid page", device: newTraced(g), log: log, data: data, tx: 7, images: []storagewalpreparecommit.PageImage{{PageID: -1, Image: make([]byte, 8)}}, want: storagepagelayout.ErrInvalidPage},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			result, err := Publish(tc.device, tc.log, tc.data, tc.tx, tc.images)
			if !reflect.DeepEqual(result, Result{}) || !errors.Is(err, tc.want) || len(tc.device.writes) != 0 || tc.device.flushes != 0 {
				t.Fatalf("result=%+v err=%v writes=%v flushes=%d", result, err, tc.device.writes, tc.device.flushes)
			}
		})
	}
	// Damaged page zero refuses before preparation and preserves the lower cause.
	bad := newTraced(g)
	broken, _ := storagewalpageslot.EncodePage(56, record(storagewalrecord.BeginTransaction, 1, 1, 0, nil))
	broken[len(broken)-1] = 1
	for part := int64(0); part < 7; part++ {
		bad.blocks[part] = append([]byte(nil), broken[part*8:(part+1)*8]...)
	}
	result, err := Publish(bad, log, data, 2, request())
	var damaged *storagewalappend.DamagedTailError
	if !reflect.DeepEqual(result, Result{}) || !errors.As(err, &damaged) || damaged.Page != 0 || !errors.Is(err, storagewalpageslot.ErrPadding) || len(bad.writes) != 0 || bad.flushes != 0 {
		t.Fatalf("damaged refusal result=%+v err=%v", result, err)
	}
}

func TestPublishStopsAtEveryFailedRecordBlock(t *testing.T) {
	log, data := layouts()
	for recordIndex := int64(0); recordIndex < 3; recordIndex++ {
		for offset := int64(0); offset < 7; offset++ {
			t.Run(fmt.Sprintf("record-%d-block-%d", recordIndex, offset), func(t *testing.T) {
				device := newTraced(geometry())
				device.writeErr[recordIndex*7+offset] = errInjected
				result, err := Publish(device, log, data, 7, request())
				assertFailureHasNoCommit(t, result, err)
				if err != errInjected || result.AcknowledgedRecords != int(recordIndex) || result.LastLSN != uint64(recordIndex+1) || result.LastAppend.Page != recordIndex || result.LastAppend.WrittenBlocks != offset || !result.LastAppend.Attempted || result.LastAppend.Acknowledged {
					t.Fatalf("result=%+v err=%v", result, err)
				}
				if len(device.writes) != int(recordIndex*7+offset+1) || device.flushes != int(recordIndex) {
					t.Fatalf("continued after refusal writes=%v flushes=%d", device.writes, device.flushes)
				}
			})
		}
	}
}

func TestPublishStopsAtEveryFailedBarrier(t *testing.T) {
	log, data := layouts()
	for barrier := 1; barrier <= 3; barrier++ {
		t.Run(fmt.Sprintf("barrier-%d", barrier), func(t *testing.T) {
			device := newTraced(geometry())
			device.flushErr[barrier] = errInjected
			result, err := Publish(device, log, data, 7, request())
			assertFailureHasNoCommit(t, result, err)
			if err != errInjected || result.AcknowledgedRecords != barrier-1 || result.LastLSN != uint64(barrier) || result.LastAppend.Page != int64(barrier-1) || result.LastAppend.WrittenBlocks != 7 || !result.LastAppend.Attempted || result.LastAppend.Acknowledged {
				t.Fatalf("result=%+v err=%v", result, err)
			}
			if len(device.writes) != barrier*7 || device.flushes != barrier {
				t.Fatalf("continued after barrier failure writes=%d flushes=%d", len(device.writes), device.flushes)
			}
		})
	}
}

func TestPublishRealMemoryAndHDDDevices(t *testing.T) {
	log, data := layouts()
	g := geometry()
	want := request()[0].Image
	for _, backend := range []string{"memory", "hdd"} {
		t.Run(backend, func(t *testing.T) {
			var device blockdevicecontract.Device
			switch backend {
			case "memory":
				memory, err := memoryblockdevice.New(g)
				if err != nil {
					t.Fatal(err)
				}
				device = memory
			case "hdd":
				device = hddblockdevice.New()
			}
			result, err := Publish(device, log, data, 15, request())
			if err != nil || result.AcknowledgedRecords != 3 || len(result.Committed) != 3 {
				t.Fatalf("Publish=(%+v,%v)", result, err)
			}
			pager, err := storagepageio.Open(device, log)
			if err != nil {
				t.Fatal(err)
			}
			var pages [][]byte
			for page := int64(0); page < log.PageCount; page++ {
				value, err := pager.ReadPage(page)
				if err != nil {
					t.Fatal(err)
				}
				pages = append(pages, value)
			}
			scan, err := storagewalscan.ScanPages(pages)
			if err != nil || scan.Tail != storagewalscan.Vacant || len(scan.Records) != 3 {
				t.Fatalf("published scan=(%+v,%v)", scan, err)
			}
			dataPager, err := storagepageio.Open(device, data)
			if err != nil {
				t.Fatal(err)
			}
			got, err := dataPager.ReadPage(1)
			if err != nil || string(got) != string(make([]byte, 8)) {
				t.Fatalf("reserved data changed=%v err=%v", got, err)
			}
			if string(scan.Records[1].Image) != string(want) {
				t.Fatalf("published image=%v", scan.Records[1].Image)
			}
		})
	}
}

// lostAckHDD injects one whole-media, lost-ack barrier on the selected flush.
type lostAckHDD struct {
	device          *hddblockdevice.HDD
	failAt, flushes int
}

func (d *lostAckHDD) Geometry() blockdevicecontract.Geometry { return d.device.Geometry() }
func (d *lostAckHDD) ReadBlock(block int64) ([]byte, error)  { return d.device.ReadBlock(block) }
func (d *lostAckHDD) WriteBlock(block int64, data []byte) error {
	return d.device.WriteBlock(block, data)
}
func (d *lostAckHDD) Flush() error {
	d.flushes++
	if d.flushes == d.failAt {
		receipt, err := d.device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AfterAllMedia})
		if err != nil {
			return err
		}
		if !receipt.Acknowledged {
			return errInjected
		}
		return nil
	}
	return d.device.Flush()
}

func TestPublishLostCommitAcknowledgementKeepsCommittedBytesButNoCommitResult(t *testing.T) {
	log, data := layouts()
	base := hddblockdevice.New()
	device := &lostAckHDD{device: base, failAt: 3}
	result, err := Publish(device, log, data, 22, request())
	assertFailureHasNoCommit(t, result, err)
	if err != errInjected || result.AcknowledgedRecords != 2 || result.LastLSN != 3 || result.LastAppend.Page != 2 || !result.LastAppend.Attempted || result.LastAppend.Acknowledged {
		t.Fatalf("lost-ack result=%+v err=%v", result, err)
	}
	restarted, err := base.Restart()
	if err != nil {
		t.Fatal(err)
	}
	pager, err := storagepageio.Open(restarted, log)
	if err != nil {
		t.Fatal(err)
	}
	var pages [][]byte
	for page := int64(0); page < log.PageCount; page++ {
		v, err := pager.ReadPage(page)
		if err != nil {
			t.Fatal(err)
		}
		pages = append(pages, v)
	}
	scan, err := storagewalscan.ScanPages(pages)
	if err != nil || len(scan.Records) != 3 || scan.Tail != storagewalscan.Vacant || scan.Records[2].Kind != storagewalrecord.CommitTransaction {
		t.Fatalf("restart scan=(%+v,%v)", scan, err)
	}
}

func FuzzPublishHistories(f *testing.F) {
	for _, seed := range [][]byte{{0}, {1, 2, 3, 4, 5, 6, 7, 8}, {2, 1, 255, 9, 8, 7, 6, 5, 4, 3, 2}, {3, 2, 4, 6, 8, 10, 12, 14, 16}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) == 0 {
			t.Skip()
		}
		log, data := layouts()
		device := newTraced(geometry())
		count := int(raw[0] % 3)
		images := make([]storagewalpreparecommit.PageImage, count)
		for i := range images {
			page := int64(0)
			if i+1 < len(raw) {
				page = int64(raw[i+1] % 4)
			}
			image := make([]byte, 8)
			for j := range image {
				if i+j+2 < len(raw) {
					image[j] = raw[i+j+2]
				}
			}
			images[i] = storagewalpreparecommit.PageImage{PageID: page, Image: image}
		}
		fault := raw[0] >> 2 % 5
		if fault == 1 {
			device.writeErr[0] = errInjected
		} else if fault == 2 {
			device.flushErr[1] = errInjected
		}
		result, err := Publish(device, log, data, uint64(raw[0]>>3%250+1), images)
		if fault == 1 || fault == 2 {
			if err != errInjected || result.Committed != nil || result.AcknowledgedRecords != 0 || result.LastLSN != 1 {
				t.Fatalf("fault result=%+v err=%v", result, err)
			}
			return
		}
		if err != nil || result.AcknowledgedRecords != count+2 || len(result.Committed) != count+2 || result.LastLSN != uint64(count+2) {
			t.Fatalf("valid history result=%+v err=%v", result, err)
		}
		if !reflect.DeepEqual(result.Committed[0], record(storagewalrecord.BeginTransaction, 1, result.Committed[0].TxID, 0, nil)) || result.Committed[len(result.Committed)-1].Kind != storagewalrecord.CommitTransaction {
			t.Fatalf("bad published sequence=%+v", result.Committed)
		}
		for i := range images {
			if result.Committed[i+1].PageID != images[i].PageID || string(result.Committed[i+1].Image) != string(images[i].Image) {
				t.Fatalf("published image mismatch")
			}
		}
	})
}
