import 'package:coding_adventures_wave/wave.dart';
import 'package:test/test.dart';

import '../../../../specs/fixtures/phy00-phy01-v1/dart/generated_cases.dart';
import '../../../../specs/fixtures/phy00-phy01-v1/dart/validated_fixture_loader.dart';

double decodeScalar(Map<String, dynamic> scalar) {
  switch (scalar['kind']) {
    case 'finite':
      return double.parse(scalar['decimal'] as String);
    case 'positive-infinity':
      return double.infinity;
    case 'negative-infinity':
      return double.negativeInfinity;
    case 'nan':
      return double.nan;
    default:
      throw StateError('unknown scalar kind: ${scalar['kind']}');
  }
}

Wave constructWave(Map<String, dynamic> fixtureCase) {
  final input = fixtureCase['input'] as Map<String, dynamic>;
  final parameters = input['wave'] as Map<String, dynamic>;
  return Wave(
    decodeScalar(parameters['amplitude'] as Map<String, dynamic>),
    decodeScalar(parameters['frequency'] as Map<String, dynamic>),
    decodeScalar(parameters['phase'] as Map<String, dynamic>),
  );
}

double? evaluateCase(Map<String, dynamic> fixtureCase) {
  final wave = constructWave(fixtureCase);
  switch (fixtureCase['operation']) {
    case 'construct':
      return null;
    case 'period':
      return wave.period();
    case 'angular-frequency':
      return wave.angularFrequency();
    case 'evaluate':
      final input = fixtureCase['input'] as Map<String, dynamic>;
      return wave.evaluate(decodeScalar(input['time'] as Map<String, dynamic>));
    default:
      throw StateError('unknown operation: ${fixtureCase['operation']}');
  }
}

void expectValue(double actual, Map<String, dynamic> expected) {
  final expectedValue = decodeScalar(expected['value'] as Map<String, dynamic>);
  final comparison = expected['comparison'] as Map<String, dynamic>;
  switch (comparison['kind']) {
    case 'exact':
      if (expectedValue.isNaN) {
        expect(actual.isNaN, isTrue);
      } else if (expectedValue == 0.0) {
        expect(actual, 0.0);
        expect(actual.isNegative, expectedValue.isNegative);
      } else {
        expect(actual, expectedValue);
      }
      return;
    case 'absolute':
      final tolerance = double.parse(comparison['tolerance'] as String);
      expect((actual - expectedValue).abs(), lessThanOrEqualTo(tolerance));
      return;
    case 'relative':
      final tolerance = double.parse(comparison['tolerance'] as String);
      expect(
        (actual - expectedValue).abs(),
        lessThanOrEqualTo(tolerance * expectedValue.abs()),
      );
      return;
    default:
      throw StateError('unknown comparison: ${comparison['kind']}');
  }
}

void main() {
  final document = decodeValidatedFixture(
    phy01WaveFixtureBase64,
    expectedSuite: 'phy01-wave',
  );
  final fixtureCases = document['cases'] as List;

  group('PHY01 language-neutral conformance corpus', () {
    for (final entry in fixtureCases) {
      final fixtureCase = entry as Map<String, dynamic>;
      test(fixtureCase['id'] as String, () {
        final expected = fixtureCase['expected'] as Map<String, dynamic>;
        if (expected['outcome'] == 'error') {
          expect(() => evaluateCase(fixtureCase), throwsArgumentError);
          return;
        }

        final actual = evaluateCase(fixtureCase);
        if (expected['outcome'] == 'accepted') {
          expect(actual, isNull);
        } else if (expected['outcome'] == 'property') {
          expect(
            expected['predicate'],
            'finite-absolute-not-greater-than-amplitude',
          );
          expect(actual, isNotNull);
          expect(actual!.isFinite, isTrue);
          final amplitude = constructWave(fixtureCase).amplitude;
          expect(actual.abs(), lessThanOrEqualTo(amplitude));
        } else {
          expect(expected['outcome'], 'value');
          expectValue(actual!, expected);
        }
      });
    }
  });
}
