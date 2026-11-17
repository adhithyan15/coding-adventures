// conformance_test.dart — Conformance tests for Hamurabi against cases.json
import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_hamurabi/coding_adventures_hamurabi.dart';
import 'package:test/test.dart';

File _findCasesJson() {
  const candidates = [
    '../../../specs/fixtures/hamurabi-v1/cases.json',
    '../../specs/fixtures/hamurabi-v1/cases.json',
    'code/specs/fixtures/hamurabi-v1/cases.json',
  ];
  for (final path in candidates) {
    final f = File(path);
    if (f.existsSync()) return f;
  }
  throw StateError('Could not find cases.json in candidate paths');
}

void main() {
  group('Hamurabi Conformance Suite (cases.json)', () {
    final casesFile = _findCasesJson();
    final rawText = casesFile.readAsStringSync();
    // Quote 64-bit unsigned integers to preserve precision under Dart VM 64-bit signed int limit
    final transformed = rawText.replaceAllMapped(
      RegExp(r'"(rng_state|rng_inc|seed)":\s*(\d+)'),
      (m) => '"${m[1]}": "${m[2]}"',
    );
    final Map<String, dynamic> doc = jsonDecode(transformed) as Map<String, dynamic>;
    final List<dynamic> cases = doc['cases'] as List<dynamic>;

    test('corpus loaded and has at least 31 cases', () {
      expect(cases.length, greaterThanOrEqualTo(31));
    });

    for (final rawCase in cases) {
      final caseMap = rawCase as Map<String, dynamic>;
      final id = caseMap['id'] as String;
      final seedStr = caseMap['seed']?.toString();
      final seed = seedStr != null ? BigInt.parse(seedStr).toUnsigned(64) : null;
      final initialJson = caseMap['initial_state'] as Map<String, dynamic>;
      final initialState = HamurabiState.fromJson(initialJson);

      if (caseMap.containsKey('turns')) {
        test('simulation case: $id', () {
          if (seed != null && initialState.year == 1) {
            final generated = newGame(seed);
            expect(generated, equals(initialState), reason: '$id newGame mismatch');
          }

          var currentState = initialState;
          final turns = caseMap['turns'] as List<dynamic>;
          for (final rawTurn in turns) {
            final turnMap = rawTurn as Map<String, dynamic>;
            final order = TurnOrder.fromJson(turnMap['order'] as Map<String, dynamic>);
            final expectedState = HamurabiState.fromJson(turnMap['expected_state'] as Map<String, dynamic>);
            final expectedReport = turnMap['expected_report'] as String;

            final result = stepYear(currentState, order);
            expect(result.isOk, isTrue, reason: '$id stepYear failed on turn ${turnMap["turn"]}');
            final nextState = result.unwrap;
            expect(nextState, equals(expectedState), reason: '$id state mismatch on turn ${turnMap["turn"]}');

            final report = formatReport(nextState);
            expect(report, equals(expectedReport), reason: '$id report mismatch on turn ${turnMap["turn"]}');

            currentState = nextState;
          }
        });
      } else if (caseMap.containsKey('expected_error')) {
        test('refusal case: $id', () {
          final order = TurnOrder.fromJson(caseMap['order'] as Map<String, dynamic>);
          final expectedErrCode = caseMap['expected_error'] as String;
          final expectedErr = HamurabiError.fromCode(expectedErrCode);

          final valErr = validateOrder(initialState, order);
          expect(valErr, equals(expectedErr), reason: '$id validateOrder error mismatch');

          final stepRes = stepYear(initialState, order);
          expect(stepRes.isErr, isTrue, reason: '$id stepYear should fail with Err');
          expect(stepRes.unwrapErr, equals(expectedErr), reason: '$id stepYear error mismatch');
        });
      }
    }
  });
}
