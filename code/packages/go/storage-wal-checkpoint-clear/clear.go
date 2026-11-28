// Package storagewalcheckpointclear clears the prior WAL only under a selected Resetting checkpoint.
package storagewalcheckpointclear

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointcapture "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
)

var ErrCheckpointNotResetting = errors.New("selected checkpoint is not Resetting")

// Receipt identifies the durable checkpoint authorizing the clear, the
// successfully written log-block prefix, and acknowledgement of the final barrier.
type Receipt struct {
	Checkpoint    storagewalcheckpointrecord.Record
	BlocksWritten int64
	Acknowledged  bool
}

// Clear writes zero to the validated log reservation only when the selected
// checkpoint is Resetting, then acknowledges one barrier.
func Clear(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Receipt, error) {
	captured, err := storagewalcheckpointcapture.Capture(device, log, data, checkpoint)
	if err != nil {
		return Receipt{}, err
	}
	if captured.Selection.Record.Phase != storagewalcheckpointrecord.Resetting {
		return Receipt{}, ErrCheckpointNotResetting
	}

	receipt := Receipt{Checkpoint: captured.Selection.Record}
	zeroBlock := make([]byte, int(log.Geometry.BytesPerBlock))
	end := log.FirstBlock + log.PageCount*log.BlocksPerPage
	for block := log.FirstBlock; block < end; block++ {
		if err := device.WriteBlock(block, zeroBlock); err != nil {
			return receipt, err
		}
		receipt.BlocksWritten++
	}
	if err := device.Flush(); err != nil {
		return receipt, err
	}
	receipt.Acknowledged = true
	return receipt, nil
}
