// Package storagerecordframe encodes bounded, checksummed storage records.
package storagerecordframe

import (
	"encoding/binary"
	"errors"

	"github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c"
)

const (
	maxFrameBytes   = 65_536
	headerBytes     = 12
	trailerBytes    = 4
	frameOverhead   = headerBytes + trailerBytes
	MaxPayloadBytes = maxFrameBytes - frameOverhead
)

var (
	ErrFrameSize     = errors.New("storage record frame has invalid size")
	ErrMagic         = errors.New("storage record frame has invalid magic")
	ErrVersion       = errors.New("storage record frame has unsupported version")
	ErrReserved      = errors.New("storage record frame has nonzero reserved bytes")
	ErrPayloadLength = errors.New("storage record payload length is invalid")
	ErrChecksum      = errors.New("storage record checksum mismatch")
)

const magic = "DBRF"

// Encode returns an owned DBRF version 1 frame containing payload.
func Encode(payload []byte) ([]byte, error) {
	if len(payload) > MaxPayloadBytes {
		return nil, ErrPayloadLength
	}

	frame := make([]byte, len(payload)+frameOverhead)
	copy(frame[:4], magic)
	frame[4] = 1
	binary.LittleEndian.PutUint32(frame[8:12], uint32(len(payload)))
	copy(frame[headerBytes:headerBytes+len(payload)], payload)
	checksumEnd := headerBytes + len(payload)
	binary.LittleEndian.PutUint32(frame[checksumEnd:], storagecrc32c.Checksum(frame[:checksumEnd]))
	return frame, nil
}

// Decode validates a complete DBRF frame and returns an owned payload copy.
func Decode(frame []byte) ([]byte, error) {
	if len(frame) < frameOverhead || len(frame) > maxFrameBytes {
		return nil, ErrFrameSize
	}
	if string(frame[:4]) != magic {
		return nil, ErrMagic
	}
	if frame[4] != 1 {
		return nil, ErrVersion
	}
	if frame[5] != 0 || frame[6] != 0 || frame[7] != 0 {
		return nil, ErrReserved
	}

	payloadLength := binary.LittleEndian.Uint32(frame[8:12])
	available := len(frame) - frameOverhead
	if payloadLength != uint32(available) {
		return nil, ErrPayloadLength
	}
	payloadEnd := headerBytes + int(payloadLength)
	storedChecksum := binary.LittleEndian.Uint32(frame[payloadEnd:])
	if checksum := storagecrc32c.Checksum(frame[:payloadEnd]); checksum != storedChecksum {
		return nil, ErrChecksum
	}
	if payloadLength == 0 {
		return nil, nil
	}
	payload := make([]byte, int(payloadLength))
	copy(payload, frame[headerBytes:payloadEnd])
	return payload, nil
}
