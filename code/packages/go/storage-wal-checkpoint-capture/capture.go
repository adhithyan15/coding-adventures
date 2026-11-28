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

// ReadPages validates and reads both complete checkpoint pages in physical
// ordinal order. It checks reserved padding but does not inspect either frame.
func ReadPages(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) ([2][]byte, error) {
	if err := storagewalcheckpointlayout.Validate(log, data, checkpoint); err != nil {
		return [2][]byte{}, err
	}
	pager, err := storagepageio.Open(device, checkpoint)
	if err != nil {
		return [2][]byte{}, err
	}
	var pages [2][]byte
	for ordinal := range pages {
		pages[ordinal], err = pager.ReadPage(int64(ordinal))
		if err != nil {
			return [2][]byte{}, err
		}
	}
	for _, page := range pages {
		for _, value := range page[storagewalcheckpointrecord.FrameBytes:] {
			if value != 0 {
				return [2][]byte{}, ErrCheckpointPadding
			}
		}
	}
	return pages, nil
}

// Capture validates and reads both checkpoint pages, checks their physical
// padding, then selects from their exact frame prefixes.
func Capture(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Snapshot, error) {
	pages, err := ReadPages(device, log, data, checkpoint)
	if err != nil {
		return Snapshot{}, err
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
