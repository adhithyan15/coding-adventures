// Package hddwritecrashcuts models one byte-ordered interrupted HDD sector
// write as a pure durable-image transition.
package hddwritecrashcuts

import (
	"errors"
)

const (
	SectorCount    = 32
	BytesPerSector = 8
)

// CutKind identifies the physical point at which this write is interrupted.
type CutKind string

const (
	BeforeMediaWrite              CutKind = "before_media_write"
	DuringMediaWrite              CutKind = "during_media_write"
	AfterMediaWriteBeforeResponse CutKind = "after_media_write_before_response"
	Acknowledged                  CutKind = "acknowledged"
)

// Stable refusal errors identify invalid image, address, data, or cut shape.
var (
	ErrInvalidImageShape = errors.New("invalid_image_shape")
	ErrInvalidLBA        = errors.New("invalid_lba")
	ErrInvalidDataLength = errors.New("invalid_data_length")
	ErrInvalidCut        = errors.New("invalid_cut")
	ErrInvalidPrefix     = errors.New("invalid_prefix")
)

// Cut selects a cut point. WrittenPrefix is required only during a write.
// The pointer lets callers distinguish an absent prefix from an explicit zero.
type Cut struct {
	Kind          CutKind
	WrittenPrefix *int
}

// Receipt is the caller's acknowledgement that the requested bytes are
// durable. Its fixed array is independent from all image storage.
type Receipt struct {
	LBA  int64
	Data [BytesPerSector]byte
}

// Result is a detached durable image plus caller receipt and simulator
// evidence. PersistedPrefixBytes is evidence, not an acknowledgement.
type Result struct {
	Image                [][]byte
	Receipt              *Receipt
	PersistedPrefixBytes int
}

// Apply validates all inputs before producing the selected cut transition.
// The source image and data remain caller-owned; every result row is copied.
func Apply(before [][]byte, lba int64, data []byte, cut Cut) (Result, error) {
	if !validImage(before) {
		return Result{}, ErrInvalidImageShape
	}
	if lba < 0 || lba >= SectorCount {
		return Result{}, ErrInvalidLBA
	}
	if len(data) != BytesPerSector {
		return Result{}, ErrInvalidDataLength
	}
	persisted, acknowledged, err := validateCut(cut)
	if err != nil {
		return Result{}, err
	}

	image := cloneImage(before)
	copy(image[int(lba)][:persisted], data[:persisted])
	var receipt *Receipt
	if acknowledged {
		value := &Receipt{LBA: lba}
		copy(value.Data[:], data)
		receipt = value
	}
	return Result{Image: image, Receipt: receipt, PersistedPrefixBytes: persisted}, nil
}

// Describe validates a cut and reports its exact persisted prefix and
// acknowledgement without inspecting or allocating media bytes.
func Describe(cut Cut) (persistedPrefixBytes int, acknowledged bool, err error) {
	return validateCut(cut)
}

func validImage(image [][]byte) bool {
	if len(image) != SectorCount {
		return false
	}
	for _, sector := range image {
		if len(sector) != BytesPerSector {
			return false
		}
	}
	return true
}

func validateCut(cut Cut) (int, bool, error) {
	switch cut.Kind {
	case BeforeMediaWrite:
		if cut.WrittenPrefix != nil {
			return 0, false, ErrInvalidPrefix
		}
		return 0, false, nil
	case DuringMediaWrite:
		if cut.WrittenPrefix == nil || *cut.WrittenPrefix < 1 || *cut.WrittenPrefix > BytesPerSector-1 {
			return 0, false, ErrInvalidPrefix
		}
		return *cut.WrittenPrefix, false, nil
	case AfterMediaWriteBeforeResponse:
		if cut.WrittenPrefix != nil {
			return 0, false, ErrInvalidPrefix
		}
		return BytesPerSector, false, nil
	case Acknowledged:
		if cut.WrittenPrefix != nil {
			return 0, false, ErrInvalidPrefix
		}
		return BytesPerSector, true, nil
	default:
		return 0, false, ErrInvalidCut
	}
}

func cloneImage(image [][]byte) [][]byte {
	result := make([][]byte, SectorCount)
	for index, sector := range image {
		result[index] = append([]byte(nil), sector...)
	}
	return result
}
