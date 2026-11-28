// Package hddtimedmedia applies pure HDD seek plans and byte-prefix cuts to
// sparse durable sector storage.
package hddtimedmedia

import (
	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
	hddseektiming "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-seek-timing"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
)

const sectorBytes = hddwritecrashcuts.BytesPerSector

// ReadResult reports an owned sector value and the timing applied to the read.
type ReadResult struct {
	Data   [sectorBytes]byte
	Timing hddseektiming.Result
}

// WriteResult reports timing and the acknowledged media-cut evidence.
type WriteResult struct {
	Timing               hddseektiming.Result
	PersistedPrefixBytes int
	Acknowledged         bool
}

// Snapshot is a detached sparse view of all nonzero durable sectors.
type Snapshot struct {
	Geometry hddgeometry.Geometry
	State    hddseektiming.State
	Sectors  map[int64][sectorBytes]byte
}

// Medium holds only nonzero sectors and the current observable head/clock.
// Callers serialize operations and keep write inputs stable for the call.
type Medium struct {
	geometry hddgeometry.Geometry
	state    hddseektiming.State
	sectors  map[int64][sectorBytes]byte
}

// New validates geometry and creates an empty medium at cylinder and tick 0.
func New(geometry hddgeometry.Geometry) (*Medium, error) {
	if _, err := geometry.BlockGeometry(); err != nil {
		return nil, err
	}
	return &Medium{geometry: geometry, sectors: make(map[int64][sectorBytes]byte)}, nil
}

// ReadSector plans and applies one read, returning zero bytes for an absent
// sparse sector. State advances only after a complete successful plan.
func (medium *Medium) ReadSector(block int64) (ReadResult, error) {
	if medium == nil {
		return ReadResult{}, blockdevicecontract.ErrInvalidGeometry
	}
	timing, err := hddseektiming.Plan(medium.geometry, medium.state, block)
	if err != nil {
		return ReadResult{}, err
	}
	result := ReadResult{Data: medium.sectors[block], Timing: timing}
	medium.state = timing.End
	return result, nil
}

// WriteSector validates geometry/address/data, describes the cut, and plans
// service before changing state or durable bytes.
func (medium *Medium) WriteSector(block int64, data []byte, cut hddwritecrashcuts.Cut) (WriteResult, error) {
	if medium == nil {
		return WriteResult{}, blockdevicecontract.ErrInvalidGeometry
	}
	geometry, err := medium.geometry.BlockGeometry()
	if err != nil {
		return WriteResult{}, err
	}
	if err := geometry.CheckWrite(block, data); err != nil {
		return WriteResult{}, err
	}
	persisted, acknowledged, err := hddwritecrashcuts.Describe(cut)
	if err != nil {
		return WriteResult{}, err
	}
	timing, err := planForCut(medium.geometry, medium.state, block, persisted)
	if err != nil {
		return WriteResult{}, err
	}
	var sector [sectorBytes]byte
	sector = medium.sectors[block]
	copy(sector[:persisted], data[:persisted])
	if sector == ([sectorBytes]byte{}) {
		delete(medium.sectors, block)
	} else {
		medium.sectors[block] = sector
	}
	medium.state = timing.End
	return WriteResult{Timing: timing, PersistedPrefixBytes: persisted, Acknowledged: acknowledged}, nil
}

// Snapshot returns a detached copy of the sparse nonzero durable sectors.
func (medium *Medium) Snapshot() (Snapshot, error) {
	if medium == nil {
		return Snapshot{}, blockdevicecontract.ErrInvalidGeometry
	}
	if _, err := medium.geometry.BlockGeometry(); err != nil {
		return Snapshot{}, err
	}
	sectors := make(map[int64][sectorBytes]byte, len(medium.sectors))
	for block, sector := range medium.sectors {
		sectors[block] = sector
	}
	return Snapshot{Geometry: medium.geometry, State: medium.state, Sectors: sectors}, nil
}

// Restart returns an independent medium with the same durable bytes and fresh
// head/clock state.
func (medium *Medium) Restart() (*Medium, error) {
	if medium == nil {
		return nil, blockdevicecontract.ErrInvalidGeometry
	}
	next, err := New(medium.geometry)
	if err != nil {
		return nil, err
	}
	for block, sector := range medium.sectors {
		next.sectors[block] = sector
	}
	return next, nil
}

// planForCut requires admission of the full service. A before-media cut then
// omits the unconsumed transfer tick from the reported state and totals.
func planForCut(geometry hddgeometry.Geometry, current hddseektiming.State, block int64, persisted int) (hddseektiming.Result, error) {
	plan, err := hddseektiming.Plan(geometry, current, block)
	if err != nil {
		return hddseektiming.Result{}, err
	}
	if persisted == 0 {
		plan.End.Tick--
		plan.TotalTicks--
		plan.TransferTicks = 0
	}
	return plan, nil
}
