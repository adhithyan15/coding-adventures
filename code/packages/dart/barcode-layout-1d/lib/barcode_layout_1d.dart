/// Pure integer geometry for the language-neutral barcode-layout-1d v1 core.
library;

import 'dart:convert';

import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

const _maxPattern = 65567;
const _maxRuns = 40979;
const _maxContent = 65567;
const _maxQuiet = 4096;
const _roles = {
  'data',
  'start',
  'stop',
  'guard',
  'check',
  'inter-character-gap',
};

final class Barcode1DV1Error extends ArgumentError {
  final String errorId;
  Barcode1DV1Error(this.errorId) : super(errorId);
}

final class Barcode1DRun {
  final String color;
  final int modules;
  final String sourceLabel;
  final int sourceIndex;
  final String role;
  const Barcode1DRun(
    this.color,
    this.modules,
    this.sourceLabel,
    this.sourceIndex,
    this.role,
  );
}

final class Barcode1DSymbol {
  final String label;
  final int modules;
  final int sourceIndex;
  final String role;
  const Barcode1DSymbol(this.label, this.modules, this.sourceIndex, this.role);
}

final class Barcode1DSymbolLayout {
  final String label;
  final int startModule;
  final int endModule;
  final int sourceIndex;
  final String role;
  const Barcode1DSymbolLayout(
    this.label,
    this.startModule,
    this.endModule,
    this.sourceIndex,
    this.role,
  );
}

final class Barcode1DLayout {
  final int leftQuietZoneModules;
  final int rightQuietZoneModules;
  final int contentModules;
  final int totalModules;
  final List<Barcode1DSymbolLayout> symbolLayouts;
  const Barcode1DLayout(
    this.leftQuietZoneModules,
    this.rightQuietZoneModules,
    this.contentModules,
    this.totalModules,
    this.symbolLayouts,
  );
}

final class Barcode1DSceneOptions {
  final int moduleWidth;
  final int barHeight;
  final String foreground;
  final String background;
  final String label;
  final Map<String, String> metadata;
  final String? humanReadableText;
  final bool includeHumanReadableText;
  final List<Barcode1DSymbol>? symbols;
  const Barcode1DSceneOptions({
    this.moduleWidth = 4,
    this.barHeight = 120,
    this.foreground = '#000000',
    this.background = '#ffffff',
    this.label = '1D barcode',
    this.metadata = const {},
    this.humanReadableText,
    this.includeHumanReadableText = false,
    this.symbols,
  });
}

/// The portable entry point has no native font, backend, IO or host authority.
abstract final class BarcodeLayout1DV1 {
  static Never _fail(String id) => throw Barcode1DV1Error(id);

  static int _scalars(String value, String error) {
    var count = 0;
    final units = value.codeUnits;
    for (var index = 0; index < units.length; index++) {
      final unit = units[index];
      if (unit >= 0xd800 && unit <= 0xdbff) {
        if (++index >= units.length ||
            units[index] < 0xdc00 ||
            units[index] > 0xdfff) {
          _fail(error);
        }
      } else if (unit >= 0xdc00 && unit <= 0xdfff) {
        _fail(error);
      }
      count++;
    }
    return count;
  }

  static void _source(String label, int index, String role) {
    if (_scalars(label, 'invalid-source-attribution') > 4096 ||
        index < -2147483648 ||
        index > 2147483647 ||
        !_roles.contains(role)) {
      _fail('invalid-source-attribution');
    }
  }

  static List<Barcode1DRun> expandBinary(
    String pattern,
    String sourceLabel,
    int sourceIndex,
    String role,
  ) {
    final length = _scalars(pattern, 'invalid-binary-token');
    if (length > _maxPattern) _fail('pattern-too-long');
    if (length == 0) _fail('empty-pattern');
    if (pattern.codeUnits.any((unit) => unit != 48 && unit != 49)) {
      _fail('invalid-binary-token');
    }
    _source(sourceLabel, sourceIndex, role);
    final result = <Barcode1DRun>[];
    var current = pattern[0];
    var count = 1;
    for (var index = 1; index < pattern.length; index++) {
      final token = pattern[index];
      if (token == current) {
        count++;
        continue;
      }
      if (result.length >= _maxRuns) _fail('too-many-runs');
      result.add(
        Barcode1DRun(
          current == '1' ? 'bar' : 'space',
          count,
          sourceLabel,
          sourceIndex,
          role,
        ),
      );
      current = token;
      count = 1;
    }
    if (result.length >= _maxRuns) _fail('too-many-runs');
    result.add(
      Barcode1DRun(
        current == '1' ? 'bar' : 'space',
        count,
        sourceLabel,
        sourceIndex,
        role,
      ),
    );
    return List.unmodifiable(result);
  }

  static List<Barcode1DRun> expandWidth(
    String pattern,
    String narrowMarker,
    String wideMarker,
    int narrowModules,
    int wideModules,
    String startingColor,
    String sourceLabel,
    int sourceIndex,
    String role,
  ) {
    final length = _scalars(pattern, 'invalid-width-token');
    if (length > _maxPattern) _fail('pattern-too-long');
    if (length == 0) _fail('empty-pattern');
    if (_scalars(narrowMarker, 'invalid-marker-configuration') != 1 ||
        _scalars(wideMarker, 'invalid-marker-configuration') != 1 ||
        narrowMarker == wideMarker) {
      _fail('invalid-marker-configuration');
    }
    final narrow = narrowMarker.runes.single;
    final wide = wideMarker.runes.single;
    final tokens = pattern.runes.toList();
    if (tokens.any((token) => token != narrow && token != wide)) {
      _fail('invalid-width-token');
    }
    _source(sourceLabel, sourceIndex, role);
    if (narrowModules <= 0 || wideModules <= 0) _fail('invalid-module-count');
    if (startingColor != 'bar' && startingColor != 'space') {
      _fail('invalid-marker-configuration');
    }
    if (length > _maxRuns) _fail('too-many-runs');
    final result = <Barcode1DRun>[];
    var content = 0;
    for (var index = 0; index < tokens.length; index++) {
      final modules = tokens[index] == wide ? wideModules : narrowModules;
      if (modules > _maxContent - content) _fail('content-too-wide');
      content += modules;
      final color = index.isEven
          ? startingColor
          : (startingColor == 'bar' ? 'space' : 'bar');
      result.add(Barcode1DRun(color, modules, sourceLabel, sourceIndex, role));
    }
    return List.unmodifiable(result);
  }

  static Barcode1DLayout computeLayout(
    List<Barcode1DRun> runs,
    int quietZoneModules, [
    List<Barcode1DSymbol>? symbols,
  ]) {
    if (runs.length > _maxRuns) _fail('too-many-runs');
    var content = 0;
    String? previous;
    for (final run in runs) {
      if ((run.color != 'bar' && run.color != 'space') ||
          !_roles.contains(run.role)) _fail('invalid-source-attribution');
      if (run.modules <= 0) _fail('invalid-module-count');
      _source(run.sourceLabel, run.sourceIndex, run.role);
      if (run.modules > _maxContent - content) _fail('content-too-wide');
      content += run.modules;
      if (run.color == previous) _fail('non-alternating-runs');
      previous = run.color;
    }
    if (quietZoneModules < 1 || quietZoneModules > _maxQuiet) {
      _fail('invalid-quiet-zone');
    }
    final layouts = <Barcode1DSymbolLayout>[];
    if (symbols != null) {
      if (symbols.length > _maxRuns) _fail('too-many-symbols');
      var cursor = 0;
      for (final symbol in symbols) {
        if (symbol.modules <= 0) _fail('invalid-module-count');
        _source(symbol.label, symbol.sourceIndex, symbol.role);
        if (symbol.role == 'inter-character-gap') {
          _fail('invalid-source-attribution');
        }
        if (symbol.modules > _maxContent - cursor) {
          _fail('symbol-width-mismatch');
        }
        final end = cursor + symbol.modules;
        layouts.add(
          Barcode1DSymbolLayout(
            symbol.label,
            cursor,
            end,
            symbol.sourceIndex,
            symbol.role,
          ),
        );
        cursor = end;
      }
      if (cursor != content) _fail('symbol-width-mismatch');
    } else {
      var cursor = 0;
      Barcode1DRun? active;
      var start = 0;
      for (final run in runs) {
        if (run.role != 'inter-character-gap') {
          final changed = active == null ||
              active.sourceLabel != run.sourceLabel ||
              active.sourceIndex != run.sourceIndex ||
              active.role != run.role;
          if (changed) {
            if (active != null) {
              layouts.add(
                Barcode1DSymbolLayout(
                  active.sourceLabel,
                  start,
                  cursor,
                  active.sourceIndex,
                  active.role,
                ),
              );
            }
            active = run;
            start = cursor;
          }
        }
        cursor += run.modules;
      }
      if (active != null) {
        layouts.add(
          Barcode1DSymbolLayout(
            active.sourceLabel,
            start,
            cursor,
            active.sourceIndex,
            active.role,
          ),
        );
      }
      if (layouts.length > _maxRuns) _fail('too-many-symbols');
    }
    return Barcode1DLayout(
      quietZoneModules,
      quietZoneModules,
      content,
      quietZoneModules + content + quietZoneModules,
      List.unmodifiable(layouts),
    );
  }

  static PaintScene projectScene(
    List<Barcode1DRun> runs,
    int quietZoneModules, [
    Barcode1DSceneOptions options = const Barcode1DSceneOptions(),
  ]) {
    if (options.includeHumanReadableText || options.humanReadableText != null) {
      _fail('human-readable-text-unsupported');
    }
    if (options.moduleWidth < 1 ||
        options.moduleWidth > 8192 ||
        options.barHeight < 1 ||
        options.barHeight > 8192 ||
        _scalars(options.foreground, 'invalid-render-config') > 128 ||
        _scalars(options.background, 'invalid-render-config') > 128) {
      _fail('invalid-render-config');
    }
    final layout = computeLayout(runs, quietZoneModules, options.symbols);
    if (options.metadata.length > 64) _fail('metadata-too-large');
    final metadata = <String, String>{};
    var bytes = 0;
    for (final entry in options.metadata.entries) {
      if (_scalars(entry.key, 'metadata-too-large') > 128 ||
          _scalars(entry.value, 'metadata-too-large') > 4096) {
        _fail('metadata-too-large');
      }
      bytes += utf8.encode(entry.key).length + utf8.encode(entry.value).length;
      if (bytes > 65536) _fail('metadata-too-large');
      metadata[entry.key] = entry.value;
    }
    if (_scalars(options.label, 'metadata-too-large') > 4096) {
      _fail('metadata-too-large');
    }
    final rectangles = <PaintInstruction>[];
    var cursor = quietZoneModules;
    for (final run in runs) {
      final end = cursor + run.modules;
      if (run.color == 'bar') {
        rectangles.add(
          PaintRect(
            x: cursor * options.moduleWidth,
            y: 0,
            width: run.modules * options.moduleWidth,
            height: options.barHeight,
            fill: options.foreground,
            metadata: Map.unmodifiable({
              'sourceLabel': run.sourceLabel,
              'sourceIndex': run.sourceIndex.toString(),
              'role': run.role,
              'moduleStart': cursor.toString(),
              'moduleEnd': end.toString(),
            }),
          ),
        );
      }
      cursor = end;
    }
    final sceneWidth = layout.totalModules * options.moduleWidth;
    metadata.addAll({
      'label': options.label,
      'leftQuietZoneModules': layout.leftQuietZoneModules.toString(),
      'rightQuietZoneModules': layout.rightQuietZoneModules.toString(),
      'contentModules': layout.contentModules.toString(),
      'totalModules': layout.totalModules.toString(),
      'moduleWidthPx': options.moduleWidth.toString(),
      'barHeightPx': options.barHeight.toString(),
      'sceneWidthPx': sceneWidth.toString(),
      'sceneHeightPx': options.barHeight.toString(),
      'symbolCount': layout.symbolLayouts.length.toString(),
    });
    return PaintScene(
      width: sceneWidth,
      height: options.barHeight,
      background: options.background,
      instructions: List.unmodifiable(rectangles),
      metadata: Map.unmodifiable(metadata),
    );
  }
}
