package ftlnandrelocation

import (
	"errors"

	ftleraseroleevidence "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-role-evidence"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
)

var (
	ErrEraseIntentBlockIncomplete = errors.New("block has incomplete erase progress")
	ErrInvalidEraseIntentSequence = errors.New("erase intent sequence must be nonzero")
)

// CaptureEraseIntent snapshots pre-erase stale roles for a durable intent
// record. The caller supplies its operation fence and persists this return
// value before issuing the first media erase command.
func (device Device) CaptureEraseIntent(block int64, sequence uint64) (ftleraseroleevidence.RetainedIntent, error) {
	snapshot := device.table.Snapshot()
	first, end, err := device.validatedBlockRange(snapshot, block)
	if err != nil {
		return ftleraseroleevidence.RetainedIntent{}, err
	}
	if err := validateEraseRoles(snapshot.PhysicalStatus, first, end); err != nil {
		return ftleraseroleevidence.RetainedIntent{}, err
	}
	progress := device.blocks[int(block)]
	if progress.IncompleteErase {
		return ftleraseroleevidence.RetainedIntent{}, ErrEraseIntentBlockIncomplete
	}
	if progress.CompletedErases >= device.maxCompletedErases {
		return ftleraseroleevidence.RetainedIntent{}, nandblockerasecuts.ErrEraseLimit
	}
	if sequence == 0 {
		return ftleraseroleevidence.RetainedIntent{}, ErrInvalidEraseIntentSequence
	}

	stale := make([]bool, int(device.pagesPerBlock))
	for physical := first; physical < end; physical++ {
		stale[int(physical-first)] = snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Stale
	}
	return ftleraseroleevidence.RetainedIntent{
		Sequence: sequence, Block: block, CompletedErases: progress.CompletedErases,
		Stale: stale,
	}, nil
}

// validateEraseRoles is the single admission predicate for a raw erase and
// its pre-erase intent capture. It keeps the historical role-error order.
func validateEraseRoles(statuses []ftlpagemap.PhysicalStatus, first, end int64) error {
	hasStale := false
	for physical := first; physical < end; physical++ {
		switch statuses[int(physical)] {
		case ftlpagemap.Stale:
			hasStale = true
		case ftlpagemap.Free:
		default:
			return ErrIneligibleBlock
		}
	}
	if !hasStale {
		return ErrIneligibleBlock
	}
	return nil
}
