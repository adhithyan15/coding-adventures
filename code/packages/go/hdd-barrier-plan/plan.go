// Package hddbarrierplan prepares ordered HDD sector-cut plans without
// applying media effects.
package hddbarrierplan

import (
	"errors"

	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
	hddseektiming "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-seek-timing"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
)

var ErrDirtyOrder = errors.New("dirty blocks must be strictly ascending")

// Step selects one relative sector write and its lower media cut.
type Step struct {
	Block int64
	Cut   hddwritecrashcuts.Cut
}

// Plan is a detached ordered write sequence and its barrier receipt.
type Plan struct {
	Steps   []Step
	Receipt hddwritebackcache.BarrierReceipt
}

// Prepare validates geometry, the captured dirty set, the barrier descriptor,
// and every full service timing before returning an effect-free plan.
func Prepare(geometry hddgeometry.Geometry, current hddseektiming.State, dirty []int64, cut hddwritebackcache.BarrierCut) (Plan, error) {
	blocks, err := geometry.BlockGeometry()
	if err != nil {
		return Plan{}, err
	}
	for index, block := range dirty {
		if err := blocks.CheckBlock(block); err != nil {
			return Plan{}, err
		}
		if index > 0 && block <= dirty[index-1] {
			return Plan{}, ErrDirtyOrder
		}
	}

	terminalPrefix, err := validateBarrierCut(dirty, cut)
	if err != nil {
		return Plan{}, err
	}
	stepCount := len(dirty)
	receipt := hddwritebackcache.BarrierReceipt{}
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector:
		return Plan{}, nil
	case hddwritebackcache.AtSector:
		stepCount = int(cut.SectorIndex) + 1
		receipt.PriorCompletedSectors = int(cut.SectorIndex)
		receipt.TerminalPrefixBytes = terminalPrefix
	case hddwritebackcache.AfterAllMedia:
		receipt.PriorCompletedSectors = len(dirty)
	case hddwritebackcache.BarrierComplete:
		receipt.PriorCompletedSectors = len(dirty)
		receipt.Acknowledged = true
	}
	if stepCount == 0 {
		return Plan{Receipt: receipt}, nil
	}

	steps := make([]Step, stepCount)
	state := current
	for ordinal := 0; ordinal < stepCount; ordinal++ {
		block := dirty[ordinal]
		mediaCut := hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}
		if cut.Kind == hddwritebackcache.AtSector && ordinal == int(cut.SectorIndex) {
			mediaCut = detachCut(cut.MediaCut)
		}
		steps[ordinal] = Step{Block: block, Cut: mediaCut}
		service, err := hddseektiming.Plan(geometry, state, block)
		if err != nil {
			return Plan{}, err
		}
		state = service.End
	}
	return Plan{Steps: steps, Receipt: receipt}, nil
}

func validateBarrierCut(dirty []int64, cut hddwritebackcache.BarrierCut) (int, error) {
	zeroCut := hddwritecrashcuts.Cut{}
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector, hddwritebackcache.AfterAllMedia, hddwritebackcache.BarrierComplete:
		if cut.SectorIndex != 0 || cut.MediaCut != zeroCut {
			return 0, hddwritebackcache.ErrInvalidBarrierCut
		}
		return 0, nil
	case hddwritebackcache.AtSector:
		if cut.SectorIndex < 0 || cut.SectorIndex >= int64(len(dirty)) {
			return 0, hddwritebackcache.ErrInvalidBarrierCut
		}
		if cut.MediaCut.Kind == hddwritecrashcuts.Acknowledged {
			return 0, hddwritebackcache.ErrInvalidBarrierCut
		}
		prefix, _, err := hddwritecrashcuts.Describe(cut.MediaCut)
		if err != nil {
			return 0, err
		}
		return prefix, nil
	default:
		return 0, hddwritebackcache.ErrInvalidBarrierCut
	}
}

func detachCut(cut hddwritecrashcuts.Cut) hddwritecrashcuts.Cut {
	if cut.WrittenPrefix == nil {
		return cut
	}
	prefix := *cut.WrittenPrefix
	cut.WrittenPrefix = &prefix
	return cut
}
