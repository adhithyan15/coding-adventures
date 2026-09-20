// Package dertlv splits canonical DER tag-length-value frames without
// interpreting ASN.1 value semantics. Elements retain the caller's byte slice
// and expose sub-slices, so declared wire lengths are checked before slicing
// and never cause an allocation.
package dertlv

import "fmt"

const (
	DefaultMaxInputLen = 1024 * 1024
	DefaultMaxValueLen = 1024 * 1024
	DefaultMaxElements = 4096
)

type TagClass string

const (
	Universal       TagClass = "universal"
	Application     TagClass = "application"
	ContextSpecific TagClass = "context-specific"
	Private         TagClass = "private"
)

type ErrorKind string

const (
	EmptyInput           ErrorKind = "empty-input"
	TruncatedHighTag     ErrorKind = "truncated-high-tag"
	TruncatedLength      ErrorKind = "truncated-length"
	TruncatedValue       ErrorKind = "truncated-value"
	EndOfContents        ErrorKind = "end-of-contents"
	NonMinimalTag        ErrorKind = "non-minimal-tag"
	TagOverflow          ErrorKind = "tag-overflow"
	IndefiniteLength     ErrorKind = "indefinite-length"
	ReservedLength       ErrorKind = "reserved-length"
	NonMinimalLength     ErrorKind = "non-minimal-length"
	LengthTooWide        ErrorKind = "length-too-wide"
	LengthHostOverflow   ErrorKind = "length-host-overflow"
	InputLimitExceeded   ErrorKind = "input-limit-exceeded"
	ValueLimitExceeded   ErrorKind = "value-limit-exceeded"
	ElementLimitExceeded ErrorKind = "element-limit-exceeded"
	TagLimitExceeded     ErrorKind = "tag-limit-exceeded"
	TrailingData         ErrorKind = "trailing-data"
)

type Error struct {
	Kind   ErrorKind
	Offset int
}

func (e *Error) Error() string {
	return fmt.Sprintf("DER framing error %s at byte %d", e.Kind, e.Offset)
}

type Limits struct {
	MaxInputLen  int
	MaxValueLen  uint64
	MaxElements  int
	MaxTagNumber uint32
}

func DefaultLimits() Limits {
	return Limits{
		MaxInputLen:  DefaultMaxInputLen,
		MaxValueLen:  DefaultMaxValueLen,
		MaxElements:  DefaultMaxElements,
		MaxTagNumber: ^uint32(0),
	}
}

type Tag struct {
	Class       TagClass
	Constructed bool
	Number      uint32
}

type Element struct {
	Tag        Tag
	input      []byte
	start      int
	headerLen  int
	encodedLen int
}

func (e Element) Header() []byte {
	return e.input[e.start : e.start+e.headerLen]
}

func (e Element) Value() []byte {
	return e.input[e.start+e.headerLen : e.start+e.encodedLen]
}

func (e Element) Encoded() []byte {
	return e.input[e.start : e.start+e.encodedLen]
}

func fail(kind ErrorKind, offset int) (*Element, int, error) {
	return nil, 0, &Error{Kind: kind, Offset: offset}
}

func decodeAt(input []byte, start, available int, limits Limits) (*Element, int, error) {
	if available > limits.MaxInputLen {
		return fail(InputLimitExceeded, start)
	}
	if available == 0 {
		return fail(EmptyInput, start)
	}
	first := input[start]
	classes := [...]TagClass{Universal, Application, ContextSpecific, Private}
	class := classes[first>>6]
	constructed := first&0x20 != 0
	low := first & 0x1f
	var number uint32
	identifierLen := 1
	if low != 0x1f {
		number = uint32(low)
		if number > limits.MaxTagNumber {
			return fail(TagLimitExceeded, start)
		}
	} else {
		index := 1
		for {
			if index >= available {
				return fail(TruncatedHighTag, start+index)
			}
			octet := input[start+index]
			payload := uint64(octet & 0x7f)
			if index == 1 && payload == 0 {
				return fail(NonMinimalTag, start+index)
			}
			candidate := uint64(number)*128 + payload
			if candidate > uint64(^uint32(0)) {
				return fail(TagOverflow, start+index)
			}
			number = uint32(candidate)
			if number > limits.MaxTagNumber {
				return fail(TagLimitExceeded, start+index)
			}
			index++
			if octet&0x80 == 0 {
				break
			}
		}
		if number < 31 {
			return fail(NonMinimalTag, start)
		}
		identifierLen = index
	}
	if class == Universal && number == 0 {
		return fail(EndOfContents, start)
	}

	lengthOffset := start + identifierLen
	if identifierLen >= available {
		return fail(TruncatedLength, lengthOffset)
	}
	firstLength := input[lengthOffset]
	var valueLen uint64
	lengthLen := 1
	if firstLength < 0x80 {
		valueLen = uint64(firstLength)
	} else {
		if firstLength == 0x80 {
			return fail(IndefiniteLength, lengthOffset)
		}
		if firstLength == 0xff {
			return fail(ReservedLength, lengthOffset)
		}
		count := int(firstLength & 0x7f)
		if count > 8 {
			return fail(LengthTooWide, lengthOffset)
		}
		valueStart := identifierLen + 1
		valueEnd := valueStart + count
		if valueEnd > available {
			return fail(TruncatedLength, start+available)
		}
		if input[start+valueStart] == 0 {
			return fail(NonMinimalLength, start+valueStart)
		}
		for index := valueStart; index < valueEnd; index++ {
			valueLen = valueLen*256 + uint64(input[start+index])
		}
		if valueLen < 128 {
			return fail(NonMinimalLength, lengthOffset)
		}
		lengthLen = 1 + count
	}
	maxInt := uint64(^uint(0) >> 1)
	if valueLen > maxInt {
		return fail(LengthHostOverflow, lengthOffset)
	}
	if valueLen > limits.MaxValueLen {
		return fail(ValueLimitExceeded, lengthOffset)
	}
	headerLen := identifierLen + lengthLen
	if valueLen > maxInt-uint64(headerLen) {
		return fail(LengthHostOverflow, lengthOffset)
	}
	encodedLen := headerLen + int(valueLen)
	if encodedLen > available {
		return fail(TruncatedValue, start+available)
	}
	element := &Element{
		Tag:   Tag{Class: class, Constructed: constructed, Number: number},
		input: input, start: start, headerLen: headerLen, encodedLen: encodedLen,
	}
	return element, start + encodedLen, nil
}

func DecodeOne(input []byte, limits Limits) (Element, []byte, error) {
	element, next, err := decodeAt(input, 0, len(input), limits)
	if err != nil {
		return Element{}, nil, err
	}
	return *element, input[next:], nil
}

func DecodeExact(input []byte, limits Limits) (Element, error) {
	element, next, err := decodeAt(input, 0, len(input), limits)
	if err != nil {
		return Element{}, err
	}
	if next != len(input) {
		return Element{}, &Error{Kind: TrailingData, Offset: next}
	}
	return *element, nil
}

type Cursor struct {
	input        []byte
	limits       Limits
	offset       int
	elementsRead int
}

func NewCursor(input []byte, limits Limits) (*Cursor, error) {
	if len(input) > limits.MaxInputLen {
		return nil, &Error{Kind: InputLimitExceeded, Offset: 0}
	}
	return &Cursor{input: input, limits: limits}, nil
}

func (c *Cursor) Read() (*Element, error) {
	if c.offset == len(c.input) {
		return nil, nil
	}
	if c.elementsRead >= c.limits.MaxElements {
		return nil, &Error{Kind: ElementLimitExceeded, Offset: c.offset}
	}
	element, next, err := decodeAt(c.input, c.offset, len(c.input)-c.offset, c.limits)
	if err != nil {
		return nil, err
	}
	c.offset = next
	c.elementsRead++
	return element, nil
}

func (c *Cursor) Finish() error {
	if c.offset != len(c.input) {
		return &Error{Kind: TrailingData, Offset: c.offset}
	}
	return nil
}

func (c *Cursor) ElementsRead() int { return c.elementsRead }
func (c *Cursor) Remaining() []byte { return c.input[c.offset:] }
