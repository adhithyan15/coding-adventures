// Package storagewalcheckpointadvance composes recovery, checkpoint publication,
// and WAL clearing into one explicit epoch transition.
package storagewalcheckpointadvance

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointclear "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-clear"
	storagewalcheckpointpublication "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-publication"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalrecovery "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-recovery"
)

const maxPortableEpoch uint64 = 9_007_199_254_740_991

var (
	ErrTargetEpoch     = errors.New("target checkpoint epoch is invalid")
	ErrCheckpointOrder = errors.New("checkpoint does not permit the requested epoch transition")
)

// Report retains the observed checkpoint, recovered boundary, and each
// completed or partially attempted durable transition.
type Report struct {
	Observed  storagewalcheckpointrecord.Record
	Replay    storagewalrecovery.Result
	Resetting storagewalcheckpointpublication.Receipt
	Clear     storagewalcheckpointclear.Receipt
	Ready     storagewalcheckpointpublication.Receipt
}

// Advance resumes or advances one explicit checkpoint epoch. A selected
// Resetting record skips replay and continues clearing the already-authorized
// prior log; a Ready record advances only to its immediate successor.
func Advance(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout, target uint64) (Report, error) {
	if target == 0 || target > maxPortableEpoch {
		return Report{}, ErrTargetEpoch
	}
	captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
	if err != nil {
		return Report{}, err
	}
	observed := captured.Selection.Record
	report := Report{Observed: observed}
	initial := storagewalcheckpointrecord.Record{
		Phase:       storagewalcheckpointrecord.Ready,
		Epoch:       target,
		PreviousLSN: observed.PreviousLSN,
	}

	switch observed.Phase {
	case storagewalcheckpointrecord.Ready:
		if observed.Epoch == target {
			report.Ready = storagewalcheckpointpublication.Receipt{
				Slot:           captured.Selection.Slot,
				Confirmed:      true,
				AlreadyCurrent: true,
			}
			return report, nil
		}
		if observed.Epoch == maxPortableEpoch || target != observed.Epoch+1 {
			return Report{}, ErrCheckpointOrder
		}
		replay, err := storagewalrecovery.RecoverWithBoundary(device, log, data)
		report.Replay = replay
		if err != nil {
			return report, err
		}
		initial.PreviousLSN = replay.LastLSN
		resetting := storagewalcheckpointrecord.Record{
			Phase:       storagewalcheckpointrecord.Resetting,
			Epoch:       target,
			PreviousLSN: replay.LastLSN,
		}
		report.Resetting, err = storagewalcheckpointpublication.Publish(device, log, data, checkpoint, resetting)
		if err != nil {
			return report, err
		}
	case storagewalcheckpointrecord.Resetting:
		if observed.Epoch != target {
			return Report{}, ErrCheckpointOrder
		}
		initial.PreviousLSN = observed.PreviousLSN
	default:
		return Report{}, ErrCheckpointOrder
	}

	report.Clear, err = storagewalcheckpointclear.Clear(device, log, data, checkpoint)
	if err != nil {
		return report, err
	}
	report.Ready, err = storagewalcheckpointpublication.Publish(device, log, data, checkpoint, initial)
	if err != nil {
		return report, err
	}
	return report, nil
}
