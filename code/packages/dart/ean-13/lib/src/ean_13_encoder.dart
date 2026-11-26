import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

import 'constants.dart';
import 'encoded_digit.dart';
import 'errors.dart';
import 'messages.dart';

/// Pure Dart implementation of the EAN-13 1D barcode symbology.
///
/// Literate design and symbology mechanics (GS1):
/// 1. Fixed geometry: every symbol is exactly 95 modules — a 3-module
///    start guard (`101`), six left-half digits, a 5-module center
///    guard (`01010`), six right-half digits, and a 3-module end guard
///    (`101`).
/// 2. The leading digit is never bar-encoded. Its value selects one of
///    ten L/G parity mixes for the left half, so the parity pattern
///    carries the thirteenth digit implicitly.
/// 3. Check digit: weight the first 12 digits from the right,
///    alternating 3/1 starting with 3 at the rightmost; the check digit
///    is `(10 - (sum % 10)) % 10`. Twelve-digit input gains its digit;
///    thirteen-digit input must already carry the right one.
/// 4. Scalar counting vs. code units: length constraints apply strictly
///    to Unicode scalar values, not raw UTF-16 code units. Lone
///    surrogate halves are rejected before counting to prevent
///    truncation and smuggling.
/// 5. Strict ASCII digits: only `0`-`9` (U+0030..U+0039). Fullwidth and
///    localized Unicode digits are never coerced.
/// 6. Validation precedence follows the v1 symbology contract (which
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
    throw ArgumentError(ean13MessageNullInput);
  }
  var count = 0;
  final units = data.codeUnits;
  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit >= ean13HighSurrogateMin && unit <= ean13HighSurrogateMax) {
      if (++index >= units.length ||
          units[index] < ean13LowSurrogateMin ||
          units[index] > ean13LowSurrogateMax) {
        throw ArgumentError(ean13MessageInvalidScalars);
      }
    } else if (unit >= ean13LowSurrogateMin && unit <= ean13LowSurrogateMax) {
      throw ArgumentError(ean13MessageInvalidScalars);
    }
    count++;
  }
  return count;
}

/// Validates the shape of [data]: scalar limit, then structural length,
/// then the ASCII-digit alphabet scanned left to right.
void _assertPayloadShape(String data, List<int> acceptedLengths) {
  if (_scalarCount(data) > ean13MaxScalarLimit) {
    throw InvalidEan13InputError(Ean13ErrorCode.inputTooLong);
  }
  // Length is measured in scalars, but any input reaching the alphabet
  // scan below is ASCII-only, where scalars and code units coincide.
  // Counting scalars again here keeps the limit exact for hostile input
  // mixing astral characters with digits.
  if (!acceptedLengths.contains(_scalarCount(data))) {
    throw InvalidEan13InputError(Ean13ErrorCode.invalidLength);
  }
  for (final unit in data.codeUnits) {
    if (unit < ean13AsciiZero || unit > ean13AsciiNine) {
      throw InvalidEan13InputError(Ean13ErrorCode.invalidCharacter);
    }
  }
}

/// Computes the EAN-13 check digit for a 12-digit payload.
String computeEan13CheckDigit(String payload12) {
  _assertPayloadShape(payload12, const [12]);

  var total = 0;
  for (var index = 0; index < payload12.length; index++) {
    final value = payload12.codeUnitAt(
          payload12.length - 1 - index,
        ) -
        ean13AsciiZero;
    total += value * (index.isEven ? 3 : 1);
  }

  return ((10 - (total % 10)) % 10).toString();
}

/// Validates and normalizes EAN-13 payload data to 13 digits.
///
/// Twelve digits gain a computed check digit; thirteen digits must
/// already carry the correct one.
String normalizeEan13(String data) {
  _assertPayloadShape(data, ean13AcceptedLengths);

  if (_scalarCount(data) == 12) {
    return '$data${computeEan13CheckDigit(data)}';
  }

  final expected = computeEan13CheckDigit(data.substring(0, 12));
  final actual = data.substring(12, 13);
  if (expected != actual) {
    throw InvalidEan13InputError(Ean13ErrorCode.invalidCheckDigit);
  }

  return data;
}

/// Returns the left-half L/G parity selection for the leading digit.
String leftParityPattern(String data) {
  final normalized = normalizeEan13(data);
  final leading = normalized.codeUnitAt(0) - ean13AsciiZero;
  return ean13LeftParityPatterns[leading];
}

/// Encodes normalized input into per-digit L/G/R patterns.
///
/// Twelve entries cover indexes 1-12; the leading digit selects parity
/// but emits no bars of its own.
List<EncodedDigit> encodeEan13(String data) {
  final normalized = normalizeEan13(data);
  final parity = leftParityPattern(normalized);
  final digits = <EncodedDigit>[];

  for (var offset = 0; offset < ean13HalfLength; offset++) {
    final digit = normalized[offset + 1];
    final value = normalized.codeUnitAt(offset + 1) - ean13AsciiZero;
    final encoding = parity[offset];
    digits.add(
      EncodedDigit(
        digit: digit,
        encoding: encoding,
        pattern:
            encoding == 'L' ? ean13LeftPatterns[value] : ean13GPatterns[value],
        sourceIndex: offset + 1,
        role: 'data',
      ),
    );
  }

  for (var offset = 0; offset < ean13HalfLength; offset++) {
    final digit = normalized[offset + 7];
    final value = normalized.codeUnitAt(offset + 7) - ean13AsciiZero;
    digits.add(
      EncodedDigit(
        digit: digit,
        encoding: 'R',
        pattern: ean13RightPatterns[value],
        sourceIndex: offset + 7,
        role: offset == ean13HalfLength - 1 ? 'check' : 'data',
      ),
    );
  }

  return List.unmodifiable(digits);
}

/// Expands normalized input into barcode runs with guard segments.
List<Barcode1DRun> expandEan13Runs(String data) {
  final encoded = encodeEan13(data);
  final runs = <Barcode1DRun>[];

  runs.addAll(
    BarcodeLayout1DV1.expandBinary(ean13SideGuard, 'start', -1, 'guard'),
  );
  for (final entry in encoded.sublist(0, ean13HalfLength)) {
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
    BarcodeLayout1DV1.expandBinary(ean13CenterGuard, 'center', -2, 'guard'),
  );
  for (final entry in encoded.sublist(ean13HalfLength)) {
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
    BarcodeLayout1DV1.expandBinary(ean13SideGuard, 'end', -3, 'guard'),
  );

  return List.unmodifiable(runs);
}

List<Barcode1DSymbol> _buildSymbols(List<EncodedDigit> encoded) {
  // NOTE: the reference builds a zero-module leading-digit descriptor
  // and then filters it out before layout. The lane rejects zero-module
  // descriptors outright, so the port never creates it — identical
  // observable symbols either way.
  final symbols = <Barcode1DSymbol>[
    const Barcode1DSymbol('start', 3, -1, 'guard'),
  ];
  for (final entry in encoded.sublist(0, ean13HalfLength)) {
    symbols.add(
      Barcode1DSymbol(
        entry.digit,
        ean13DigitModules,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }
  symbols.add(const Barcode1DSymbol('center', 5, -2, 'guard'));
  for (final entry in encoded.sublist(ean13HalfLength)) {
    symbols.add(
      Barcode1DSymbol(
        entry.digit,
        ean13DigitModules,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }
  symbols.add(const Barcode1DSymbol('end', 3, -3, 'guard'));
  return symbols;
}

/// Projects the EAN-13 barcode into a 2D vector paint scene with quiet
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
PaintScene layoutEan13(String data, [Barcode1DSceneOptions? options]) {
  final normalized = normalizeEan13(data);
  final encoded = encodeEan13(normalized);
  final runs = expandEan13Runs(normalized);

  final callerMetadata = options?.metadata ?? const {};
  final metadata = Map<String, String>.from(callerMetadata);
  metadata['symbology'] = 'ean-13';
  metadata['humanReadableText'] = normalized;
  metadata['leadingDigit'] = normalized[0];
  metadata['leftParity'] = leftParityPattern(normalized);

  final mergedOptions = Barcode1DSceneOptions(
    moduleWidth: options?.moduleWidth ?? 4,
    barHeight: options?.barHeight ?? 120,
    foreground: options?.foreground ?? '#000000',
    background: options?.background ?? '#ffffff',
    label: options?.label ?? 'EAN-13 barcode for $normalized',
    metadata: Map.unmodifiable(metadata),
    humanReadableText: options?.humanReadableText,
    includeHumanReadableText: options?.includeHumanReadableText ?? false,
    symbols: List.unmodifiable(_buildSymbols(encoded)),
  );

  return BarcodeLayout1DV1.projectScene(
    runs,
    ean13QuietZoneModules,
    mergedOptions,
  );
}

/// Alias for [layoutEan13].
PaintScene drawEan13(String data, [Barcode1DSceneOptions? options]) {
  return layoutEan13(data, options);
}
