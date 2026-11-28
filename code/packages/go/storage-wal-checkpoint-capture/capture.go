// Package storagewalcheckpointcapture reads and selects two persisted checkpoint pages.
package storagewalcheckpointcapture

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointlayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-layout"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointselection "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-selection"
)

var ErrCheckpointPadding = errors.New("checkpoint page has nonzero reserved padding")

// Snapshot preserves the selected slot and both complete, detached physical pages.
type Snapshot struct {
	Selection storagewalcheckpointselection.Selection
	Pages     [2][]byte
}

// Capture validates and reads both checkpoint pages in order, checks reserved
// padding, and then selects from their exact frame prefixes.
func Capture(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Snapshot, error) {
	if err := storagewalcheckpointlayout.Validate(log, data, checkpoint); err != nil {
		return Snapshot{}, err
	}
	pager, err := storagepageio.Open(device, checkpoint)
	if err != nil {
		return Snapshot{}, err
	}
	var pages [2][]byte
	for ordinal := range pages {
		pages[ordinal], err = pager.ReadPage(int64(ordinal))
		if err != nil {
			return Snapshot{}, err
		}
	}
	for _, page := range pages {
		for _, value := range page[storagewalcheckpointrecord.FrameBytes:] {
			if value != 0 {
				return Snapshot{}, ErrCheckpointPadding
			}
		}
	}
	frames := [2][]byte{
		pages[0][:storagewalcheckpointrecord.FrameBytes],
		pages[1][:storagewalcheckpointrecord.FrameBytes],
	}
	selection, err := storagewalcheckpointselection.Select(frames)
	if err != nil {
		return Snapshot{}, err
	}
	return Snapshot{Selection: selection, Pages: pages}, nil
}
