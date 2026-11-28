// Package storagewalcheckpointselection selects the published generation for WAL restart.
package storagewalcheckpointselection

import (
	"errors"

	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
)

var (
	ErrNoPublishedCheckpoint = errors.New("no published checkpoint")
	ErrCheckpointConflict    = errors.New("same-epoch checkpoint records conflict")
)

// Selection identifies the physical slot and exact record governing restart.
type Selection struct {
	Slot   uint8
	Record storagewalcheckpointrecord.Record
}

// Select inspects both physical slots in ordinal order, then applies epoch and
// phase precedence to the published records.
func Select(slots [2][]byte) (Selection, error) {
	var found [2]bool
	var records [2]storagewalcheckpointrecord.Record
	for ordinal := range slots {
		state, record, err := storagewalcheckpointslot.Inspect(slots[ordinal])
		if err != nil {
			return Selection{}, err
		}
		if state == storagewalcheckpointslot.Published {
			found[ordinal] = true
			records[ordinal] = record
		}
	}
	if !found[0] && !found[1] {
		return Selection{}, ErrNoPublishedCheckpoint
	}
	if found[0] && !found[1] {
		return Selection{Slot: 0, Record: records[0]}, nil
	}
	if found[1] && !found[0] {
		return Selection{Slot: 1, Record: records[1]}, nil
	}
	first, second := records[0], records[1]
	if first.Epoch > second.Epoch {
		return Selection{Slot: 0, Record: first}, nil
	}
	if second.Epoch > first.Epoch {
		return Selection{Slot: 1, Record: second}, nil
	}
	if first.PreviousLSN != second.PreviousLSN {
		return Selection{}, ErrCheckpointConflict
	}
	if first.Phase != second.Phase {
		if second.Phase == storagewalcheckpointrecord.Ready {
			return Selection{Slot: 1, Record: second}, nil
		}
		return Selection{Slot: 0, Record: first}, nil
	}
	return Selection{Slot: 0, Record: first}, nil
}
