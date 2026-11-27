// Package hddwritebackcache models volatile writes over a fixed durable HDD.
//
// The disk image is a value. A write changes only a pending overlay; flushing
// passes the selected sector through the lower byte-prefix cut model. This
// keeps caller-visible cache contents separate from what survives a crash.
package hddwritebackcache

import (
	"errors"

	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
)

const (
	SectorCount    = hddwritecrashcuts.SectorCount
	BytesPerSector = hddwritecrashcuts.BytesPerSector
)

var ErrSectorClean = errors.New("sector_clean")

// State is an immutable branch point. Its arrays are private so callers can
// retain old values without observing mutation from a later transition.
type State struct {
	durable [SectorCount][BytesPerSector]byte
	pending [SectorCount][BytesPerSector]byte
	dirty   [SectorCount]bool
}

// Snapshot is a detached view of durable bytes, visible bytes, and dirty bits.
type Snapshot struct {
	Durable [SectorCount][BytesPerSector]byte
	Visible [SectorCount][BytesPerSector]byte
	Dirty   [SectorCount]bool
}

// FlushResult reports the lower device's acknowledgement and persisted prefix.
// PersistedPrefixBytes is evidence about media, not a caller receipt.
type FlushResult struct {
	Acknowledged         bool
	PersistedPrefixBytes int
}

// Snapshot returns durable and reader-visible sector images by value.
func (state State) Snapshot() Snapshot {
	snapshot := Snapshot{Durable: state.durable, Dirty: state.dirty}
	snapshot.Visible = state.durable
	for index, dirty := range state.dirty {
		if dirty {
			snapshot.Visible[index] = state.pending[index]
		}
	}
	return snapshot
}

// ReadSector returns the pending bytes when dirty, otherwise durable bytes.
func (state State) ReadSector(lba int64) ([BytesPerSector]byte, error) {
	if !validLBA(lba) {
		return [BytesPerSector]byte{}, hddwritecrashcuts.ErrInvalidLBA
	}
	index := int(lba)
	if state.dirty[index] {
		return state.pending[index], nil
	}
	return state.durable[index], nil
}

// WriteSector replaces one pending sector. LBA validation precedes data length.
func (state State) WriteSector(lba int64, data []byte) (State, error) {
	if !validLBA(lba) {
		return state, hddwritecrashcuts.ErrInvalidLBA
	}
	if len(data) != BytesPerSector {
		return state, hddwritecrashcuts.ErrInvalidDataLength
	}
	next := state
	index := int(lba)
	copy(next.pending[index][:], data)
	next.dirty[index] = true
	return next, nil
}

// FlushSector runs the lower HDD byte-prefix transition for one dirty sector.
// Acknowledgement clears only that pending entry; any valid unacknowledged cut
// represents power loss and clears every pending entry.
func (state State) FlushSector(lba int64, cut hddwritecrashcuts.Cut) (State, FlushResult, error) {
	if !validLBA(lba) {
		return state, FlushResult{}, hddwritecrashcuts.ErrInvalidLBA
	}
	index := int(lba)
	if !state.dirty[index] {
		return state, FlushResult{}, ErrSectorClean
	}
	result, err := hddwritecrashcuts.Apply(imageRows(state.durable), lba, state.pending[index][:], cut)
	if err != nil {
		return state, FlushResult{}, err
	}
	next := state
	installImage(&next.durable, result.Image)
	flush := FlushResult{Acknowledged: result.Receipt != nil, PersistedPrefixBytes: result.PersistedPrefixBytes}
	if flush.Acknowledged {
		next.pending[index] = [BytesPerSector]byte{}
		next.dirty[index] = false
	} else {
		next = next.PowerLoss()
	}
	return next, flush, nil
}

// PowerLoss drops every pending write and preserves the durable image.
func (state State) PowerLoss() State {
	state.pending = [SectorCount][BytesPerSector]byte{}
	state.dirty = [SectorCount]bool{}
	return state
}

func validLBA(lba int64) bool { return lba >= 0 && lba < SectorCount }

func imageRows(image [SectorCount][BytesPerSector]byte) [][]byte {
	rows := make([][]byte, SectorCount)
	for index := range image {
		rows[index] = image[index][:]
	}
	return rows
}

func installImage(destination *[SectorCount][BytesPerSector]byte, source [][]byte) {
	for index := range destination {
		copy(destination[index][:], source[index])
	}
}
