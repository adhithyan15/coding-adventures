// Package storagewalappend appends one WAL record and acknowledges its barrier.
package storagewalappend

import (
	"errors"
	"fmt"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalreservation "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

const MaxLogPages = storagewalreservation.MaxLogPages

var (
	ErrLogCapacity    = storagewalreservation.ErrLogCapacity
	ErrLogFull        = errors.New("WAL reservation has no vacant page")
	ErrDamagedTail    = errors.New("WAL reservation ends in a damaged page")
	ErrAppendSequence = errors.New("record LSN is not the next WAL sequence")
)

// DamagedTailError identifies the reservation page and lower decoder error
// that established a damaged append tail.
type DamagedTailError struct {
	Page  int64
	Cause error
}

func (err *DamagedTailError) Error() string {
	return fmt.Sprintf("WAL damaged tail at page %d: %v", err.Page, err.Cause)
}

func (err *DamagedTailError) Unwrap() []error {
	if err.Cause == nil {
		return []error{ErrDamagedTail}
	}
	return []error{ErrDamagedTail, err.Cause}
}

// Receipt reports whether a write was attempted, its successful block prefix,
// and whether the following durability barrier acknowledged it.
type Receipt struct {
	Attempted     bool
	Page          int64
	WrittenBlocks int64
	Acknowledged  bool
}

// Append validates the reservation and current WAL prefix, writes the next
// page once, and then asks the block device to make the write durable.
func Append(device blockdevicecontract.Device, layout storagepagelayout.Layout, record storagewalrecord.Record) (Receipt, error) {
	pager, scan, err := storagewalreservation.Capture(device, layout)
	if err != nil {
		return Receipt{}, err
	}
	pageBytes := layout.BlocksPerPage * layout.Geometry.BytesPerBlock
	if scan.Tail == storagewalscan.Full {
		return Receipt{}, ErrLogFull
	}
	if scan.Tail == storagewalscan.Damaged {
		return Receipt{}, &DamagedTailError{Page: scan.TailPage, Cause: scan.Cause}
	}
	expectedLSN := uint64(len(scan.Records) + 1)
	if record.LSN != expectedLSN {
		return Receipt{}, ErrAppendSequence
	}
	page, err := storagewalpageslot.EncodePage(pageBytes, record)
	if err != nil {
		return Receipt{}, err
	}

	destination := scan.TailPage
	written, err := pager.WritePage(destination, page)
	receipt := Receipt{Attempted: true, Page: destination, WrittenBlocks: written}
	if err != nil {
		return receipt, err
	}
	if err := pager.Flush(); err != nil {
		return receipt, err
	}
	receipt.Acknowledged = true
	return receipt, nil
}
