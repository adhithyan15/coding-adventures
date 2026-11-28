// Package hddblockdevice adapts the HDD write-back cache to logical blocks.
package hddblockdevice

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
)

const (
	blockCount    int64 = hddwritebackcache.SectorCount
	bytesPerBlock int64 = hddwritebackcache.BytesPerSector
)

var _ blockdevicecontract.Device = (*HDD)(nil)

// HDD holds the immutable lower cache state for one 32-block, eight-byte HDD.
type HDD struct {
	cache hddwritebackcache.State
}

// New creates a fresh HDD whose blocks are all zero and have no pending writes.
func New() *HDD { return &HDD{} }

// Geometry returns zero for a nil receiver and this profile's fixed geometry
// for every actual HDD value, including its zero value.
func (device *HDD) Geometry() blockdevicecontract.Geometry {
	if device == nil {
		return blockdevicecontract.Geometry{}
	}
	return blockdevicecontract.Geometry{BlockCount: blockCount, BytesPerBlock: bytesPerBlock}
}

// ReadBlock returns a detached copy of the current pending-or-durable block.
func (device *HDD) ReadBlock(block int64) ([]byte, error) {
	if err := device.Geometry().CheckBlock(block); err != nil {
		return nil, err
	}
	sector, err := device.cache.ReadSector(block)
	if err != nil {
		return nil, err
	}
	return append([]byte(nil), sector[:]...), nil
}

// WriteBlock copies the exact block into the volatile cache overlay.
func (device *HDD) WriteBlock(block int64, data []byte) error {
	if err := device.Geometry().CheckWrite(block, data); err != nil {
		return err
	}
	next, err := device.cache.WriteSector(block, data)
	if err != nil {
		return err
	}
	device.cache = next
	return nil
}

// Flush commits the complete dirty set and acknowledges it.
func (device *HDD) Flush() error {
	_, err := device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
	return err
}

// FlushWithCut applies one exact whole-barrier cut. A successful cut stores
// its returned cache state even when acknowledgement was lost. A refusal
// leaves the receiver unchanged and returns a zero receipt.
func (device *HDD) FlushWithCut(cut hddwritebackcache.BarrierCut) (hddwritebackcache.BarrierReceipt, error) {
	if err := device.Geometry().Validate(); err != nil {
		return hddwritebackcache.BarrierReceipt{}, err
	}
	next, receipt, err := device.cache.FlushDirty(cut)
	if err != nil {
		return hddwritebackcache.BarrierReceipt{}, err
	}
	device.cache = next
	return receipt, nil
}

// Restart returns a separate HDD reconstructed from durable bytes, dropping
// only this instance's pending image.
func (device *HDD) Restart() (*HDD, error) {
	if err := device.Geometry().Validate(); err != nil {
		return nil, err
	}
	return &HDD{cache: device.cache.PowerLoss()}, nil
}
