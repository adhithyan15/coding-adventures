import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

import 'constants.dart';
import 'encoded_symbol.dart';
import 'errors.dart';
import 'messages.dart';

/// Pure Dart implementation of the Code 128 Code Set B 1D barcode
/// symbology.
///
/// Literate design and symbology mechanics (ISO/IEC 15417):
/// 1. Value domain: printable ASCII (space through tilde) maps to values
///    0-94 by subtracting 32. Start Code B is 104, stop is 106.
/// 2. Weighted checksum: 104 plus each data value times its 1-based
///    position, modulo 103. The checksum symbol rides between the data
///    and the stop symbol.
/// 3. Framing: start + data + checksum + stop, with no inter-character
///    gaps — every pattern starts with a bar and (except stop) ends
///    with a space, so concatenation alternates naturally. Data and
///    check symbols are 11 modules; stop is 13.
/// 4. Empty input is valid (bare start/checksum/stop triple).
/// 5. Lone surrogate halves are rejected before counting to prevent
///    truncation and smuggling. The 4096-scalar cap precedes the
///    alphabet scan.

/// Validates host-string shape and returns the scalar list.
///
/// Throws [ArgumentError] on null or unpaired UTF-16 surrogates
/// (outside the v1 API domain, so no stable identifier).
List<int> _scalars(String data) {
  // ignore: unnecessary_null_comparison
  if (data == null) {
    throw ArgumentError(code128MessageNullInput);
  }
  final result = <int>[];
  final units = data.codeUnits;
  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit >= code128HighSurrogateMin && unit <= code128HighSurrogateMax) {
      if (++index >= units.length ||
          units[index] < code128LowSurrogateMin ||
          units[index] > code128LowSurrogateMax) {
        throw ArgumentError(code128MessageInvalidScalars);
      }
      result.add(
        0x10000 +
            ((unit - code128HighSurrogateMin) << 10) +
            (units[index] - code128LowSurrogateMin),
      );
    } else if (unit >= code128LowSurrogateMin &&
        unit <= code128LowSurrogateMax) {
      throw ArgumentError(code128MessageInvalidScalars);
    } else {
      result.add(unit);
    }
  }
  return result;
}

/// Validates Code 128 Set B input and returns it unchanged.
///
/// The domain is printable ASCII (space through tilde); empty input is
/// valid. Only the scalar cap and the alphabet scan can fail.
String normalizeCode128B(String data) {
  final scalars = _scalars(data);
  if (scalars.length > code128MaxScalarLimit) {
    throw InvalidCode128InputError(Code128ErrorCode.inputTooLong);
  }
  for (final scalar in scalars) {
    if (scalar < code128AsciiMin || scalar > code128AsciiMax) {
      throw InvalidCode128InputError(Code128ErrorCode.invalidCharacter);
    }
  }
  return data;
}

/// Computes the Code 128 checksum over data symbol [values].
///
/// The weighted sum starts at Start B (104); each value contributes
/// `value * (index + 1)`; the result is taken modulo 103.
int computeCode128Checksum(List<int> values) {
  var sum = code128StartB;
  for (var index = 0; index < values.length; index++) {
    sum += values[index] * (index + 1);
  }
  return sum % code128Modulus;
}

/// Encodes validated input into framed symbols with a checksum.
List<EncodedCode128Symbol> encodeCode128B(String data) {
  final normalized = normalizeCode128B(data);
  final dataSymbols = <EncodedCode128Symbol>[];
  for (var index = 0; index < normalized.length; index++) {
    final ch = normalized[index];
    final value = normalized.codeUnitAt(index) - code128ValueBase;
    dataSymbols.add(
      EncodedCode128Symbol(
        label: ch,
        value: value,
        pattern: code128Patterns[value],
        sourceIndex: index,
        role: 'data',
      ),
    );
  }
  final checksum = computeCode128Checksum(
    dataSymbols.map((symbol) => symbol.value).toList(growable: false),
  );

  return List.unmodifiable([
    EncodedCode128Symbol(
      label: 'Start B',
      value: code128StartB,
      pattern: code128Patterns[code128StartB],
      sourceIndex: -1,
      role: 'start',
    ),
    ...dataSymbols,
    EncodedCode128Symbol(
      label: 'Checksum $checksum',
      value: checksum,
      pattern: code128Patterns[checksum],
      sourceIndex: normalized.length,
      role: 'check',
    ),
    EncodedCode128Symbol(
      label: 'Stop',
      value: code128Stop,
      pattern: code128Patterns[code128Stop],
      sourceIndex: normalized.length + 1,
      role: 'stop',
    ),
  ]);
}

List<Barcode1DSymbol> _buildSymbols(List<EncodedCode128Symbol> encoded) {
  final symbols = <Barcode1DSymbol>[];
  for (final entry in encoded) {
    symbols.add(
      Barcode1DSymbol(
        entry.label,
        entry.role == 'stop' ? code128StopModules : code128SymbolModules,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }
  return symbols;
}

/// Expands validated input into barcode runs (no inter-character gaps).
List<Barcode1DRun> expandCode128Runs(String data) {
  final encoded = encodeCode128B(data);
  final runs = <Barcode1DRun>[];

  for (final entry in encoded) {
    runs.addAll(
      BarcodeLayout1DV1.expandBinary(
        entry.pattern,
        entry.label,
        entry.sourceIndex,
        entry.role,
      ),
    );
  }

  return List.unmodifiable(runs);
}

/// Projects the Code 128 barcode into a 2D vector paint scene with
/// quiet zones and metadata.
///
/// The reference sets `human_readable_text` on the paint options, which
/// — with glyph rendering disabled (the default) — manifests only as
/// the `humanReadableText` scene-metadata key. The portable lane entry
/// rejects the text *option*, so the port writes the metadata key
/// directly: identical observable scenes without touching font
/// authority. Passing `includeHumanReadableText: true` (or a text
/// option) reaches the lane's specified `human-readable-text-unsupported`
/// error, exactly like the sibling lanes.
PaintScene layoutCode128(String data, [Barcode1DSceneOptions? options]) {
  final normalized = normalizeCode128B(data);
  final encoded = encodeCode128B(normalized);
  final runs = expandCode128Runs(normalized);
  final checksum = encoded[encoded.length - 2].value;

  final callerMetadata = options?.metadata ?? const {};
  final metadata = Map<String, String>.from(callerMetadata);
  metadata['symbology'] = 'code128';
  metadata['codeSet'] = 'B';
  metadata['checksum'] = checksum.toString();
  metadata['humanReadableText'] = normalized;

  final mergedOptions = Barcode1DSceneOptions(
    moduleWidth: options?.moduleWidth ?? 4,
    barHeight: options?.barHeight ?? 120,
    foreground: options?.foreground ?? '#000000',
    background: options?.background ?? '#ffffff',
    label: options?.label ?? 'Code 128 barcode for $normalized',
    metadata: Map.unmodifiable(metadata),
    humanReadableText: options?.humanReadableText,
    includeHumanReadableText: options?.includeHumanReadableText ?? false,
    symbols: List.unmodifiable(_buildSymbols(encoded)),
  );

  return BarcodeLayout1DV1.projectScene(
    runs,
    code128QuietZoneModules,
    mergedOptions,
  );
}

/// Alias for [layoutCode128].
PaintScene drawCode128(String data, [Barcode1DSceneOptions? options]) {
  return layoutCode128(data, options);
}
