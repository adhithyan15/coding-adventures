import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

import 'constants.dart';
import 'encoded_pair.dart';
import 'errors.dart';
import 'messages.dart';

/// Pure Dart implementation of the Interleaved 2 of 5 (ITF) 1D barcode symbology.
///
/// Literate Design & Symbology Mechanics (AIM-BC2 / ISO/IEC 16390):
/// 1. Dual-digit interleaving: ITF achieves compact numeric representation by encoding
///    digit pairs together. The first digit defines the 5 bar widths (2 wide, 3 narrow),
///    while the second digit defines the 5 interleaved space widths (2 wide, 3 narrow),
///    yielding exactly 10 runs per pair.
/// 2. Scalar counting vs. code units: ITF string length constraints apply strictly to
///    Unicode scalar values (code points), not raw UTF-16 code units. Lone surrogate halves
///    are rejected immediately prior to capacity counting to prevent truncation and smuggling.
/// 3. Strict ASCII digits: Symbology standard restricts values to ASCII '0'-'9' (0x30..0x39).
///    Fullwidth and localized Unicode digits must never be coerced.

/// Validates and normalizes ITF payload data.
///
/// Validation precedence:
/// 1. Null or unpaired UTF-16 surrogates -> [ArgumentError]
/// 2. Unicode scalar count > 4096 -> [ItfErrorCode.inputTooLong]
/// 3. Unicode scalar count is 0 or odd -> [ItfErrorCode.invalidLength]
/// 4. Strict ASCII digits ('0'-'9') only -> [ItfErrorCode.invalidCharacter]
String normalizeItf(String data) {
  // ignore: unnecessary_null_comparison
  if (data == null) {
    throw ArgumentError(itfMessageNullInput);
  }

  var scalarCount = 0;
  final units = data.codeUnits;
  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit >= itfHighSurrogateMin && unit <= itfHighSurrogateMax) {
      if (++index >= units.length ||
          units[index] < itfLowSurrogateMin ||
          units[index] > itfLowSurrogateMax) {
        throw ArgumentError(itfMessageInvalidScalars);
      }
    } else if (unit >= itfLowSurrogateMin && unit <= itfLowSurrogateMax) {
      throw ArgumentError(itfMessageInvalidScalars);
    }
    scalarCount++;
  }

  if (scalarCount > itfMaxScalarLimit) {
    throw InvalidItfInputError(ItfErrorCode.inputTooLong);
  }

  if (scalarCount == 0 || scalarCount % itfPairLength != 0) {
    throw InvalidItfInputError(ItfErrorCode.invalidLength);
  }

  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit < itfAsciiZero || unit > itfAsciiNine) {
      throw InvalidItfInputError(ItfErrorCode.invalidCharacter);
    }
  }

  return data;
}

EncodedPair _encodePair(String pair, int sourceIndex) {
  final firstDigit = pair.codeUnitAt(0) - itfAsciiZero;
  final secondDigit = pair.codeUnitAt(1) - itfAsciiZero;
  final barPattern = itfDigitPatterns[firstDigit];
  final spacePattern = itfDigitPatterns[secondDigit];

  final sb = StringBuffer();
  for (var i = 0; i < itfDigitElementCount; i++) {
    sb.write(
      barPattern.codeUnitAt(i) == itfWideMarkerCodeUnit
          ? itfWideBar
          : itfNarrowBar,
    );
    sb.write(
      spacePattern.codeUnitAt(i) == itfWideMarkerCodeUnit
          ? itfWideSpace
          : itfNarrowSpace,
    );
  }

  return EncodedPair(
    pair: pair,
    barPattern: barPattern,
    spacePattern: spacePattern,
    binaryPattern: sb.toString(),
    sourceIndex: sourceIndex,
  );
}

/// Encodes normalized ITF data into pairs of interleaved bar/space elements.
List<EncodedPair> encodeItf(String data) {
  final normalized = normalizeItf(data);
  final pairs = <EncodedPair>[];
  for (var i = 0; i < normalized.length; i += itfPairLength) {
    final pair = normalized.substring(i, i + itfPairLength);
    pairs.add(_encodePair(pair, i ~/ itfPairLength));
  }
  return List.unmodifiable(pairs);
}

/// Internal helper to expand pre-encoded pairs without re-normalizing.
List<Barcode1DRun> _expandEncodedPairs(List<EncodedPair> encodedPairs) {
  final runs = <Barcode1DRun>[];

  runs.addAll(
    BarcodeLayout1DV1.expandBinary(
      itfStartPattern,
      itfLabelStart,
      itfSourceIndexStart,
      itfRoleStart,
    ),
  );

  for (final pair in encodedPairs) {
    runs.addAll(
      BarcodeLayout1DV1.expandBinary(
        pair.binaryPattern,
        pair.pair,
        pair.sourceIndex,
        itfRoleData,
      ),
    );
  }

  runs.addAll(
    BarcodeLayout1DV1.expandBinary(
      itfStopPattern,
      itfLabelStop,
      itfSourceIndexStop,
      itfRoleStop,
    ),
  );

  return List.unmodifiable(runs);
}

/// Expands ITF barcode runs including start pattern, interleaved pairs, and stop pattern.
List<Barcode1DRun> expandItfRuns(String data) {
  final encodedPairs = encodeItf(data);
  return _expandEncodedPairs(encodedPairs);
}

/// Projects the ITF barcode into a 2D vector paint scene with quiet zones and metadata.
PaintScene layoutItf(String data, [Barcode1DSceneOptions? options]) {
  final encodedPairs = encodeItf(data);
  final runs = _expandEncodedPairs(encodedPairs);

  final symbols = <Barcode1DSymbol>[
    const Barcode1DSymbol(
      itfLabelStart,
      itfStartPatternModules,
      itfSourceIndexStart,
      itfRoleStart,
    ),
  ];

  for (final pair in encodedPairs) {
    symbols.add(
      Barcode1DSymbol(
        pair.pair,
        pair.binaryPattern.length,
        pair.sourceIndex,
        itfRoleData,
      ),
    );
  }

  symbols.add(
    const Barcode1DSymbol(
      itfLabelStop,
      itfStopPatternModules,
      itfSourceIndexStop,
      itfRoleStop,
    ),
  );

  final callerMetadata = options?.metadata ?? const {};
  final metadata = Map<String, String>.from(callerMetadata);
  metadata[itfMetadataKeySymbology] = itfMetadataValueSymbology;
  metadata[itfMetadataKeyPairCount] = encodedPairs.length.toString();

  final mergedOptions = Barcode1DSceneOptions(
    moduleWidth: options?.moduleWidth ?? itfDefaultModuleWidth,
    barHeight: options?.barHeight ?? itfDefaultBarHeight,
    foreground: options?.foreground ?? itfDefaultForeground,
    background: options?.background ?? itfDefaultBackground,
    label: options?.label ?? itfDefaultSceneLabel,
    metadata: Map.unmodifiable(metadata),
    humanReadableText: options?.humanReadableText,
    includeHumanReadableText: options?.includeHumanReadableText ?? false,
    symbols: List.unmodifiable(symbols),
  );

  return BarcodeLayout1DV1.projectScene(
    runs,
    itfQuietZoneModules,
    mergedOptions,
  );
}

/// Alias for [layoutItf].
PaintScene drawItf(String data, [Barcode1DSceneOptions? options]) {
  return layoutItf(data, options);
}
