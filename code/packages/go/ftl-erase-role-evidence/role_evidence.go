// Package ftleraseroleevidence reconstructs one interrupted erase's page roles
// from a retained intent and the marker state observed in the physical block.
package ftleraseroleevidence

import (
	"errors"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

var (
	ErrInvalidGeometry          = errors.New("invalid role-evidence geometry")
	ErrInvalidEraseCount        = errors.New("invalid completed erase count")
	ErrInvalidBlock             = errors.New("invalid observed block ordinal")
	ErrBlockNotIncomplete       = errors.New("observed block is not incomplete")
	ErrEraseBudgetExhausted     = errors.New("no erase budget remains")
	ErrInvalidEvidenceShape     = errors.New("invalid role-evidence array shape")
	ErrIntentSequenceMismatch   = errors.New("erase intent sequence does not match")
	ErrIntentBlockMismatch      = errors.New("erase intent block does not match")
	ErrIntentGenerationMismatch = errors.New("erase intent generation does not match")
	ErrNoStalePages             = errors.New("erase intent retains no stale pages")
	ErrProgrammedFreePage       = errors.New("programmed marker is not backed by stale intent")
)

const (
	maxPhysicalPages  int64  = 128
	maxSafeEraseCount uint64 = 9007199254740991
)

// Geometry bounds one physical device and the configured per-block erase
// ceiling. The status qualifier only needs block and page counts.
type Geometry struct {
	BlockCount         int64
	PagesPerBlock      int64
	MaxCompletedErases uint64
}

// RetainedIntent is the durable evidence selected by a recovery stream. Stale
// is ordered by page ordinal and must remain stable until Qualify returns.
type RetainedIntent struct {
	Sequence        uint64
	Block           int64
	CompletedErases uint64
	Stale           []bool
}

// BlockObservation contains the current physical identity and marker state.
// Programmed must remain stable with the intent while Qualify runs.
type BlockObservation struct {
	Block           int64
	CompletedErases uint64
	IncompleteErase bool
	Programmed      []bool
}

// Qualify reconstructs detached page roles for the observed block. Sequence
// is the nonzero erase-intent sequence selected by the recovery stream.
func Qualify(geometry Geometry, sequence uint64, intent RetainedIntent, observed BlockObservation) ([]ftlpagemap.PhysicalStatus, error) {
	if err := ValidateGeometry(geometry); err != nil {
		return nil, err
	}
	if observed.Block < 0 || observed.Block >= geometry.BlockCount {
		return nil, ErrInvalidBlock
	}
	if !observed.IncompleteErase {
		return nil, ErrBlockNotIncomplete
	}
	if observed.CompletedErases > geometry.MaxCompletedErases {
		return nil, ErrInvalidEraseCount
	}
	if observed.CompletedErases == geometry.MaxCompletedErases {
		return nil, ErrEraseBudgetExhausted
	}
	pageCount := int(geometry.PagesPerBlock)
	if len(intent.Stale) != pageCount || len(observed.Programmed) != pageCount {
		return nil, ErrInvalidEvidenceShape
	}
	if sequence == 0 || intent.Sequence != sequence {
		return nil, ErrIntentSequenceMismatch
	}
	if intent.Block != observed.Block {
		return nil, ErrIntentBlockMismatch
	}
	if intent.CompletedErases != observed.CompletedErases {
		return nil, ErrIntentGenerationMismatch
	}

	hasStale := false
	for _, stale := range intent.Stale {
		hasStale = hasStale || stale
	}
	if !hasStale {
		return nil, ErrNoStalePages
	}
	for page, stale := range intent.Stale {
		if observed.Programmed[page] && !stale {
			return nil, ErrProgrammedFreePage
		}
	}
	roles := make([]ftlpagemap.PhysicalStatus, pageCount)
	for page, stale := range intent.Stale {
		if stale {
			roles[page] = ftlpagemap.Stale
		} else {
			roles[page] = ftlpagemap.Free
		}
	}
	return roles, nil
}

// ValidateGeometry checks the bounded physical dimensions before the portable
// erase-count ceiling. It is shared by callers that must validate an image even
// when no individual block will be qualified.
func ValidateGeometry(geometry Geometry) error {
	if !validGeometry(geometry) {
		return ErrInvalidGeometry
	}
	if geometry.MaxCompletedErases > maxSafeEraseCount {
		return ErrInvalidEraseCount
	}
	return nil
}

func validGeometry(geometry Geometry) bool {
	if geometry.BlockCount < 1 || geometry.PagesPerBlock < 1 ||
		geometry.BlockCount > maxPhysicalPages || geometry.PagesPerBlock > maxPhysicalPages ||
		geometry.BlockCount > maxPhysicalPages/geometry.PagesPerBlock {
		return false
	}
	return geometry.BlockCount*geometry.PagesPerBlock >= 2
}
