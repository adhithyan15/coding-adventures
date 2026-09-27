// Package x509extension decodes the generic RFC 5280 Extension container.
package x509extension

import (
	"fmt"

	derasn1 "github.com/adhithyan15/coding-adventures/code/packages/go/der-asn1"
	dertlv "github.com/adhithyan15/coding-adventures/code/packages/go/der-tlv"
)

const booleanTag uint32 = 1

type ErrorKind string

const (
	Structure              ErrorKind = "structure"
	MissingExtensionID     ErrorKind = "missing-extension-id"
	InvalidExtensionID     ErrorKind = "invalid-extension-id"
	InvalidCritical        ErrorKind = "invalid-critical"
	EncodedDefaultCritical ErrorKind = "encoded-default-critical"
	MissingExtensionValue  ErrorKind = "missing-extension-value"
	InvalidExtensionValue  ErrorKind = "invalid-extension-value"
	TrailingElement        ErrorKind = "trailing-element"
)

type Error struct {
	Kind        ErrorKind
	ASN1Kind    *derasn1.ErrorKind
	FramingKind *dertlv.ErrorKind
	Offset      int
}

func (e *Error) Error() string {
	return fmt.Sprintf("X.509 extension error %s at byte %d", e.Kind, e.Offset)
}

type X509Extension struct {
	extensionID    derasn1.ObjectIdentifier
	critical       bool
	extensionValue []byte
	valid          bool
}

func (e X509Extension) requireValid() {
	if !e.valid {
		panic("X509Extension is not a validated value")
	}
}

func (e X509Extension) ExtensionID() derasn1.ObjectIdentifier {
	e.requireValid()
	return e.extensionID
}

func (e X509Extension) Critical() bool {
	e.requireValid()
	return e.critical
}

func (e X509Extension) ExtensionValue() []byte {
	e.requireValid()
	return append([]byte(nil), e.extensionValue...)
}

func DecodeX509Extension(decoder *derasn1.ASN1Decoder, element derasn1.ASN1Element) (X509Extension, error) {
	valueOffset := len(element.Header())
	valueLength := len(element.Value())
	fields, err := decoder.Sequence(element)
	if err != nil {
		return X509Extension{}, structureError(err, 0, 0, false)
	}

	idOffset := childOffset(valueOffset, valueLength, len(fields.Remaining()))
	idElement, err := readChild(decoder, fields, valueOffset, idOffset)
	if err != nil {
		return X509Extension{}, err
	}
	if idElement == nil {
		return X509Extension{}, &Error{Kind: MissingExtensionID, Offset: idOffset}
	}
	id, err := derasn1.DecodeObjectIdentifier(*idElement, decoder.Limits())
	if err != nil {
		return X509Extension{}, semanticError(InvalidExtensionID, err, idOffset)
	}

	secondOffset := childOffset(valueOffset, valueLength, len(fields.Remaining()))
	second, err := readChild(decoder, fields, valueOffset, secondOffset)
	if err != nil {
		return X509Extension{}, err
	}
	if second == nil {
		return X509Extension{}, &Error{Kind: MissingExtensionValue, Offset: secondOffset}
	}

	critical := false
	valueElement := second
	valueElementOffset := secondOffset
	if second.Tag().Number == booleanTag {
		critical, err = derasn1.DecodeBoolean(*second)
		if err != nil {
			return X509Extension{}, semanticError(InvalidCritical, err, secondOffset)
		}
		if !critical {
			return X509Extension{}, &Error{Kind: EncodedDefaultCritical, Offset: secondOffset}
		}
		valueElementOffset = childOffset(valueOffset, valueLength, len(fields.Remaining()))
		valueElement, err = readChild(decoder, fields, valueOffset, valueElementOffset)
		if err != nil {
			return X509Extension{}, err
		}
		if valueElement == nil {
			return X509Extension{}, &Error{Kind: MissingExtensionValue, Offset: valueElementOffset}
		}
	}

	value, err := derasn1.DecodeOctetString(*valueElement)
	if err != nil {
		return X509Extension{}, semanticError(InvalidExtensionValue, err, valueElementOffset)
	}
	trailingOffset := childOffset(valueOffset, valueLength, len(fields.Remaining()))
	trailing, err := readChild(decoder, fields, valueOffset, trailingOffset)
	if err != nil {
		return X509Extension{}, err
	}
	if trailing != nil {
		return X509Extension{}, &Error{Kind: TrailingElement, Offset: trailingOffset}
	}
	return X509Extension{
		extensionID:    id,
		critical:       critical,
		extensionValue: append([]byte(nil), value...),
		valid:          true,
	}, nil
}

func readChild(decoder *derasn1.ASN1Decoder, fields *derasn1.ASN1Cursor, valueOffset, childOffset int) (*derasn1.ASN1Element, error) {
	element, err := fields.Read(decoder)
	if err != nil {
		return nil, structureError(err, valueOffset, childOffset, true)
	}
	return element, nil
}

func structureError(err error, valueOffset, childOffset int, child bool) error {
	nested, ok := err.(*derasn1.Error)
	if !ok {
		return err
	}
	offset := nested.Offset
	if child {
		if nested.Kind == derasn1.Framing {
			offset += valueOffset
		} else {
			offset += childOffset
		}
	}
	kind := nested.Kind
	return &Error{Kind: Structure, ASN1Kind: &kind, FramingKind: nested.FramingKind, Offset: offset}
}

func semanticError(kind ErrorKind, err error, childOffset int) error {
	nested, ok := err.(*derasn1.Error)
	if !ok {
		return err
	}
	asn1Kind := nested.Kind
	return &Error{Kind: kind, ASN1Kind: &asn1Kind, FramingKind: nested.FramingKind, Offset: childOffset + nested.Offset}
}

func childOffset(valueOffset, valueLength, remainingLength int) int {
	return valueOffset + valueLength - remainingLength
}
