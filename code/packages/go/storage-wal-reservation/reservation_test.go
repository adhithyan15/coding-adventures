package storagewalreservation

import (
	"errors"
	"fmt"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

type scriptedDevice struct {
	geometry  blockdevicecontract.Geometry
	blocks    map[int64][]byte
	reads     []int64
	readErr   map[int64]error
	readWidth map[int64]int
	writes    int
	flushes   int
}

func newScripted(geometry blockdevicecontract.Geometry) *scriptedDevice {
	return &scriptedDevice{geometry: geometry, blocks: make(map[int64][]byte), readErr: make(map[int64]error), readWidth: make(map[int64]int)}
}
func (device *scriptedDevice) Geometry() blockdevicecontract.Geometry { return device.geometry }
func (device *scriptedDevice) ReadBlock(block int64) ([]byte, error) {
	device.reads = append(device.reads, block)
	if err := device.readErr[block]; err != nil {
		return nil, err
	}
	if width, ok := device.readWidth[block]; ok {
		return make([]byte, width), nil
	}
	if data, ok := device.blocks[block]; ok {
		return append([]byte(nil), data...), nil
	}
	return make([]byte, int(device.geometry.BytesPerBlock)), nil
}
func (device *scriptedDevice) WriteBlock(block int64, data []byte) error {
	device.writes++
	device.blocks[block] = append([]byte(nil), data...)
	return nil
}
func (device *scriptedDevice) Flush() error { device.flushes++; return nil }

func TestCaptureReadsFullReservationInOrderAtNonzeroOffset(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	layout := layout(geometry, 3, 3, 7)
	device := newScripted(geometry)
	seedPage(t, device, layout, 0, record(1, 10))
	pager, scan, err := Capture(device, layout)
	if err != nil || scan.Tail != storagewalscan.Vacant || scan.TailPage != 1 || len(scan.Records) != 1 {
		t.Fatalf("Capture=(%+v,%+v,%v)", pager, scan, err)
	}
	if pager == (storagepageio.Pager{}) {
		t.Fatal("successful capture returned zero pager")
	}
	if len(device.reads) != 21 || device.reads[0] != 3 || device.reads[20] != 23 {
		t.Fatalf("read interval=%v", device.reads)
	}
	for i, block := range device.reads {
		if block != 3+int64(i) {
			t.Fatalf("read[%d]=%d", i, block)
		}
	}
	if device.writes != 0 || device.flushes != 0 {
		t.Fatalf("capture mutated device: writes=%d flushes=%d", device.writes, device.flushes)
	}
	page, err := pager.ReadPage(0)
	if err != nil {
		t.Fatal(err)
	}
	decoded, present, err := storagewalpageslot.DecodePage(page)
	if err != nil || !present || decoded.LSN != 1 {
		t.Fatalf("returned pager page=(%+v,%v,%v)", decoded, present, err)
	}
}

func TestCaptureSupportsMinimumAndMaximumPageWidths(t *testing.T) {
	for _, test := range []struct {
		name     string
		geometry blockdevicecontract.Geometry
		blocks   int64
		pages    int64
		width    int64
	}{
		{name: "minimum", geometry: blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 4}, blocks: 12, pages: 2, width: 48},
		{name: "maximum", geometry: blockdevicecontract.Geometry{BlockCount: 512, BytesPerBlock: 256}, blocks: 256, pages: 1, width: 65_536},
	} {
		t.Run(test.name, func(t *testing.T) {
			device := newScripted(test.geometry)
			layout := layout(test.geometry, 5, test.pages, test.blocks)
			_, scan, err := Capture(device, layout)
			if err != nil || scan.Tail != storagewalscan.Vacant {
				t.Fatalf("Capture width=%d scan=(%+v,%v)", test.width, scan, err)
			}
			if int64(len(device.reads))*test.geometry.BytesPerBlock != test.pages*test.width {
				t.Fatalf("read count=%d width=%d", len(device.reads), test.width)
			}
		})
	}
}

func TestCapturePageCountBoundaryAndPrecedence(t *testing.T) {
	if MaxLogPages != 1024 {
		t.Fatalf("MaxLogPages=%d", MaxLogPages)
	}
	geometry := blockdevicecontract.Geometry{BlockCount: MaxLogPages, BytesPerBlock: 48}
	device := newScripted(geometry)
	_, scan, err := Capture(device, layout(geometry, 0, MaxLogPages, 1))
	if err != nil || scan.Tail != storagewalscan.Vacant || len(device.reads) != int(MaxLogPages) {
		t.Fatalf("exact capacity scan=(%+v,%v) reads=%d", scan, err, len(device.reads))
	}
	tooManyGeometry := blockdevicecontract.Geometry{BlockCount: (MaxLogPages + 1) * 32, BytesPerBlock: 1}
	tooMany := newScripted(tooManyGeometry)
	tooWideCount := layout(tooManyGeometry, 0, MaxLogPages+1, 32)
	if got, scan, err := Capture(tooMany, tooWideCount); got != (storagepageio.Pager{}) || !zeroScan(scan) || !errors.Is(err, ErrLogCapacity) || len(tooMany.reads) != 0 {
		t.Fatalf("capacity=(%+v,%+v,%v), reads=%d", got, scan, err, len(tooMany.reads))
	}
	invalidLayout := storagepagelayout.Layout{Geometry: blockdevicecontract.Geometry{}, FirstBlock: -1, PageCount: MaxLogPages + 1, BlocksPerPage: 1}
	if got, scan, err := Capture(nil, invalidLayout); got != (storagepageio.Pager{}) || !zeroScan(scan) || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("open precedence=(%+v,%+v,%v)", got, scan, err)
	}
}

func TestCaptureWidthRefusalFollowsCapacityBeforeAnyRead(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 64, BytesPerBlock: 1}
	device := newScripted(geometry)
	got, scan, err := Capture(device, layout(geometry, 0, 1, 32))
	if got != (storagepageio.Pager{}) || !zeroScan(scan) || !errors.Is(err, storagewalpageslot.ErrPageSize) || len(device.reads) != 0 {
		t.Fatalf("width=(%+v,%+v,%v), reads=%v", got, scan, err, device.reads)
	}
}

func TestCaptureStopsAtEveryBlockReadFaultEvenAfterEarlyMalformedPage(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	layout := layout(geometry, 2, 3, 7)
	for failed := int64(0); failed < 21; failed++ {
		t.Run(fmt.Sprintf("block-%02d", failed), func(t *testing.T) {
			device := newScripted(geometry)
			bad := make([]byte, 56)
			bad[0] = 1
			seedRawPage(t, device, layout, 0, bad)
			failure := errors.New("read-failure")
			device.readErr[2+failed] = failure
			pager, scan, err := Capture(device, layout)
			if pager != (storagepageio.Pager{}) || !zeroScan(scan) || !errors.Is(err, failure) {
				t.Fatalf("Capture=(%+v,%+v,%v)", pager, scan, err)
			}
			if len(device.reads) != int(failed+1) {
				t.Fatalf("read calls=%d want=%d", len(device.reads), failed+1)
			}
			if device.writes != 0 || device.flushes != 0 {
				t.Fatalf("reads mutated device")
			}
		})
	}
}

func TestCaptureRejectsBlockWidthAndPreservesHardScanErrors(t *testing.T) {
	t.Run("unexpected device width", func(t *testing.T) {
		geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
		layout := layout(geometry, 1, 2, 7)
		device := newScripted(geometry)
		device.readWidth[8] = 7
		pager, scan, err := Capture(device, layout)
		if pager != (storagepageio.Pager{}) || !zeroScan(scan) || !errors.Is(err, storagepageio.ErrBlockWidthMismatch) || len(device.reads) != 8 {
			t.Fatalf("Capture=(%+v,%+v,%v) reads=%v", pager, scan, err, device.reads)
		}
	})
	t.Run("sequence refusal", func(t *testing.T) {
		geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
		layout := layout(geometry, 1, 3, 7)
		device := newScripted(geometry)
		seedPage(t, device, layout, 0, record(1, 1))
		seedPage(t, device, layout, 1, record(3, 2))
		pager, scan, err := Capture(device, layout)
		if pager != (storagepageio.Pager{}) || !zeroScan(scan) || !errors.Is(err, storagewalscan.ErrSequence) || len(device.reads) != 21 {
			t.Fatalf("Capture=(%+v,%+v,%v)", pager, scan, err)
		}
	})
	t.Run("occupied after first vacancy", func(t *testing.T) {
		geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
		layout := layout(geometry, 1, 3, 7)
		device := newScripted(geometry)
		seedPage(t, device, layout, 1, record(1, 1))
		pager, scan, err := Capture(device, layout)
		if pager != (storagepageio.Pager{}) || !zeroScan(scan) || !errors.Is(err, storagewalscan.ErrPageAfterTail) || len(device.reads) != 21 {
			t.Fatalf("Capture=(%+v,%+v,%v)", pager, scan, err)
		}
	})
}

func TestCaptureClassifiesFullVacantAndDamagedTailWithoutPolicy(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	layout := layout(geometry, 1, 3, 7)
	full := newScripted(geometry)
	for page := int64(0); page < 3; page++ {
		seedPage(t, full, layout, page, record(uint64(page+1), uint64(page+1)))
	}
	if pager, scan, err := Capture(full, layout); err != nil || pager == (storagepageio.Pager{}) || scan.Tail != storagewalscan.Full || len(scan.Records) != 3 {
		t.Fatalf("full=(%+v,%+v,%v)", pager, scan, err)
	}
	blank := newScripted(geometry)
	if _, scan, err := Capture(blank, layout); err != nil || scan.Tail != storagewalscan.Vacant || scan.TailPage != 0 {
		t.Fatalf("vacant=(%+v,%v)", scan, err)
	}
	damaged := newScripted(geometry)
	seedPage(t, damaged, layout, 0, record(1, 1))
	raw := make([]byte, 56)
	raw[0] = 1
	seedRawPage(t, damaged, layout, 1, raw)
	if pager, scan, err := Capture(damaged, layout); err != nil || pager == (storagepageio.Pager{}) || scan.Tail != storagewalscan.Damaged || scan.TailPage != 1 || !errors.Is(scan.Cause, storagewalpageslot.ErrSlotFrameLength) {
		t.Fatalf("damaged=(%+v,%+v,%v)", pager, scan, err)
	}
}

func TestCaptureReturnedRecordImagesAreDetachedAndNeverWrites(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
	layout := layout(geometry, 4, 2, 7)
	device := newScripted(geometry)
	image := bytesOf(0x5a, 4)
	record := storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: 1, TxID: 7, PageID: 1, Image: image}
	seedPage(t, device, layout, 0, record)
	_, scan, err := Capture(device, layout)
	if err != nil || len(scan.Records) != 1 || scan.Records[0].Image[0] != 0x5a {
		t.Fatalf("Capture=(%+v,%v)", scan, err)
	}
	scan.Records[0].Image[0] = 0
	_, again, err := Capture(device, layout)
	if err != nil || again.Records[0].Image[0] != 0x5a {
		t.Fatalf("record image aliased device: %+v (%v)", again, err)
	}
	if device.writes != 0 || device.flushes != 0 {
		t.Fatalf("capture wrote device: %d/%d", device.writes, device.flushes)
	}
}

func FuzzCaptureClassifications(f *testing.F) {
	f.Add([]byte{0, 1, 0})
	f.Add([]byte{1, 1, 0})
	f.Add([]byte{2, 0, 0})
	f.Add([]byte{3, 2, 1})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 128 {
			t.Skip()
		}
		pageCount := 1
		if len(input) > 1 {
			pageCount = int(input[1]%3) + 1
		}
		geometry := blockdevicecontract.Geometry{BlockCount: 32, BytesPerBlock: 8}
		layout := layout(geometry, 2, int64(pageCount), 7)
		device := newScripted(geometry)
		pages := make([][]byte, pageCount)
		for page := 0; page < pageCount; page++ {
			kind := byte(0)
			if len(input) > 0 {
				kind = input[page%len(input)] % 4
			}
			pages[page] = make([]byte, 56)
			switch kind {
			case 1:
				encoded, err := storagewalpageslot.EncodePage(56, record(uint64(page+1), uint64(page+1)))
				if err != nil {
					t.Fatal(err)
				}
				pages[page] = encoded
			case 2:
				pages[page][0] = 1
			case 3:
				encoded, err := storagewalpageslot.EncodePage(56, record(uint64(page+2), uint64(page+1)))
				if err != nil {
					t.Fatal(err)
				}
				pages[page] = encoded
			}
			seedRawPage(t, device, layout, int64(page), pages[page])
		}
		want, wantErr := storagewalscan.ScanPages(pages)
		pager, got, gotErr := Capture(device, layout)
		if !sameError(gotErr, wantErr) {
			t.Fatalf("Capture error=%v oracle=%v", gotErr, wantErr)
		}
		if wantErr != nil {
			if pager != (storagepageio.Pager{}) || !zeroScan(got) {
				t.Fatalf("hard refusal returned state: pager=%+v scan=%+v", pager, got)
			}
			return
		}
		if pager == (storagepageio.Pager{}) || !sameScan(got, want) {
			t.Fatalf("scan=%+v oracle=%+v", got, want)
		}
		if device.writes != 0 || device.flushes != 0 {
			t.Fatalf("capture mutated device")
		}
	})
}

func layout(geometry blockdevicecontract.Geometry, first, pages, blocks int64) storagepagelayout.Layout {
	return storagepagelayout.Layout{Geometry: geometry, FirstBlock: first, PageCount: pages, BlocksPerPage: blocks}
}
func record(lsn, tx uint64) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: lsn, TxID: tx}
}
func seedPage(t *testing.T, device *scriptedDevice, layout storagepagelayout.Layout, page int64, record storagewalrecord.Record) {
	t.Helper()
	data, err := storagewalpageslot.EncodePage(layout.BlocksPerPage*layout.Geometry.BytesPerBlock, record)
	if err != nil {
		t.Fatal(err)
	}
	seedRawPage(t, device, layout, page, data)
}
func seedRawPage(t *testing.T, device *scriptedDevice, layout storagepagelayout.Layout, page int64, data []byte) {
	start := layout.FirstBlock + page*layout.BlocksPerPage
	for offset := int64(0); offset < layout.BlocksPerPage; offset++ {
		low := offset * layout.Geometry.BytesPerBlock
		high := low + layout.Geometry.BytesPerBlock
		device.blocks[start+offset] = append([]byte(nil), data[low:high]...)
	}
}
func zeroScan(scan storagewalscan.Scan) bool {
	return scan.Records == nil && scan.Tail == 0 && scan.TailPage == 0 && scan.Cause == nil
}
func sameScan(left, right storagewalscan.Scan) bool {
	if left.Tail != right.Tail || left.TailPage != right.TailPage || !sameError(left.Cause, right.Cause) || len(left.Records) != len(right.Records) {
		return false
	}
	for i := range left.Records {
		a, b := left.Records[i], right.Records[i]
		if a.Kind != b.Kind || a.LSN != b.LSN || a.TxID != b.TxID || a.PageID != b.PageID || string(a.Image) != string(b.Image) {
			return false
		}
	}
	return true
}
func sameError(left, right error) bool {
	if left == nil || right == nil {
		return left == nil && right == nil
	}
	return errors.Is(left, right) && errors.Is(right, left)
}
func bytesOf(value byte, width int) []byte {
	result := make([]byte, width)
	for i := range result {
		result[i] = value
	}
	return result
}
