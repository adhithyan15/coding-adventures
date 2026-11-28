// Package hddsparseblockdevice combines a sparse write-back overlay with an
// effect-free whole-barrier planner and a timed durable HDD medium.
package hddsparseblockdevice

import (
	"sort"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddbarrierplan "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-barrier-plan"
	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
	hddtimedmedia "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-timed-media"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
)

const bytesPerBlock = hddwritebackcache.BytesPerSector

// Device exposes fixed HDD blocks over a volatile sparse write-back overlay.
// Callers serialize operations and keep write bytes stable for the call.
type Device struct {
	hddGeometry hddgeometry.Geometry
	geometry    blockdevicecontract.Geometry
	medium      *hddtimedmedia.Medium
	pending     map[int64][bytesPerBlock]byte
}

// New validates and owns a fresh timed medium and empty overlay.
func New(geometry hddgeometry.Geometry) (*Device, error) {
	logicalGeometry, err := geometry.BlockGeometry()
	if err != nil {
		return nil, err
	}
	medium, err := hddtimedmedia.New(geometry)
	if err != nil {
		return nil, err
	}
	return &Device{
		hddGeometry: geometry,
		geometry:    logicalGeometry,
		medium:      medium,
		pending:     make(map[int64][bytesPerBlock]byte),
	}, nil
}

// Geometry returns the fixed logical geometry, or zero for nil/unconfigured
// receivers.
func (device *Device) Geometry() blockdevicecontract.Geometry {
	if device == nil || device.medium == nil || device.geometry.Validate() != nil {
		return blockdevicecontract.Geometry{}
	}
	return device.geometry
}

// ReadBlock returns a detached visible block. A pending hit does not access or
// advance the timed medium; a miss reads its durable sparse sector.
func (device *Device) ReadBlock(block int64) ([]byte, error) {
	if err := device.validate(); err != nil {
		return nil, err
	}
	if err := device.geometry.CheckBlock(block); err != nil {
		return nil, err
	}
	if pending, ok := device.pending[block]; ok {
		return copyBlock(pending), nil
	}
	read, err := device.medium.ReadSector(block)
	if err != nil {
		return nil, err
	}
	return copyBlock(read.Data), nil
}

// WriteBlock copies one complete block into the volatile overlay.
func (device *Device) WriteBlock(block int64, data []byte) error {
	if err := device.validate(); err != nil {
		return err
	}
	if err := device.geometry.CheckWrite(block, data); err != nil {
		return err
	}
	var pending [bytesPerBlock]byte
	copy(pending[:], data)
	device.pending[block] = pending
	return nil
}

// Flush drains every pending block as an acknowledged barrier.
func (device *Device) Flush() error {
	_, err := device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
	return err
}

// FlushWithCut preflights the entire captured dirty set, then applies only the
// planner's ordered media writes and returns its exact barrier receipt.
func (device *Device) FlushWithCut(cut hddwritebackcache.BarrierCut) (hddwritebackcache.BarrierReceipt, error) {
	if err := device.validate(); err != nil {
		return hddwritebackcache.BarrierReceipt{}, err
	}
	dirty := make([]int64, 0, len(device.pending))
	for block := range device.pending {
		dirty = append(dirty, block)
	}
	sort.Slice(dirty, func(left, right int) bool { return dirty[left] < dirty[right] })
	position, err := device.medium.Position()
	if err != nil {
		return hddwritebackcache.BarrierReceipt{}, err
	}
	plan, err := hddbarrierplan.Prepare(device.hddGeometry, position, dirty, cut)
	if err != nil {
		return hddwritebackcache.BarrierReceipt{}, err
	}
	for _, step := range plan.Steps {
		data := device.pending[step.Block]
		if _, err := device.medium.WriteSector(step.Block, data[:], step.Cut); err != nil {
			return hddwritebackcache.BarrierReceipt{}, err
		}
	}
	clear(device.pending)
	return plan.Receipt, nil
}

// Restart drops volatile writes and preserves an independent restarted medium.
func (device *Device) Restart() (*Device, error) {
	if err := device.validate(); err != nil {
		return nil, err
	}
	medium, err := device.medium.Restart()
	if err != nil {
		return nil, err
	}
	return &Device{
		hddGeometry: device.hddGeometry,
		geometry:    device.geometry,
		medium:      medium,
		pending:     make(map[int64][bytesPerBlock]byte),
	}, nil
}

func (device *Device) validate() error {
	if device == nil || device.medium == nil {
		return blockdevicecontract.ErrInvalidGeometry
	}
	return device.geometry.Validate()
}

func copyBlock(value [bytesPerBlock]byte) []byte {
	result := make([]byte, bytesPerBlock)
	copy(result, value[:])
	return result
}
