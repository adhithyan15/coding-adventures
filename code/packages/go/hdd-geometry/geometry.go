// Package hddgeometry maps portable logical blocks to cylinder/surface/sector coordinates.
package hddgeometry

import blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"

const (
	bytesPerBlock int64 = 8
	maxBlockCount int64 = 9_007_199_254_740_991
)

// Geometry describes a fixed geometry whose tracks are grouped by cylinder.
type Geometry struct {
	Cylinders       int64
	Surfaces        int64
	SectorsPerTrack int64
}

// Address is one zero-based cylinder, surface, and sector coordinate.
type Address struct {
	Cylinder int64
	Surface  int64
	Sector   int64
}

// BlockGeometry returns the corresponding portable eight-byte logical blocks.
func (geometry Geometry) BlockGeometry() (blockdevicecontract.Geometry, error) {
	count, ok := geometry.blockCount()
	if !ok {
		return blockdevicecontract.Geometry{}, blockdevicecontract.ErrInvalidGeometry
	}
	return blockdevicecontract.Geometry{BlockCount: count, BytesPerBlock: bytesPerBlock}, nil
}

// Locate maps a relative linear block number to its zero-based address.
func (geometry Geometry) Locate(block int64) (Address, error) {
	blockGeometry, err := geometry.BlockGeometry()
	if err != nil {
		return Address{}, err
	}
	if err := blockGeometry.CheckBlock(block); err != nil {
		return Address{}, err
	}
	track := block / geometry.SectorsPerTrack
	return Address{
		Cylinder: track / geometry.Surfaces,
		Surface:  track % geometry.Surfaces,
		Sector:   block % geometry.SectorsPerTrack,
	}, nil
}

// Block maps a zero-based address to its relative linear block number.
func (geometry Geometry) Block(address Address) (int64, error) {
	if _, err := geometry.BlockGeometry(); err != nil {
		return 0, err
	}
	if address.Cylinder < 0 || address.Cylinder >= geometry.Cylinders ||
		address.Surface < 0 || address.Surface >= geometry.Surfaces ||
		address.Sector < 0 || address.Sector >= geometry.SectorsPerTrack {
		return 0, ErrInvalidAddress
	}
	return (address.Cylinder*geometry.Surfaces+address.Surface)*geometry.SectorsPerTrack + address.Sector, nil
}

// blockCount validates signed dimensions and bounds each product before it is
// computed, keeping accepted counts exactly representable by portable clients.
func (geometry Geometry) blockCount() (int64, bool) {
	if geometry.Cylinders <= 0 || geometry.Surfaces <= 0 || geometry.SectorsPerTrack <= 0 {
		return 0, false
	}
	if geometry.Cylinders > maxBlockCount/geometry.Surfaces {
		return 0, false
	}
	tracks := geometry.Cylinders * geometry.Surfaces
	if tracks > maxBlockCount/geometry.SectorsPerTrack {
		return 0, false
	}
	return tracks * geometry.SectorsPerTrack, true
}
