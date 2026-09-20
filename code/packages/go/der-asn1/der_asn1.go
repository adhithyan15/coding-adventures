// Package derasn1 provides bounded typed ASN.1 DER value decoding.
package derasn1

import (
	"fmt"

	dertlv "github.com/adhithyan15/coding-adventures/code/packages/go/der-tlv"
)

const (
	DefaultMaxDepth         = 32
	DefaultMaxTotalElements = 16_384
	DefaultMaxOIDArcs       = 128
)

type ASN1Limits struct {
	DER              dertlv.Limits
	MaxDepth         int
	MaxTotalElements int
	MaxOIDArcs       int
}

func DefaultLimits() ASN1Limits {
	return ASN1Limits{
		DER: dertlv.DefaultLimits(), MaxDepth: DefaultMaxDepth,
		MaxTotalElements: DefaultMaxTotalElements, MaxOIDArcs: DefaultMaxOIDArcs,
	}
}

type ErrorKind string

const (
	Framing                      ErrorKind = "framing"
	UnexpectedTag                ErrorKind = "unexpected-tag"
	DecoderLimitMismatch         ErrorKind = "decoder-limit-mismatch"
	DepthLimitExceeded           ErrorKind = "depth-limit-exceeded"
	ElementLimitExceeded         ErrorKind = "element-limit-exceeded"
	InvalidBooleanLength         ErrorKind = "invalid-boolean-length"
	InvalidBooleanValue          ErrorKind = "invalid-boolean-value"
	EmptyInteger                 ErrorKind = "empty-integer"
	NonMinimalInteger            ErrorKind = "non-minimal-integer"
	NegativeInteger              ErrorKind = "negative-integer"
	IntegerOverflow              ErrorKind = "integer-overflow"
	MissingUnusedBitCount        ErrorKind = "missing-unused-bit-count"
	InvalidUnusedBitCount        ErrorKind = "invalid-unused-bit-count"
	NonZeroBitPadding            ErrorKind = "non-zero-bit-padding"
	BitLengthOverflow            ErrorKind = "bit-length-overflow"
	NonEmptyNull                 ErrorKind = "non-empty-null"
	NonASCIIIA5String            ErrorKind = "non-ascii-ia5-string"
	EmptyObjectIdentifier        ErrorKind = "empty-object-identifier"
	UnterminatedObjectIdentifier ErrorKind = "unterminated-object-identifier"
	NonMinimalObjectIdentifier   ErrorKind = "non-minimal-object-identifier"
	ObjectIdentifierOverflow     ErrorKind = "object-identifier-overflow"
	OIDArcLimitExceeded          ErrorKind = "oid-arc-limit-exceeded"
)

type Error struct {
	Kind        ErrorKind
	FramingKind *dertlv.ErrorKind
	Offset      int
}

func (e *Error) Error() string {
	return fmt.Sprintf("ASN.1 DER value error %s at byte %d", e.Kind, e.Offset)
}

func fail(kind ErrorKind, offset int) error { return &Error{Kind: kind, Offset: offset} }

func framing(err error) error {
	var derErr *dertlv.Error
	if !asDERError(err, &derErr) {
		return err
	}
	kind := derErr.Kind
	return &Error{Kind: Framing, FramingKind: &kind, Offset: derErr.Offset}
}

func asDERError(err error, target **dertlv.Error) bool {
	value, ok := err.(*dertlv.Error)
	if ok {
		*target = value
	}
	return ok
}

type ASN1Element struct {
	Element dertlv.Element
	Depth   int
}

func (e ASN1Element) Tag() dertlv.Tag  { return e.Element.Tag }
func (e ASN1Element) Header() []byte   { return e.Element.Header() }
func (e ASN1Element) Value() []byte    { return e.Element.Value() }
func (e ASN1Element) Encoded() []byte  { return e.Element.Encoded() }
func (e ASN1Element) valueOffset() int { return len(e.Header()) }

type ASN1Decoder struct {
	limits       ASN1Limits
	elementsRead int
}

func NewDecoder(limits ASN1Limits) *ASN1Decoder { return &ASN1Decoder{limits: limits} }
func (d *ASN1Decoder) Limits() ASN1Limits       { return d.limits }
func (d *ASN1Decoder) ElementsRead() int        { return d.elementsRead }

func (d *ASN1Decoder) DecodeExact(input []byte) (ASN1Element, error) {
	if d.limits.MaxDepth == 0 {
		return ASN1Element{}, fail(DepthLimitExceeded, 0)
	}
	if err := d.requireCapacity(0); err != nil {
		return ASN1Element{}, err
	}
	element, err := dertlv.DecodeExact(input, d.limits.DER)
	if err != nil {
		return ASN1Element{}, framing(err)
	}
	d.elementsRead++
	return ASN1Element{Element: element}, nil
}

func (d *ASN1Decoder) Sequence(element ASN1Element) (*ASN1Cursor, error) {
	return d.constructed(element, dertlv.Universal, 16)
}

func (d *ASN1Decoder) Set(element ASN1Element) (*ASN1Cursor, error) {
	return d.constructed(element, dertlv.Universal, 17)
}

func (d *ASN1Decoder) Explicit(element ASN1Element, tagNumber uint32) (ASN1Element, error) {
	if err := expectTag(element, dertlv.ContextSpecific, true, tagNumber); err != nil {
		return ASN1Element{}, err
	}
	depth, err := d.childDepth(element)
	if err != nil {
		return ASN1Element{}, err
	}
	if err := d.requireCapacity(element.valueOffset()); err != nil {
		return ASN1Element{}, err
	}
	child, err := dertlv.DecodeExact(element.Value(), d.limits.DER)
	if err != nil {
		return ASN1Element{}, framing(err)
	}
	d.elementsRead++
	return ASN1Element{Element: child, Depth: depth}, nil
}

func (d *ASN1Decoder) constructed(element ASN1Element, class dertlv.TagClass, number uint32) (*ASN1Cursor, error) {
	if err := expectTag(element, class, true, number); err != nil {
		return nil, err
	}
	depth, err := d.childDepth(element)
	if err != nil {
		return nil, err
	}
	cursor, err := dertlv.NewCursor(element.Value(), d.limits.DER)
	if err != nil {
		return nil, framing(err)
	}
	return &ASN1Cursor{cursor: cursor, childDepth: depth, limits: d.limits}, nil
}

func (d *ASN1Decoder) childDepth(element ASN1Element) (int, error) {
	depth := element.Depth + 1
	if depth >= d.limits.MaxDepth {
		return 0, fail(DepthLimitExceeded, 0)
	}
	return depth, nil
}

func (d *ASN1Decoder) requireCapacity(offset int) error {
	if d.elementsRead >= d.limits.MaxTotalElements {
		return fail(ElementLimitExceeded, offset)
	}
	return nil
}

type ASN1Cursor struct {
	cursor     *dertlv.Cursor
	childDepth int
	limits     ASN1Limits
}

func (c *ASN1Cursor) Remaining() []byte { return c.cursor.Remaining() }

func (c *ASN1Cursor) Read(decoder *ASN1Decoder) (*ASN1Element, error) {
	if len(c.cursor.Remaining()) == 0 {
		return nil, nil
	}
	if decoder.limits != c.limits {
		return nil, fail(DecoderLimitMismatch, 0)
	}
	if err := decoder.requireCapacity(0); err != nil {
		return nil, err
	}
	element, err := c.cursor.Read()
	if err != nil {
		return nil, framing(err)
	}
	if element == nil {
		return nil, nil
	}
	decoder.elementsRead++
	return &ASN1Element{Element: *element, Depth: c.childDepth}, nil
}

func (c *ASN1Cursor) Finish() error {
	if err := c.cursor.Finish(); err != nil {
		return framing(err)
	}
	return nil
}

type DERInteger struct {
	SignedBytes []byte
	valueOffset int
}

func (i DERInteger) IsNegative() bool { return i.SignedBytes[0]&0x80 != 0 }

func (i DERInteger) ToUint64() (uint64, error) {
	if i.IsNegative() {
		return 0, fail(NegativeInteger, i.valueOffset)
	}
	magnitude := i.SignedBytes
	if magnitude[0] == 0 {
		magnitude = magnitude[1:]
	}
	if len(magnitude) > 8 {
		return 0, fail(IntegerOverflow, i.valueOffset)
	}
	var value uint64
	for _, octet := range magnitude {
		value = value<<8 | uint64(octet)
	}
	return value, nil
}

type DERBitString struct {
	Bytes      []byte
	UnusedBits uint8
	BitLength  int
}

type ObjectIdentifier struct {
	Encoded []byte
	Arcs    []uint64
}

func (o ObjectIdentifier) Equals(expected []uint64) bool {
	if len(o.Arcs) != len(expected) {
		return false
	}
	for index := range expected {
		if o.Arcs[index] != expected[index] {
			return false
		}
	}
	return true
}

func DecodeBoolean(element ASN1Element) (bool, error) {
	if err := expectUniversalPrimitive(element, 1); err != nil {
		return false, err
	}
	value := element.Value()
	if len(value) != 1 {
		return false, fail(InvalidBooleanLength, element.valueOffset())
	}
	if value[0] == 0 {
		return false, nil
	}
	if value[0] == 0xff {
		return true, nil
	}
	return false, fail(InvalidBooleanValue, element.valueOffset())
}

func DecodeInteger(element ASN1Element) (DERInteger, error) {
	if err := expectUniversalPrimitive(element, 2); err != nil {
		return DERInteger{}, err
	}
	value := element.Value()
	if len(value) == 0 {
		return DERInteger{}, fail(EmptyInteger, element.valueOffset())
	}
	if len(value) > 1 && ((value[0] == 0 && value[1]&0x80 == 0) || (value[0] == 0xff && value[1]&0x80 != 0)) {
		return DERInteger{}, fail(NonMinimalInteger, element.valueOffset())
	}
	return DERInteger{SignedBytes: value, valueOffset: element.valueOffset()}, nil
}

func DecodeBitString(element ASN1Element) (DERBitString, error) {
	if err := expectUniversalPrimitive(element, 3); err != nil {
		return DERBitString{}, err
	}
	value := element.Value()
	if len(value) == 0 {
		return DERBitString{}, fail(MissingUnusedBitCount, element.valueOffset())
	}
	unused := value[0]
	payload := value[1:]
	if unused > 7 || (len(payload) == 0 && unused != 0) {
		return DERBitString{}, fail(InvalidUnusedBitCount, element.valueOffset())
	}
	if unused != 0 && payload[len(payload)-1]&byte((1<<unused)-1) != 0 {
		return DERBitString{}, fail(NonZeroBitPadding, element.valueOffset()+len(value)-1)
	}
	return DERBitString{Bytes: payload, UnusedBits: unused, BitLength: len(payload)*8 - int(unused)}, nil
}

func DecodeOctetString(element ASN1Element) ([]byte, error) {
	if err := expectUniversalPrimitive(element, 4); err != nil {
		return nil, err
	}
	return element.Value(), nil
}

func DecodeImplicitOctetString(element ASN1Element, tagNumber uint32) ([]byte, error) {
	if err := expectContextPrimitive(element, tagNumber); err != nil {
		return nil, err
	}
	return element.Value(), nil
}

func DecodeIA5String(element ASN1Element) (string, error) {
	if err := expectUniversalPrimitive(element, 22); err != nil {
		return "", err
	}
	return decodeIA5Contents(element)
}

func DecodeImplicitIA5String(element ASN1Element, tagNumber uint32) (string, error) {
	if err := expectContextPrimitive(element, tagNumber); err != nil {
		return "", err
	}
	return decodeIA5Contents(element)
}

func DecodeNull(element ASN1Element) error {
	if err := expectUniversalPrimitive(element, 5); err != nil {
		return err
	}
	if len(element.Value()) != 0 {
		return fail(NonEmptyNull, element.valueOffset())
	}
	return nil
}

func DecodeObjectIdentifier(element ASN1Element, limits ASN1Limits) (ObjectIdentifier, error) {
	if err := expectUniversalPrimitive(element, 6); err != nil {
		return ObjectIdentifier{}, err
	}
	return decodeOIDContents(element, limits)
}

func DecodeImplicitObjectIdentifier(element ASN1Element, tagNumber uint32, limits ASN1Limits) (ObjectIdentifier, error) {
	if err := expectContextPrimitive(element, tagNumber); err != nil {
		return ObjectIdentifier{}, err
	}
	return decodeOIDContents(element, limits)
}

func decodeIA5Contents(element ASN1Element) (string, error) {
	for offset, octet := range element.Value() {
		if octet > 0x7f {
			return "", fail(NonASCIIIA5String, element.valueOffset()+offset)
		}
	}
	return string(element.Value()), nil
}

func decodeOIDContents(element ASN1Element, limits ASN1Limits) (ObjectIdentifier, error) {
	encoded := element.Value()
	if len(encoded) == 0 {
		return ObjectIdentifier{}, fail(EmptyObjectIdentifier, element.valueOffset())
	}
	combined, offset, err := parseBase128(encoded, 0, element.valueOffset())
	if err != nil {
		return ObjectIdentifier{}, err
	}
	var arcs []uint64
	if combined < 40 {
		arcs = []uint64{0, combined}
	} else if combined < 80 {
		arcs = []uint64{1, combined - 40}
	} else {
		arcs = []uint64{2, combined - 80}
	}
	if len(arcs) > limits.MaxOIDArcs {
		return ObjectIdentifier{}, fail(OIDArcLimitExceeded, element.valueOffset())
	}
	for offset < len(encoded) {
		start := offset
		arc, next, parseErr := parseBase128(encoded, offset, element.valueOffset())
		if parseErr != nil {
			return ObjectIdentifier{}, parseErr
		}
		arcs = append(arcs, arc)
		if len(arcs) > limits.MaxOIDArcs {
			return ObjectIdentifier{}, fail(OIDArcLimitExceeded, element.valueOffset()+start)
		}
		offset = next
	}
	return ObjectIdentifier{Encoded: encoded, Arcs: arcs}, nil
}

func parseBase128(encoded []byte, start, valueOffset int) (uint64, int, error) {
	if encoded[start] == 0x80 {
		return 0, 0, fail(NonMinimalObjectIdentifier, valueOffset+start)
	}
	var value uint64
	for offset := start; ; offset++ {
		if offset >= len(encoded) {
			return 0, 0, fail(UnterminatedObjectIdentifier, valueOffset+offset)
		}
		payload := uint64(encoded[offset] & 0x7f)
		if value > (^uint64(0)-payload)/128 {
			return 0, 0, fail(ObjectIdentifierOverflow, valueOffset+offset)
		}
		value = value*128 + payload
		if encoded[offset]&0x80 == 0 {
			return value, offset + 1, nil
		}
	}
}

func expectUniversalPrimitive(element ASN1Element, number uint32) error {
	return expectTag(element, dertlv.Universal, false, number)
}

func expectContextPrimitive(element ASN1Element, number uint32) error {
	return expectTag(element, dertlv.ContextSpecific, false, number)
}

func expectTag(element ASN1Element, class dertlv.TagClass, constructed bool, number uint32) error {
	tag := element.Tag()
	if tag.Class != class || tag.Constructed != constructed || tag.Number != number {
		return fail(UnexpectedTag, 0)
	}
	return nil
}
