// Package storagewalreservation captures and classifies a fixed WAL reservation.
package storagewalreservation

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalscan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan"
)

const MaxLogPages int64 = 1_024

var ErrLogCapacity = errors.New("WAL reservation exceeds page capacity")

// Capture opens a checked pager, reads the entire bounded reservation in page
// order, and classifies its captured WAL prefix without writing or flushing.
func Capture(device blockdevicecontract.Device, layout storagepagelayout.Layout) (storagepageio.Pager, storagewalscan.Scan, error) {
	pager, err := storagepageio.Open(device, layout)
	if err != nil {
		return storagepageio.Pager{}, storagewalscan.Scan{}, err
	}
	if layout.PageCount > MaxLogPages {
		return storagepageio.Pager{}, storagewalscan.Scan{}, ErrLogCapacity
	}
	pageBytes := layout.BlocksPerPage * layout.Geometry.BytesPerBlock
	if pageBytes < storagewalpageslot.MinPageBytes || pageBytes > storagewalpageslot.MaxPageBytes {
		return storagepageio.Pager{}, storagewalscan.Scan{}, storagewalpageslot.ErrPageSize
	}

	pages := make([][]byte, int(layout.PageCount))
	for page := int64(0); page < layout.PageCount; page++ {
		pages[int(page)], err = pager.ReadPage(page)
		if err != nil {
			return storagepageio.Pager{}, storagewalscan.Scan{}, err
		}
	}
	scan, err := storagewalscan.ScanPages(pages)
	if err != nil {
		return storagepageio.Pager{}, storagewalscan.Scan{}, err
	}
	return pager, scan, nil
}
