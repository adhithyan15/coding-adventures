// Package storagepagelayout maps fixed-width database pages onto logical blocks.
package storagepagelayout

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
)

// MaxPageBytes is the portable maximum width of one storage page.
const MaxPageBytes int64 = 65_536

// ErrInvalidLayout identifies a reservation that does not fit its geometry.
// ErrInvalidPage identifies a page ID outside a valid reservation.
var (
	ErrInvalidLayout = errors.New("invalid_layout")
	ErrInvalidPage   = errors.New("invalid_page")
)

// Layout reserves a contiguous interval of logical blocks for fixed-size pages.
type Layout struct {
	Geometry      blockdevicecontract.Geometry
	FirstBlock    int64
	PageCount     int64
	BlocksPerPage int64
}

// BlockSpan describes the exact block interval and byte width of one page.
type BlockSpan struct {
	FirstBlock   int64
	BlockCount   int64
	BytesPerPage int64
}

// Validate checks the underlying device geometry first, then the page
// reservation without multiplying untrusted signed values.
func (layout Layout) Validate() error {
	if err := layout.Geometry.Validate(); err != nil {
		return err
	}
	if layout.FirstBlock < 0 || layout.FirstBlock >= layout.Geometry.BlockCount || layout.PageCount < 1 || layout.BlocksPerPage < 1 {
		return ErrInvalidLayout
	}
	if layout.BlocksPerPage > MaxPageBytes/layout.Geometry.BytesPerBlock {
		return ErrInvalidLayout
	}
	remainingBlocks := layout.Geometry.BlockCount - layout.FirstBlock
	if layout.PageCount > remainingBlocks/layout.BlocksPerPage {
		return ErrInvalidLayout
	}
	return nil
}

// Span returns the contiguous blocks occupied by a zero-based page ID.
func (layout Layout) Span(page int64) (BlockSpan, error) {
	if err := layout.Validate(); err != nil {
		return BlockSpan{}, err
	}
	if page < 0 || page >= layout.PageCount {
		return BlockSpan{}, ErrInvalidPage
	}
	return BlockSpan{
		FirstBlock:   layout.FirstBlock + page*layout.BlocksPerPage,
		BlockCount:   layout.BlocksPerPage,
		BytesPerPage: layout.BlocksPerPage * layout.Geometry.BytesPerBlock,
	}, nil
}
