package nandpageprogramcuts

import "errors"

const maxPageSize = 65_536

var (
	ErrInvalidPageSize    = errors.New("page size outside supported range")
	ErrInvalidPageState   = errors.New("invalid NAND page state")
	ErrWrongPayloadLength = errors.New("payload length must equal page size")
	ErrInvalidCut         = errors.New("invalid NAND program cut")
	ErrAlreadyProgrammed  = errors.New("page has already been programmed since erase")
)

// PageState contains the bytes on one page and the command-cycle marker.
// Programmed is independent of Bytes because an all-ones payload changes no
// media bits while still consuming the program opportunity.
type PageState struct {
	Bytes      []byte
	Programmed bool
}

// CutKind identifies the deterministic point where power is lost.
type CutKind uint8

const (
	BeforeAccept CutKind = iota
	AfterPrefix
	AfterMedia
	Complete
)

// Cut selects the write progress. PrefixBytes is meaningful only for
// AfterPrefix and counts the payload bytes already persisted from the front.
type Cut struct {
	Kind        CutKind
	PrefixBytes int
}

// Outcome reports the durable page image and whether the caller received an
// acknowledgement. Every successful outcome owns a fresh byte slice.
type Outcome struct {
	State        PageState
	Acknowledged bool
}

// Apply computes one page-program transition. The caller retains ownership of
// before.Bytes and payload and must keep them stable until this call returns.
// Refusals return the zero Outcome and one stable domain error.
func Apply(pageSize int, before PageState, payload []byte, cut Cut) (Outcome, error) {
	if pageSize < 1 || pageSize > maxPageSize {
		return Outcome{}, ErrInvalidPageSize
	}
	if len(before.Bytes) != pageSize || (!before.Programmed && !allErased(before.Bytes)) {
		return Outcome{}, ErrInvalidPageState
	}
	if len(payload) != pageSize {
		return Outcome{}, ErrWrongPayloadLength
	}
	if err := validateCut(cut, pageSize); err != nil {
		return Outcome{}, err
	}
	if before.Programmed {
		return Outcome{}, ErrAlreadyProgrammed
	}

	bytes := make([]byte, pageSize)
	acknowledged := false
	programmed := false
	switch cut.Kind {
	case BeforeAccept:
		copy(bytes, before.Bytes)
	case AfterPrefix:
		copy(bytes[:cut.PrefixBytes], payload[:cut.PrefixBytes])
		copy(bytes[cut.PrefixBytes:], before.Bytes[cut.PrefixBytes:])
		programmed = true
	case AfterMedia:
		copy(bytes, payload)
		programmed = true
	case Complete:
		copy(bytes, payload)
		programmed = true
		acknowledged = true
	}

	return Outcome{
		State: PageState{
			Bytes:      bytes,
			Programmed: programmed,
		},
		Acknowledged: acknowledged,
	}, nil
}

func validateCut(cut Cut, pageSize int) error {
	switch cut.Kind {
	case BeforeAccept, AfterMedia, Complete:
		if cut.PrefixBytes != 0 {
			return ErrInvalidCut
		}
	case AfterPrefix:
		if cut.PrefixBytes < 0 || cut.PrefixBytes >= pageSize {
			return ErrInvalidCut
		}
	default:
		return ErrInvalidCut
	}
	return nil
}

func allErased(bytes []byte) bool {
	for _, value := range bytes {
		if value != 0xff {
			return false
		}
	}
	return true
}
