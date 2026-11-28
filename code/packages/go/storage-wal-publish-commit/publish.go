// Package storagewalpublishcommit appends one fully prepared WAL transaction.
package storagewalpublishcommit

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalappend "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-append"
	storagewaldatalayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-data-layout"
	storagewalpreparecommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-prepare-commit"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalreservation "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

// Result reports the exact last append attempt and any acknowledged prefix.
// Committed is nonnil only after every transaction record is acknowledged.
type Result struct {
	AcknowledgedRecords int
	LastLSN             uint64
	LastAppend          storagewalappend.Receipt
	Committed           []storagewalrecord.Record
}

// Publish validates and captures the existing WAL, prepares a complete
// transaction, then appends each record once and in order.
func Publish(device blockdevicecontract.Device, log, data storagepagelayout.Layout, txID uint64, images []storagewalpreparecommit.PageImage) (Result, error) {
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
	prepared, err := storagewalpreparecommit.Prepare(log, data, scan.Records, txID, images)
	if err != nil {
		return Result{}, err
	}

	var result Result
	for _, record := range prepared {
		appendResult, err := storagewalappend.Append(device, log, record)
		result.LastLSN = record.LSN
		result.LastAppend = appendResult
		if err != nil {
			return result, err
		}
		result.AcknowledgedRecords++
	}
	result.Committed = prepared
	return result, nil
}
