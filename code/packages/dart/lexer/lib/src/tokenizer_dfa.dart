import 'package:coding_adventures_state_machine/state_machine.dart';

String classifyChar(String? char) {
  if (char == null) {
    return 'eof';
  }
  if (char == ' ' || char == '\t' || char == '\r') {
    return 'whitespace';
  }
  if (char == '\n') {
    return 'newline';
  }
  if (char.compareTo('0') >= 0 && char.compareTo('9') <= 0) {
    return 'digit';
  }
  if ((char.compareTo('a') >= 0 && char.compareTo('z') <= 0) ||
      (char.compareTo('A') >= 0 && char.compareTo('Z') <= 0)) {
    return 'alpha';
  }
  if (char == '_') {
    return 'underscore';
  }
  if (char == '"') {
    return 'quote';
  }
  if (char == '=') {
    return 'equals';
  }
  if ('+-*/'.contains(char)) {
    return 'operator';
  }

  const delimiters = <String, String>{
    '(': 'open_paren',
    ')': 'close_paren',
    ',': 'comma',
    ':': 'colon',
    ';': 'semicolon',
    '{': 'open_brace',
    '}': 'close_brace',
    '[': 'open_bracket',
    ']': 'close_bracket',
    '.': 'dot',
    '!': 'bang',
  };
  return delimiters[char] ?? 'other';
}

DFA newTokenizerDfa() {
  final states = <String>{
    'start',
    'in_number',
    'in_name',
    'in_string',
    'in_operator',
    'in_equals',
    'at_newline',
    'at_whitespace',
    'done',
    'error',
  };
  final alphabet = <String>{
    'digit',
    'alpha',
    'underscore',
    'quote',
    'newline',
    'whitespace',
    'operator',
    'equals',
    'open_paren',
    'close_paren',
    'comma',
    'colon',
    'semicolon',
    'open_brace',
    'close_brace',
    'open_bracket',
    'close_bracket',
    'dot',
    'bang',
    'eof',
    'other',
  };

  final startDispatch = <String, String>{
    'digit': 'in_number',
    'alpha': 'in_name',
    'underscore': 'in_name',
    'quote': 'in_string',
    'newline': 'at_newline',
    'whitespace': 'at_whitespace',
    'operator': 'in_operator',
    'equals': 'in_equals',
    'open_paren': 'in_operator',
    'close_paren': 'in_operator',
    'comma': 'in_operator',
    'colon': 'in_operator',
    'semicolon': 'in_operator',
    'open_brace': 'in_operator',
    'close_brace': 'in_operator',
    'open_bracket': 'in_operator',
    'close_bracket': 'in_operator',
    'dot': 'in_operator',
    'bang': 'in_operator',
    'eof': 'done',
    'other': 'error',
  };

  final transitions = <String, String>{};
  for (final entry in startDispatch.entries) {
    transitions[transitionKey('start', entry.key)] = entry.value;
  }

  const handlers = <String>[
    'in_number',
    'in_name',
    'in_string',
    'in_operator',
    'in_equals',
    'at_newline',
    'at_whitespace',
  ];
  for (final handler in handlers) {
    for (final symbol in alphabet) {
      transitions[transitionKey(handler, symbol)] = 'start';
    }
  }

  for (final terminal in const <String>['done', 'error']) {
    for (final symbol in alphabet) {
      transitions[transitionKey(terminal, symbol)] = terminal;
    }
  }

  return DFA(
    states,
    alphabet,
    transitions,
    'start',
    <String>{'done'},
  );
}

