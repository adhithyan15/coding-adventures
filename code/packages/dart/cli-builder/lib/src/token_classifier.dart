import 'types.dart';

enum TokenKind {
  endOfFlags,
  longFlag,
  longFlagWithValue,
  singleDashLong,
  shortFlag,
  shortFlagWithValue,
  stackedFlags,
  positional,
  unknownFlag,
}

class TokenEvent {
  const TokenEvent({
    required this.kind,
    this.name,
    this.value,
    this.chars = const <String>[],
    required this.raw,
  });

  final TokenKind kind;
  final String? name;
  final String? value;
  final List<String> chars;
  final String raw;
}

class TokenClassifier {
  TokenClassifier(List<FlagDef> activeFlags) {
    for (final flag in activeFlags) {
      if (flag.longName != null) {
        _longFlags.putIfAbsent(flag.longName!, () => flag);
      }
      if (flag.shortName != null) {
        _shortFlags.putIfAbsent(flag.shortName!, () => flag);
      }
      if (flag.singleDashLong != null) {
        _singleDashLongs.putIfAbsent(flag.singleDashLong!, () => flag);
      }
    }
  }

  final Map<String, FlagDef> _longFlags = <String, FlagDef>{};
  final Map<String, FlagDef> _shortFlags = <String, FlagDef>{};
  final Map<String, FlagDef> _singleDashLongs = <String, FlagDef>{};

  TokenEvent classify(String token) {
    if (token == '-') {
      return TokenEvent(kind: TokenKind.positional, name: token, raw: token);
    }
    if (token == '--') {
      return const TokenEvent(kind: TokenKind.endOfFlags, raw: '--');
    }
    if (token.startsWith('--')) {
      final rest = token.substring(2);
      final equalsIndex = rest.indexOf('=');
      if (equalsIndex >= 0) {
        return TokenEvent(
          kind: TokenKind.longFlagWithValue,
          name: rest.substring(0, equalsIndex),
          value: rest.substring(equalsIndex + 1),
          raw: token,
        );
      }
      if (_longFlags.containsKey(rest)) {
        return TokenEvent(kind: TokenKind.longFlag, name: rest, raw: token);
      }
      return TokenEvent(kind: TokenKind.unknownFlag, name: rest, raw: token);
    }
    if (token.startsWith('-') && token.length >= 2) {
      final rest = token.substring(1);
      if (_singleDashLongs.containsKey(rest)) {
        return TokenEvent(kind: TokenKind.singleDashLong, name: rest, raw: token);
      }

      final first = rest.substring(0, 1);
      final flag = _shortFlags[first];
      if (flag != null) {
        final valueTaking = flag.type != ValueType.boolean && flag.type != ValueType.count;
        if (!valueTaking) {
          if (rest.length == 1) {
            return TokenEvent(kind: TokenKind.shortFlag, name: first, raw: token);
          }
          return _classifyStacked(rest, token);
        }
        if (rest.length == 1) {
          return TokenEvent(kind: TokenKind.shortFlag, name: first, raw: token);
        }

        final suffix = rest.substring(1);
        final suffixAllFlags = suffix.split('').every(_shortFlags.containsKey);
        if (suffixAllFlags) {
          return TokenEvent(kind: TokenKind.unknownFlag, name: first, raw: token);
        }
        return TokenEvent(
          kind: TokenKind.shortFlagWithValue,
          name: first,
          value: suffix,
          raw: token,
        );
      }

      if (rest.length > 1) {
        return _classifyStacked(rest, token);
      }
      return TokenEvent(kind: TokenKind.unknownFlag, name: rest, raw: token);
    }

    return TokenEvent(kind: TokenKind.positional, name: token, raw: token);
  }

  TokenEvent classifyTraditional(String token, Set<String> knownSubcommands) {
    if (token.startsWith('-') || knownSubcommands.contains(token)) {
      return classify(token);
    }
    final stacked = _classifyStacked(token, token);
    if (stacked.kind == TokenKind.stackedFlags) {
      return stacked;
    }
    return TokenEvent(kind: TokenKind.positional, name: token, raw: token);
  }

  FlagDef? lookupByLong(String name) => _longFlags[name];
  FlagDef? lookupByShort(String name) => _shortFlags[name];
  FlagDef? lookupBySingleDashLong(String name) => _singleDashLongs[name];

  List<String> knownLongNames() {
    final result = _longFlags.keys.map((key) => '--$key').toList()..sort();
    return List<String>.unmodifiable(result);
  }

  List<String> knownShortNames() {
    final result = _shortFlags.keys.map((key) => '-$key').toList()..sort();
    return List<String>.unmodifiable(result);
  }

  TokenEvent _classifyStacked(String chars, String raw) {
    final result = <String>[];
    final runes = chars.split('');
    for (var index = 0; index < runes.length; index++) {
      final char = runes[index];
      final flag = _shortFlags[char];
      if (flag == null) {
        return TokenEvent(kind: TokenKind.unknownFlag, name: char, raw: raw);
      }
      final isValueTaking = flag.type != ValueType.boolean && flag.type != ValueType.count;
      if (isValueTaking && index < runes.length - 1) {
        return TokenEvent(kind: TokenKind.unknownFlag, name: char, raw: raw);
      }
      result.add(char);
    }
    return TokenEvent(kind: TokenKind.stackedFlags, chars: result, raw: raw);
  }
}
