// Package lzw implements LZW (Lempel-Ziv-Welch, 1984) lossless compression.
//
// # What Is LZW?
//
// LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
// added before encoding begins (codes 0–255). This eliminates LZ78's mandatory
// next_char byte — every symbol is already in the dictionary, so the encoder
// can emit pure codes.
//
// With only codes to transmit, LZW uses variable-width bit-packing: codes start
// at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
//
// # Reserved Codes
//
//	0–255:  Pre-seeded single-byte entries.
//	256:    ClearCode — reset to initial 256-entry state.
//	257:    StopCode  — end of code stream.
//	258+:   Dynamically added entries.
//
// # Wire Format (CMP03)
//
//	Bytes 0–3:  original_length (big-endian uint32)
//	Bytes 4+:   bit-packed variable-width codes, LSB-first
//
// # The Tricky Token
//
// During decoding the decoder may receive code C == next_code (not yet added).
// This happens when the input has the form xyx...x. The fix:
//
//	entry = dict[prevCode] + []byte{dict[prevCode][0]}
//
// # The Series
//
//	CMP00 (LZ77,    1977) — Sliding-window backreferences.
//	CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//	CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//	CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF. (this package)
//	CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//	CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
package lzw

import (
	"encoding/binary"
	"errors"
)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const (
	// ClearCode instructs the decoder to reset its dictionary and code size.
	ClearCode = 256

	// StopCode marks the end of the compressed code stream.
	StopCode = 257

	// InitialNextCode is the first dynamically assigned dictionary code.
	InitialNextCode = 258

	// InitialCodeSize is the starting bit-width for codes (covers 0–511).
	InitialCodeSize = 9

	// MaxCodeSize is the maximum bit-width; dict caps at 65536 entries.
	MaxCodeSize = 16
)

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

// bitWriter accumulates variable-width codes into a byte slice, LSB-first.
//
// Bits within each byte are filled from the least-significant end. This
// matches the GIF and Unix compress conventions.
type bitWriter struct {
	buf    uint64
	bitPos uint
	out    []byte
}

func (w *bitWriter) write(code uint, codeSize uint) {
	w.buf |= uint64(code) << w.bitPos
	w.bitPos += codeSize
	for w.bitPos >= 8 {
		w.out = append(w.out, byte(w.buf&0xFF))
		w.buf >>= 8
		w.bitPos -= 8
	}
}

func (w *bitWriter) flush() {
	if w.bitPos > 0 {
		w.out = append(w.out, byte(w.buf&0xFF))
		w.buf = 0
		w.bitPos = 0
	}
}

// bitReader reads variable-width codes from a byte slice, LSB-first.
type bitReader struct {
	data   []byte
	pos    int
	buf    uint64
	bitPos uint
}

// read returns the next codeSize-bit code.
// Returns (0, io.ErrUnexpectedEOF) when the stream is exhausted.
func (r *bitReader) read(codeSize uint) (uint, error) {
	for r.bitPos < codeSize {
		if r.pos >= len(r.data) {
			return 0, errors.New("unexpected end of bit stream")
		}
		r.buf |= uint64(r.data[r.pos]) << r.bitPos
		r.pos++
		r.bitPos += 8
	}
	code := r.buf & ((1 << codeSize) - 1)
	r.buf >>= codeSize
	r.bitPos -= codeSize
	return uint(code), nil
}

func (r *bitReader) exhausted() bool {
	return r.pos >= len(r.data) && r.bitPos == 0
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

// encodeCodes encodes data into a slice of LZW codes including ClearCode and
// StopCode.
//
// The encode dictionary maps byte sequences (as Go strings for hashability)
// to codes. Starting with all 256 single-byte entries, the encoder extends
// the current prefix byte-by-byte. When the prefix + new byte is not in the
// dictionary, the code for the current prefix is emitted, the new sequence is
// added (if room exists), and the prefix resets to just the new byte.
func encodeCodes(data []byte) (codes []uint, originalLength int) {
	originalLength = len(data)

	// Encode dictionary: sequence string → code.
	encDict := make(map[string]uint, 512)
	for b := 0; b < 256; b++ {
		encDict[string([]byte{byte(b)})] = uint(b)
	}
	nextCode := uint(InitialNextCode)
	maxEntries := uint(1 << MaxCodeSize)

	codes = append(codes, ClearCode)

	w := []byte{} // current working prefix

	for _, b := range data {
		wb := append(w, b) //nolint:gocritic // intentional append to create new slice
		wbStr := string(wb)
		if _, ok := encDict[wbStr]; ok {
			w = wb
		} else {
			// Emit code for w.
			codes = append(codes, encDict[string(w)])

			if nextCode < maxEntries {
				encDict[wbStr] = nextCode
				nextCode++
			} else if nextCode == maxEntries {
				// Dictionary full — emit CLEAR and reset.
				codes = append(codes, ClearCode)
				encDict = make(map[string]uint, 512)
				for i := 0; i < 256; i++ {
					encDict[string([]byte{byte(i)})] = uint(i)
				}
				nextCode = InitialNextCode
			}

			w = []byte{b}
		}
	}

	// Flush remaining prefix.
	if len(w) > 0 {
		codes = append(codes, encDict[string(w)])
	}

	codes = append(codes, StopCode)
	return codes, originalLength
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

// decodeCodes decodes a slice of LZW codes back to a byte slice.
//
// The decode dictionary is a slice of byte slices indexed by code. New entries
// are built as dict[prevCode] + []byte{entry[0]}.
//
// The tricky-token case (code == nextCode) is handled by constructing the
// missing entry from the previous entry extended by its own first byte.
func decodeCodes(codes []uint) []byte {
	// Initialise decode dictionary with 256 single-byte entries + 2 placeholders.
	decDict := make([][]byte, 258, 1024)
	for b := 0; b < 256; b++ {
		decDict[b] = []byte{byte(b)}
	}
	// slots 256 and 257 are placeholders for ClearCode and StopCode.
	decDict[256] = nil
	decDict[257] = nil

	nextCode := uint(InitialNextCode)

	var output []byte
	prevCode := ^uint(0) // sentinel "no previous code"

	for _, code := range codes {
		if code == ClearCode {
			// Reset dictionary.
			decDict = decDict[:258]
			for b := 0; b < 256; b++ {
				decDict[b] = []byte{byte(b)}
			}
			nextCode = InitialNextCode
			prevCode = ^uint(0)
			continue
		}

		if code == StopCode {
			break
		}

		var entry []byte

		if code < uint(len(decDict)) {
			entry = decDict[code]
		} else if code == nextCode {
			// Tricky token: code not yet in dict.
			if prevCode == ^uint(0) {
				// Malformed — no previous code.
				continue
			}
			prev := decDict[prevCode]
			entry = make([]byte, len(prev)+1)
			copy(entry, prev)
			entry[len(prev)] = prev[0]
		} else {
			// Invalid code — skip.
			continue
		}

		output = append(output, entry...)

		// Add new entry.
		if prevCode != ^uint(0) && nextCode < (1<<MaxCodeSize) {
			prev := decDict[prevCode]
			newEntry := make([]byte, len(prev)+1)
			copy(newEntry, prev)
			newEntry[len(prev)] = entry[0]
			decDict = append(decDict, newEntry)
			nextCode++
		}

		prevCode = code
	}

	return output
}

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------

// packCodes packs a list of LZW codes into the CMP03 wire format.
//
// The code size starts at InitialCodeSize (9) and grows when nextCode crosses
// the next power-of-2 boundary. ClearCode resets code size back to 9.
func packCodes(codes []uint, originalLength int) []byte {
	w := &bitWriter{}
	codeSize := uint(InitialCodeSize)
	nextCode := uint(InitialNextCode)

	for _, code := range codes {
		w.write(code, codeSize)

		switch {
		case code == ClearCode:
			codeSize = InitialCodeSize
			nextCode = InitialNextCode
		case code != StopCode:
			if nextCode < (1 << MaxCodeSize) {
				nextCode++
				if nextCode > (1<<codeSize) && codeSize < MaxCodeSize {
					codeSize++
				}
			}
		}
	}
	w.flush()

	header := make([]byte, 4)
	binary.BigEndian.PutUint32(header, uint32(originalLength))
	return append(header, w.out...)
}

// unpackCodes reads CMP03 wire-format bytes into a list of LZW codes.
//
// Returns (codes, originalLength). Stops at StopCode or stream exhaustion.
func unpackCodes(data []byte) (codes []uint, originalLength int) {
	if len(data) < 4 {
		return []uint{ClearCode, StopCode}, 0
	}

	originalLength = int(binary.BigEndian.Uint32(data[:4]))
	r := &bitReader{data: data[4:]}

	codeSize := uint(InitialCodeSize)
	nextCode := uint(InitialNextCode)

	for !r.exhausted() {
		code, err := r.read(codeSize)
		if err != nil {
			break
		}
		codes = append(codes, code)

		switch {
		case code == StopCode:
			return codes, originalLength
		case code == ClearCode:
			codeSize = InitialCodeSize
			nextCode = InitialNextCode
		default:
			if nextCode < (1 << MaxCodeSize) {
				nextCode++
				if nextCode > (1<<codeSize) && codeSize < MaxCodeSize {
					codeSize++
				}
			}
		}
	}

	return codes, originalLength
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// Compress compresses data using LZW and returns CMP03 wire-format bytes.
//
// The returned bytes begin with a 4-byte big-endian original_length header
// followed by LSB-first variable-width bit-packed codes.
func Compress(data []byte) []byte {
	codes, originalLength := encodeCodes(data)
	return packCodes(codes, originalLength)
}

// Decompress decompresses CMP03 wire-format data and returns the original bytes.
func Decompress(data []byte) []byte {
	codes, originalLength := unpackCodes(data)
	result := decodeCodes(codes)
	if len(result) > originalLength {
		return result[:originalLength]
	}
	return result
}
