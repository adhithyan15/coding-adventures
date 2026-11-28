// Package storagewalredoapply replays a committed WAL redo plan through page I/O.
package storagewalredoapply

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalredoplan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-plan"
)

// Receipt reports completed image barriers or the exact image operation that
// interrupted replay. PageID and RecordLSN are zero after complete replay.
type Receipt struct {
	AcknowledgedWrites int
	Interrupted        bool
	PageID             int64
	RecordLSN          uint64
	WrittenBlocks      int64
}

// Replay applies each committed full-page image and requests a durability
// barrier before moving to the next image.
func Replay(device blockdevicecontract.Device, layout storagepagelayout.Layout, records []storagewalrecord.Record) (Receipt, error) {
	pager, err := storagepageio.Open(device, layout)
	if err != nil {
		return Receipt{}, err
	}
	plan, err := storagewalredoplan.Plan(layout, records)
	if err != nil {
		return Receipt{}, err
	}
	var receipt Receipt
	for _, write := range plan {
		written, err := pager.WritePage(write.Record.PageID, write.Record.Image)
		if err == nil {
			err = pager.Flush()
		}
		if err != nil {
			return Receipt{
				AcknowledgedWrites: receipt.AcknowledgedWrites,
				Interrupted:        true,
				PageID:             write.Record.PageID,
				RecordLSN:          write.Record.LSN,
				WrittenBlocks:      written,
			}, err
		}
		receipt.AcknowledgedWrites++
	}
	return receipt, nil
}
