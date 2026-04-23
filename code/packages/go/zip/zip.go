// Package zip implements the ZIP archive format (CMP09, PKZIP 1989).
//
// ZIP bundles one or more files into a single .zip archive, compressing each
// entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
// The same format underlies Java JARs, Office Open XML (.docx/.xlsx), Android
// APKs (.apk), Python wheels (.whl), and many more.
//
// # Architecture
//
//	┌─────────────────────────────────────────────────────┐
//	│  [Local File Header + File Data]  ← entry 1         │
//	│  [Local File Header + File Data]  ← entry 2         │
//	│  ...                                                │
//	│  ══════════ Central Directory ══════════            │
//	│  [Central Dir Header]  ← entry 1 (has local offset)│
//	│  [Central Dir Header]  ← entry 2                   │
//	│  [End of Central Directory Record]                  │
//	└─────────────────────────────────────────────────────┘
//
// The dual-header design enables two workflows:
//   - Sequential write: append Local Headers one-by-one, write CD at the end.
//   - Random-access read: seek to EOCD at the end, read CD, jump to any entry.
//
// # Wire Format (all integers little-endian)
//
// Local File Header (30 + n + e bytes):
//
//	0x04034B50  signature
//	version_needed uint16   20=DEFLATE, 10=Stored
//	flags uint16            bit 11 = UTF-8 filename
//	method uint16           0=Stored, 8=DEFLATE
//	mod_time uint16         MS-DOS packed time
//	mod_date uint16         MS-DOS packed date
//	crc32 uint32
//	compressed_size uint32
//	uncompressed_size uint32
//	name_len uint16
//	extra_len uint16
//	name bytes...
//	extra bytes...
//	file data...
//
// # DEFLATE Inside ZIP
//
// ZIP method 8 stores raw RFC 1951 DEFLATE — no zlib wrapper. This
// implementation produces RFC 1951 fixed-Huffman compressed blocks (BTYPE=01)
// using the lzss package for LZ77 match-finding.
//
// # Series
//
//	CMP00 (LZ77,    1977) — Sliding-window backreferences.
//	CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//	CMP02 (LZSS,    1982) — LZ77 + flag bits.
//	CMP03 (LZW,     1984) — LZ78 + pre-initialized alphabet; GIF.
//	CMP04 (Huffman, 1952) — Entropy coding.
//	CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
//	CMP09 (ZIP,     1989) — DEFLATE container; universal archive. (this package)
package zip

import (
	"encoding/binary"
	"errors"
	"fmt"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lzss"
)

// =============================================================================
// CRC-32
// =============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
// Detects accidental corruption of decompressed content — not a cryptographic hash.

var crcTable [256]uint32

func init() {
	for i := 0; i < 256; i++ {
		c := uint32(i)
		for k := 0; k < 8; k++ {
			if c&1 != 0 {
				c = 0xEDB88320 ^ (c >> 1)
			} else {
				c >>= 1
			}
		}
		crcTable[i] = c
	}
}

// CRC32 computes a CRC-32 checksum over data, starting from initial (0 for a fresh hash).
// For an incremental update pass the previous result as initial.
func CRC32(data []byte, initial uint32) uint32 {
	crc := initial ^ 0xFFFFFFFF
	for _, b := range data {
		crc = crcTable[(crc^uint32(b))&0xFF] ^ (crc >> 8)
	}
	return crc ^ 0xFFFFFFFF
}

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O
// =============================================================================
//
// RFC 1951 packs bits LSB-first. Huffman codes are written MSB-first logically,
// so we bit-reverse them before writing LSB-first. Extra bits are written directly.

type bitWriter struct {
	buf  uint64
	bits uint32
	out  []byte
}

func newBitWriter() *bitWriter { return &bitWriter{} }

// writeLSB writes the low nbits of value, LSB-first.
func (bw *bitWriter) writeLSB(value uint32, nbits uint32) {
	bw.buf |= uint64(value) << bw.bits
	bw.bits += nbits
	for bw.bits >= 8 {
		bw.out = append(bw.out, byte(bw.buf&0xFF))
		bw.buf >>= 8
		bw.bits -= 8
	}
}

// writeHuffman writes a Huffman code: bit-reverse code, then write LSB-first.
func (bw *bitWriter) writeHuffman(code uint32, nbits uint32) {
	reversed := bits32Reverse(code, nbits)
	bw.writeLSB(reversed, nbits)
}

// align flushes any partial byte (used before stored blocks).
func (bw *bitWriter) align() {
	if bw.bits > 0 {
		bw.out = append(bw.out, byte(bw.buf&0xFF))
		bw.buf = 0
		bw.bits = 0
	}
}

func (bw *bitWriter) finish() []byte {
	bw.align()
	return bw.out
}

// bits32Reverse reverses the low nbits of value.
func bits32Reverse(value, nbits uint32) uint32 {
	result := uint32(0)
	for i := uint32(0); i < nbits; i++ {
		result = (result << 1) | (value & 1)
		value >>= 1
	}
	return result
}

type bitReader struct {
	data []byte
	pos  int
	buf  uint64
	bits uint32
}

func newBitReader(data []byte) *bitReader {
	return &bitReader{data: data}
}

func (br *bitReader) fill(need uint32) bool {
	for br.bits < need {
		if br.pos >= len(br.data) {
			return false
		}
		br.buf |= uint64(br.data[br.pos]) << br.bits
		br.pos++
		br.bits += 8
	}
	return true
}

// readLSB reads nbits bits, LSB-first. Returns (value, ok).
func (br *bitReader) readLSB(nbits uint32) (uint32, bool) {
	if nbits == 0 {
		return 0, true
	}
	if !br.fill(nbits) {
		return 0, false
	}
	mask := uint64((1 << nbits) - 1)
	val := uint32(br.buf & mask)
	br.buf >>= nbits
	br.bits -= nbits
	return val, true
}

// readMSB reads nbits bits and reverses them (for Huffman codes).
func (br *bitReader) readMSB(nbits uint32) (uint32, bool) {
	v, ok := br.readLSB(nbits)
	if !ok {
		return 0, false
	}
	return bits32Reverse(v, nbits), true
}

// align discards partial byte bits to align to byte boundary.
func (br *bitReader) align() {
	discard := br.bits % 8
	if discard > 0 {
		br.buf >>= discard
		br.bits -= discard
	}
}

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// =============================================================================
//
// Fixed Huffman code lengths per RFC 1951 §3.2.6:
//   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
//   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
//   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
//   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
// Distance codes 0–29: 5-bit codes equal to the code number.

func fixedLLEncode(sym uint16) (code uint32, nbits uint32) {
	switch {
	case sym <= 143:
		return 0b00110000 + uint32(sym), 8
	case sym <= 255:
		return 0b110010000 + uint32(sym-144), 9
	case sym <= 279:
		return uint32(sym - 256), 7
	case sym <= 287:
		return 0b11000000 + uint32(sym-280), 8
	}
	panic(fmt.Sprintf("fixedLLEncode: invalid symbol %d", sym))
}

func fixedLLDecode(br *bitReader) (uint16, bool) {
	v7, ok := br.readMSB(7)
	if !ok {
		return 0, false
	}
	if v7 <= 23 {
		return uint16(v7 + 256), true // 7-bit: symbols 256-279
	}
	extra, ok := br.readLSB(1)
	if !ok {
		return 0, false
	}
	v8 := (v7 << 1) | extra
	switch {
	case v8 >= 48 && v8 <= 191:
		return uint16(v8 - 48), true // literals 0-143
	case v8 >= 192 && v8 <= 199:
		return uint16(v8 + 88), true // symbols 280-287
	}
	extra2, ok := br.readLSB(1)
	if !ok {
		return 0, false
	}
	v9 := (v8 << 1) | extra2
	if v9 >= 400 && v9 <= 511 {
		return uint16(v9 - 256), true // literals 144-255
	}
	return 0, false // malformed
}

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// =============================================================================

type tableEntry struct{ base, extra uint32 }

// lengthTable maps LL symbols 257..284 to (base_length, extra_bits).
var lengthTable = [28]tableEntry{
	{3, 0}, {4, 0}, {5, 0}, {6, 0}, {7, 0}, {8, 0}, {9, 0}, {10, 0}, // 257-264
	{11, 1}, {13, 1}, {15, 1}, {17, 1}, // 265-268
	{19, 2}, {23, 2}, {27, 2}, {31, 2}, // 269-272
	{35, 3}, {43, 3}, {51, 3}, {59, 3}, // 273-276
	{67, 4}, {83, 4}, {99, 4}, {115, 4}, // 277-280
	{131, 5}, {163, 5}, {195, 5}, {227, 5}, // 281-284
}

// distTable maps distance codes 0..29 to (base_offset, extra_bits).
var distTable = [30]tableEntry{
	{1, 0}, {2, 0}, {3, 0}, {4, 0},
	{5, 1}, {7, 1}, {9, 2}, {13, 2},
	{17, 3}, {25, 3}, {33, 4}, {49, 4},
	{65, 5}, {97, 5}, {129, 6}, {193, 6},
	{257, 7}, {385, 7}, {513, 8}, {769, 8},
	{1025, 9}, {1537, 9}, {2049, 10}, {3073, 10},
	{4097, 11}, {6145, 11}, {8193, 12}, {12289, 12},
	{16385, 13}, {24577, 13},
}

func encodeLength(length uint8) (sym uint16, base, extra uint32) {
	l := uint32(length)
	for i := len(lengthTable) - 1; i >= 0; i-- {
		if l >= lengthTable[i].base {
			return uint16(257 + i), lengthTable[i].base, lengthTable[i].extra
		}
	}
	panic(fmt.Sprintf("encodeLength: unreachable for length=%d", length))
}

func encodeDist(offset uint16) (code uint8, base, extra uint32) {
	o := uint32(offset)
	for i := len(distTable) - 1; i >= 0; i-- {
		if o >= distTable[i].base {
			return uint8(i), distTable[i].base, distTable[i].extra
		}
	}
	panic(fmt.Sprintf("encodeDist: unreachable for offset=%d", offset))
}

// =============================================================================
// RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
// =============================================================================

func deflateCompress(data []byte) []byte {
	bw := newBitWriter()

	if len(data) == 0 {
		// Empty stored block: BFINAL=1 BTYPE=00 + LEN=0 + NLEN=0xFFFF.
		bw.writeLSB(1, 1)       // BFINAL=1
		bw.writeLSB(0, 2)       // BTYPE=00 (stored)
		bw.align()
		bw.writeLSB(0x0000, 16) // LEN=0
		bw.writeLSB(0xFFFF, 16) // NLEN=~0
		return bw.finish()
	}

	// LZ77/LZSS tokenizer with 32 KB window (RFC 1951 distance range).
	tokens := lzss.Encode(data, 32768, 255, 3)

	// Block header: BFINAL=1 (last), BTYPE=01 (fixed Huffman).
	bw.writeLSB(1, 1) // BFINAL
	bw.writeLSB(1, 1) // BTYPE bit 0 = 1
	bw.writeLSB(0, 1) // BTYPE bit 1 = 0  → BTYPE = 01

	for _, tok := range tokens {
		if tok.Kind == lzss.KindLiteral {
			code, nbits := fixedLLEncode(uint16(tok.Byte))
			bw.writeHuffman(code, nbits)
		} else {
			// Match: length code + extra bits + distance code + extra bits.
			sym, baseLen, extraLenBits := encodeLength(tok.Length)
			code, nbits := fixedLLEncode(sym)
			bw.writeHuffman(code, nbits)
			if extraLenBits > 0 {
				bw.writeLSB(uint32(tok.Length)-baseLen, extraLenBits)
			}
			distCode, baseDist, extraDistBits := encodeDist(tok.Offset)
			bw.writeHuffman(uint32(distCode), 5) // 5-bit fixed distance code
			if extraDistBits > 0 {
				bw.writeLSB(uint32(tok.Offset)-baseDist, extraDistBits)
			}
		}
	}

	// End-of-block symbol (256).
	eobCode, eobBits := fixedLLEncode(256)
	bw.writeHuffman(eobCode, eobBits)

	return bw.finish()
}

// =============================================================================
// RFC 1951 DEFLATE — Decompress
// =============================================================================
//
// Handles stored blocks (BTYPE=00) and fixed Huffman blocks (BTYPE=01).

const maxOutput = 256 * 1024 * 1024 // 256 MB decompression bomb cap

// decodeFixedHuffmanBlock decodes one BTYPE=01 block into out and returns it.
func decodeFixedHuffmanBlock(br *bitReader, out []byte) ([]byte, error) {
	for {
		sym, ok := fixedLLDecode(br)
		if !ok {
			return nil, errors.New("deflate: EOF decoding fixed Huffman symbol")
		}
		switch {
		case sym < 256:
			if len(out) >= maxOutput {
				return nil, errors.New("deflate: output size limit exceeded")
			}
			out = append(out, byte(sym))
		case sym == 256:
			return out, nil // end-of-block
		case sym >= 257 && sym <= 285:
			idx := int(sym - 257)
			if idx >= len(lengthTable) {
				return nil, fmt.Errorf("deflate: invalid length sym %d", sym)
			}
			baseLen, extraLenBits := lengthTable[idx].base, lengthTable[idx].extra
			extraLen, ok := br.readLSB(extraLenBits)
			if !ok {
				return nil, errors.New("deflate: EOF reading length extra bits")
			}
			length := int(baseLen + extraLen)

			distCode, ok := br.readMSB(5)
			if !ok {
				return nil, errors.New("deflate: EOF reading distance code")
			}
			if distCode >= uint32(len(distTable)) {
				return nil, fmt.Errorf("deflate: invalid dist code %d", distCode)
			}
			baseDist, extraDistBits := distTable[distCode].base, distTable[distCode].extra
			extraDist, ok := br.readLSB(extraDistBits)
			if !ok {
				return nil, errors.New("deflate: EOF reading distance extra bits")
			}
			offset := int(baseDist + extraDist)
			if offset > len(out) {
				return nil, fmt.Errorf("deflate: back-reference offset %d > output len %d", offset, len(out))
			}
			if len(out)+length > maxOutput {
				return nil, errors.New("deflate: output size limit exceeded")
			}
			for i := 0; i < length; i++ {
				out = append(out, out[len(out)-offset])
			}
		default:
			return nil, fmt.Errorf("deflate: invalid LL symbol %d", sym)
		}
	}
}

func deflateDecompress(data []byte) ([]byte, error) {
	br := newBitReader(data)
	var out []byte
	var err error

	for {
		bfinal, ok := br.readLSB(1)
		if !ok {
			return nil, errors.New("deflate: unexpected EOF reading BFINAL")
		}
		btype, ok := br.readLSB(2)
		if !ok {
			return nil, errors.New("deflate: unexpected EOF reading BTYPE")
		}

		switch btype {
		case 0b00:
			// ── Stored block ─────────────────────────────────────────────
			br.align()
			lenVal, ok := br.readLSB(16)
			if !ok {
				return nil, errors.New("deflate: EOF reading stored LEN")
			}
			nlen, ok := br.readLSB(16)
			if !ok {
				return nil, errors.New("deflate: EOF reading stored NLEN")
			}
			if (nlen^0xFFFF) != lenVal {
				return nil, fmt.Errorf("deflate: stored block LEN/NLEN mismatch: %d vs %d", lenVal, nlen)
			}
			if len(out)+int(lenVal) > maxOutput {
				return nil, errors.New("deflate: output size limit exceeded")
			}
			for i := uint32(0); i < lenVal; i++ {
				b, ok := br.readLSB(8)
				if !ok {
					return nil, errors.New("deflate: EOF inside stored block data")
				}
				out = append(out, byte(b))
			}

		case 0b01:
			// ── Fixed Huffman block ───────────────────────────────────────
			if out, err = decodeFixedHuffmanBlock(br, out); err != nil {
				return nil, err
			}

		case 0b10:
			return nil, errors.New("deflate: dynamic Huffman blocks (BTYPE=10) not supported")
		default:
			return nil, errors.New("deflate: reserved BTYPE=11")
		}

		if bfinal == 1 {
			break
		}
	}
	return out, nil
}

// =============================================================================
// MS-DOS Date / Time Encoding
// =============================================================================
//
// ZIP timestamps in MS-DOS packed format:
//   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
//   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
// Combined 32-bit value: (date << 16) | time.

// DOSDatetime encodes a calendar datetime into the 32-bit MS-DOS format.
func DOSDatetime(year, month, day, hour, min, sec uint16) uint32 {
	if year < 1980 {
		year = 1980
	}
	t := (hour << 11) | (min << 5) | (sec / 2)
	d := ((year - 1980) << 9) | (month << 5) | day
	return (uint32(d) << 16) | uint32(t)
}

// DOSEpoch is the fixed timestamp 1980-01-01 00:00:00.
// date = (0<<9)|(1<<5)|1 = 33 = 0x0021; time = 0 → 0x00210000.
const DOSEpoch uint32 = 0x00210000

// =============================================================================
// ZIP Write — ZipWriter
// =============================================================================

type cdRecord struct {
	name             []byte
	method           uint16
	dosDatetime      uint32
	crc              uint32
	compressedSize   uint32
	uncompressedSize uint32
	localOffset      uint32
	externalAttrs    uint32
}

// ZipWriter builds a ZIP archive incrementally in memory.
type ZipWriter struct {
	buf     []byte
	entries []cdRecord
}

// NewZipWriter creates a new, empty ZipWriter.
func NewZipWriter() *ZipWriter { return &ZipWriter{} }

// AddFile adds a file entry. If compress is true, DEFLATE is attempted; the
// compressed form is used only if it is strictly smaller than the original.
func (zw *ZipWriter) AddFile(name string, data []byte, compress bool) {
	zw.addEntry(name, data, compress, 0o100644)
}

// AddDirectory adds a directory entry (name should end with '/').
func (zw *ZipWriter) AddDirectory(name string) {
	zw.addEntry(name, nil, false, 0o040755)
}

func (zw *ZipWriter) addEntry(name string, data []byte, compress bool, unixMode uint32) {
	nameBytes := []byte(name)
	checksum := CRC32(data, 0)
	uncompressedSize := uint32(len(data))

	var method uint16
	var fileData []byte
	if compress && len(data) > 0 {
		compressed := deflateCompress(data)
		if len(compressed) < len(data) {
			method = 8
			fileData = compressed
		} else {
			method = 0
			fileData = data
		}
	} else {
		method = 0
		fileData = data
	}

	compressedSize := uint32(len(fileData))
	localOffset := uint32(len(zw.buf))

	versionNeeded := uint16(10)
	if method == 8 {
		versionNeeded = 20
	}
	flags := uint16(0x0800) // GP flag bit 11 = UTF-8 filename

	// ── Local File Header ─────────────────────────────────────────────────
	zw.buf = appendLE32(zw.buf, 0x04034B50) // signature
	zw.buf = appendLE16(zw.buf, versionNeeded)
	zw.buf = appendLE16(zw.buf, flags)
	zw.buf = appendLE16(zw.buf, method)
	zw.buf = appendLE16(zw.buf, uint16(DOSEpoch&0xFFFF))   // mod_time
	zw.buf = appendLE16(zw.buf, uint16(DOSEpoch>>16))      // mod_date
	zw.buf = appendLE32(zw.buf, checksum)
	zw.buf = appendLE32(zw.buf, compressedSize)
	zw.buf = appendLE32(zw.buf, uncompressedSize)
	zw.buf = appendLE16(zw.buf, uint16(len(nameBytes)))
	zw.buf = appendLE16(zw.buf, 0) // extra_field_length = 0
	zw.buf = append(zw.buf, nameBytes...)
	zw.buf = append(zw.buf, fileData...)

	zw.entries = append(zw.entries, cdRecord{
		name:             nameBytes,
		method:           method,
		dosDatetime:      DOSEpoch,
		crc:              checksum,
		compressedSize:   compressedSize,
		uncompressedSize: uncompressedSize,
		localOffset:      localOffset,
		externalAttrs:    unixMode << 16,
	})
}

// Finish appends Central Directory and EOCD; returns the complete archive bytes.
func (zw *ZipWriter) Finish() []byte {
	cdOffset := uint32(len(zw.buf))
	numEntries := uint16(len(zw.entries))

	// ── Central Directory ─────────────────────────────────────────────────
	cdStart := len(zw.buf)
	for _, e := range zw.entries {
		versionNeeded := uint16(10)
		if e.method == 8 {
			versionNeeded = 20
		}
		zw.buf = appendLE32(zw.buf, 0x02014B50) // signature
		zw.buf = appendLE16(zw.buf, 0x031E)     // version_made_by (Unix, v30)
		zw.buf = appendLE16(zw.buf, versionNeeded)
		zw.buf = appendLE16(zw.buf, 0x0800)                         // flags (UTF-8)
		zw.buf = appendLE16(zw.buf, e.method)
		zw.buf = appendLE16(zw.buf, uint16(e.dosDatetime))          // mod_time
		zw.buf = appendLE16(zw.buf, uint16(e.dosDatetime>>16))      // mod_date
		zw.buf = appendLE32(zw.buf, e.crc)
		zw.buf = appendLE32(zw.buf, e.compressedSize)
		zw.buf = appendLE32(zw.buf, e.uncompressedSize)
		zw.buf = appendLE16(zw.buf, uint16(len(e.name)))
		zw.buf = appendLE16(zw.buf, 0) // extra_len
		zw.buf = appendLE16(zw.buf, 0) // comment_len
		zw.buf = appendLE16(zw.buf, 0) // disk_start
		zw.buf = appendLE16(zw.buf, 0) // internal_attrs
		zw.buf = appendLE32(zw.buf, e.externalAttrs)
		zw.buf = appendLE32(zw.buf, e.localOffset)
		zw.buf = append(zw.buf, e.name...)
	}
	cdSize := uint32(len(zw.buf) - cdStart)

	// ── End of Central Directory Record ──────────────────────────────────
	zw.buf = appendLE32(zw.buf, 0x06054B50) // signature
	zw.buf = appendLE16(zw.buf, 0)          // disk_number
	zw.buf = appendLE16(zw.buf, 0)          // cd_disk
	zw.buf = appendLE16(zw.buf, numEntries) // entries this disk
	zw.buf = appendLE16(zw.buf, numEntries) // entries total
	zw.buf = appendLE32(zw.buf, cdSize)
	zw.buf = appendLE32(zw.buf, cdOffset)
	zw.buf = appendLE16(zw.buf, 0) // comment_len

	return zw.buf
}

// =============================================================================
// ZIP Read — ZipEntry and ZipReader
// =============================================================================
//
// ZipReader uses the "EOCD-first" strategy:
//   1. Scan backwards for EOCD signature (PK\x05\x06).
//   2. Read CD offset + size from EOCD.
//   3. Parse all Central Directory headers into ZipEntry objects.
//   4. On Read(entry): seek to Local Header, skip name + extra, read compressed
//      data, decompress, verify CRC-32.

// ZipEntry is the metadata for a single archive entry.
type ZipEntry struct {
	Name             string
	Size             uint32 // uncompressed
	CompressedSize   uint32
	Method           uint16
	CRC32            uint32
	IsDirectory      bool
	localOffset      uint32
}

// ZipReader reads entries from an in-memory ZIP archive.
type ZipReader struct {
	data    []byte
	entries []ZipEntry
}

// NewZipReader parses an in-memory ZIP archive.
// Returns an error if no valid EOCD record is found.
func NewZipReader(data []byte) (*ZipReader, error) {
	zr := &ZipReader{data: data}
	eocdOffset, ok := zr.findEOCD()
	if !ok {
		return nil, errors.New("zip: no End of Central Directory record found")
	}

	cdOffset, ok1 := readLE32(data, eocdOffset+16)
	cdSize, ok2 := readLE32(data, eocdOffset+12)
	if !ok1 || !ok2 {
		return nil, errors.New("zip: EOCD too short")
	}
	if int(cdOffset)+int(cdSize) > len(data) {
		return nil, fmt.Errorf("zip: Central Directory [%d, %d) out of bounds (file size %d)",
			cdOffset, int(cdOffset)+int(cdSize), len(data))
	}

	pos := int(cdOffset)
	end := int(cdOffset) + int(cdSize)
	for pos+4 <= end {
		sig, _ := readLE32(data, pos)
		if sig != 0x02014B50 {
			break
		}

		method, ok1 := readLE16(data, pos+10)
		crc32Val, ok2 := readLE32(data, pos+16)
		compSize, ok3 := readLE32(data, pos+20)
		size, ok4 := readLE32(data, pos+24)
		nameLen, ok5 := readLE16(data, pos+28)
		extraLen, ok6 := readLE16(data, pos+30)
		commentLen, ok7 := readLE16(data, pos+32)
		localOff, ok8 := readLE32(data, pos+42)
		if !ok1 || !ok2 || !ok3 || !ok4 || !ok5 || !ok6 || !ok7 || !ok8 {
			return nil, errors.New("zip: CD entry truncated")
		}

		nameStart := pos + 46
		nameEnd := nameStart + int(nameLen)
		if nameEnd > len(data) {
			return nil, errors.New("zip: CD entry name out of bounds")
		}
		name := string(data[nameStart:nameEnd])

		zr.entries = append(zr.entries, ZipEntry{
			Name:           name,
			Size:           size,
			CompressedSize: compSize,
			Method:         method,
			CRC32:          crc32Val,
			IsDirectory:    strings.HasSuffix(name, "/"),
			localOffset:    localOff,
		})

		pos = nameEnd + int(extraLen) + int(commentLen)
	}

	return zr, nil
}

// Entries returns all entries in the archive (files and directories).
func (zr *ZipReader) Entries() []ZipEntry { return zr.entries }

// Read decompresses and returns the data for entry. Verifies CRC-32.
func (zr *ZipReader) Read(entry ZipEntry) ([]byte, error) {
	if entry.IsDirectory {
		return nil, nil
	}

	// Reject encrypted entries (GP flag bit 0).
	localFlags, ok := readLE16(zr.data, int(entry.localOffset)+6)
	if !ok {
		return nil, errors.New("zip: local header out of bounds")
	}
	if localFlags&1 != 0 {
		return nil, fmt.Errorf("zip: entry %q is encrypted; not supported", entry.Name)
	}

	// Re-read Local Header name_len + extra_len to find data start.
	lhNameLen, ok1 := readLE16(zr.data, int(entry.localOffset)+26)
	lhExtraLen, ok2 := readLE16(zr.data, int(entry.localOffset)+28)
	if !ok1 || !ok2 {
		return nil, errors.New("zip: local header truncated")
	}

	dataStart := int(entry.localOffset) + 30 + int(lhNameLen) + int(lhExtraLen)
	dataEnd := dataStart + int(entry.CompressedSize)
	if dataEnd > len(zr.data) {
		return nil, fmt.Errorf("zip: entry %q data [%d, %d) out of bounds", entry.Name, dataStart, dataEnd)
	}

	compressed := zr.data[dataStart:dataEnd]

	var decompressed []byte
	switch entry.Method {
	case 0:
		decompressed = make([]byte, len(compressed))
		copy(decompressed, compressed)
	case 8:
		var err error
		decompressed, err = deflateDecompress(compressed)
		if err != nil {
			return nil, fmt.Errorf("zip: entry %q: %w", entry.Name, err)
		}
	default:
		return nil, fmt.Errorf("zip: unsupported compression method %d for %q", entry.Method, entry.Name)
	}

	// Trim to declared uncompressed size.
	if len(decompressed) > int(entry.Size) {
		decompressed = decompressed[:entry.Size]
	}

	// Verify CRC-32.
	actualCRC := CRC32(decompressed, 0)
	if actualCRC != entry.CRC32 {
		return nil, fmt.Errorf("zip: CRC-32 mismatch for %q: expected %08X, got %08X",
			entry.Name, entry.CRC32, actualCRC)
	}

	return decompressed, nil
}

// ReadByName finds an entry by name and returns its decompressed data.
func (zr *ZipReader) ReadByName(name string) ([]byte, error) {
	for _, e := range zr.entries {
		if e.Name == name {
			return zr.Read(e)
		}
	}
	return nil, fmt.Errorf("zip: entry %q not found", name)
}

// findEOCD scans backwards for the EOCD signature 0x06054B50.
// The EOCD is at most 22 + 65535 bytes from the end.
func (zr *ZipReader) findEOCD() (int, bool) {
	const eocdSig = 0x06054B50
	const maxComment = 65535
	const eocdMinSize = 22
	data := zr.data

	if len(data) < eocdMinSize {
		return 0, false
	}

	scanStart := len(data) - eocdMinSize - maxComment
	if scanStart < 0 {
		scanStart = 0
	}

	for i := len(data) - eocdMinSize; i >= scanStart; i-- {
		sig, ok := readLE32(data, i)
		if !ok || sig != eocdSig {
			continue
		}
		commentLen, ok := readLE16(data, i+20)
		if ok && i+eocdMinSize+int(commentLen) == len(data) {
			return i, true
		}
	}
	return 0, false
}

// =============================================================================
// Convenience Functions
// =============================================================================

// Zip compresses a list of (name, data) pairs into a ZIP archive.
// Each file is compressed with DEFLATE if it reduces size; otherwise stored.
func Zip(entries []struct{ Name string; Data []byte }) []byte {
	zw := NewZipWriter()
	for _, e := range entries {
		zw.AddFile(e.Name, e.Data, true)
	}
	return zw.Finish()
}

// Unzip extracts all file entries from a ZIP archive.
// Returns a map of name → data. Directories are skipped.
func Unzip(data []byte) (map[string][]byte, error) {
	zr, err := NewZipReader(data)
	if err != nil {
		return nil, err
	}
	out := make(map[string][]byte)
	for _, entry := range zr.entries {
		if !entry.IsDirectory {
			content, err := zr.Read(entry)
			if err != nil {
				return nil, err
			}
			out[entry.Name] = content
		}
	}
	return out, nil
}

// =============================================================================
// Little-endian helpers
// =============================================================================

func appendLE16(dst []byte, v uint16) []byte {
	return binary.LittleEndian.AppendUint16(dst, v)
}

func appendLE32(dst []byte, v uint32) []byte {
	return binary.LittleEndian.AppendUint32(dst, v)
}

func readLE16(data []byte, offset int) (uint16, bool) {
	if offset+2 > len(data) {
		return 0, false
	}
	return binary.LittleEndian.Uint16(data[offset:]), true
}

func readLE32(data []byte, offset int) (uint32, bool) {
	if offset+4 > len(data) {
		return 0, false
	}
	return binary.LittleEndian.Uint32(data[offset:]), true
}
