// Package storagewalcommit publishes a transaction to the WAL before replaying its data images.
package storagewalcommit

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpreparecommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-prepare-commit"
	storagewalpublishcommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-publish-commit"
	storagewalredoapply "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-apply"
)

// Result preserves both the acknowledged log publication and its subsequent data replay.
type Result struct {
	Log  storagewalpublishcommit.Result
	Data storagewalredoapply.Receipt
}

// Commit publishes one transaction to the WAL, then replays only that transaction's
// detached records. A log refusal prevents data effects; a data refusal retains the
// acknowledged log receipt so recovery can finish the committed transaction later.
func Commit(device blockdevicecontract.Device, log, data storagepagelayout.Layout, txID uint64, images []storagewalpreparecommit.PageImage) (Result, error) {
	published, err := storagewalpublishcommit.Publish(device, log, data, txID, images)
	if err != nil {
		return Result{Log: published}, err
	}
	replayed, err := storagewalredoapply.Replay(device, data, published.Committed)
	return Result{Log: published, Data: replayed}, err
}
