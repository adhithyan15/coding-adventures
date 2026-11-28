// Package storagewalcheckpointlayout admits two checkpoint pages beside WAL and data.
package storagewalcheckpointlayout

import (
	"errors"

	storagepageio "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewaldatalayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-data-layout"
)

var (
	ErrCheckpointShape   = errors.New("checkpoint reservation has invalid shape")
	ErrCheckpointOverlap = errors.New("checkpoint reservation overlaps log or data")
)

// Validate admits the exact two-page checkpoint interval after the existing
// log/data pair and checkpoint page layout are individually validated.
func Validate(log, data, checkpoint storagepagelayout.Layout) error {
	if err := storagewaldatalayout.Validate(log, data); err != nil {
		return err
	}
	if err := checkpoint.Validate(); err != nil {
		return err
	}
	if checkpoint.Geometry != log.Geometry || checkpoint.Geometry != data.Geometry {
		return storagepageio.ErrGeometryMismatch
	}
	requiredBlocks := int64(1 + (storagewalcheckpointrecord.FrameBytes-1)/int(checkpoint.Geometry.BytesPerBlock))
	if checkpoint.PageCount != 2 || checkpoint.BlocksPerPage != requiredBlocks {
		return ErrCheckpointShape
	}
	if overlaps(checkpoint, log) || overlaps(checkpoint, data) {
		return ErrCheckpointOverlap
	}
	return nil
}

func overlaps(left, right storagepagelayout.Layout) bool {
	leftEnd := left.FirstBlock + left.PageCount*left.BlocksPerPage
	rightEnd := right.FirstBlock + right.PageCount*right.BlocksPerPage
	return left.FirstBlock < rightEnd && right.FirstBlock < leftEnd
}
