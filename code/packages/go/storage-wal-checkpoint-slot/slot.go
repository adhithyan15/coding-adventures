// Package storagewalcheckpointslot stages checkpoint records behind a separate guard byte.
package storagewalcheckpointslot

import (
	"errors"

	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
)

const (
	GuardOffset      = 18
	StagedGuard byte = 0xa5
)

type State uint8

const (
	Vacant State = iota + 1
	Staged
	Published
)

var (
	ErrSlotWidth = errors.New("checkpoint slot has invalid width")
	ErrSlotGuard = errors.New("checkpoint slot has invalid publication guard")
)

// Stage returns the checkpoint frame with its publication guard set. The
// frame checksum remains that of the final, zero-guard representation.
func Stage(record storagewalcheckpointrecord.Record) ([]byte, error) {
	frame, err := storagewalcheckpointrecord.Encode(record)
	if err != nil {
		return nil, err
	}
	frame[GuardOffset] = StagedGuard
	return frame, nil
}

// Inspect classifies one fixed-width slot, decoding only a published frame.
func Inspect(slot []byte) (State, storagewalcheckpointrecord.Record, error) {
	if len(slot) != storagewalcheckpointrecord.FrameBytes {
		return 0, storagewalcheckpointrecord.Record{}, ErrSlotWidth
	}
	if slot[GuardOffset] == StagedGuard {
		return Staged, storagewalcheckpointrecord.Record{}, nil
	}
	if slot[GuardOffset] != 0 {
		return 0, storagewalcheckpointrecord.Record{}, ErrSlotGuard
	}
	allZero := true
	for _, value := range slot {
		if value != 0 {
			allZero = false
			break
		}
	}
	if allZero {
		return Vacant, storagewalcheckpointrecord.Record{}, nil
	}
	record, err := storagewalcheckpointrecord.Decode(slot)
	if err != nil {
		return 0, storagewalcheckpointrecord.Record{}, err
	}
	return Published, record, nil
}
