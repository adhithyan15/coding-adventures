// Package memoryblockdevice implements a sparse, deterministic, flush-backed
// logical block device for testing sequential storage clients.
package memoryblockdevice

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
)

// Memory keeps pending and durable complete blocks in separate sparse maps.
// Its private byte slices are immutable after insertion and can be shared by
// Restart results; every public byte result is detached.
type Memory struct {
	geometry blockdevicecontract.Geometry
	durable  map[int64][]byte
	pending  map[int64][]byte
}

var _ blockdevicecontract.Device = (*Memory)(nil)

// New creates an empty logical device without allocating any block contents.
func New(geometry blockdevicecontract.Geometry) (*Memory, error) {
	if err := geometry.Validate(); err != nil {
		return nil, err
	}
	return &Memory{
		geometry: geometry,
		durable:  make(map[int64][]byte),
		pending:  make(map[int64][]byte),
	}, nil
}

// Geometry returns this device's fixed logical dimensions. A nil receiver has
// the same zero geometry as an unconfigured value.
func (device *Memory) Geometry() blockdevicecontract.Geometry {
	if device == nil {
		return blockdevicecontract.Geometry{}
	}
	return device.geometry
}

// ReadBlock returns pending bytes first, then durable bytes, and otherwise a
// fresh all-zero logical block.
func (device *Memory) ReadBlock(block int64) ([]byte, error) {
	geometry := device.Geometry()
	if err := geometry.CheckBlock(block); err != nil {
		return nil, err
	}
	if device != nil {
		if data, exists := device.pending[block]; exists {
			return cloneBytes(data), nil
		}
		if data, exists := device.durable[block]; exists {
			return cloneBytes(data), nil
		}
	}
	return make([]byte, int(geometry.BytesPerBlock)), nil
}

// WriteBlock copies a complete caller block into the sparse pending image.
func (device *Memory) WriteBlock(block int64, data []byte) error {
	geometry := device.Geometry()
	if err := geometry.CheckWrite(block, data); err != nil {
		return err
	}
	if device.pending == nil {
		device.pending = make(map[int64][]byte)
	}
	device.pending[block] = cloneBytes(data)
	return nil
}

// Flush publishes all pending blocks to the durable image and empties the
// pending image. Flushing an empty pending image succeeds.
func (device *Memory) Flush() error {
	if err := device.Geometry().Validate(); err != nil {
		return err
	}
	if len(device.pending) == 0 {
		return nil
	}
	if device.durable == nil {
		device.durable = make(map[int64][]byte, len(device.pending))
	}
	for block, data := range device.pending {
		device.durable[block] = data
	}
	device.pending = make(map[int64][]byte)
	return nil
}

// Restart creates a new device from the current durable image, discarding
// pending writes while preserving this device and all of its observations.
func (device *Memory) Restart() (*Memory, error) {
	if err := device.Geometry().Validate(); err != nil {
		return nil, err
	}
	return &Memory{
		geometry: device.geometry,
		durable:  cloneMap(device.durable),
		pending:  make(map[int64][]byte),
	}, nil
}

func cloneBytes(data []byte) []byte {
	return append([]byte(nil), data...)
}

func cloneMap(source map[int64][]byte) map[int64][]byte {
	result := make(map[int64][]byte, len(source))
	for block, data := range source {
		result[block] = data
	}
	return result
}
