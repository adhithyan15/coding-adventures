// Package storagewalcheckpointrecord encodes framed WAL checkpoint generation records.
package storagewalcheckpointrecord

import (
	"encoding/binary"
	"errors"

	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
)

const (
	FrameBytes                = 40
	payloadBytes              = 24
	maxPortableInteger uint64 = 9_007_199_254_740_991
)

type Phase uint8

const (
	Resetting Phase = 1
	Ready     Phase = 2
)

var (
	ErrPhase       = errors.New("checkpoint phase is invalid")
	ErrEpoch       = errors.New("checkpoint epoch is invalid")
	ErrPreviousLSN = errors.New("checkpoint previous LSN is invalid")
	ErrPayloadSize = errors.New("checkpoint payload has invalid size")
	ErrMagic       = errors.New("checkpoint payload has invalid magic")
	ErrVersion     = errors.New("checkpoint payload has unsupported version")
	ErrReserved    = errors.New("checkpoint payload has nonzero reserved bytes")
)

// Record identifies one checkpoint phase and the rank of the preceding WAL.
type Record struct {
	Phase       Phase
	Epoch       uint64
	PreviousLSN uint64
}

func validate(record Record) error {
	if record.Phase != Resetting && record.Phase != Ready {
		return ErrPhase
	}
	if record.Epoch == 0 || record.Epoch > maxPortableInteger {
		return ErrEpoch
	}
	if record.PreviousLSN > maxPortableInteger {
		return ErrPreviousLSN
	}
	return nil
}

// Encode returns an independently owned DBRF v1 frame for record.
func Encode(record Record) ([]byte, error) {
	if err := validate(record); err != nil {
		return nil, err
	}
	payload := make([]byte, payloadBytes)
	copy(payload[:4], "DBCP")
	payload[4] = 1
	payload[5] = byte(record.Phase)
	binary.LittleEndian.PutUint64(payload[8:16], record.Epoch)
	binary.LittleEndian.PutUint64(payload[16:24], record.PreviousLSN)
	return storagerecordframe.Encode(payload)
}

// Decode validates the lower frame before the checkpoint payload and fields.
func Decode(frame []byte) (Record, error) {
	payload, err := storagerecordframe.Decode(frame)
	if err != nil {
		return Record{}, err
	}
	if len(payload) != payloadBytes {
		return Record{}, ErrPayloadSize
	}
	if string(payload[:4]) != "DBCP" {
		return Record{}, ErrMagic
	}
	if payload[4] != 1 {
		return Record{}, ErrVersion
	}
	if payload[6] != 0 || payload[7] != 0 {
		return Record{}, ErrReserved
	}
	record := Record{
		Phase:       Phase(payload[5]),
		Epoch:       binary.LittleEndian.Uint64(payload[8:16]),
		PreviousLSN: binary.LittleEndian.Uint64(payload[16:24]),
	}
	if err := validate(record); err != nil {
		return Record{}, err
	}
	return record, nil
}
