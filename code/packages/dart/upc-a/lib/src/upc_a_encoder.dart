import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

import 'constants.dart';
import 'encoded_digit.dart';
import 'errors.dart';
import 'messages.dart';

/// Pure Dart implementation of the UPC-A 1D barcode symbology.
///
/// Literate design and symbology mechanics (GS1):
/// 1. Fixed geometry: every symbol is exactly 95 modules — a 3-module
///    start guard (`101`), six L-coded (odd-parity) digits, a 5-module
///    center guard (`01010`), six R-coded (even-parity) digits, and a
///    3-module end guard (`101`).
/// 2. Check digit: the 12th digit guards the payload. Weight the first
///    11 digits alternating 3/1 from the left; the check digit is
///    `(10 - (sum % 10)) % 10`. Eleven-digit input gains its digit;
///    twelve-digit input must already carry the right one.
/// 3. Scalar counting vs. code units: length constraints apply strictly
///    to Unicode scalar values, not raw UTF-16 code units. Lone
///    surrogate halves are rejected before counting to prevent
///    truncation and smuggling.
/// 4. Strict ASCII digits: only `0`-`9` (U+0030..U+0039). Fullwidth and
///    localized Unicode digits are never coerced.
/// 5. Validation precedence follows the v1 symbology contract (which
///    governs where the reference disagrees): `input-too-long`, then
///    `invalid-length`, then `invalid-character` scanned left to right,
///    then `invalid-check-digit`. The reference scans characters before
///    checking length and has no 4096 cap; the spec and corpus require
///    length-first with the cap, so the port follows them.

/// Counts Unicode scalars, rejecting ill-formed host strings.
///
/// Throws [ArgumentError] on unpaired UTF-16 surrogates (outside the
/// v1 API domain, so no stable identifier) and returns the scalar count
/// otherwise.
int _scalarCount(String data) {
  // ignore: unnecessary_null_comparison
  if (data == null) {
    throw ArgumentError(upcAMessageNullInput);
  }
  var count = 0;
  final units = data.codeUnits;
  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit >= upcAHighSurrogateMin && unit <= upcAHighSurrogateMax) {
      if (++index >= units.length ||
          units[index] < upcALowSurrogateMin ||
          units[index] > upcALowSurrogateMax) {
        throw ArgumentError(upcAMessageInvalidScalars);
      }
    } else if (unit >= upcALowSurrogateMin && unit <= upcALowSurrogateMax) {
      throw ArgumentError(upcAMessageInvalidScalars);
    }
    count++;
  }
  return count;
}

/// Validates the shape of [data]: scalar limit, then structural length,
/// then the ASCII-digit alphabet scanned left to right.
void _assertPayloadShape(String data, List<int> acceptedLengths) {
  if (_scalarCount(data) > upcAMaxScalarLimit) {
    throw InvalidUpcAInputError(UpcAErrorCode.inputTooLong);
  }
  // Length is measured in scalars, but any input reaching the alphabet
  // scan below is ASCII-only, where scalars and code units coincide.
  // Counting scalars again here keeps the limit exact for hostile input
  // mixing astral characters with digits.
  if (!acceptedLengths.contains(_scalarCount(data))) {
    throw InvalidUpcAInputError(UpcAErrorCode.invalidLength);
  }
  for (final unit in data.codeUnits) {
    if (unit < upcAAsciiZero || unit > upcAAsciiNine) {
      throw InvalidUpcAInputError(UpcAErrorCode.invalidCharacter);
    }
  }
}

/// Computes the UPC-A check digit for an 11-digit payload.
String computeUpcACheckDigit(String payload11) {
  _assertPayloadShape(payload11, const [11]);

  var oddSum = 0;
  var evenSum = 0;
  for (var index = 0; index < payload11.length; index++) {
    final value = payload11.codeUnitAt(index) - upcAAsciiZero;
    if (index.isEven) {
      oddSum += value;
    } else {
      evenSum += value;
    }
  }

  return (((10 - ((oddSum * 3 + evenSum) % 10)) % 10)).toString();
}

/// Validates and normalizes UPC-A payload data to 12 digits.
///
/// Eleven digits gain a computed check digit; twelve digits must
/// already carry the correct one.
String normalizeUpcA(String data) {
  _assertPayloadShape(data, upcAAcceptedLengths);

  if (_scalarCount(data) == 11) {
    return '$data${computeUpcACheckDigit(data)}';
  }

  final expected = computeUpcACheckDigit(data.substring(0, 11));
  final actual = data.substring(11, 12);
  if (expected != actual) {
    throw InvalidUpcAInputError(UpcAErrorCode.invalidCheckDigit);
  }

  return data;
}

/// Encodes normalized input into per-digit L/R patterns.
List<EncodedDigit> encodeUpcA(String data) {
  final normalized = normalizeUpcA(data);
  final digits = <EncodedDigit>[];
  for (var index = 0; index < normalized.length; index++) {
    final digit = normalized[index];
    final value = normalized.codeUnitAt(index) - upcAAsciiZero;
    final left = index < upcAHalfLength;
    digits.add(
      EncodedDigit(
        digit: digit,
        encoding: left ? 'L' : 'R',
        pattern: left ? upcALeftPatterns[value] : upcARightPatterns[value],
        sourceIndex: index,
        role: index == normalized.length - 1 ? 'check' : 'data',
      ),
    );
  }
  return List.unmodifiable(digits);
}

/// Expands normalized input into barcode runs with guard segments.
List<Barcode1DRun> expandUpcARuns(String data) {
  final encoded = encodeUpcA(data);
  final runs = <Barcode1DRun>[];

  runs.addAll(
    BarcodeLayout1DV1.expandBinary(upcASideGuard, 'start', -1, 'guard'),
  );
  for (final entry in encoded.sublist(0, upcAHalfLength)) {
    runs.addAll(
      BarcodeLayout1DV1.expandBinary(
        entry.pattern,
        entry.digit,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }
  runs.addAll(
    BarcodeLayout1DV1.expandBinary(upcACenterGuard, 'center', -2, 'guard'),
  );
  for (final entry in encoded.sublist(upcAHalfLength)) {
    runs.addAll(
      BarcodeLayout1DV1.expandBinary(
        entry.pattern,
        entry.digit,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }
  runs.addAll(
    BarcodeLayout1DV1.expandBinary(upcASideGuard, 'end', -3, 'guard'),
  );

  return List.unmodifiable(runs);
}

List<Barcode1DSymbol> _buildSymbols(List<EncodedDigit> encoded) {
  final symbols = <Barcode1DSymbol>[
    const Barcode1DSymbol('start', 3, -1, 'guard'),
  ];
  for (final entry in encoded.sublist(0, upcAHalfLength)) {
    symbols.add(
      Barcode1DSymbol(
        entry.digit,
        upcADigitModules,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }
  symbols.add(const Barcode1DSymbol('center', 5, -2, 'guard'));
  for (final entry in encoded.sublist(upcAHalfLength)) {
    symbols.add(
      Barcode1DSymbol(
        entry.digit,
        upcADigitModules,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }
  symbols.add(const Barcode1DSymbol('end', 3, -3, 'guard'));
  return symbols;
}

/// Projects the UPC-A barcode into a 2D vector paint scene with quiet
/// zones and metadata.
///
/// The reference sets `human_readable_text` on the paint options, which
/// — with glyph rendering disabled (the default) — manifests only as
/// the `humanReadableText` scene-metadata key. The portable lane entry
/// rejects the text *option*, so the port writes the metadata key
/// directly: identical observable scenes without touching font
/// authority. Passing `includeHumanReadableText: true` (or a text
/// option) reaches the lane's specified `human-readable-text-unsupported`
/// error, exactly like the ITF lane.
PaintScene layoutUpcA(String data, [Barcode1DSceneOptions? options]) {
  final normalized = normalizeUpcA(data);
  final encoded = encodeUpcA(normalized);
  final runs = expandUpcARuns(normalized);

  final callerMetadata = options?.metadata ?? const {};
  final metadata = Map<String, String>.from(callerMetadata);
  metadata['symbology'] = 'upc-a';
  metadata['humanReadableText'] = normalized;

  final mergedOptions = Barcode1DSceneOptions(
    moduleWidth: options?.moduleWidth ?? 4,
    barHeight: options?.barHeight ?? 120,
    foreground: options?.foreground ?? '#000000',
    background: options?.background ?? '#ffffff',
    label: options?.label ?? 'UPC-A barcode for $normalized',
    metadata: Map.unmodifiable(metadata),
    humanReadableText: options?.humanReadableText,
    includeHumanReadableText: options?.includeHumanReadableText ?? false,
    symbols: List.unmodifiable(_buildSymbols(encoded)),
  );

  return BarcodeLayout1DV1.projectScene(
    runs,
    upcAQuietZoneModules,
    mergedOptions,
  );
}

/// Alias for [layoutUpcA].
PaintScene drawUpcA(String data, [Barcode1DSceneOptions? options]) {
  return layoutUpcA(data, options);
}
