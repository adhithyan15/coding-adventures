// Package storagewaldatalayout admits disjoint log and data page reservations.
package storagewaldatalayout

import (
	"errors"

	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalreservation "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation"
)

var (
	ErrLogTooSmall     = errors.New("WAL log reservation needs at least three slots")
	ErrLayoutOverlap   = errors.New("WAL log and data reservations overlap")
	ErrImageDoesNotFit = errors.New("data page image does not fit in one WAL slot")
)

// Validate checks the log first, then the data layout, and admits both only
// when they occupy disjoint blocks on one device and a full data image fits.
func Validate(log, data storagepagelayout.Layout) error {
	if err := log.Validate(); err != nil {
		return err
	}
	if err := data.Validate(); err != nil {
		return err
	}
	if log.Geometry != data.Geometry {
		return storagepageio.ErrGeometryMismatch
	}
	if log.PageCount > storagewalreservation.MaxLogPages {
		return storagewalreservation.ErrLogCapacity
	}
	if log.PageCount < 3 {
		return ErrLogTooSmall
	}

	logEnd := log.FirstBlock + log.PageCount*log.BlocksPerPage
	dataEnd := data.FirstBlock + data.PageCount*data.BlocksPerPage
	if log.FirstBlock < dataEnd && data.FirstBlock < logEnd {
		return ErrLayoutOverlap
	}

	logPageBytes := log.BlocksPerPage * log.Geometry.BytesPerBlock
	if logPageBytes < storagewalpageslot.MinPageBytes || logPageBytes > storagewalpageslot.MaxPageBytes {
		return storagewalpageslot.ErrPageSize
	}
	dataPageBytes := data.BlocksPerPage * data.Geometry.BytesPerBlock
	if dataPageBytes > logPageBytes-storagewalpageslot.MinPageBytes {
		return ErrImageDoesNotFit
	}
	return nil
}
