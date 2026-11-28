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

var (
	ErrCheckpointOccupied  = errors.New("checkpoint initialization slot is occupied")
	ErrInitialLogNonempty  = errors.New("initial WAL reservation is not empty")
	ErrInitialDataNonempty = errors.New("initial data reservation is not empty")
)

// Initialize publishes the first Ready checkpoint only when slot one is vacant
// and both the log and data reservations contain only zero bytes.
func Initialize(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Receipt, error) {
	pages, err := storagewalcheckpointcapture.ReadPages(device, log, data, checkpoint)
	if err != nil {
		return Receipt{}, err
	}

	var states [2]storagewalcheckpointslot.State
	var records [2]storagewalcheckpointrecord.Record
	for ordinal := range pages {
		frames := pages[ordinal][:storagewalcheckpointrecord.FrameBytes]
		states[ordinal], records[ordinal], err = storagewalcheckpointslot.Inspect(frames)
		if err != nil {
			return Receipt{}, err
		}
	}
	if states[1] != storagewalcheckpointslot.Vacant {
		return Receipt{}, ErrCheckpointOccupied
	}

	initial := storagewalcheckpointrecord.Record{
		Phase: storagewalcheckpointrecord.Ready,
		Epoch: 1,
	}
	alreadyCurrent := states[0] == storagewalcheckpointslot.Published && records[0] == initial
	if !alreadyCurrent && states[0] != storagewalcheckpointslot.Vacant && states[0] != storagewalcheckpointslot.Staged {
		return Receipt{}, ErrCheckpointOccupied
	}
	if err := requireZeroReservation(device, log, ErrInitialLogNonempty); err != nil {
		return Receipt{}, err
	}
	if err := requireZeroReservation(device, data, ErrInitialDataNonempty); err != nil {
		return Receipt{}, err
	}
	if alreadyCurrent {
		return Receipt{Slot: 0, Confirmed: true, AlreadyCurrent: true}, nil
	}

	stagedFrame, err := storagewalcheckpointslot.Stage(initial)
	if err != nil {
		return Receipt{}, err
	}
	return publishPrepared(device, checkpoint, 0, pages[0], stagedFrame)
}

func requireZeroReservation(device blockdevicecontract.Device, layout storagepagelayout.Layout, nonempty error) error {
	end := layout.FirstBlock + layout.PageCount*layout.BlocksPerPage
	width := int(layout.Geometry.BytesPerBlock)
	for block := layout.FirstBlock; block < end; block++ {
		value, err := device.ReadBlock(block)
		if err != nil {
			return err
		}
		if len(value) != width {
			return storagepageio.ErrBlockWidthMismatch
		}
		for _, octet := range value {
			if octet != 0 {
				return nonempty
			}
		}
	}
	return nil
}
