import 'dart:typed_data';

import 'package:coding_adventures_der_asn1/der_asn1.dart';
import 'package:coding_adventures_der_tlv/der_tlv.dart';

enum X509ExtensionErrorKind {
  structure('structure'),
  missingExtensionId('missing-extension-id'),
  invalidExtensionId('invalid-extension-id'),
  invalidCritical('invalid-critical'),
  encodedDefaultCritical('encoded-default-critical'),
  missingExtensionValue('missing-extension-value'),
  invalidExtensionValue('invalid-extension-value'),
  trailingElement('trailing-element');

  const X509ExtensionErrorKind(this.id);
  final String id;
}

final class X509ExtensionError implements Exception {
  const X509ExtensionError(
    this.kind,
    this.offset, [
    this.asn1Kind,
    this.framingKind,
  ]);

  final X509ExtensionErrorKind kind;
  final int offset;
  final Asn1ErrorKind? asn1Kind;
  final DerErrorKind? framingKind;

  @override
  String toString() => 'X.509 extension error ${kind.id} at byte $offset';
}

final class X509ExtensionIdentifier {
  X509ExtensionIdentifier._(ObjectIdentifier oid)
      : _encoded = Uint8List.fromList(oid.encoded),
        arcs = List<BigInt>.unmodifiable(oid.arcs);

  final Uint8List _encoded;
  final List<BigInt> arcs;

  Uint8List get encoded => Uint8List.fromList(_encoded);
}

final class X509ExtensionValue {
  X509ExtensionValue._(
    this.extensionId,
    this.critical,
    Uint8List extensionValue,
  ) : _extensionValue = Uint8List.fromList(extensionValue);

  final X509ExtensionIdentifier extensionId;
  final bool critical;
  final Uint8List _extensionValue;

  Uint8List get extensionValue => Uint8List.fromList(_extensionValue);
}

X509ExtensionError _structure(
  Asn1Exception error, [
  int? valueOffset,
  int? childOffset,
]) {
  var offset = error.offset;
  if (valueOffset != null && childOffset != null) {
    offset += error.kind == Asn1ErrorKind.framing ? valueOffset : childOffset;
  }
  return X509ExtensionError(
    X509ExtensionErrorKind.structure,
    offset,
    error.kind,
    error.framingKind,
  );
}

X509ExtensionError _semantic(
  X509ExtensionErrorKind kind,
  Asn1Exception error,
  int childOffset,
) =>
    X509ExtensionError(
      kind,
      childOffset + error.offset,
      error.kind,
      error.framingKind,
    );

int _childOffset(int valueOffset, int valueLength, int remainingLength) =>
    valueOffset + valueLength - remainingLength;

X509ExtensionValue decodeX509Extension(
  Asn1Decoder decoder,
  Asn1Element element,
) {
  final valueOffset = element.header.length;
  final valueLength = element.value.length;
  late final Asn1Cursor fields;
  try {
    fields = decoder.sequence(element);
  } on Asn1Exception catch (error) {
    throw _structure(error);
  }

  Asn1Element? read(int offset) {
    try {
      return fields.read(decoder);
    } on Asn1Exception catch (error) {
      throw _structure(error, valueOffset, offset);
    }
  }

  final idOffset = _childOffset(
    valueOffset,
    valueLength,
    fields.remaining.length,
  );
  final idElement = read(idOffset);
  if (idElement == null) {
    throw X509ExtensionError(
      X509ExtensionErrorKind.missingExtensionId,
      idOffset,
    );
  }
  late final ObjectIdentifier oid;
  try {
    oid = decodeObjectIdentifier(idElement, decoder.limits);
  } on Asn1Exception catch (error) {
    throw _semantic(X509ExtensionErrorKind.invalidExtensionId, error, idOffset);
  }

  final secondOffset = _childOffset(
    valueOffset,
    valueLength,
    fields.remaining.length,
  );
  final second = read(secondOffset);
  if (second == null) {
    throw X509ExtensionError(
      X509ExtensionErrorKind.missingExtensionValue,
      secondOffset,
    );
  }

  var critical = false;
  var valueElement = second;
  var valueElementOffset = secondOffset;
  if (second.tag.number == 1) {
    try {
      critical = decodeBoolean(second);
    } on Asn1Exception catch (error) {
      throw _semantic(X509ExtensionErrorKind.invalidCritical, error, secondOffset);
    }
    if (!critical) {
      throw X509ExtensionError(
        X509ExtensionErrorKind.encodedDefaultCritical,
        secondOffset,
      );
    }
    valueElementOffset = _childOffset(
      valueOffset,
      valueLength,
      fields.remaining.length,
    );
    final third = read(valueElementOffset);
    if (third == null) {
      throw X509ExtensionError(
        X509ExtensionErrorKind.missingExtensionValue,
        valueElementOffset,
      );
    }
    valueElement = third;
  }

  late final Uint8List extensionValue;
  try {
    extensionValue = decodeOctetString(valueElement);
  } on Asn1Exception catch (error) {
    throw _semantic(
      X509ExtensionErrorKind.invalidExtensionValue,
      error,
      valueElementOffset,
    );
  }

  final trailingOffset = _childOffset(
    valueOffset,
    valueLength,
    fields.remaining.length,
  );
  if (read(trailingOffset) != null) {
    throw X509ExtensionError(
      X509ExtensionErrorKind.trailingElement,
      trailingOffset,
    );
  }
  return X509ExtensionValue._(
    X509ExtensionIdentifier._(oid),
    critical,
    extensionValue,
  );
}
