package intel4004packager

import (
	"encoding/hex"
	"fmt"
	"strings"
)

const (
	bytesPerRecord = 16
	recordTypeData = 0x00
	recordTypeEOF  = 0x01
	maxImageSize   = 0x1000
)

type DecodedHex struct {
	Origin int
	Binary []byte
}

func EncodeHex(binary []byte, origin int) (string, error) {
	if len(binary) == 0 {
		return "", fmt.Errorf("binary must be non-empty")
	}
	if origin < 0 || origin > 0xFFFF {
		return "", fmt.Errorf("origin must be 0-65535, got 0x%X", origin)
	}
	if origin+len(binary) > 0x10000 {
		return "", fmt.Errorf("image overflows 16-bit address space: origin=0x%X, size=%d", origin, len(binary))
	}
	lines := []string{}
	for offset := 0; offset < len(binary); offset += bytesPerRecord {
		end := offset + bytesPerRecord
		if end > len(binary) {
			end = len(binary)
		}
		lines = append(lines, dataRecord(origin+offset, binary[offset:end]))
	}
	lines = append(lines, ":00000001FF\n")
	return strings.Join(lines, ""), nil
}

func DecodeHex(text string) (DecodedHex, error) {
	segments := map[int][]byte{}
	lines := strings.Split(strings.ReplaceAll(text, "\r\n", "\n"), "\n")
	for index, raw := range lines {
		line := strings.TrimSpace(raw)
		if line == "" {
			continue
		}
		if !strings.HasPrefix(line, ":") {
			return DecodedHex{}, fmt.Errorf("line %d: expected ':'", index+1)
		}
		payload := line[1:]
		if len(payload)%2 != 0 {
			return DecodedHex{}, fmt.Errorf("line %d: invalid hex length", index+1)
		}
		record, err := hex.DecodeString(payload)
		if err != nil {
			return DecodedHex{}, fmt.Errorf("line %d: invalid hex byte", index+1)
		}
		if len(record) < 5 {
			return DecodedHex{}, fmt.Errorf("line %d: record too short", index+1)
		}
		byteCount := int(record[0])
		address := (int(record[1]) << 8) | int(record[2])
		recordType := int(record[3])
		expectedLength := 4 + byteCount + 1
		if len(record) < expectedLength {
			return DecodedHex{}, fmt.Errorf("line %d: truncated record", index+1)
		}
		data := append([]byte{}, record[4:4+byteCount]...)
		storedChecksum := int(record[4+byteCount])
		computedChecksum := checksum(record[:4+byteCount])
		if computedChecksum != storedChecksum {
			return DecodedHex{}, fmt.Errorf("line %d: checksum mismatch", index+1)
		}
		if recordType == recordTypeEOF {
			break
		}
		if recordType != recordTypeData {
			return DecodedHex{}, fmt.Errorf("line %d: unsupported record type 0x%X", index+1, recordType)
		}
		segments[address] = data
	}
	if len(segments) == 0 {
		return DecodedHex{}, nil
	}
	minAddress := -1
	maxEnd := 0
	for address, data := range segments {
		if minAddress == -1 || address < minAddress {
			minAddress = address
		}
		if address+len(data) > maxEnd {
			maxEnd = address + len(data)
		}
	}
	if maxEnd-minAddress > maxImageSize {
		return DecodedHex{}, fmt.Errorf("decoded image too large: %d bytes (maximum %d bytes for Intel 4004 ROM)", maxEnd-minAddress, maxImageSize)
	}
	buffer := make([]byte, maxEnd-minAddress)
	for address, data := range segments {
		copy(buffer[address-minAddress:], data)
	}
	return DecodedHex{Origin: minAddress, Binary: buffer}, nil
}

func dataRecord(address int, chunk []byte) string {
	byteCount := len(chunk)
	addrHi := (address >> 8) & 0xFF
	addrLo := address & 0xFF
	fields := []byte{byte(byteCount), byte(addrHi), byte(addrLo), byte(recordTypeData)}
	fields = append(fields, chunk...)
	cs := checksum(fields)
	return fmt.Sprintf(":%02X%02X%02X00%s%02X\n", byteCount, addrHi, addrLo, strings.ToUpper(hex.EncodeToString(chunk)), cs)
}

func checksum(fields []byte) int {
	total := 0
	for _, value := range fields {
		total += int(value)
	}
	return (0x100 - (total % 256)) % 256
}
