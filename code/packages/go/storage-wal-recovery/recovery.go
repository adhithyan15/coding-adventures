// Package storagewalrecovery replays committed WAL images after a restart.
package storagewalrecovery

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalappend "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-append"
	storagewaldatalayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-data-layout"
	storagewalredoapply "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-apply"
	storagewalreservation "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

// Recover validates the paired reservations and captured WAL tail before
// passing the complete accepted record prefix to the redo application layer.
func Recover(device blockdevicecontract.Device, log, data storagepagelayout.Layout) (storagewalredoapply.Receipt, error) {
	if err := storagewaldatalayout.Validate(log, data); err != nil {
		return storagewalredoapply.Receipt{}, err
	}
	_, scan, err := storagewalreservation.Capture(device, log)
	if err != nil {
		return storagewalredoapply.Receipt{}, err
	}
	if scan.Tail == storagewalscan.Damaged {
		return storagewalredoapply.Receipt{}, &storagewalappend.DamagedTailError{Page: scan.TailPage, Cause: scan.Cause}
	}
	return storagewalredoapply.Replay(device, data, scan.Records)
}
