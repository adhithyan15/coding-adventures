import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

import 'constants.dart';
import 'encoded_symbol.dart';
import 'errors.dart';
import 'messages.dart';

/// Pure Dart implementation of the Codabar 1D barcode symbology.
///
/// Literate design and symbology mechanics (AIM-BC3):
/// 1. Guards frame the body: input may already carry `A`/`B`/`C`/`D` on
///    both edges, or an unguarded body takes `start`/`stop` options that
///    default independently to `A`. Empty body input normalizes to `AA`.
/// 2. Guard form is strict: a lone edge guard, a single guard, an
///    interior guard, or a non-guard option fails with `invalid-guard`;
///    guarded input whose explicit options disagree fails with
///    `guard-conflict` — before the body alphabet is scanned.
/// 3. The body alphabet is `0-9`, `-`, `$`, `:`, `/`, `.`, `+`. ASCII
///    lowercase folds to uppercase in the input and in option values;
///    nothing else folds.
/// 4. Each symbol expands from its binary table pattern; a one-module
///    space separates symbols. Patterns canonically project 2:1.
/// 5. Lone surrogate halves are rejected before counting to prevent
///    truncation and smuggling. The 4096-scalar cap precedes all
///    symbol work.

/// Validates host-string shape and returns the scalar list.
///
/// Throws [ArgumentError] on null or unpaired UTF-16 surrogates
/// (outside the v1 API domain, so no stable identifier).
List<int> _scalars(String data) {
  // ignore: unnecessary_null_comparison
  if (data == null) {
    throw ArgumentError(codabarMessageNullInput);
  }
  final result = <int>[];
  final units = data.codeUnits;
  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit >= codabarHighSurrogateMin && unit <= codabarHighSurrogateMax) {
      if (++index >= units.length ||
          units[index] < codabarLowSurrogateMin ||
          units[index] > codabarLowSurrogateMax) {
        throw ArgumentError(codabarMessageInvalidScalars);
      }
      result.add(
        0x10000 +
            ((unit - codabarHighSurrogateMin) << 10) +
            (units[index] - codabarLowSurrogateMin),
      );
    } else if (unit >= codabarLowSurrogateMin &&
        unit <= codabarLowSurrogateMax) {
      throw ArgumentError(codabarMessageInvalidScalars);
    } else {
      result.add(unit);
    }
  }
  return result;
}

/// Folds one scalar: ASCII `a`-`z` to uppercase, everything else kept.
int _foldAscii(int scalar) {
  if (scalar >= codabarAsciiLowerA && scalar <= codabarAsciiLowerZ) {
    return scalar - codabarAsciiCaseDelta;
  }
  return scalar;
}

bool _isGuard(String ch) => codabarGuards.contains(ch);

/// Folds and validates a guard option; null stays null (default later).
///
/// Throws [InvalidCodabarInputError] with `invalid-guard` for a present
/// option that is not exactly one guard character after folding.
String? _foldOption(String? option) {
  if (option == null) return null;
  final scalars = _scalars(option);
  if (scalars.length != 1) {
    throw InvalidCodabarInputError(CodabarErrorCode.invalidGuard);
  }
  final folded = String.fromCharCode(_foldAscii(scalars.single));
  if (!_isGuard(folded)) {
    throw InvalidCodabarInputError(CodabarErrorCode.invalidGuard);
  }
  return folded;
}

/// Validates and normalizes Codabar input to a guarded string.
///
/// Unguarded bodies gain `start`/`stop` guards (each defaulting to `A`
/// independently); already-guarded input must agree with any explicit
/// options.
String normalizeCodabar(String data, {String? start, String? stop}) {
  final scalars = _scalars(data);
  if (scalars.length > codabarMaxScalarLimit) {
    throw InvalidCodabarInputError(CodabarErrorCode.inputTooLong);
  }
  final folded =
      String.fromCharCodes(scalars.map(_foldAscii).toList(growable: false));

  // Guard form first: fully guarded, one-sided, or unguarded.
  final hasEdges = folded.length >= 2;
  final firstIsGuard = folded.isNotEmpty && _isGuard(folded[0]);
  final lastIsGuard = folded.isNotEmpty && _isGuard(folded[folded.length - 1]);
  if ((firstIsGuard || lastIsGuard) &&
      !(hasEdges && firstIsGuard && lastIsGuard)) {
    throw InvalidCodabarInputError(CodabarErrorCode.invalidGuard);
  }

  if (hasEdges && firstIsGuard && lastIsGuard) {
    // Guarded input: options must agree, before the body scan.
    final wantStart = _foldOption(start);
    final wantStop = _foldOption(stop);
    if ((wantStart != null && wantStart != folded[0]) ||
        (wantStop != null && wantStop != folded[folded.length - 1])) {
      throw InvalidCodabarInputError(CodabarErrorCode.guardConflict);
    }
    _assertBody(folded.substring(1, folded.length - 1));
    return folded;
  }

  // Unguarded input: options, then the body scan.
  final useStart = _foldOption(start) ?? codabarDefaultGuard;
  final useStop = _foldOption(stop) ?? codabarDefaultGuard;
  _assertBody(folded);
  return '$useStart$folded$useStop';
}

/// Scans a body left to right: guards and out-of-alphabet characters
/// fail at the first offender's position.
void _assertBody(String body) {
  for (var i = 0; i < body.length; i++) {
    final ch = body[i];
    if (_isGuard(ch)) {
      throw InvalidCodabarInputError(CodabarErrorCode.invalidGuard);
    }
    if (!codabarPatterns.containsKey(ch)) {
      throw InvalidCodabarInputError(CodabarErrorCode.invalidCharacter);
    }
  }
}

/// Encodes guarded input into per-character binary patterns.
List<EncodedCodabarSymbol> encodeCodabar(
  String data, {
  String? start,
  String? stop,
}) {
  final normalized = normalizeCodabar(data, start: start, stop: stop);
  final symbols = <EncodedCodabarSymbol>[];
  for (var index = 0; index < normalized.length; index++) {
    final ch = normalized[index];
    symbols.add(
      EncodedCodabarSymbol(
        ch: ch,
        pattern: codabarPatterns[ch]!,
        sourceIndex: index,
        role: index == 0
            ? 'start'
            : (index == normalized.length - 1 ? 'stop' : 'data'),
      ),
    );
  }
  return List.unmodifiable(symbols);
}

/// Expands guarded input into barcode runs with inter-character gaps.
List<Barcode1DRun> expandCodabarRuns(
  String data, {
  String? start,
  String? stop,
}) {
  final encoded = encodeCodabar(data, start: start, stop: stop);
  final runs = <Barcode1DRun>[];

  for (var index = 0; index < encoded.length; index++) {
    final entry = encoded[index];
    runs.addAll(
      BarcodeLayout1DV1.expandBinary(
        entry.pattern,
        entry.ch,
        entry.sourceIndex,
        entry.role,
      ),
    );
    if (index < encoded.length - 1) {
      runs.add(
        Barcode1DRun(
          'space',
          1,
          entry.ch,
          entry.sourceIndex,
          'inter-character-gap',
        ),
      );
    }
  }

  return List.unmodifiable(runs);
}

/// Projects the Codabar barcode into a 2D vector paint scene with quiet
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
PaintScene layoutCodabar(
  String data, {
  String? start,
  String? stop,
  Barcode1DSceneOptions? options,
}) {
  final normalized = normalizeCodabar(data, start: start, stop: stop);
  final runs = expandCodabarRuns(normalized);

  final callerMetadata = options?.metadata ?? const {};
  final metadata = Map<String, String>.from(callerMetadata);
  metadata['symbology'] = 'codabar';
  metadata['start'] = normalized[0];
  metadata['stop'] = normalized[normalized.length - 1];
  metadata['humanReadableText'] = normalized;

  final mergedOptions = Barcode1DSceneOptions(
    moduleWidth: options?.moduleWidth ?? 4,
    barHeight: options?.barHeight ?? 120,
    foreground: options?.foreground ?? '#000000',
    background: options?.background ?? '#ffffff',
    label: options?.label ?? 'Codabar barcode for $normalized',
    metadata: Map.unmodifiable(metadata),
    humanReadableText: options?.humanReadableText,
    includeHumanReadableText: options?.includeHumanReadableText ?? false,
  );

  return BarcodeLayout1DV1.projectScene(
    runs,
    codabarQuietZoneModules,
    mergedOptions,
  );
}

/// Alias for [layoutCodabar].
PaintScene drawCodabar(
  String data, {
  String? start,
  String? stop,
  Barcode1DSceneOptions? options,
}) {
  return layoutCodabar(data, start: start, stop: stop, options: options);
}
