// Package storagewalrecord encodes the first portable redo WAL records.
package storagewalrecord

import (
	"encoding/binary"
	"errors"

	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
)

const (
	maxIdentifier     uint64 = 9_007_199_254_740_991
	fixedPayloadBytes        = 28
	maxImageBytes            = storagerecordframe.MaxPayloadBytes - fixedPayloadBytes
)

// Kind identifies one of the initial WAL record payloads.
type Kind uint8

const (
	// BeginTransaction records a transaction start.
	BeginTransaction Kind = 1
	// WritePageImage contains a full-page redo image.
	WritePageImage Kind = 2
	// CommitTransaction records a transaction commit.
	CommitTransaction Kind = 3
)

// Record is one begin, full-page redo, or commit record.
type Record struct {
	Kind   Kind
	LSN    uint64
	TxID   uint64
	PageID int64
	Image  []byte
}

var (
	ErrKind        = errors.New("WAL record kind is invalid")
	ErrLSN         = errors.New("WAL record LSN is invalid")
	ErrTxID        = errors.New("WAL record transaction ID is invalid")
	ErrPageID      = errors.New("WAL record page ID is invalid")
	ErrImageLength = errors.New("WAL record image length is invalid")
	ErrHeaderSize  = errors.New("WAL record payload header is truncated")
	ErrReserved    = errors.New("WAL record has nonzero reserved bytes")
)

func validKind(kind Kind) bool {
	return kind == BeginTransaction || kind == WritePageImage || kind == CommitTransaction
}

// Encode returns an owned checksummed frame for record.
func Encode(record Record) ([]byte, error) {
	if !validKind(record.Kind) {
		return nil, ErrKind
	}
	if record.LSN == 0 || record.LSN > maxIdentifier {
		return nil, ErrLSN
	}
	if record.TxID == 0 || record.TxID > maxIdentifier {
		return nil, ErrTxID
	}
	if record.PageID < 0 || uint64(record.PageID) > maxIdentifier {
		return nil, ErrPageID
	}
	if record.Kind != WritePageImage && record.PageID != 0 {
		return nil, ErrPageID
	}
	if record.Kind == WritePageImage {
		if len(record.Image) < 1 || len(record.Image) > maxImageBytes {
			return nil, ErrImageLength
		}
	} else if len(record.Image) != 0 {
		return nil, ErrImageLength
	}

	payload := make([]byte, fixedPayloadBytes+len(record.Image))
	payload[0] = byte(record.Kind)
	binary.LittleEndian.PutUint64(payload[4:12], record.LSN)
	binary.LittleEndian.PutUint64(payload[12:20], record.TxID)
	binary.LittleEndian.PutUint64(payload[20:28], uint64(record.PageID))
	copy(payload[fixedPayloadBytes:], record.Image)
	return storagerecordframe.Encode(payload)
}

// Decode validates one checksummed WAL record and returns owned image bytes.
func Decode(frame []byte) (Record, error) {
	payload, err := storagerecordframe.Decode(frame)
	if err != nil {
		return Record{}, err
	}
	if len(payload) < fixedPayloadBytes {
		return Record{}, ErrHeaderSize
	}

	kind := Kind(payload[0])
	if !validKind(kind) {
		return Record{}, ErrKind
	}
	if payload[1] != 0 || payload[2] != 0 || payload[3] != 0 {
		return Record{}, ErrReserved
	}

	lsn := binary.LittleEndian.Uint64(payload[4:12])
	if lsn == 0 || lsn > maxIdentifier {
		return Record{}, ErrLSN
	}
	txID := binary.LittleEndian.Uint64(payload[12:20])
	if txID == 0 || txID > maxIdentifier {
		return Record{}, ErrTxID
	}
	pageIDValue := binary.LittleEndian.Uint64(payload[20:28])
	if pageIDValue > maxIdentifier {
		return Record{}, ErrPageID
	}
	pageID := int64(pageIDValue)
	image := payload[fixedPayloadBytes:]
	if kind != WritePageImage {
		if pageID != 0 {
			return Record{}, ErrPageID
		}
		if len(image) != 0 {
			return Record{}, ErrImageLength
		}
		return Record{Kind: kind, LSN: lsn, TxID: txID}, nil
	}
	if len(image) == 0 {
		return Record{}, ErrImageLength
	}
	// The framing decoder bounds payloads to MaxPayloadBytes, so image is
	// already at most maxImageBytes. Its payload is newly allocated and private.
	return Record{Kind: kind, LSN: lsn, TxID: txID, PageID: pageID, Image: image}, nil
}
