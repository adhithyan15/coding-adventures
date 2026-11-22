/// Pure in-memory Verilog preprocessor with zero ambient filesystem authority.
///
/// Handles `define, `undef, `ifdef, `ifndef, `else, `endif, `timescale,
/// and stubbed `include directives while preserving original line numbers.
library;

import 'constants.dart';
import 'errors.dart';

/// Representation of a defined Verilog preprocessor macro.
final class MacroDef {
  MacroDef({
    required this.name,
    required this.body,
    this.params,
  });

  /// Macro identifier name without leading backtick.
  final String name;

  /// Replacement text body.
  final String body;

  /// Parameter names for parameterized macros, or `null` for simple macros.
  final List<String>? params;
}

final RegExp _defineWithParams = RegExp(
  r'^\s*`define\s+([a-zA-Z_]\w*)\(([^)]*)\)\s*(.*)',
);
final RegExp _defineSimple = RegExp(r'^\s*`define\s+([a-zA-Z_]\w*)\s*(.*)');
final RegExp _undefPattern = RegExp(r'^\s*`undef\s+([a-zA-Z_]\w*)');
final RegExp _ifdefPattern = RegExp(r'^\s*`ifdef\s+([a-zA-Z_]\w*)');
final RegExp _ifndefPattern = RegExp(r'^\s*`ifndef\s+([a-zA-Z_]\w*)');
final RegExp _elsePattern = RegExp(r'^\s*`else\b');
final RegExp _endifPattern = RegExp(r'^\s*`endif\b');
final RegExp _includePattern = RegExp(r'^\s*`include\s+"([^"]*)"');
final RegExp _timescalePattern = RegExp(r'^\s*`timescale\b.*');
final RegExp _macroRef = RegExp(r'`([a-zA-Z_]\w*)');
final RegExp _macroCall = RegExp(r'`([a-zA-Z_]\w*)\(');

/// Validates that [source] does not exceed [maxSourceLength] and has valid UTF-16 surrogate pairs.
void validateVerilogSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidVerilogSourceException.sourceTooLarge();
  }
  for (var i = 0; i < source.length; i++) {
    final code = source.codeUnitAt(i);
    if (code >= minSurrogateCodePoint && code <= maxSurrogateCodePoint) {
      if (code <= maxHighSurrogate) {
        if (i + 1 < source.length) {
          final next = source.codeUnitAt(i + 1);
          if (next >= minLowSurrogate && next <= maxLowSurrogate) {
            i++;
            continue;
          }
        }
        throw const InvalidVerilogSourceException.invalidSurrogates();
      } else {
        throw const InvalidVerilogSourceException.invalidSurrogates();
      }
    }
  }
}

/// Preprocesses Verilog source code, expanding macros and resolving conditionals.
///
/// Line numbers are strictly preserved by replacing stripped or excluded lines
/// with empty strings. Operates strictly in memory with zero ambient authority.
String verilogPreprocess(
  String source, {
  Map<String, String>? predefined,
}) {
  validateVerilogSource(source);

  final macros = <String, MacroDef>{};
  if (predefined != null) {
    for (final entry in predefined.entries) {
      macros[entry.key] = MacroDef(name: entry.key, body: entry.value);
    }
  }

  // Condition stack: true = include, false = skip.
  final conditionStack = <bool>[true];
  final lines = source.split('\n');
  final result = <String>[];
  var totalLength = 0;

  for (final line in lines) {
    final active = conditionStack.every((e) => e);

    // Conditional directives (evaluated even when inactive to maintain nesting)
    final ifdefMatch = _ifdefPattern.firstMatch(line);
    if (ifdefMatch != null) {
      if (conditionStack.length >= maxConditionalDepth) {
        throw const InvalidVerilogSourceException.conditionalDepthExceeded();
      }
      final name = ifdefMatch.group(1)!;
      conditionStack.add(active ? macros.containsKey(name) : false);
      result.add('');
      continue;
    }

    final ifndefMatch = _ifndefPattern.firstMatch(line);
    if (ifndefMatch != null) {
      if (conditionStack.length >= maxConditionalDepth) {
        throw const InvalidVerilogSourceException.conditionalDepthExceeded();
      }
      final name = ifndefMatch.group(1)!;
      conditionStack.add(active ? !macros.containsKey(name) : false);
      result.add('');
      continue;
    }

    if (_elsePattern.hasMatch(line)) {
      if (conditionStack.length > 1) {
        final parentActive = conditionStack
            .sublist(0, conditionStack.length - 1)
            .every((e) => e);
        if (parentActive) {
          conditionStack.last = !conditionStack.last;
        }
      }
      result.add('');
      continue;
    }

    if (_endifPattern.hasMatch(line)) {
      if (conditionStack.length > 1) {
        conditionStack.removeLast();
      }
      result.add('');
      continue;
    }

    // Skip inactive lines, preserving line counts
    if (!active) {
      result.add('');
      continue;
    }

    // Macro definitions
    final defineParamsMatch = _defineWithParams.firstMatch(line);
    if (defineParamsMatch != null) {
      final name = defineParamsMatch.group(1)!;
      final rawParams = defineParamsMatch.group(2)!;
      final body = defineParamsMatch.group(3)!.trim();
      final params = rawParams
          .split(',')
          .map((p) => p.trim())
          .where((p) => p.isNotEmpty)
          .toList();
      macros[name] = MacroDef(name: name, body: body, params: params);
      result.add('');
      continue;
    }

    final defineSimpleMatch = _defineSimple.firstMatch(line);
    if (defineSimpleMatch != null) {
      final name = defineSimpleMatch.group(1)!;
      final body = defineSimpleMatch.group(2)!.trim();
      macros[name] = MacroDef(name: name, body: body);
      result.add('');
      continue;
    }

    final undefMatch = _undefPattern.firstMatch(line);
    if (undefMatch != null) {
      final name = undefMatch.group(1)!;
      macros.remove(name);
      result.add('');
      continue;
    }

    // `include stubbing with zero filesystem access
    final includeMatch = _includePattern.firstMatch(line);
    if (includeMatch != null) {
      final filename = includeMatch.group(1)!;
      final stub = '/* `include "$filename" — not resolved */';
      totalLength += stub.length + 1;
      if (totalLength > maxSourceLength) {
        throw const InvalidVerilogSourceException.sourceTooLarge();
      }
      result.add(stub);
      continue;
    }

    // `timescale stripping
    if (_timescalePattern.hasMatch(line)) {
      result.add('');
      continue;
    }

    // Macro expansion
    final expanded = _expandMacros(line, macros);
    totalLength += expanded.length + 1;
    if (totalLength > maxSourceLength) {
      throw const InvalidVerilogSourceException.sourceTooLarge();
    }
    result.add(expanded);
  }

  final output = result.join('\n');
  if (output.length > maxSourceLength) {
    throw const InvalidVerilogSourceException.sourceTooLarge();
  }
  return output;
}

String _expandMacros(String line, Map<String, MacroDef> macros) {
  var pos = 0;
  final sb = StringBuffer();

  while (pos < line.length) {
    final sub = line.substring(pos);
    final match = _macroRef.firstMatch(sub);
    if (match == null) {
      sb.write(sub);
      break;
    }

    // Add text preceding macro reference
    sb.write(sub.substring(0, match.start));
    final name = match.group(1)!;
    final macro = macros[name];

    if (macro == null) {
      // Undefined or system directive: preserve verbatim
      sb.write(match.group(0)!);
      pos += match.end;
      continue;
    }

    if (macro.params != null) {
      final callSub = sub.substring(match.start);
      final callMatch = _macroCall.firstMatch(callSub);
      if (callMatch != null && callMatch.start == 0) {
        final argsStart = pos + match.start + callMatch.end;
        final (argsStr, endPos) = _extractMacroArgs(line, argsStart);
        final args = argsStr.split(',').map((a) => a.trim()).toList();

        var body = macro.body;
        for (var i = 0; i < macro.params!.length; i++) {
          if (i < args.length) {
            body = body.replaceAll(macro.params![i], args[i]);
          }
        }
        sb.write(body);
        pos = endPos;
      } else {
        sb.write(match.group(0)!);
        pos += match.end;
      }
    } else {
      sb.write(macro.body);
      pos += match.end;
    }
  }

  return sb.toString();
}

(String, int) _extractMacroArgs(String text, int start) {
  var depth = 1;
  var pos = start;
  while (pos < text.length && depth > 0) {
    final ch = text[pos];
    if (ch == '(') {
      depth++;
    } else if (ch == ')') {
      depth--;
    }
    pos++;
  }
  return (text.substring(start, pos > 0 ? pos - 1 : start), pos);
}
