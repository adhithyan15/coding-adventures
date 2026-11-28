// Package storagepageio reads and writes fixed-size pages through logical blocks.
package storagepageio

import (
	"errors"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
)

var (
	ErrInvalidDevice      = errors.New("invalid_device")
	ErrGeometryMismatch   = errors.New("geometry_mismatch")
	ErrBlockWidthMismatch = errors.New("block_width_mismatch")
	ErrInvalidPageData    = errors.New("invalid_page_data")
)

// Pager binds a fixed layout to a sequential logical block device.
type Pager struct {
	device blockdevicecontract.Device
	layout storagepagelayout.Layout
}

// Open validates the layout before the device and retains the exact device
// handle for subsequent block operations.
func Open(device blockdevicecontract.Device, layout storagepagelayout.Layout) (Pager, error) {
	if err := layout.Validate(); err != nil {
		return Pager{}, err
	}
	if device == nil {
		return Pager{}, ErrInvalidDevice
	}
	if device.Geometry() != layout.Geometry {
		return Pager{}, ErrGeometryMismatch
	}
	return Pager{device: device, layout: layout}, nil
}

// ReadPage reads blocks in ascending order and assembles one detached page.
func (pager Pager) ReadPage(page int64) ([]byte, error) {
	if pager.device == nil {
		return nil, ErrInvalidDevice
	}
	span, err := pager.layout.Span(page)
	if err != nil {
		return nil, err
	}
	bytesPerBlock := pager.layout.Geometry.BytesPerBlock
	result := make([]byte, int(span.BytesPerPage))
	for offset := int64(0); offset < span.BlockCount; offset++ {
		block, err := pager.device.ReadBlock(span.FirstBlock + offset)
		if err != nil {
			return nil, err
		}
		if int64(len(block)) != bytesPerBlock {
			return nil, ErrBlockWidthMismatch
		}
		start := int(offset * bytesPerBlock)
		copy(result[start:start+int(bytesPerBlock)], block)
	}
	return result, nil
}

// WritePage splits one page into ascending block writes and reports the
// successful prefix if the lower device refuses a later block.
func (pager Pager) WritePage(page int64, data []byte) (int64, error) {
	if pager.device == nil {
		return 0, ErrInvalidDevice
	}
	span, err := pager.layout.Span(page)
	if err != nil {
		return 0, err
	}
	if int64(len(data)) != span.BytesPerPage {
		return 0, ErrInvalidPageData
	}
	bytesPerBlock := pager.layout.Geometry.BytesPerBlock
	for offset := int64(0); offset < span.BlockCount; offset++ {
		start := int(offset * bytesPerBlock)
		end := start + int(bytesPerBlock)
		if err := pager.device.WriteBlock(span.FirstBlock+offset, data[start:end]); err != nil {
			return offset, err
		}
	}
	return span.BlockCount, nil
}

// Flush forwards the sequential device's durability barrier and its error.
func (pager Pager) Flush() error {
	if pager.device == nil {
		return ErrInvalidDevice
	}
	return pager.device.Flush()
}
