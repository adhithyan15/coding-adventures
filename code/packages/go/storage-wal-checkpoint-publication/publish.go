// Package storagewalcheckpointpublication publishes a checkpoint successor in three durable steps.
package storagewalcheckpointpublication

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
)

// Step identifies the publication step attempted by a write path.
type Step uint8

const (
	NoWrite Step = iota
	InvalidateGuard
	StageBody
	PublishGuard
)

var ErrCheckpointTransition = errors.New("checkpoint record is not an allowed successor")

// Receipt records the attempted destination and acknowledged publication progress.
type Receipt struct {
	Slot           uint8
	Attempted      Step
	BodyBlocks     int64
	Confirmed      bool
	AlreadyCurrent bool
}

// Publish validates and publishes desired in the slot other than the selected
// current slot. It performs three ordered write/barrier steps and preserves the
// first lower error.
func Publish(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout, desired storagewalcheckpointrecord.Record) (Receipt, error) {
	stagedFrame, err := storagewalcheckpointslot.Stage(desired)
	if err != nil {
		return Receipt{}, err
	}
	current, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
	if err != nil {
		return Receipt{}, err
	}
	if desired == current.Selection.Record {
		return Receipt{Slot: current.Selection.Slot, Confirmed: true, AlreadyCurrent: true}, nil
	}
	if !validSuccessor(current.Selection.Record, desired) {
		return Receipt{}, ErrCheckpointTransition
	}

	destination := uint8(1 - current.Selection.Slot)
	return publishPrepared(device, checkpoint, destination, current.Pages[destination], stagedFrame)
}

// publishPrepared performs the three guarded write/barrier steps against a
// preflighted destination page. Callers own responsibility for admission.
func publishPrepared(device blockdevicecontract.Device, checkpoint storagepagelayout.Layout, destination uint8, oldPage, stagedFrame []byte) (Receipt, error) {
	span, err := checkpoint.Span(int64(destination))
	if err != nil {
		return Receipt{}, err
	}
	bytesPerBlock := int(checkpoint.Geometry.BytesPerBlock)
	guardBlockOffset := storagewalcheckpointslot.GuardOffset / bytesPerBlock
	guardByteOffset := storagewalcheckpointslot.GuardOffset % bytesPerBlock
	guardBlock := span.FirstBlock + int64(guardBlockOffset)
	pageOffset := guardBlockOffset * bytesPerBlock
	guard := append([]byte(nil), oldPage[pageOffset:pageOffset+bytesPerBlock]...)
	pager, err := storagepageio.Open(device, checkpoint)
	if err != nil {
		return Receipt{}, err
	}

	receipt := Receipt{Slot: destination}
	receipt.Attempted = InvalidateGuard
	guard[guardByteOffset] = storagewalcheckpointslot.StagedGuard
	if err := device.WriteBlock(guardBlock, guard); err != nil {
		return receipt, err
	}
	if err := device.Flush(); err != nil {
		return receipt, err
	}

	stagedPage := make([]byte, len(oldPage))
	copy(stagedPage, stagedFrame)
	receipt.Attempted = StageBody
	receipt.BodyBlocks, err = pager.WritePage(int64(destination), stagedPage)
	if err != nil {
		return receipt, err
	}
	if err := pager.Flush(); err != nil {
		return receipt, err
	}

	receipt.Attempted = PublishGuard
	copy(guard, stagedPage[pageOffset:pageOffset+bytesPerBlock])
	guard[guardByteOffset] = 0
	if err := device.WriteBlock(guardBlock, guard); err != nil {
		return receipt, err
	}
	if err := device.Flush(); err != nil {
		return receipt, err
	}
	receipt.Confirmed = true
	return receipt, nil
}

func validSuccessor(current, desired storagewalcheckpointrecord.Record) bool {
	if current.Phase == storagewalcheckpointrecord.Ready {
		return desired.Phase == storagewalcheckpointrecord.Resetting && desired.Epoch == current.Epoch+1
	}
	return current.Phase == storagewalcheckpointrecord.Resetting &&
		desired.Phase == storagewalcheckpointrecord.Ready &&
		desired.Epoch == current.Epoch &&
		desired.PreviousLSN == current.PreviousLSN
}
