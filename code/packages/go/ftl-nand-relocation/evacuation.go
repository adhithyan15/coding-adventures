package ftlnandrelocation

import (
	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
)

// EvacuationProgress reports current map ownership in the active target.
// RemainingLivePages is derived from the current map rather than a cached
// command count.
type EvacuationProgress struct {
	Block              int64
	RemainingLivePages int64
	IncompleteErase    bool
}

// EvacuationCopyReceipt records the freshly planned move and its media result.
type EvacuationCopyReceipt struct {
	Move    ftlevacuationplan.Copy
	Receipt Receipt
}

// PlanRetirementDrain returns a read-only, ordered copy plan for a readable
// block. Caller and device-owned block fences both constrain destinations;
// the source may itself be caller-excluded, but not active or mid-erase.
func (device Device) PlanRetirementDrain(block int64, excludedBlocks []bool) (ftlevacuationplan.Plan, error) {
	if !device.configured() {
		return ftlevacuationplan.Plan{}, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return ftlevacuationplan.Plan{}, ErrEraseDisabled
	}
	if block < 0 || block >= int64(len(device.blocks)) {
		return ftlevacuationplan.Plan{}, ErrInvalidBlockIndex
	}
	if len(excludedBlocks) != 0 && len(excludedBlocks) != len(device.blocks) {
		return ftlevacuationplan.Plan{}, ftlevacuationplan.ErrInvalidBlockMask
	}
	if device.blocks[int(block)].IncompleteErase {
		return ftlevacuationplan.Plan{}, ErrEvacuationBlockIncomplete
	}
	if device.evacuationActive && block == device.evacuationBlock {
		return ftlevacuationplan.Plan{}, ErrEvacuationActive
	}

	// Start from the device's detached quarantine mask, then add caller policy
	// and the active target. The source bit is allowed by PlanDrain.
	destinationFences := device.incompleteBlockMask()
	for blockIndex, excluded := range excludedBlocks {
		if excluded {
			destinationFences[blockIndex] = true
		}
	}
	if device.evacuationActive {
		destinationFences[int(device.evacuationBlock)] = true
	}
	return ftlevacuationplan.PlanDrain(device.table, device.pagesPerBlock, block, destinationFences)
}

// BeginEvacuation validates and fences one stale-containing block after the
// planner confirms enough current destinations for its live pages.
func (device Device) BeginEvacuation(block int64) (Device, error) {
	return device.BeginEvacuationExcluding(block, nil)
}

// BeginEvacuationExcluding admits one stale-containing block when the planner
// can move all current live pages outside the victim, quarantined extents, and
// caller-excluded destinations. The caller's mask is borrowed only for this
// call and is never retained or changed. The victim bit is ignored because no
// copy destination may be inside the victim block.
func (device Device) BeginEvacuationExcluding(block int64, destinationExcluded []bool) (Device, error) {
	if !device.configured() {
		return device, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return device, ErrEraseDisabled
	}
	if block < 0 || block >= int64(len(device.blocks)) {
		return device, ErrInvalidBlockIndex
	}
	if len(destinationExcluded) != 0 && len(destinationExcluded) != len(device.blocks) {
		return device, ftlevacuationplan.ErrInvalidBlockMask
	}
	if device.evacuationActive {
		return device, ErrEvacuationActive
	}
	if device.blocks[int(block)].IncompleteErase {
		return device, ErrEvacuationBlockIncomplete
	}
	if device.blocks[int(block)].CompletedErases >= device.maxCompletedErases {
		return device, nandblockerasecuts.ErrEraseLimit
	}
	destinationFences := device.evacuationDestinationMask(block, destinationExcluded)
	if _, err := ftlevacuationplan.PlanEvacuation(device.table, device.pagesPerBlock, block, destinationFences); err != nil {
		return device, err
	}
	next := device
	next.evacuationActive = true
	next.evacuationBlock = block
	return next, nil
}

// EvacuationProgress returns current live ownership and active erase state.
func (device Device) EvacuationProgress() (EvacuationProgress, bool) {
	if !device.configured() || !device.evacuationActive {
		return EvacuationProgress{}, false
	}
	return device.evacuationProgress(), true
}

func (device Device) evacuationProgress() EvacuationProgress {
	first := device.evacuationBlock * device.pagesPerBlock
	last := first + device.pagesPerBlock
	remaining := int64(0)
	statuses := device.table.Snapshot().PhysicalStatus
	for physical := first; physical < last; physical++ {
		if statuses[int(physical)] == ftlpagemap.Live {
			remaining++
		}
	}
	return EvacuationProgress{
		Block: device.evacuationBlock, RemainingLivePages: remaining,
		IncompleteErase: device.blocks[int(device.evacuationBlock)].IncompleteErase,
	}
}

// CopyNextEvacuationPage replans current ownership and applies the first move.
// The selected move is returned for all accepted media cuts, including cuts
// that did not publish it.
func (device Device) CopyNextEvacuationPage(cut Cut) (Device, EvacuationCopyReceipt, error) {
	return device.CopyNextEvacuationPageExcluding(cut, nil)
}

// CopyNextEvacuationPageExcluding replans the active victim's complete set of
// live pages against current device quarantine and caller destination fences,
// then applies the first assignment. The mask is borrowed for this call. Its
// victim bit is ignored because every destination is outside that extent.
func (device Device) CopyNextEvacuationPageExcluding(cut Cut, destinationExcluded []bool) (Device, EvacuationCopyReceipt, error) {
	if !device.configured() {
		return device, EvacuationCopyReceipt{}, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return device, EvacuationCopyReceipt{}, ErrEraseDisabled
	}
	if !device.evacuationActive {
		return device, EvacuationCopyReceipt{}, ErrNoActiveEvacuation
	}
	if len(destinationExcluded) != 0 && len(destinationExcluded) != len(device.blocks) {
		return device, EvacuationCopyReceipt{}, ftlevacuationplan.ErrInvalidBlockMask
	}
	progress := device.evacuationProgress()
	if progress.RemainingLivePages == 0 {
		return device, EvacuationCopyReceipt{}, ErrEvacuationNoLivePages
	}
	destinationFences := device.evacuationDestinationMask(device.evacuationBlock, destinationExcluded)
	plan, err := ftlevacuationplan.PlanEvacuation(
		device.table, device.pagesPerBlock, device.evacuationBlock, destinationFences,
	)
	if err != nil {
		return device, EvacuationCopyReceipt{}, err
	}
	move := plan.Copies[0]
	next, receipt, err := device.copyLivePage(device.evacuationBlock, move, cut, true)
	if err != nil {
		return device, EvacuationCopyReceipt{}, err
	}
	return next, EvacuationCopyReceipt{Move: move, Receipt: receipt}, nil
}

// FinishEvacuation erases the active target once no logical page remains live.
// A completed media result clears the fence even if acknowledgement is lost.
func (device Device) FinishEvacuation(cut EraseCut) (Device, EraseReceipt, error) {
	if !device.configured() {
		return device, EraseReceipt{}, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return device, EraseReceipt{}, ErrEraseDisabled
	}
	if !device.evacuationActive {
		return device, EraseReceipt{}, ErrNoActiveEvacuation
	}
	progress := device.evacuationProgress()
	if progress.RemainingLivePages != 0 {
		return device, EraseReceipt{}, ErrEvacuationStillLive
	}
	next, receipt, err := device.eraseBlock(device.evacuationBlock, cut, true)
	if err != nil {
		return device, EraseReceipt{}, err
	}
	if receipt.Released {
		next.evacuationActive = false
		next.evacuationBlock = 0
	}
	return next, receipt, nil
}

// CancelEvacuation drops an intact active fence while preserving all media.
func (device Device) CancelEvacuation() (Device, error) {
	if !device.configured() {
		return device, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return device, ErrEraseDisabled
	}
	if !device.evacuationActive {
		return device, ErrNoActiveEvacuation
	}
	if device.blocks[int(device.evacuationBlock)].IncompleteErase {
		return device, ErrEvacuationBlockIncomplete
	}
	next := device
	next.evacuationActive = false
	next.evacuationBlock = 0
	return next, nil
}

func (device Device) incompleteBlockMask() []bool {
	mask := make([]bool, len(device.blocks))
	for index, block := range device.blocks {
		mask[index] = block.IncompleteErase
	}
	return mask
}

// evacuationDestinationMask combines caller policy with every incomplete
// extent and clears the source block because it cannot receive its own copy.
func (device Device) evacuationDestinationMask(block int64, excluded []bool) []bool {
	mask := device.incompleteBlockMask()
	for index, blocked := range excluded {
		if blocked {
			mask[index] = true
		}
	}
	mask[int(block)] = false
	return mask
}
