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

// Result contains the replay receipt and the last rank admitted by the captured
// WAL prefix. LastLSN is zero when the log is empty or recovery is refused.
type Result struct {
	Replay  storagewalredoapply.Receipt
	LastLSN uint64
}

// Recover validates the paired reservations and captured WAL tail before
// passing the complete accepted record prefix to the redo application layer.
// It shares the same single-capture operation as RecoverWithBoundary.
func Recover(device blockdevicecontract.Device, log, data storagepagelayout.Layout) (storagewalredoapply.Receipt, error) {
	result, err := RecoverWithBoundary(device, log, data)
	return result.Replay, err
}

// RecoverWithBoundary replays the accepted record prefix and reports its last
// admitted LSN only when replay completes successfully. Unfinished transaction
// records count toward the boundary even though redo excludes their writes.
func RecoverWithBoundary(device blockdevicecontract.Device, log, data storagepagelayout.Layout) (Result, error) {
	if err := storagewaldatalayout.Validate(log, data); err != nil {
		return Result{}, err
	}
	_, scan, err := storagewalreservation.Capture(device, log)
	if err != nil {
		return Result{}, err
	}
	if scan.Tail == storagewalscan.Damaged {
		return Result{}, &storagewalappend.DamagedTailError{Page: scan.TailPage, Cause: scan.Cause}
	}
	receipt, err := storagewalredoapply.Replay(device, data, scan.Records)
	if err != nil {
		return Result{Replay: receipt}, err
	}
	result := Result{Replay: receipt}
	if len(scan.Records) > 0 {
		result.LastLSN = scan.Records[len(scan.Records)-1].LSN
	}
	return result, nil
}
