// Package ftlevacuationplan creates deterministic copy plans without changing
// a page map or issuing any media operation.
package ftlevacuationplan

import (
	"errors"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

var (
	ErrInvalidBlockIndex    = errors.New("invalid evacuation block index")
	ErrInvalidBlockMask     = errors.New("invalid evacuation block mask")
	ErrExcludedBlock        = errors.New("evacuation target block is excluded")
	ErrNoStalePage          = errors.New("evacuation target has no stale page")
	ErrInsufficientCapacity = errors.New("insufficient free destinations for evacuation")
)

// Copy maps one live source page to one free destination page.
type Copy struct {
	Logical             int64
	SourcePhysical      int64
	DestinationPhysical int64
}

// Plan owns an ordered list of copies for one aligned physical block.
type Plan struct {
	Block         int64
	FirstPhysical int64
	PageCount     int64
	Copies        []Copy
}

// PlanEvacuation pairs live sources in the target block with the lowest free
// destinations outside both the target and excluded blocks. It never mutates
// the input table or exclusion mask.
func PlanEvacuation(table ftlpagemap.Table, pagesPerBlock, block int64, excludedBlocks []bool) (Plan, error) {
	snapshot, err := validatePlanInput(table, pagesPerBlock, block, excludedBlocks)
	if err != nil {
		return Plan{}, err
	}
	if len(excludedBlocks) != 0 && excludedBlocks[int(block)] {
		return Plan{}, ErrExcludedBlock
	}
	return enumerateCopies(snapshot, pagesPerBlock, block, excludedBlocks, true)
}

// PlanDrain pairs every readable live source in a draining block with the
// lowest free destination outside that source extent and all excluded blocks.
// The target block may itself be excluded because it is being retired.
func PlanDrain(table ftlpagemap.Table, pagesPerBlock, block int64, excludedBlocks []bool) (Plan, error) {
	snapshot, err := validatePlanInput(table, pagesPerBlock, block, excludedBlocks)
	if err != nil {
		return Plan{}, err
	}
	return enumerateCopies(snapshot, pagesPerBlock, block, excludedBlocks, false)
}

func validatePlanInput(table ftlpagemap.Table, pagesPerBlock, block int64, excludedBlocks []bool) (ftlpagemap.Snapshot, error) {
	snapshot := table.Snapshot()
	logicalCount := int64(len(snapshot.LogicalToPhysical))
	physicalCount := int64(len(snapshot.PhysicalStatus))
	if logicalCount < 1 || logicalCount > 64 || physicalCount < 2 || physicalCount > 128 || physicalCount <= logicalCount {
		return ftlpagemap.Snapshot{}, ftlpagemap.ErrInvalidGeometry
	}
	if pagesPerBlock < 1 || pagesPerBlock > physicalCount || physicalCount%pagesPerBlock != 0 {
		return ftlpagemap.Snapshot{}, ftlpagemap.ErrInvalidGeometry
	}
	blockCount := physicalCount / pagesPerBlock
	if block < 0 || block >= blockCount {
		return ftlpagemap.Snapshot{}, ErrInvalidBlockIndex
	}
	if len(excludedBlocks) != 0 && int64(len(excludedBlocks)) != blockCount {
		return ftlpagemap.Snapshot{}, ErrInvalidBlockMask
	}
	if snapshot.Reservation != nil {
		return ftlpagemap.Snapshot{}, ftlpagemap.ErrReservationPending
	}
	return snapshot, nil
}

func enumerateCopies(snapshot ftlpagemap.Snapshot, pagesPerBlock, block int64, excludedBlocks []bool, requireStale bool) (Plan, error) {
	first := block * pagesPerBlock
	plan := Plan{Block: block, FirstPhysical: first, PageCount: pagesPerBlock, Copies: make([]Copy, 0, pagesPerBlock)}
	logicalAtPhysical := make([]int64, len(snapshot.PhysicalStatus))
	for index := range logicalAtPhysical {
		logicalAtPhysical[index] = -1
	}
	for logical, physical := range snapshot.LogicalToPhysical {
		if physical >= 0 {
			logicalAtPhysical[int(physical)] = int64(logical)
		}
	}
	stale := false
	for physical := first; physical < first+pagesPerBlock; physical++ {
		switch snapshot.PhysicalStatus[int(physical)] {
		case ftlpagemap.Stale:
			stale = true
		case ftlpagemap.Live:
			plan.Copies = append(plan.Copies, Copy{Logical: logicalAtPhysical[int(physical)], SourcePhysical: physical})
		}
	}
	if requireStale && !stale {
		return Plan{}, ErrNoStalePage
	}

	destinationIndex := 0
	for physical, status := range snapshot.PhysicalStatus {
		if status != ftlpagemap.Free {
			continue
		}
		physicalIndex := int64(physical)
		candidateBlock := physicalIndex / pagesPerBlock
		if candidateBlock == block || (len(excludedBlocks) != 0 && excludedBlocks[int(candidateBlock)]) {
			continue
		}
		if destinationIndex < len(plan.Copies) {
			plan.Copies[destinationIndex].DestinationPhysical = physicalIndex
		}
		destinationIndex++
	}
	if destinationIndex < len(plan.Copies) {
		return Plan{}, ErrInsufficientCapacity
	}
	return plan, nil
}
