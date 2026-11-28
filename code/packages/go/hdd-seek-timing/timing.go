// Package hddseektiming plans bounded seek, rotation, and transfer costs.
package hddseektiming

import (
	"errors"

	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
)

const (
	seekTicksPerCylinder uint64 = 3
	transferTicks        uint64 = 1
	maxPortableTick      uint64 = 9_007_199_254_740_991
)

// State is the current head cylinder and absolute portable clock tick.
type State struct {
	HeadCylinder int64
	Tick         uint64
}

// Result is the planned target and exact component timing.
type Result struct {
	Target        hddgeometry.Address
	End           State
	SeekTicks     uint64
	RotationTicks uint64
	TransferTicks uint64
	TotalTicks    uint64
}

var (
	ErrHeadCylinder  = errors.New("head cylinder is outside HDD geometry")
	ErrClockOverflow = errors.New("HDD clock overflow")
)

// Plan locates block, validates the current head and clock, then computes the
// bounded seek/rotation/transfer transition without observing any media.
func Plan(geometry hddgeometry.Geometry, current State, block int64) (Result, error) {
	target, err := geometry.Locate(block)
	if err != nil {
		return Result{}, err
	}
	if current.HeadCylinder < 0 || current.HeadCylinder >= geometry.Cylinders {
		return Result{}, ErrHeadCylinder
	}
	if current.Tick > maxPortableTick {
		return Result{}, ErrClockOverflow
	}

	distance := target.Cylinder - current.HeadCylinder
	if distance < 0 {
		distance = -distance
	}
	remaining := maxPortableTick - current.Tick
	if uint64(distance) > remaining/seekTicksPerCylinder {
		return Result{}, ErrClockOverflow
	}
	seek := uint64(distance) * seekTicksPerCylinder
	period := uint64(geometry.SectorsPerTrack)
	phase := addModulo(current.Tick%period, seek%period, period)
	rotation := forwardWait(phase, uint64(target.Sector), period)
	remainingAfterSeek := remaining - seek
	if rotation >= remainingAfterSeek {
		return Result{}, ErrClockOverflow
	}
	total := seek + rotation + transferTicks
	return Result{
		Target:    target,
		End:       State{HeadCylinder: target.Cylinder, Tick: current.Tick + total},
		SeekTicks: seek, RotationTicks: rotation,
		TransferTicks: transferTicks, TotalTicks: total,
	}, nil
}

func addModulo(base, advance, period uint64) uint64 {
	untilWrap := period - base
	if advance >= untilWrap {
		return advance - untilWrap
	}
	return base + advance
}

func forwardWait(phase, target, period uint64) uint64 {
	if target >= phase {
		return target - phase
	}
	return period - (phase - target)
}
