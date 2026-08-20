// Package imagecodecpng implements the bounded IC18 portable PNG profile.
//
// The codec is pure and in-memory. It deliberately delegates CRC-32 and raw
// RFC 1951 work to the repository's ZIP package so every portable image lane
// shares one compression implementation and one set of bomb limits.
package imagecodecpng

import (
	"bytes"
	"encoding/binary"
	"errors"
	"math"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
	zipcodec "github.com/adhithyan15/coding-adventures/code/packages/go/zip"
)

const (
	// PNGMaxDimension is the largest accepted width or height.
	PNGMaxDimension uint32 = 16384
	// PNGMaxPixels is the default and hard pixel-count ceiling.
	PNGMaxPixels uint64 = 32 * 1024 * 1024
)

// PngErrorCode is a stable, payload-blind IC18 failure identifier.
type PngErrorCode string

const (
	InvalidMaxPixels       PngErrorCode = "invalid-max-pixels"
	InvalidImageDimensions PngErrorCode = "invalid-image-dimensions"
	InvalidPixelDataLength PngErrorCode = "invalid-pixel-data-length"
	FileTooShort           PngErrorCode = "file-too-short"
	InvalidSignature       PngErrorCode = "invalid-signature"
	TruncatedChunk         PngErrorCode = "truncated-chunk"
	InvalidChunkType       PngErrorCode = "invalid-chunk-type"
	ChunkCRCMismatch       PngErrorCode = "chunk-crc-mismatch"
	ChunkBeforeIHDR        PngErrorCode = "chunk-before-ihdr"
	DuplicateIHDR          PngErrorCode = "duplicate-ihdr"
	InvalidIHDRLength      PngErrorCode = "invalid-ihdr-length"
	InvalidDimensions      PngErrorCode = "invalid-dimensions"
	DimensionLimit         PngErrorCode = "dimension-limit"
	PixelLimit             PngErrorCode = "pixel-limit"
	UnsupportedFeature     PngErrorCode = "unsupported-feature"
	InvalidPLTE            PngErrorCode = "invalid-plte"
	InvalidTRNS            PngErrorCode = "invalid-trns"
	NonconsecutiveIDAT     PngErrorCode = "nonconsecutive-idat"
	InvalidIEND            PngErrorCode = "invalid-iend"
	TrailingData           PngErrorCode = "trailing-data"
	UnknownCriticalChunk   PngErrorCode = "unknown-critical-chunk"
	MissingRequiredChunk   PngErrorCode = "missing-required-chunk"
	InvalidZlibHeader      PngErrorCode = "invalid-zlib-header"
	PresetDictionary       PngErrorCode = "preset-dictionary"
	InflateFailed          PngErrorCode = "inflate-failed"
	InflatedLengthMismatch PngErrorCode = "inflated-length-mismatch"
	IDATCavity             PngErrorCode = "idat-cavity"
	AdlerMismatch          PngErrorCode = "adler-mismatch"
	InvalidFilter          PngErrorCode = "invalid-filter"
)

var pngErrorCodes = []string{
	string(InvalidMaxPixels),
	string(InvalidImageDimensions),
	string(InvalidPixelDataLength),
	string(FileTooShort),
	string(InvalidSignature),
	string(TruncatedChunk),
	string(InvalidChunkType),
	string(ChunkCRCMismatch),
	string(ChunkBeforeIHDR),
	string(DuplicateIHDR),
	string(InvalidIHDRLength),
	string(InvalidDimensions),
	string(DimensionLimit),
	string(PixelLimit),
	string(UnsupportedFeature),
	string(InvalidPLTE),
	string(InvalidTRNS),
	string(NonconsecutiveIDAT),
	string(InvalidIEND),
	string(TrailingData),
	string(UnknownCriticalChunk),
	string(MissingRequiredChunk),
	string(InvalidZlibHeader),
	string(PresetDictionary),
	string(InflateFailed),
	string(InflatedLengthMismatch),
	string(IDATCavity),
	string(AdlerMismatch),
	string(InvalidFilter),
}

// PNGErrorCodes returns the closed IC18 taxonomy in normative order.
func PNGErrorCodes() []string { return append([]string(nil), pngErrorCodes...) }

// PngError is a portable failure whose Error string contains no input data.
type PngError struct {
	Code PngErrorCode
}

func (e *PngError) Error() string { return string(e.Code) }

func fail(code PngErrorCode) error { return &PngError{Code: code} }

// DecodeOptions configures a single DecodePNG call.
//
// MaxPixels is a pointer so nil means "use the default", while zero remains an
// explicitly invalid caller value. A caller may only lower the hard ceiling.
type DecodeOptions struct {
	MaxPixels *float64
}

func maxPixels(options []DecodeOptions) (uint64, error) {
	if len(options) > 1 {
		return 0, fail(InvalidMaxPixels)
	}
	if len(options) == 0 || options[0].MaxPixels == nil {
		return PNGMaxPixels, nil
	}
	value := *options[0].MaxPixels
	if math.IsNaN(value) || math.IsInf(value, 0) || math.Trunc(value) != value ||
		value <= 0 || value > float64(PNGMaxPixels) {
		return 0, fail(InvalidMaxPixels)
	}
	return uint64(value), nil
}

// PngCodec implements pixelcontainer.ImageCodec. The zero value uses the
// default limit; NewPngCodec validates a caller-lowered limit eagerly.
type PngCodec struct {
	options DecodeOptions
}

// NewPngCodec constructs an interface codec with an optional lower limit.
func NewPngCodec(options ...DecodeOptions) (*PngCodec, error) {
	if _, err := maxPixels(options); err != nil {
		return nil, err
	}
	if len(options) == 0 {
		return &PngCodec{}, nil
	}
	return &PngCodec{options: options[0]}, nil
}

// MimeType returns the registered PNG media type.
func (PngCodec) MimeType() string { return "image/png" }

// Encode implements pixelcontainer.ImageCodec. The legacy interface cannot
// return an error, so malformed in-memory containers panic just as the existing
// Go paint adapter does; direct callers should prefer EncodePNG.
func (PngCodec) Encode(pixels *pc.PixelContainer) []byte {
	encoded, err := EncodePNG(pixels)
	if err != nil {
		panic(err)
	}
	return encoded
}

// Decode implements pixelcontainer.ImageCodec.
func (codec PngCodec) Decode(data []byte) (*pc.PixelContainer, error) {
	return DecodePNG(data, codec.options)
}

var signature = [...]byte{0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a}

const adlerMod uint32 = 65521

// Adler32 computes the RFC 1950 checksum used by the PNG zlib wrapper.
func Adler32(data []byte) uint32 {
	var a uint32 = 1
	var b uint32
	for start := 0; start < len(data); start += 5552 {
		end := min(start+5552, len(data))
		for _, value := range data[start:end] {
			a += uint32(value)
			b += a
		}
		a %= adlerMod
		b %= adlerMod
	}
	return b<<16 | a
}

func paeth(a, b, c byte) byte {
	p := int(a) + int(b) - int(c)
	pa := abs(p - int(a))
	pb := abs(p - int(b))
	pcDistance := abs(p - int(c))
	if pa <= pb && pa <= pcDistance {
		return a
	}
	if pb <= pcDistance {
		return b
	}
	return c
}

func abs(value int) int {
	if value < 0 {
		return -value
	}
	return value
}

func applyFilter(filter byte, raw, prior []byte, bytesPerPixel int, out []byte) {
	for i, value := range raw {
		var left, above, aboveLeft byte
		if i >= bytesPerPixel {
			left = raw[i-bytesPerPixel]
			aboveLeft = prior[i-bytesPerPixel]
		}
		above = prior[i]
		var predicted byte
		switch filter {
		case 1:
			predicted = left
		case 2:
			predicted = above
		case 3:
			predicted = byte((uint16(left) + uint16(above)) / 2)
		case 4:
			predicted = paeth(left, above, aboveLeft)
		}
		out[i] = value - predicted
	}
}

func chooseFilter(raw, prior []byte, bytesPerPixel int, scratch, best []byte) byte {
	var bestFilter byte
	bestScore := math.MaxInt
	for filter := byte(0); filter <= 4; filter++ {
		applyFilter(filter, raw, prior, bytesPerPixel, scratch)
		score := 0
		for _, value := range scratch {
			if value < 128 {
				score += int(value)
			} else {
				score += 256 - int(value)
			}
		}
		if score < bestScore {
			bestScore = score
			bestFilter = filter
			copy(best, scratch)
		}
	}
	return bestFilter
}

func undoFilter(filter byte, row, prior []byte, bytesPerPixel int) error {
	switch filter {
	case 0:
		return nil
	case 1:
		for i := bytesPerPixel; i < len(row); i++ {
			row[i] += row[i-bytesPerPixel]
		}
	case 2:
		for i := range row {
			row[i] += prior[i]
		}
	case 3:
		for i := range row {
			var left byte
			if i >= bytesPerPixel {
				left = row[i-bytesPerPixel]
			}
			row[i] += byte((uint16(left) + uint16(prior[i])) / 2)
		}
	case 4:
		for i := range row {
			var left, aboveLeft byte
			if i >= bytesPerPixel {
				left = row[i-bytesPerPixel]
				aboveLeft = prior[i-bytesPerPixel]
			}
			row[i] += paeth(left, prior[i], aboveLeft)
		}
	default:
		return fail(InvalidFilter)
	}
	return nil
}

func appendChunk(out []byte, chunkType string, data []byte) []byte {
	out = binary.BigEndian.AppendUint32(out, uint32(len(data)))
	typeBytes := []byte(chunkType)
	out = append(out, typeBytes...)
	out = append(out, data...)
	crc := zipcodec.CRC32(typeBytes, 0)
	crc = zipcodec.CRC32(data, crc)
	return binary.BigEndian.AppendUint32(out, crc)
}

// EncodePNG encodes an RGBA PixelContainer as a bounded 8-bit colour-type-6
// PNG using the normative signed filter heuristic.
func EncodePNG(pixels *pc.PixelContainer) ([]byte, error) {
	if pixels == nil || pixels.Width == 0 || pixels.Height == 0 ||
		pixels.Width > PNGMaxDimension || pixels.Height > PNGMaxDimension {
		return nil, fail(InvalidImageDimensions)
	}
	pixelCount := uint64(pixels.Width) * uint64(pixels.Height)
	if pixelCount > PNGMaxPixels {
		return nil, fail(InvalidImageDimensions)
	}
	if uint64(len(pixels.Data)) != pixelCount*4 {
		return nil, fail(InvalidPixelDataLength)
	}

	out := append([]byte(nil), signature[:]...)
	ihdr := make([]byte, 13)
	binary.BigEndian.PutUint32(ihdr[0:4], pixels.Width)
	binary.BigEndian.PutUint32(ihdr[4:8], pixels.Height)
	ihdr[8], ihdr[9] = 8, 6
	out = appendChunk(out, "IHDR", ihdr)

	stride := int(pixels.Width) * 4
	filtered := make([]byte, int(pixels.Height)*(stride+1))
	prior := make([]byte, stride)
	scratch := make([]byte, stride)
	best := make([]byte, stride)
	for y := 0; y < int(pixels.Height); y++ {
		raw := pixels.Data[y*stride : (y+1)*stride]
		at := y * (stride + 1)
		filtered[at] = chooseFilter(raw, prior, 4, scratch, best)
		copy(filtered[at+1:at+1+stride], best)
		copy(prior, raw)
	}

	deflated := zipcodec.RawDeflate(filtered)
	idat := make([]byte, 2+len(deflated)+4)
	idat[0], idat[1] = 0x78, 0x9c
	copy(idat[2:], deflated)
	binary.BigEndian.PutUint32(idat[len(idat)-4:], Adler32(filtered))
	out = appendChunk(out, "IDAT", idat)
	out = appendChunk(out, "IEND", nil)
	return out, nil
}

func validChunkType(chunkType []byte) bool {
	if len(chunkType) != 4 || chunkType[2]&0x20 != 0 {
		return false
	}
	for _, value := range chunkType {
		if !((value >= 'A' && value <= 'Z') || (value >= 'a' && value <= 'z')) {
			return false
		}
	}
	return true
}

// DecodePNG decodes the bounded, non-interlaced 8-bit IC18 PNG profile.
func DecodePNG(data []byte, options ...DecodeOptions) (*pc.PixelContainer, error) {
	limit, err := maxPixels(options)
	if err != nil {
		return nil, err
	}
	if len(data) < len(signature) {
		return nil, fail(FileTooShort)
	}
	if !bytes.Equal(data[:len(signature)], signature[:]) {
		return nil, fail(InvalidSignature)
	}

	var width, height uint32
	var bitDepth, colourType byte
	var sawIHDR, sawIEND, sawPLTE, sawTRNS bool
	var inIDAT, idatEnded bool
	var transparentGrey *byte
	var transparentRGB *[3]byte
	var idatParts [][]byte

	for pos := len(signature); pos < len(data); {
		if len(data)-pos < 8 {
			return nil, fail(TruncatedChunk)
		}
		length := binary.BigEndian.Uint32(data[pos : pos+4])
		if uint64(length)+12 > uint64(len(data)-pos) {
			return nil, fail(TruncatedChunk)
		}
		typeStart := pos + 4
		dataStart := pos + 8
		dataEnd := dataStart + int(length)
		chunkTypeBytes := data[typeStart:dataStart]
		if !validChunkType(chunkTypeBytes) {
			return nil, fail(InvalidChunkType)
		}
		declaredCRC := binary.BigEndian.Uint32(data[dataEnd : dataEnd+4])
		if zipcodec.CRC32(data[typeStart:dataEnd], 0) != declaredCRC {
			return nil, fail(ChunkCRCMismatch)
		}
		chunkType := string(chunkTypeBytes)
		chunkData := data[dataStart:dataEnd]
		if !sawIHDR && chunkType != "IHDR" {
			return nil, fail(ChunkBeforeIHDR)
		}

		switch chunkType {
		case "IHDR":
			if sawIHDR {
				return nil, fail(DuplicateIHDR)
			}
			if length != 13 {
				return nil, fail(InvalidIHDRLength)
			}
			width = binary.BigEndian.Uint32(chunkData[0:4])
			height = binary.BigEndian.Uint32(chunkData[4:8])
			bitDepth, colourType = chunkData[8], chunkData[9]
			if width == 0 || height == 0 {
				return nil, fail(InvalidDimensions)
			}
			if width > PNGMaxDimension || height > PNGMaxDimension {
				return nil, fail(DimensionLimit)
			}
			if uint64(width)*uint64(height) > limit {
				return nil, fail(PixelLimit)
			}
			if chunkData[10] != 0 || chunkData[11] != 0 || chunkData[12] != 0 {
				return nil, fail(UnsupportedFeature)
			}
			if colourType == 3 || (colourType != 0 && colourType != 2 && colourType != 4 && colourType != 6) || bitDepth != 8 {
				return nil, fail(UnsupportedFeature)
			}
			sawIHDR = true

		case "PLTE":
			if sawPLTE || len(idatParts) > 0 || sawTRNS || (colourType != 2 && colourType != 6) ||
				length < 3 || length > 768 || length%3 != 0 {
				return nil, fail(InvalidPLTE)
			}
			sawPLTE = true

		case "tRNS":
			if sawTRNS || len(idatParts) > 0 {
				return nil, fail(InvalidTRNS)
			}
			switch colourType {
			case 0:
				if length != 2 || binary.BigEndian.Uint16(chunkData) > math.MaxUint8 {
					return nil, fail(InvalidTRNS)
				}
				value := byte(binary.BigEndian.Uint16(chunkData))
				transparentGrey = &value
			case 2:
				if length != 6 {
					return nil, fail(InvalidTRNS)
				}
				var rgb [3]byte
				for i := range rgb {
					value := binary.BigEndian.Uint16(chunkData[i*2 : i*2+2])
					if value > math.MaxUint8 {
						return nil, fail(InvalidTRNS)
					}
					rgb[i] = byte(value)
				}
				transparentRGB = &rgb
			default:
				return nil, fail(InvalidTRNS)
			}
			sawTRNS = true

		case "IDAT":
			if idatEnded {
				return nil, fail(NonconsecutiveIDAT)
			}
			idatParts = append(idatParts, chunkData)
			inIDAT = true

		case "IEND":
			if length != 0 {
				return nil, fail(InvalidIEND)
			}
			if dataEnd+4 != len(data) {
				return nil, fail(TrailingData)
			}
			sawIEND = true
			pos = dataEnd + 4
			continue

		case "acTL", "fcTL", "fdAT":
			// APNG is outside IC18. These chunks are ancillary by PNG's bit
			// convention but semantically change the image, so never skip them.
			return nil, fail(UnsupportedFeature)

		default:
			if chunkTypeBytes[0]&0x20 == 0 {
				return nil, fail(UnknownCriticalChunk)
			}
		}

		if chunkType != "IDAT" && inIDAT {
			inIDAT = false
			idatEnded = true
		}
		pos = dataEnd + 4
	}

	if !sawIHDR || !sawIEND || len(idatParts) == 0 {
		return nil, fail(MissingRequiredChunk)
	}
	var zlibLength uint64
	for _, part := range idatParts {
		zlibLength += uint64(len(part))
	}
	if zlibLength > uint64(len(data)) {
		return nil, fail(TruncatedChunk)
	}
	zlibData := make([]byte, 0, int(zlibLength))
	for _, part := range idatParts {
		zlibData = append(zlibData, part...)
	}
	if len(zlibData) < 6 {
		return nil, fail(InvalidZlibHeader)
	}
	cmf, flg := zlibData[0], zlibData[1]
	if cmf&0x0f != 8 || cmf>>4 > 7 || (uint16(cmf)<<8|uint16(flg))%31 != 0 {
		return nil, fail(InvalidZlibHeader)
	}
	if flg&0x20 != 0 {
		return nil, fail(PresetDictionary)
	}

	channels := 4
	switch colourType {
	case 0:
		channels = 1
	case 2:
		channels = 3
	case 4:
		channels = 2
	}
	stride := uint64(width) * uint64(channels)
	expected := uint64(height) * (stride + 1)
	deflateData := zlibData[2 : len(zlibData)-4]
	result, inflateErr := zipcodec.RawInflateCounted(deflateData, int64(expected))
	if inflateErr != nil {
		var rawErr *zipcodec.RawInflateError
		if errors.As(inflateErr, &rawErr) && rawErr.Code == zipcodec.OutputLimitExceeded {
			return nil, fail(InflatedLengthMismatch)
		}
		return nil, fail(InflateFailed)
	}
	if uint64(len(result.Output)) != expected {
		return nil, fail(InflatedLengthMismatch)
	}
	if result.BytesConsumed != len(deflateData) {
		return nil, fail(IDATCavity)
	}
	if Adler32(result.Output) != binary.BigEndian.Uint32(zlibData[len(zlibData)-4:]) {
		return nil, fail(AdlerMismatch)
	}
	rowSize := int(stride) + 1
	for y := 0; y < int(height); y++ {
		if result.Output[y*rowSize] > 4 {
			return nil, fail(InvalidFilter)
		}
	}

	container := pc.New(width, height)
	prior := make([]byte, int(stride))
	for y := 0; y < int(height); y++ {
		at := y * rowSize
		row := result.Output[at+1 : at+rowSize]
		if err := undoFilter(result.Output[at], row, prior, channels); err != nil {
			return nil, err
		}
		destRow := y * int(width) * 4
		for x := 0; x < int(width); x++ {
			source := x * channels
			dest := destRow + x*4
			switch channels {
			case 1:
				value := row[source]
				container.Data[dest], container.Data[dest+1], container.Data[dest+2] = value, value, value
				container.Data[dest+3] = 255
				if transparentGrey != nil && value == *transparentGrey {
					container.Data[dest+3] = 0
				}
			case 2:
				value := row[source]
				container.Data[dest], container.Data[dest+1], container.Data[dest+2] = value, value, value
				container.Data[dest+3] = row[source+1]
			case 3:
				red, green, blue := row[source], row[source+1], row[source+2]
				container.Data[dest], container.Data[dest+1], container.Data[dest+2], container.Data[dest+3] = red, green, blue, 255
				if transparentRGB != nil && red == transparentRGB[0] && green == transparentRGB[1] && blue == transparentRGB[2] {
					container.Data[dest+3] = 0
				}
			default:
				copy(container.Data[dest:dest+4], row[source:source+4])
			}
		}
		copy(prior, row)
	}
	return container, nil
}
