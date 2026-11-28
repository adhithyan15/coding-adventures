package hddsimulator

import (
	"errors"
)

const (
	surfaceCount       = 2
	tracksPerSurface   = 4
	sectorsPerTrack    = 4
	sectorsPerCylinder = surfaceCount * sectorsPerTrack
	sectorCount        = surfaceCount * tracksPerSurface * sectorsPerTrack
	bytesPerSector     = 8
	seekPerTrack       = 3
	transferPerSector  = 1
	maxTick            = uint64(1<<53 - 1)
)

var (
	ErrInvalidArgument   = errors.New("invalid API argument")
	ErrInvalidLBA        = errors.New("LBA out of range")
	ErrInvalidSectorData = errors.New("sector data must contain exactly 8 bytes")
	ErrClockOverflow     = errors.New("clock overflow")
)

// Device is one synchronous, fixed-geometry disk. Callers must serialize
// operations on one device. Its zero value is a fresh disk with all sectors
// zero-filled.
type Device struct {
	tick      uint64
	headTrack int
	sectors   [sectorCount][bytesPerSector]byte
}

// IOTiming is the exact integer cost of one accepted sector operation.
type IOTiming struct {
	StartTick     uint64
	SeekTicks     uint64
	RotationTicks uint64
	TransferTicks uint64
	TotalTicks    uint64
	EndTick       uint64
}

// SectorResult contains independent read bytes and their timing.
type SectorResult struct {
	Data   []byte
	Timing IOTiming
}

// Snapshot is a detached copy of the entire observable device state.
type Snapshot struct {
	Tick      uint64
	HeadTrack int
	Sectors   [sectorCount][bytesPerSector]byte
}

// New constructs a fresh, zero-filled fixed-geometry disk.
func New() *Device {
	return &Device{}
}

// ReadSector returns a new copy of one sector and advances modeled time.
func (d *Device) ReadSector(lba int) (SectorResult, error) {
	if d == nil {
		return SectorResult{}, ErrInvalidArgument
	}
	if lba < 0 || lba >= sectorCount {
		return SectorResult{}, ErrInvalidLBA
	}
	timing, err := d.timingFor(lba)
	if err != nil {
		return SectorResult{}, err
	}
	data := make([]byte, bytesPerSector)
	copy(data, d.sectors[lba][:])
	d.commit(lba, timing)
	return SectorResult{Data: data, Timing: timing}, nil
}

// WriteSector copies exactly one sector into the medium and advances time.
func (d *Device) WriteSector(lba int, data []byte) (IOTiming, error) {
	if d == nil {
		return IOTiming{}, ErrInvalidArgument
	}
	if lba < 0 || lba >= sectorCount {
		return IOTiming{}, ErrInvalidLBA
	}
	if data == nil || len(data) != bytesPerSector {
		return IOTiming{}, ErrInvalidSectorData
	}
	timing, err := d.timingFor(lba)
	if err != nil {
		return IOTiming{}, err
	}
	var copied [bytesPerSector]byte
	copy(copied[:], data)
	d.sectors[lba] = copied
	d.commit(lba, timing)
	return timing, nil
}

// Snapshot returns a detached copy of clock, head, and all sector bytes.
func (d *Device) Snapshot() (Snapshot, error) {
	if d == nil {
		return Snapshot{}, ErrInvalidArgument
	}
	return Snapshot{Tick: d.tick, HeadTrack: d.headTrack, Sectors: d.sectors}, nil
}

func (d *Device) timingFor(lba int) (IOTiming, error) {
	if d.tick > maxTick {
		return IOTiming{}, ErrClockOverflow
	}
	targetTrack := lba / sectorsPerCylinder
	targetSector := lba % sectorsPerTrack
	distance := targetTrack - d.headTrack
	if distance < 0 {
		distance = -distance
	}
	seek := uint64(distance * seekPerTrack)
	phaseAfterSeek := ((d.tick % sectorsPerTrack) + (seek % sectorsPerTrack)) % sectorsPerTrack
	rotation := (uint64(targetSector) + sectorsPerTrack - phaseAfterSeek) % sectorsPerTrack
	total := seek + rotation + transferPerSector
	if total > maxTick-d.tick {
		return IOTiming{}, ErrClockOverflow
	}
	return IOTiming{
		StartTick: d.tick, SeekTicks: seek, RotationTicks: rotation,
		TransferTicks: transferPerSector, TotalTicks: total, EndTick: d.tick + total,
	}, nil
}

func (d *Device) commit(lba int, timing IOTiming) {
	d.headTrack = lba / sectorsPerCylinder
	d.tick = timing.EndTick
}
