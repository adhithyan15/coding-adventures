// Package storagewalreadycommit gates WAL publication on the durable Ready checkpoint epoch.
package storagewalreadycommit

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-commit"
	storagewalpreparecommit "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-prepare-commit"
)

const maxPortableEpoch uint64 = 9_007_199_254_740_991

var (
	ErrInvalidEpoch        = errors.New("requested checkpoint epoch is invalid")
	ErrCheckpointResetting = errors.New("checkpoint is resetting")
	ErrCheckpointEpoch     = errors.New("checkpoint epoch does not match request")
)

// Result combines the durable checkpoint that authorized a commit with the
// exact receipt returned by the underlying WAL commit.
type Result struct {
	Checkpoint storagewalcheckpointrecord.Record
	Commit     storagewalcommit.Result
}

// Commit captures the durable checkpoint and permits WAL commit only when the
// selected checkpoint is Ready at epoch. Refusals before the underlying commit
// have no log or data effects.
func Commit(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout, epoch, txID uint64, images []storagewalpreparecommit.PageImage) (Result, error) {
	if epoch == 0 || epoch > maxPortableEpoch {
		return Result{}, ErrInvalidEpoch
	}

	captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
	if err != nil {
		return Result{}, err
	}
	selected := captured.Selection.Record
	if selected.Phase == storagewalcheckpointrecord.Resetting {
		return Result{}, ErrCheckpointResetting
	}
	if selected.Epoch != epoch {
		return Result{}, ErrCheckpointEpoch
	}

	committed, err := storagewalcommit.Commit(device, log, data, txID, images)
	return Result{Checkpoint: selected, Commit: committed}, err
}
