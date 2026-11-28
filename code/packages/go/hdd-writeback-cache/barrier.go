package hddwritebackcache

import (
	"errors"

	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
)

// BarrierCutKind identifies where a whole-set flush is interrupted.
type BarrierCutKind uint8

const (
	BeforeFirstSector BarrierCutKind = iota
	AtSector
	AfterAllMedia
	BarrierComplete
)

// BarrierCut selects a barrier-level cut. SectorIndex is the ordinal in the
// ascending dirty-LBA order and MediaCut applies only to AtSector.
type BarrierCut struct {
	Kind        BarrierCutKind
	SectorIndex int64
	MediaCut    hddwritecrashcuts.Cut
}

// BarrierReceipt distinguishes completed sectors from terminal partial media
// and from acknowledgement of the whole barrier.
type BarrierReceipt struct {
	PriorCompletedSectors int
	TerminalPrefixBytes   int
	Acknowledged          bool
}

// ErrInvalidBarrierCut identifies a malformed barrier-level cut descriptor.
var ErrInvalidBarrierCut = errors.New("invalid_barrier_cut")

// FlushDirty visits the dirty sectors captured at command entry in ascending
// LBA order. A terminal lower cut or lost barrier acknowledgement discards all
// remaining pending writes through the same power-loss transition as FlushSector.
func (state State) FlushDirty(cut BarrierCut) (State, BarrierReceipt, error) {
	dirtyLBAs := make([]int64, 0, SectorCount)
	for lba, dirty := range state.dirty {
		if dirty {
			dirtyLBAs = append(dirtyLBAs, int64(lba))
		}
	}
	if err := validateBarrierCut(dirtyLBAs, cut); err != nil {
		return state, BarrierReceipt{}, err
	}
	if cut.Kind == BeforeFirstSector {
		return state.PowerLoss(), BarrierReceipt{}, nil
	}

	next := state
	receipt := BarrierReceipt{}
	for ordinal, lba := range dirtyLBAs {
		mediaCut := hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}
		if cut.Kind == AtSector && int64(ordinal) == cut.SectorIndex {
			mediaCut = cut.MediaCut
		}
		flushed, result, err := next.FlushSector(lba, mediaCut)
		if err != nil {
			return state, BarrierReceipt{}, err
		}
		next = flushed
		if cut.Kind == AtSector && int64(ordinal) == cut.SectorIndex {
			receipt.TerminalPrefixBytes = result.PersistedPrefixBytes
			break
		}
		receipt.PriorCompletedSectors++
	}
	receipt.Acknowledged = cut.Kind == BarrierComplete
	return next, receipt, nil
}

func validateBarrierCut(dirtyLBAs []int64, cut BarrierCut) error {
	switch cut.Kind {
	case BeforeFirstSector, AfterAllMedia, BarrierComplete:
		if cut.SectorIndex != 0 || cut.MediaCut != (hddwritecrashcuts.Cut{}) {
			return ErrInvalidBarrierCut
		}
		return nil
	case AtSector:
		if cut.SectorIndex < 0 || cut.SectorIndex >= int64(len(dirtyLBAs)) {
			return ErrInvalidBarrierCut
		}
		if cut.MediaCut.Kind == hddwritecrashcuts.Acknowledged {
			return ErrInvalidBarrierCut
		}
		return nil
	default:
		return ErrInvalidBarrierCut
	}
}
