import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

import 'constants.dart';
import 'encoded_character.dart';
import 'errors.dart';
import 'messages.dart';

/// Pure Dart implementation of the Code 39 1D barcode symbology.
///
/// Literate design and symbology mechanics (AIM-BC1):
/// 1. Nine-element symbols: every character is nine alternating bar/space
///    widths, each narrow or wide. The canonical projection uses one
///    module for narrow and three for wide.
/// 2. Start/stop framing: the encoder wraps the payload in `*`
///    delimiters. `*` is reserved in input and fails with
///    `reserved-character`.
/// 3. Inter-character gaps: one narrow space separates symbols, except
///    after the final stop symbol.
/// 4. ASCII-only folding: lowercase `a`-`z` folds to uppercase. Nothing
///    else folds — notably not `ß` or locale-sensitive cases. The
///    reference folds with full Unicode `to_uppercase`; the spec and
///    corpus require ASCII-only, so the port follows them.
/// 5. Empty input is valid (bare start/stop pair). No checksum exists;
///    v1 never computes or appends modulo-43.
/// 6. Validation precedence: `input-too-long` (over 4096 scalars) first,
///    then a single left-to-right scan reporting `reserved-character`
///    for `*` at its position before the generic `invalid-character`.
///    Lone surrogate halves are rejected before counting to prevent
///    truncation and smuggling.

/// Validates host-string shape and returns the scalar list.
///
/// Throws [ArgumentError] on null or unpaired UTF-16 surrogates
/// (outside the v1 API domain, so no stable identifier).
List<int> _scalars(String data) {
  // ignore: unnecessary_null_comparison
  if (data == null) {
    throw ArgumentError(code39MessageNullInput);
  }
  final result = <int>[];
  final units = data.codeUnits;
  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit >= code39HighSurrogateMin && unit <= code39HighSurrogateMax) {
      if (++index >= units.length ||
          units[index] < code39LowSurrogateMin ||
          units[index] > code39LowSurrogateMax) {
        throw ArgumentError(code39MessageInvalidScalars);
      }
      result.add(
        0x10000 +
            ((unit - code39HighSurrogateMin) << 10) +
            (units[index] - code39LowSurrogateMin),
      );
    } else if (unit >= code39LowSurrogateMin && unit <= code39LowSurrogateMax) {
      throw ArgumentError(code39MessageInvalidScalars);
    } else {
      result.add(unit);
    }
  }
  return result;
}

/// Folds one scalar: ASCII `a`-`z` to uppercase, everything else kept.
int _foldAscii(int scalar) {
  if (scalar >= code39AsciiLowerA && scalar <= code39AsciiLowerZ) {
    return scalar - code39AsciiCaseDelta;
  }
  return scalar;
}

/// Validates and normalizes Code 39 payload data.
///
/// Folds ASCII lowercase, rejects `*` and out-of-alphabet characters
/// left to right, and returns the normalized payload.
String normalizeCode39(String data) {
  final scalars = _scalars(data);
  if (scalars.length > code39MaxScalarLimit) {
    throw InvalidCode39InputError(Code39ErrorCode.inputTooLong);
  }
  final folded = scalars.map(_foldAscii).toList();
  for (final scalar in folded) {
    final value = String.fromCharCode(scalar);
    if (value == code39StartStop) {
      throw InvalidCode39InputError(Code39ErrorCode.reservedCharacter);
    }
    if (!code39Patterns.containsKey(value)) {
      throw InvalidCode39InputError(Code39ErrorCode.invalidCharacter);
    }
  }
  return String.fromCharCodes(folded);
}

/// Encodes one Code 39 character (folded form) to its width pattern.
EncodedCharacter encodeCode39Char(String ch) {
  final pattern = code39Patterns[ch];
  if (pattern == null) {
    throw InvalidCode39InputError(Code39ErrorCode.invalidCharacter);
  }
  return EncodedCharacter(
    ch: ch,
    isStartStop: ch == code39StartStop,
    pattern: _widthPattern(pattern),
  );
}

/// Maps an element pattern to narrow/wide markers: uppercase is wide.
String _widthPattern(String pattern) {
  final buffer = StringBuffer();
  for (var i = 0; i < pattern.length; i++) {
    final unit = pattern.codeUnitAt(i);
    final isUpper = unit >= 0x41 && unit <= 0x5a;
    buffer.write(isUpper ? 'W' : 'N');
  }
  return buffer.toString();
}

/// Encodes normalized input, wrapped in start/stop delimiters.
List<EncodedCharacter> encodeCode39(String data) {
  final normalized = normalizeCode39(data);
  final framed = '$code39StartStop$normalized$code39StartStop';
  // Every scalar is validated ASCII-or-folded here, so UTF-16 indexing
  // coincides with scalar indexing... except folded output can still
  // hold astral scalars? No: every folded scalar passed the alphabet
  // scan, and the alphabet is ASCII-only. Safe to index by scalar via
  // runes for clarity.
  return List.unmodifiable(
    framed.runes.map((rune) => encodeCode39Char(String.fromCharCode(rune))),
  );
}

/// Expands normalized input into barcode runs with inter-character gaps.
List<Barcode1DRun> expandCode39Runs(String data) {
  final encoded = encodeCode39(data);
  final runs = <Barcode1DRun>[];

  for (var sourceIndex = 0; sourceIndex < encoded.length; sourceIndex++) {
    final entry = encoded[sourceIndex];
    final role = _runRoleFor(sourceIndex, encoded.length, entry);
    for (var elementIndex = 0;
        elementIndex < entry.pattern.length;
        elementIndex++) {
      runs.add(
        Barcode1DRun(
          elementIndex.isEven ? 'bar' : 'space',
          entry.pattern[elementIndex] == 'W'
              ? code39WideModules
              : code39NarrowModules,
          entry.ch,
          sourceIndex,
          role,
        ),
      );
    }

    if (sourceIndex < encoded.length - 1) {
      runs.add(
        Barcode1DRun(
          'space',
          code39NarrowModules,
          entry.ch,
          sourceIndex,
          'inter-character-gap',
        ),
      );
    }
  }

  return List.unmodifiable(runs);
}

String _runRoleFor(
  int sourceIndex,
  int encodedLength,
  EncodedCharacter entry,
) {
  if (!entry.isStartStop) {
    return 'data';
  }
  if (sourceIndex == 0) {
    return 'start';
  }
  if (sourceIndex == encodedLength - 1) {
    return 'stop';
  }
  return 'guard';
}

/// Projects the Code 39 barcode into a 2D vector paint scene with quiet
/// zones and metadata.
///
/// Symbols are inferred from run attribution (the reference passes no
/// explicit descriptors). The reference sets `human_readable_text` on
/// the paint options, which — with glyph rendering disabled (the
/// default) — manifests only as the `humanReadableText` scene-metadata
/// key. The portable lane entry rejects the text *option*, so the port
/// writes the metadata key directly: identical observable scenes
/// without touching font authority. Passing
/// `includeHumanReadableText: true` (or a text option) reaches the
/// lane's specified `human-readable-text-unsupported` error, exactly
/// like the sibling lanes.
PaintScene layoutCode39(String data, [Barcode1DSceneOptions? options]) {
  final normalized = normalizeCode39(data);
  final runs = expandCode39Runs(normalized);

  final callerMetadata = options?.metadata ?? const {};
  final metadata = Map<String, String>.from(callerMetadata);
  metadata['symbology'] = 'code39';
  metadata['encodedText'] = normalized;
  metadata['humanReadableText'] = normalized;

  final mergedOptions = Barcode1DSceneOptions(
    moduleWidth: options?.moduleWidth ?? 4,
    barHeight: options?.barHeight ?? 120,
    foreground: options?.foreground ?? '#000000',
    background: options?.background ?? '#ffffff',
    label: options?.label ??
        (normalized.isEmpty
            ? 'Code 39 barcode'
            : 'Code 39 barcode for $normalized'),
    metadata: Map.unmodifiable(metadata),
    humanReadableText: options?.humanReadableText,
    includeHumanReadableText: options?.includeHumanReadableText ?? false,
  );

  return BarcodeLayout1DV1.projectScene(
    runs,
    code39QuietZoneModules,
    mergedOptions,
  );
}

/// Alias for [layoutCode39].
PaintScene drawCode39(String data, [Barcode1DSceneOptions? options]) {
  return layoutCode39(data, options);
}
