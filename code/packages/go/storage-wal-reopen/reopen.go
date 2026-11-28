// Package storagewalreopen selects the durable restart path from a WAL checkpoint.
package storagewalreopen

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointadvance "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-advance"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalrecovery "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-recovery"
)

// Result preserves the selected checkpoint, the Ready epoch, and the exact
// replay or reset-continuation reports produced by the lower operations.
type Result struct {
	Observed storagewalcheckpointrecord.Record
	Ready    storagewalcheckpointrecord.Record
	Replay   storagewalrecovery.Result
	Resume   storagewalcheckpointadvance.Report
}

// Reopen follows the selected durable checkpoint. Callers initialize vacant
// checkpoint media through the existing initializer before reopening it.
func Reopen(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Result, error) {
	captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
	if err != nil {
		return Result{}, err
	}

	observed := captured.Selection.Record
	result := Result{Observed: observed}
	switch observed.Phase {
	case storagewalcheckpointrecord.Ready:
		result.Replay, err = storagewalrecovery.RecoverWithBoundary(device, log, data)
		if err != nil {
			return result, err
		}
		result.Ready = observed
		return result, nil
	case storagewalcheckpointrecord.Resetting:
		result.Resume, err = storagewalcheckpointadvance.Advance(device, log, data, checkpoint, observed.Epoch)
		if err != nil {
			return result, err
		}
		result.Ready = storagewalcheckpointrecord.Record{
			Phase:       storagewalcheckpointrecord.Ready,
			Epoch:       observed.Epoch,
			PreviousLSN: observed.PreviousLSN,
		}
		return result, nil
	default:
		// Capture admits only the checkpoint record's two declared phases.
		return Result{}, storagewalcheckpointrecord.ErrPhase
	}
}
