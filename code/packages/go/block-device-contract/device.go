// Package blockdevicecontract defines a portable sequential contract for
// logical block devices. It owns no storage backend, timing policy, or media
// recovery rule; a concrete implementation advertises its own capabilities.
package blockdevicecontract

import "errors"

const (
	maximumSafeBlockCount int64 = 9_007_199_254_740_991
	maximumBytesPerBlock  int64 = 65_536
)

// Stable refusal identities preserve the order of geometry, address, then
// write-shape validation.
var (
	ErrInvalidGeometry  = errors.New("invalid_geometry")
	ErrInvalidBlock     = errors.New("invalid_block")
	ErrInvalidBlockData = errors.New("invalid_block_data")
)

// Geometry describes a fixed logical address space. The limits are chosen so
// that all accepted values remain portable through common exact-integer APIs.
type Geometry struct {
	BlockCount    int64
	BytesPerBlock int64
}

// Validate rejects counts and widths outside the portable logical range.
func (geometry Geometry) Validate() error {
	if geometry.BlockCount < 1 || geometry.BlockCount > maximumSafeBlockCount {
		return ErrInvalidGeometry
	}
	if geometry.BytesPerBlock < 1 || geometry.BytesPerBlock > maximumBytesPerBlock {
		return ErrInvalidGeometry
	}
	return nil
}

// CheckBlock validates geometry before checking the signed logical address.
func (geometry Geometry) CheckBlock(block int64) error {
	if err := geometry.Validate(); err != nil {
		return err
	}
	if block < 0 || block >= geometry.BlockCount {
		return ErrInvalidBlock
	}
	return nil
}

// CheckWrite validates geometry, block address, then exact data width. It does
// not inspect or modify the caller's bytes and performs no allocation.
func (geometry Geometry) CheckWrite(block int64, data []byte) error {
	if err := geometry.CheckBlock(block); err != nil {
		return err
	}
	if len(data) != int(geometry.BytesPerBlock) {
		return ErrInvalidBlockData
	}
	return nil
}

// Device is the first sequential logical block interface. A successful write
// becomes visible to subsequent reads; Flush acknowledges earlier successful
// writes as durable. Backends define their exact recovery profile separately.
type Device interface {
	Geometry() Geometry
	ReadBlock(block int64) ([]byte, error)
	WriteBlock(block int64, data []byte) error
	Flush() error
}
