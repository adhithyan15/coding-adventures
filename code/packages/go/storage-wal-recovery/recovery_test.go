package storagewalrecovery

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
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalredoapply "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-apply"
	storagewalreservation "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

var errInjected = errors.New("injected device refusal")

type tracedDevice struct {
	geometry blockdevicecontract.Geometry
	blocks   map[int64][]byte
	reads    []int64
	writes   []int64
	flushes  int
	readErr  map[int64]error
	writeErr map[int64]error
	flushErr error
}

func newTraced(g blockdevicecontract.Geometry) *tracedDevice {
	return &tracedDevice{geometry: g, blocks: make(map[int64][]byte), readErr: make(map[int64]error), writeErr: make(map[int64]error)}
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
func (d *tracedDevice) WriteBlock(block int64, bytes []byte) error {
	d.writes = append(d.writes, block)
	if err := d.writeErr[block]; err != nil {
		return err
	}
	d.blocks[block] = append([]byte(nil), bytes...)
	return nil
}
func (d *tracedDevice) Flush() error {
	d.flushes++
	return d.flushErr
}

func testGeometry() blockdevicecontract.Geometry {
	return blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
}
func logLayout(g blockdevicecontract.Geometry) storagepagelayout.Layout {
	return storagepagelayout.Layout{Geometry: g, FirstBlock: 0, PageCount: 4, BlocksPerPage: 7}
}
func dataLayout(g blockdevicecontract.Geometry) storagepagelayout.Layout {
	return storagepagelayout.Layout{Geometry: g, FirstBlock: 28, PageCount: 4, BlocksPerPage: 1}
}
func pageBytes(t *testing.T, rec storagewalrecord.Record) []byte {
	t.Helper()
	page, err := storagewalpageslot.EncodePage(56, rec)
	if err != nil {
		t.Fatal(err)
	}
	return page
}
func seedPage(device *tracedDevice, page int64, value []byte) {
	start := page * 7
	for index := int64(0); index < 7; index++ {
		device.blocks[start+index] = append([]byte(nil), value[index*8:(index+1)*8]...)
	}
}
func seedRecords(t *testing.T, device *tracedDevice, records ...storagewalrecord.Record) {
	t.Helper()
	for i, record := range records {
		seedPage(device, int64(i), pageBytes(t, record))
	}
}
func baseRecords(image []byte, pageID int64) []storagewalrecord.Record {
	return []storagewalrecord.Record{
		{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 7},
		{Kind: storagewalrecord.WritePageImage, LSN: 2, TxID: 7, PageID: pageID, Image: append([]byte(nil), image...)},
		{Kind: storagewalrecord.CommitTransaction, LSN: 3, TxID: 7},
	}
}
func resetTrace(device *tracedDevice) {
	device.reads, device.writes, device.flushes = nil, nil, 0
}
func assertNoDataEffects(t *testing.T, device *tracedDevice, receipt storagewalredoapply.Receipt, err error) {
	t.Helper()
	if receipt != (storagewalredoapply.Receipt{}) {
		t.Fatalf("refusal receipt=%+v", receipt)
	}
	if len(device.writes) != 0 || device.flushes != 0 {
		t.Fatalf("refusal applied data: writes=%v flushes=%d err=%v", device.writes, device.flushes, err)
	}
}

func TestRecoverWritesCommittedPrefixAndLeavesLogAndNeighbors(t *testing.T) {
	g := testGeometry()
	device := newTraced(g)
	image := []byte{1, 3, 5, 7, 9, 11, 13, 15}
	seedRecords(t, device, baseRecords(image, 1)...)
	device.blocks[28] = []byte{2, 2, 2, 2, 2, 2, 2, 2}
	device.blocks[29] = []byte{3, 3, 3, 3, 3, 3, 3, 3}
	device.blocks[30] = []byte{8, 7, 6, 5, 4, 3, 2, 1}
	data := dataLayout(g)
	data.FirstBlock, data.PageCount = 30, 2
	logBefore := make([][]byte, 28)
	for block := int64(0); block < 28; block++ {
		logBefore[block] = append([]byte(nil), device.blocks[block]...)
	}
	receipt, err := Recover(device, logLayout(g), data)
	if err != nil || receipt != (storagewalredoapply.Receipt{AcknowledgedWrites: 1}) {
		t.Fatalf("Recover=(%+v,%v)", receipt, err)
	}
	if got := device.blocks[31]; string(got) != string(image) {
		t.Fatalf("data page block 31=%v want %v", got, image)
	}
	if string(device.blocks[30]) != string([]byte{8, 7, 6, 5, 4, 3, 2, 1}) {
		t.Fatalf("neighbor changed: %v", device.blocks[30])
	}
	if len(device.reads) != 28 || len(device.writes) != 1 || device.writes[0] != 31 || device.flushes != 1 {
		t.Fatalf("trace reads=%d writes=%v flush=%d", len(device.reads), device.writes, device.flushes)
	}
	for block := int64(0); block < 28; block++ {
		if string(device.blocks[block]) != string(logBefore[block]) {
			t.Fatalf("log block %d changed", block)
		}
	}
	if string(device.blocks[28]) != string([]byte{2, 2, 2, 2, 2, 2, 2, 2}) || string(device.blocks[29]) != string([]byte{3, 3, 3, 3, 3, 3, 3, 3}) {
		t.Fatalf("intervening guard blocks changed: 28=%v 29=%v", device.blocks[28], device.blocks[29])
	}
}

func TestRecoverAcceptsVacantFullAndIncompletePrefixes(t *testing.T) {
	g := testGeometry()
	tests := []struct {
		name    string
		records []storagewalrecord.Record
		full    bool
		want    []byte
	}{
		{name: "vacant"},
		{name: "incomplete transaction", records: []storagewalrecord.Record{{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 9}, {Kind: storagewalrecord.WritePageImage, LSN: 2, TxID: 9, PageID: 0, Image: []byte{9, 8, 7, 6, 5, 4, 3, 2}}}},
		{name: "full", records: []storagewalrecord.Record{{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 9}, {Kind: storagewalrecord.WritePageImage, LSN: 2, TxID: 9, PageID: 0, Image: []byte{9, 8, 7, 6, 5, 4, 3, 2}}, {Kind: storagewalrecord.CommitTransaction, LSN: 3, TxID: 9}, {Kind: storagewalrecord.BeginTransaction, LSN: 4, TxID: 10}}, full: true, want: []byte{9, 8, 7, 6, 5, 4, 3, 2}},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			device := newTraced(g)
			seedRecords(t, device, tc.records...)
			receipt, err := Recover(device, logLayout(g), dataLayout(g))
			if err != nil {
				t.Fatal(err)
			}
			wantReceipt := storagewalredoapply.Receipt{}
			if len(tc.want) > 0 {
				wantReceipt.AcknowledgedWrites = 1
			}
			if receipt != wantReceipt {
				t.Fatalf("receipt=%+v want %+v", receipt, wantReceipt)
			}
			if len(tc.want) > 0 && string(device.blocks[28]) != string(tc.want) {
				t.Fatalf("replayed page=%v", device.blocks[28])
			}
			if len(tc.want) == 0 && (len(device.writes) != 0 || device.flushes != 0) {
				t.Fatalf("empty plan effects writes=%v flush=%d", device.writes, device.flushes)
			}
			if tc.full && len(device.reads) != 28 {
				t.Fatalf("full scan reads=%d", len(device.reads))
			}
		})
	}
}

func TestRecoverDamagedTailUsesAppendTypedErrorBeforeDataEffects(t *testing.T) {
	g := testGeometry()
	for _, firstPage := range []int64{0, 1} {
		t.Run(fmt.Sprintf("damage page %d", firstPage), func(t *testing.T) {
			device := newTraced(g)
			if firstPage == 1 {
				seedRecords(t, device, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1})
			}
			bad := pageBytes(t, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: uint64(firstPage + 1), TxID: 1})
			bad[len(bad)-1] = 1
			seedPage(device, firstPage, bad)
			receipt, err := Recover(device, logLayout(g), dataLayout(g))
			var damaged *storagewalappend.DamagedTailError
			if !errors.As(err, &damaged) || damaged.Page != firstPage || !errors.Is(err, storagewalappend.ErrDamagedTail) || !errors.Is(err, storagewalpageslot.ErrPadding) {
				t.Fatalf("damaged tail=%+v err=%v", damaged, err)
			}
			assertNoDataEffects(t, device, receipt, err)
		})
	}
}

func TestRecoverPreservesEveryReadAndHardScanRefusal(t *testing.T) {
	g := testGeometry()
	for failedBlock := int64(0); failedBlock < 28; failedBlock++ {
		t.Run(fmt.Sprintf("read block %d", failedBlock), func(t *testing.T) {
			device := newTraced(g)
			seedRecords(t, device, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1})
			// A malformed page at the front must not stop the full capture read.
			bad := pageBytes(t, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 2, TxID: 2})
			bad[len(bad)-1] = 1
			seedPage(device, 1, bad)
			device.readErr[failedBlock] = errInjected
			receipt, err := Recover(device, logLayout(g), dataLayout(g))
			if err != errInjected {
				t.Fatalf("read error=%v", err)
			}
			assertNoDataEffects(t, device, receipt, err)
			if len(device.reads) != int(failedBlock+1) {
				t.Fatalf("reads=%v", device.reads)
			}
		})
	}
	for _, test := range []struct {
		name string
		rows []storagewalrecord.Record
		want error
	}{
		{name: "sequence hard error", rows: []storagewalrecord.Record{{Kind: storagewalrecord.BeginTransaction, LSN: 2, TxID: 1}}, want: storagewalscan.ErrSequence},
		{name: "occupied after vacancy", rows: []storagewalrecord.Record{{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1}}, want: storagewalscan.ErrPageAfterTail},
	} {
		t.Run(test.name, func(t *testing.T) {
			device := newTraced(g)
			seedRecords(t, device, test.rows...)
			if test.name == "occupied after vacancy" {
				device.blocks[0] = make([]byte, 8)
				page := pageBytes(t, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 2, TxID: 2})
				seedPage(device, 1, page)
			}
			receipt, err := Recover(device, logLayout(g), dataLayout(g))
			if !errors.Is(err, test.want) {
				t.Fatalf("error=%v want %v", err, test.want)
			}
			assertNoDataEffects(t, device, receipt, err)
		})
	}
}

func TestRecoverPreservesLayoutCaptureAndRedoPrecedence(t *testing.T) {
	g := testGeometry()
	device := newTraced(g)
	badLog, badData := logLayout(g), dataLayout(g)
	badLog.PageCount = 0
	badData.FirstBlock = -1
	if receipt, err := Recover(device, badLog, badData); !errors.Is(err, storagepagelayout.ErrInvalidLayout) || receipt != (storagewalredoapply.Receipt{}) || len(device.reads) != 0 {
		t.Fatalf("log layout precedence receipt=%+v err=%v reads=%v", receipt, err, device.reads)
	}
	if receipt, err := Recover(device, logLayout(g), dataLayout(blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 9})); !errors.Is(err, storagepageio.ErrGeometryMismatch) || receipt != (storagewalredoapply.Receipt{}) || len(device.reads) != 0 {
		t.Fatalf("device geometry receipt=%+v err=%v reads=%v", receipt, err, device.reads)
	}
	wrongDevice := newTraced(blockdevicecontract.Geometry{BlockCount: 64, BytesPerBlock: 8})
	if receipt, err := Recover(wrongDevice, logLayout(g), dataLayout(g)); !errors.Is(err, storagepageio.ErrGeometryMismatch) || receipt != (storagewalredoapply.Receipt{}) || len(wrongDevice.reads) != 0 {
		t.Fatalf("capture geometry receipt=%+v err=%v reads=%v", receipt, err, wrongDevice.reads)
	}
	capacityGeometry := blockdevicecontract.Geometry{BlockCount: storagewalreservation.MaxLogPages + 2, BytesPerBlock: 48}
	capacityLog := storagepagelayout.Layout{Geometry: capacityGeometry, FirstBlock: 0, PageCount: storagewalreservation.MaxLogPages + 1, BlocksPerPage: 1}
	capacityData := storagepagelayout.Layout{Geometry: capacityGeometry, FirstBlock: storagewalreservation.MaxLogPages + 1, PageCount: 1, BlocksPerPage: 1}
	capacityDevice := newTraced(capacityGeometry)
	if receipt, err := Recover(capacityDevice, capacityLog, capacityData); !errors.Is(err, storagewalreservation.ErrLogCapacity) || receipt != (storagewalredoapply.Receipt{}) || len(capacityDevice.reads) != 0 {
		t.Fatalf("capacity receipt=%+v err=%v reads=%v", receipt, err, capacityDevice.reads)
	}

	widthGeometry := blockdevicecontract.Geometry{BlockCount: 142, BytesPerBlock: 1}
	widthLog := storagepagelayout.Layout{Geometry: widthGeometry, FirstBlock: 0, PageCount: 3, BlocksPerPage: 47}
	widthData := storagepagelayout.Layout{Geometry: widthGeometry, FirstBlock: 141, PageCount: 1, BlocksPerPage: 1}
	widthDevice := newTraced(widthGeometry)
	if receipt, err := Recover(widthDevice, widthLog, widthData); !errors.Is(err, storagewalpageslot.ErrPageSize) || receipt != (storagewalredoapply.Receipt{}) || len(widthDevice.reads) != 0 {
		t.Fatalf("width receipt=%+v err=%v reads=%v", receipt, err, widthDevice.reads)
	}
	// The full plan rejects an invalid target before any writes.
	redoDevice := newTraced(g)
	seedRecords(t, redoDevice, baseRecords(make([]byte, 8), 4)...)
	if receipt, err := Recover(redoDevice, logLayout(g), dataLayout(g)); !errors.Is(err, storagepagelayout.ErrInvalidPage) || receipt != (storagewalredoapply.Receipt{}) || len(redoDevice.writes) != 0 {
		t.Fatalf("redo plan receipt=%+v err=%v writes=%v", receipt, err, redoDevice.writes)
	}
}

func TestRecoverReturnsReplayFailureReceipt(t *testing.T) {
	g := testGeometry()
	device := newTraced(g)
	seedRecords(t, device, baseRecords([]byte{1, 2, 3, 4, 5, 6, 7, 8}, 1)...)
	device.writeErr[29] = errInjected
	receipt, err := Recover(device, logLayout(g), dataLayout(g))
	if err != errInjected || receipt != (storagewalredoapply.Receipt{Interrupted: true, PageID: 1, RecordLSN: 2, WrittenBlocks: 0}) {
		t.Fatalf("write refusal receipt=%+v err=%v", receipt, err)
	}
	device = newTraced(g)
	seedRecords(t, device, baseRecords([]byte{1, 2, 3, 4, 5, 6, 7, 8}, 1)...)
	device.flushErr = errInjected
	receipt, err = Recover(device, logLayout(g), dataLayout(g))
	if err != errInjected || receipt != (storagewalredoapply.Receipt{Interrupted: true, PageID: 1, RecordLSN: 2, WrittenBlocks: 1}) {
		t.Fatalf("barrier refusal receipt=%+v err=%v", receipt, err)
	}
}

func writeLogsToDevice(t *testing.T, device blockdevicecontract.Device, log storagepagelayout.Layout, records []storagewalrecord.Record) {
	t.Helper()
	pager, err := storagepageio.Open(device, log)
	if err != nil {
		t.Fatal(err)
	}
	pageBytes := log.BlocksPerPage * log.Geometry.BytesPerBlock
	for index, record := range records {
		page, err := storagewalpageslot.EncodePage(pageBytes, record)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := pager.WritePage(int64(index), page); err != nil {
			t.Fatal(err)
		}
	}
	if err := pager.Flush(); err != nil {
		t.Fatal(err)
	}
}

func TestRecoverCompleteCommitThatLostItsAcknowledgement(t *testing.T) {
	g := testGeometry()
	log, data := logLayout(g), dataLayout(g)
	image := []byte{91, 82, 73, 64, 55, 46, 37, 28}
	hdd := hddblockdevice.New()
	pager, err := storagepageio.Open(hdd, log)
	if err != nil {
		t.Fatal(err)
	}
	records := baseRecords(image, 0)
	for index := 0; index < 2; index++ {
		page, err := storagewalpageslot.EncodePage(56, records[index])
		if err != nil {
			t.Fatal(err)
		}
		if _, err := pager.WritePage(int64(index), page); err != nil {
			t.Fatal(err)
		}
		if err := pager.Flush(); err != nil {
			t.Fatal(err)
		}
	}
	commit, err := storagewalpageslot.EncodePage(56, records[2])
	if err != nil {
		t.Fatal(err)
	}
	if _, err := pager.WritePage(2, commit); err != nil {
		t.Fatal(err)
	}
	lost, err := hdd.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AfterAllMedia})
	if err != nil || lost.Acknowledged {
		t.Fatalf("commit barrier=(%+v,%v)", lost, err)
	}
	restarted, err := hdd.Restart()
	if err != nil {
		t.Fatal(err)
	}
	receipt, err := Recover(restarted, log, data)
	if err != nil || receipt != (storagewalredoapply.Receipt{AcknowledgedWrites: 1}) {
		t.Fatalf("recovery=(%+v,%v)", receipt, err)
	}
	got, err := restarted.ReadBlock(data.FirstBlock)
	if err != nil || string(got) != string(image) {
		t.Fatalf("lost-ack commit was not replayed: %v err=%v", got, err)
	}
}

func TestRecoverRepeatedOnRestartedMemoryAndHDD(t *testing.T) {
	g := testGeometry()
	log, data := logLayout(g), dataLayout(g)
	image := []byte{90, 80, 70, 60, 50, 40, 30, 20}
	for _, backend := range []string{"memory", "hdd"} {
		t.Run(backend, func(t *testing.T) {
			var device blockdevicecontract.Device
			var restart func() (blockdevicecontract.Device, error)
			switch backend {
			case "memory":
				memory, err := memoryblockdevice.New(g)
				if err != nil {
					t.Fatal(err)
				}
				device = memory
				restart = func() (blockdevicecontract.Device, error) { return memory.Restart() }
			case "hdd":
				hdd := hddblockdevice.New()
				device = hdd
				restart = func() (blockdevicecontract.Device, error) { return hdd.Restart() }
			}
			writeLogsToDevice(t, device, log, baseRecords(image, 3))
			device, err := restart()
			if err != nil {
				t.Fatal(err)
			}
			first, err := Recover(device, log, data)
			if err != nil || first != (storagewalredoapply.Receipt{AcknowledgedWrites: 1}) {
				t.Fatalf("first recovery=(%+v,%v)", first, err)
			}
			// Restart again and repeat recovery: original log remains available and redo is idempotent.
			switch restarted := device.(type) {
			case *memoryblockdevice.Memory:
				device, err = restarted.Restart()
			case *hddblockdevice.HDD:
				device, err = restarted.Restart()
			}
			if err != nil {
				t.Fatal(err)
			}
			second, err := Recover(device, log, data)
			if err != nil || second != first {
				t.Fatalf("second recovery=(%+v,%v)", second, err)
			}
			got, err := device.ReadBlock(data.FirstBlock + 3)
			if err != nil || string(got) != string(image) {
				t.Fatalf("recovered bytes=%v err=%v", got, err)
			}
		})
	}
}

// cutHDD uses one selected barrier cut and reports its lost acknowledgement as
// a failure while retaining the lower device's exact durable result.
type cutHDD struct {
	device *hddblockdevice.HDD
	cut    hddwritebackcache.BarrierCut
	used   bool
}

func (d *cutHDD) Geometry() blockdevicecontract.Geometry    { return d.device.Geometry() }
func (d *cutHDD) ReadBlock(block int64) ([]byte, error)     { return d.device.ReadBlock(block) }
func (d *cutHDD) WriteBlock(block int64, data []byte) error { return d.device.WriteBlock(block, data) }
func (d *cutHDD) Flush() error {
	if !d.used {
		d.used = true
		receipt, err := d.device.FlushWithCut(d.cut)
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

func TestRecoverHDDPartialDataBarrierThenRerunsFromIntactLog(t *testing.T) {
	g := testGeometry()
	log, data := logLayout(g), dataLayout(g)
	data.FirstBlock, data.PageCount = 30, 2
	image := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	hdd := hddblockdevice.New()
	writeLogsToDevice(t, hdd, log, baseRecords(image, 1))
	logBefore := make([][]byte, 28)
	for block := int64(0); block < 28; block++ {
		logBefore[block], _ = hdd.ReadBlock(block)
	}
	prefix := 4
	faulted := &cutHDD{device: hdd, cut: hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}}}
	receipt, err := Recover(faulted, log, data)
	if err != errInjected || receipt != (storagewalredoapply.Receipt{Interrupted: true, PageID: 1, RecordLSN: 2, WrittenBlocks: 1}) {
		t.Fatalf("cut recovery=(%+v,%v)", receipt, err)
	}
	restarted, err := hdd.Restart()
	if err != nil {
		t.Fatal(err)
	}
	if partial, _ := restarted.ReadBlock(data.FirstBlock + 1); string(partial[:prefix]) != string(image[:prefix]) || string(partial[prefix:]) != string(make([]byte, 8-prefix)) {
		t.Fatalf("partial after crash=%v", partial)
	}
	secondLostAck := &cutHDD{device: restarted, cut: hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AfterAllMedia}}
	second, err := Recover(secondLostAck, log, data)
	if err != errInjected || second != (storagewalredoapply.Receipt{Interrupted: true, PageID: 1, RecordLSN: 2, WrittenBlocks: 1}) {
		t.Fatalf("second crash recovery=(%+v,%v)", second, err)
	}
	restartedAgain, err := restarted.Restart()
	if err != nil {
		t.Fatal(err)
	}
	third, err := Recover(restartedAgain, log, data)
	if err != nil || third != (storagewalredoapply.Receipt{AcknowledgedWrites: 1}) {
		t.Fatalf("third recovery=(%+v,%v)", third, err)
	}
	got, err := restartedAgain.ReadBlock(data.FirstBlock + 1)
	if err != nil || string(got) != string(image) {
		t.Fatalf("final bytes=%v err=%v", got, err)
	}
	for block := int64(0); block < 28; block++ {
		after, _ := restartedAgain.ReadBlock(block)
		if string(logBefore[block]) != string(after) {
			t.Fatalf("log block %d changed across recovery", block)
		}
	}
	guard28, _ := restartedAgain.ReadBlock(28)
	guard29, _ := restartedAgain.ReadBlock(29)
	if string(guard28) != string(make([]byte, 8)) || string(guard29) != string(make([]byte, 8)) {
		t.Fatalf("guard blocks changed: 28=%v 29=%v", guard28, guard29)
	}
}

func FuzzRecoverAdmissionAndReplay(f *testing.F) {
	for _, seed := range [][]byte{{0}, {1}, {2}, {3}, {4}, {5}, {6, 12, 34, 56, 78, 90, 123, 222}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) == 0 {
			t.Skip()
		}
		mode := raw[0] % 7
		g := testGeometry()
		device := newTraced(g)
		image := make([]byte, 8)
		for i := range image {
			if i+1 < len(raw) {
				image[i] = raw[i+1]
			}
		}
		switch mode {
		case 0:
		case 1:
			seedRecords(t, device, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1})
		case 2:
			seedRecords(t, device, baseRecords(image, int64(raw[0]>>3)%4)...)
		case 3:
			bad := pageBytes(t, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1})
			bad[len(bad)-1] = 1
			seedPage(device, 0, bad)
		case 4:
			seedRecords(t, device, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1})
			bad := pageBytes(t, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 2, TxID: 2})
			bad[len(bad)-1] = 1
			seedPage(device, 1, bad)
		case 5:
			seedRecords(t, device, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 2, TxID: 1})
		case 6:
			seedRecords(t, device, baseRecords(image, int64(raw[0]>>3)%4)...)
		}
		resetTrace(device)
		beforeLog := make([][]byte, 28)
		for block := int64(0); block < 28; block++ {
			if value, ok := device.blocks[block]; ok {
				beforeLog[block] = append([]byte(nil), value...)
			} else {
				beforeLog[block] = make([]byte, 8)
			}
		}
		resetTrace(device)
		receipt, err := Recover(device, logLayout(g), dataLayout(g))
		switch mode {
		case 3:
			var damaged *storagewalappend.DamagedTailError
			if !errors.As(err, &damaged) || damaged.Page != 0 {
				t.Fatalf("damaged zero=%+v err=%v", damaged, err)
			}
			assertNoDataEffects(t, device, receipt, err)
		case 4:
			var damaged *storagewalappend.DamagedTailError
			if !errors.As(err, &damaged) || damaged.Page != 1 {
				t.Fatalf("damaged one=%+v err=%v", damaged, err)
			}
			assertNoDataEffects(t, device, receipt, err)
		case 5:
			if !errors.Is(err, storagewalscan.ErrSequence) {
				t.Fatalf("sequence err=%v", err)
			}
			assertNoDataEffects(t, device, receipt, err)
		case 2, 6:
			if err != nil || receipt != (storagewalredoapply.Receipt{AcknowledgedWrites: 1}) {
				t.Fatalf("valid mode=%d receipt=%+v err=%v", mode, receipt, err)
			}
			pageID := int64(raw[0]>>3) % 4
			if got := device.blocks[28+pageID]; string(got) != string(image) {
				t.Fatalf("page=%v want %v", got, image)
			}
		case 0, 1:
			if err != nil || receipt != (storagewalredoapply.Receipt{}) || len(device.writes) != 0 || device.flushes != 0 {
				t.Fatalf("empty mode=%d receipt=%+v err=%v writes=%v", mode, receipt, err, device.writes)
			}
		}
		for block := int64(0); block < 28; block++ {
			after := device.blocks[block]
			if len(after) == 0 {
				after = make([]byte, 8)
			}
			if string(after) != string(beforeLog[block]) {
				t.Fatalf("log block %d changed", block)
			}
		}
	})
}

func TestRecoverWithBoundaryReportsLastAdmittedRank(t *testing.T) {
	g := testGeometry()
	image := []byte{1, 4, 9, 16, 25, 36, 49, 64}
	cases := []struct {
		name    string
		records []storagewalrecord.Record
		last    uint64
		writes  int
	}{
		{name: "empty"},
		{name: "unfinished begin", records: []storagewalrecord.Record{{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 3}}, last: 1},
		{name: "unfinished image", records: baseRecords(image, 0)[:2], last: 2},
		{name: "full with unfinished next transaction", records: append(baseRecords(image, 0), storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 4, TxID: 8}), last: 4, writes: 1},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			device := newTraced(g)
			seedRecords(t, device, tc.records...)
			result, err := RecoverWithBoundary(device, logLayout(g), dataLayout(g))
			wantReplay := storagewalredoapply.Receipt{}
			if tc.writes != 0 {
				wantReplay.AcknowledgedWrites = tc.writes
			}
			if err != nil || result != (Result{Replay: wantReplay, LastLSN: tc.last}) {
				t.Fatalf("RecoverWithBoundary=(%+v,%v), want replay=%+v last=%d", result, err, wantReplay, tc.last)
			}
			if len(device.reads) != 28 {
				t.Fatalf("captured WAL %d block reads, want one 28-block capture", len(device.reads))
			}
		})
	}
}

func TestRecoverWithBoundaryIncludesUnfinishedTrailingRecord(t *testing.T) {
	g := testGeometry()
	device := newTraced(g)
	image := []byte{2, 4, 6, 8, 10, 12, 14, 16}
	records := append(baseRecords(image, 0), storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 4, TxID: 11})
	seedRecords(t, device, records...)
	result, err := RecoverWithBoundary(device, logLayout(g), dataLayout(g))
	want := Result{Replay: storagewalredoapply.Receipt{AcknowledgedWrites: 1}, LastLSN: 4}
	if err != nil || result != want {
		t.Fatalf("RecoverWithBoundary=(%+v,%v), want %+v", result, err, want)
	}
}

func TestRecoverWithBoundaryPreservesLegacyAndPartialReplayResults(t *testing.T) {
	g := testGeometry()
	image := []byte{8, 7, 6, 5, 4, 3, 2, 1}
	makeDevice := func() *tracedDevice {
		device := newTraced(g)
		seedRecords(t, device, baseRecords(image, 1)...)
		device.writeErr[29] = errInjected
		return device
	}
	boundaryDevice := makeDevice()
	result, boundaryErr := RecoverWithBoundary(boundaryDevice, logLayout(g), dataLayout(g))
	legacyDevice := makeDevice()
	legacyReceipt, legacyErr := Recover(legacyDevice, logLayout(g), dataLayout(g))
	wantReceipt := storagewalredoapply.Receipt{Interrupted: true, PageID: 1, RecordLSN: 2, WrittenBlocks: 0}
	if boundaryErr != errInjected || legacyErr != boundaryErr || result != (Result{Replay: wantReceipt}) || legacyReceipt != wantReceipt {
		t.Fatalf("boundary=(%+v,%v), legacy=(%+v,%v)", result, boundaryErr, legacyReceipt, legacyErr)
	}
	if len(boundaryDevice.reads) != 28 || !reflect.DeepEqual(boundaryDevice.reads, legacyDevice.reads) || !reflect.DeepEqual(boundaryDevice.writes, legacyDevice.writes) || boundaryDevice.flushes != legacyDevice.flushes {
		t.Fatalf("old/new call traces differ: boundary reads=%d writes=%v flush=%d legacy reads=%d writes=%v flush=%d", len(boundaryDevice.reads), boundaryDevice.writes, boundaryDevice.flushes, len(legacyDevice.reads), legacyDevice.writes, legacyDevice.flushes)
	}
}

func TestRecoverWithBoundaryRefusalHasZeroRank(t *testing.T) {
	g := testGeometry()
	device := newTraced(g)
	bad := pageBytes(t, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 2})
	bad[len(bad)-1] = 1
	seedPage(device, 0, bad)
	result, err := RecoverWithBoundary(device, logLayout(g), dataLayout(g))
	var damaged *storagewalappend.DamagedTailError
	if result != (Result{}) || !errors.As(err, &damaged) || damaged.Page != 0 || len(device.writes) != 0 {
		t.Fatalf("damaged boundary=(%+v,%v) writes=%v", result, err, device.writes)
	}
}

func FuzzRecoverWithBoundary(f *testing.F) {
	for mode := byte(0); mode < 6; mode++ {
		f.Add([]byte{mode, 9, 8, 7, 6, 5, 4, 3, 2})
	}
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) == 0 || len(raw) > 32 {
			t.Skip()
		}
		mode := raw[0] % 6
		g := testGeometry()
		device := newTraced(g)
		image := make([]byte, 8)
		for index := range image {
			if index+1 < len(raw) {
				image[index] = raw[index+1]
			}
		}
		var last uint64
		want := storagewalredoapply.Receipt{}
		switch mode {
		case 0:
		case 1:
			record := storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 4}
			seedRecords(t, device, record)
			last = record.LSN
		case 2:
			records := baseRecords(image, 1)
			seedRecords(t, device, records...)
			last = records[len(records)-1].LSN
			want.AcknowledgedWrites = 1
		case 3:
			records := baseRecords(image, 1)
			records = append(records, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 4, TxID: 5})
			seedRecords(t, device, records...)
			last = records[len(records)-1].LSN
			want.AcknowledgedWrites = 1
		case 4:
			bad := pageBytes(t, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1})
			bad[len(bad)-1] = 1
			seedPage(device, 0, bad)
		case 5:
			records := baseRecords(image, 1)
			seedRecords(t, device, records...)
			device.writeErr[29] = errInjected
		}
		result, err := RecoverWithBoundary(device, logLayout(g), dataLayout(g))
		if mode == 4 {
			var damaged *storagewalappend.DamagedTailError
			if result != (Result{}) || !errors.As(err, &damaged) || damaged.Page != 0 || len(device.writes) != 0 {
				t.Fatalf("damaged result=(%+v,%v)", result, err)
			}
			return
		}
		if mode == 5 {
			partial := storagewalredoapply.Receipt{Interrupted: true, PageID: 1, RecordLSN: 2}
			if result != (Result{Replay: partial}) || err != errInjected || result.LastLSN != 0 {
				t.Fatalf("partial result=(%+v,%v)", result, err)
			}
			return
		}
		if err != nil || result != (Result{Replay: want, LastLSN: last}) {
			t.Fatalf("mode %d result=(%+v,%v) want replay=%+v last=%d", mode, result, err, want, last)
		}
	})
}
