package ftlerasevictim

import (
	"errors"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

const (
	minPhysicalPages = 2
	maxPhysicalPages = 128
)

var (
	ErrInvalidGeometry    = ftlpagemap.ErrInvalidGeometry
	ErrInvalidStatus      = errors.New("invalid physical page status")
	ErrInvalidBlockMask   = errors.New("invalid physical block mask")
	ErrReservationPending = ftlpagemap.ErrReservationPending
)

// CopyAwareVictim describes a block selected by stale benefit relative to the
// live pages that must be copied out before reclamation.
type CopyAwareVictim struct {
	Block            int64
	FirstPhysical    int64
	PageCount        int64
	StalePages       int64
	LivePages        int64
	FreeDestinations int64
}

// Victim identifies an aligned whole-block extent and its reclaimable benefit.
type Victim struct {
	Block         int64
	FirstPhysical int64
	PageCount     int64
	StalePages    int64
}

// Select chooses the block with the most stale slots among blocks that contain
// stale data and no live or reserved slot. Ties stay with the lowest index.
func Select(status []ftlpagemap.PhysicalStatus, pagesPerBlock int64) (Victim, bool, error) {
	pageCount := int64(len(status))
	if pageCount < minPhysicalPages || pageCount > maxPhysicalPages ||
		pagesPerBlock < 1 || pagesPerBlock > pageCount || pageCount%pagesPerBlock != 0 {
		return Victim{}, false, ErrInvalidGeometry
	}
	for _, slot := range status {
		if slot > ftlpagemap.Stale {
			return Victim{}, false, ErrInvalidStatus
		}
	}

	blockCount := pageCount / pagesPerBlock
	var best Victim
	found := false
	for block := int64(0); block < blockCount; block++ {
		first := block * pagesPerBlock
		staleCount := int64(0)
		blocked := false
		for physical := first; physical < first+pagesPerBlock; physical++ {
			switch status[int(physical)] {
			case ftlpagemap.Stale:
				staleCount++
			case ftlpagemap.Live:
				blocked = true
			case ftlpagemap.Reserved:
				blocked = true
			}
		}
		if blocked || staleCount == 0 {
			continue
		}
		if !found || staleCount > best.StalePages {
			best = Victim{
				Block:         block,
				FirstPhysical: first,
				PageCount:     pagesPerBlock,
				StalePages:    staleCount,
			}
			found = true
		}
	}
	return best, found, nil
}

// SelectCopyAware chooses the eligible block maximizing StalePages/(LivePages+1).
// The caller's snapshot and exclusion mask are only read and must remain stable
// for the duration of the call.
func SelectCopyAware(status []ftlpagemap.PhysicalStatus, pagesPerBlock int64, excludedBlocks []bool) (CopyAwareVictim, bool, error) {
	return SelectCopyAwareWithFences(status, pagesPerBlock, excludedBlocks, excludedBlocks)
}

// SelectCopyAwareWithFences chooses a reclaim candidate using separate masks
// for victim eligibility and external copy-destination capacity. Either mask
// may be empty or have one entry per block. Inputs remain caller-owned and are
// read only for the duration of the call.
func SelectCopyAwareWithFences(
	status []ftlpagemap.PhysicalStatus,
	pagesPerBlock int64,
	victimExcluded, destinationExcluded []bool,
) (CopyAwareVictim, bool, error) {
	pageCount := int64(len(status))
	if pageCount < minPhysicalPages || pageCount > maxPhysicalPages ||
		pagesPerBlock < 1 || pagesPerBlock > pageCount || pageCount%pagesPerBlock != 0 {
		return CopyAwareVictim{}, false, ErrInvalidGeometry
	}
	blockCount := pageCount / pagesPerBlock
	if len(victimExcluded) != 0 && int64(len(victimExcluded)) != blockCount {
		return CopyAwareVictim{}, false, ErrInvalidBlockMask
	}
	if len(destinationExcluded) != 0 && int64(len(destinationExcluded)) != blockCount {
		return CopyAwareVictim{}, false, ErrInvalidBlockMask
	}
	hasReservation := false
	for _, slot := range status {
		if slot > ftlpagemap.Stale {
			return CopyAwareVictim{}, false, ErrInvalidStatus
		}
		if slot == ftlpagemap.Reserved {
			hasReservation = true
		}
	}
	if hasReservation {
		return CopyAwareVictim{}, false, ErrReservationPending
	}

	var allowedFree int64
	for physical, slot := range status {
		block := int64(physical) / pagesPerBlock
		if slot == ftlpagemap.Free && (len(destinationExcluded) == 0 || !destinationExcluded[int(block)]) {
			allowedFree++
		}
	}

	var best CopyAwareVictim
	var bestStale, bestDenominator int64
	found := false
	for block := int64(0); block < blockCount; block++ {
		if len(victimExcluded) != 0 && victimExcluded[int(block)] {
			continue
		}
		first := block * pagesPerBlock
		var stale, live, localFree int64
		for physical := first; physical < first+pagesPerBlock; physical++ {
			switch status[int(physical)] {
			case ftlpagemap.Stale:
				stale++
			case ftlpagemap.Live:
				live++
			case ftlpagemap.Free:
				if len(destinationExcluded) == 0 || !destinationExcluded[int(block)] {
					localFree++
				}
			}
		}
		if stale == 0 {
			continue
		}
		freeOutside := allowedFree - localFree
		if freeOutside < live {
			continue
		}
		denominator := live + 1
		better := !found || stale*bestDenominator > bestStale*denominator
		if found && stale*bestDenominator == bestStale*denominator {
			better = stale > bestStale || stale == bestStale && block < best.Block
		}
		if better {
			best = CopyAwareVictim{
				Block: block, FirstPhysical: first, PageCount: pagesPerBlock,
				StalePages: stale, LivePages: live, FreeDestinations: freeOutside,
			}
			bestStale, bestDenominator = stale, denominator
			found = true
		}
	}
	return best, found, nil
}
