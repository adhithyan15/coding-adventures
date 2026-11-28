// Package storagewalpageslot stores one framed WAL record in a fixed page.
package storagewalpageslot

import (
	"encoding/binary"
	"errors"

	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
)

const (
	// MinPageBytes is the smallest supported page width.
	MinPageBytes = 48
	// MaxPageBytes is the largest supported page width.
	MaxPageBytes = 65_536
	lengthBytes  = 4
	minimumFrame = 44
)

var (
	ErrPageSize           = errors.New("WAL page slot has unsupported page size")
	ErrSlotFrameLength    = errors.New("WAL slot frame length is invalid")
	ErrVacantPageContents = errors.New("vacant WAL page contains nonzero bytes")
	ErrPadding            = errors.New("WAL page slot padding is nonzero")
)

// EncodePage returns an owned fixed-width page containing one framed record.
func EncodePage(pageBytes int64, record storagewalrecord.Record) ([]byte, error) {
	if pageBytes < MinPageBytes || pageBytes > MaxPageBytes {
		return nil, ErrPageSize
	}
	frame, err := storagewalrecord.Encode(record)
	if err != nil {
		return nil, err
	}
	if len(frame) < minimumFrame || len(frame)+lengthBytes > int(pageBytes) {
		return nil, ErrSlotFrameLength
	}
	page := make([]byte, int(pageBytes))
	binary.LittleEndian.PutUint32(page[:lengthBytes], uint32(len(frame)))
	copy(page[lengthBytes:lengthBytes+len(frame)], frame)
	return page, nil
}

// DecodePage returns a zero record for vacancy or refusal and an owned record
// for a valid occupied page.
func DecodePage(page []byte) (record storagewalrecord.Record, present bool, err error) {
	if len(page) < MinPageBytes || len(page) > MaxPageBytes {
		return storagewalrecord.Record{}, false, ErrPageSize
	}
	frameLength := binary.LittleEndian.Uint32(page[:lengthBytes])
	if frameLength == 0 {
		if hasNonzero(page[lengthBytes:]) {
			return storagewalrecord.Record{}, false, ErrVacantPageContents
		}
		return storagewalrecord.Record{}, false, nil
	}
	available := len(page) - lengthBytes
	if frameLength < minimumFrame || frameLength > uint32(available) {
		return storagewalrecord.Record{}, false, ErrSlotFrameLength
	}
	frameEnd := lengthBytes + int(frameLength)
	record, err = storagewalrecord.Decode(page[lengthBytes:frameEnd])
	if err != nil {
		return storagewalrecord.Record{}, false, err
	}
	if hasNonzero(page[frameEnd:]) {
		return storagewalrecord.Record{}, false, ErrPadding
	}
	return record, true, nil
}

func hasNonzero(data []byte) bool {
	for _, value := range data {
		if value != 0 {
			return true
		}
	}
	return false
}
