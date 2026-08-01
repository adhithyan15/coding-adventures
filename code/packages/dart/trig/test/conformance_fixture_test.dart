import 'package:coding_adventures_trig/trig.dart' as trig;
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

double evaluateCase(Map<String, dynamic> fixtureCase) {
  final input = fixtureCase['input'] as Map<String, dynamic>;
  switch (fixtureCase['operation']) {
    case 'constant':
      return trig.pi;
    case 'sin':
      return trig.sin(decodeScalar(input['x'] as Map<String, dynamic>));
    case 'cos':
      return trig.cos(decodeScalar(input['x'] as Map<String, dynamic>));
    case 'tan':
      return trig.tan(decodeScalar(input['x'] as Map<String, dynamic>));
    case 'sqrt':
      return trig.sqrt(decodeScalar(input['x'] as Map<String, dynamic>));
    case 'atan':
      return trig.atan(decodeScalar(input['x'] as Map<String, dynamic>));
    case 'atan2':
      return trig.atan2(
        decodeScalar(input['y'] as Map<String, dynamic>),
        decodeScalar(input['x'] as Map<String, dynamic>),
      );
    case 'radians':
      return trig.radians(
        decodeScalar(input['degrees'] as Map<String, dynamic>),
      );
    case 'degrees':
      return trig.degrees(
        decodeScalar(input['radians'] as Map<String, dynamic>),
      );
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
    phy00TrigFixtureBase64,
    expectedSuite: 'phy00-trig',
  );
  final fixtureCases = document['cases'] as List;

  group('PHY00 language-neutral conformance corpus', () {
    for (final entry in fixtureCases) {
      final fixtureCase = entry as Map<String, dynamic>;
      test(fixtureCase['id'] as String, () {
        final expected = fixtureCase['expected'] as Map<String, dynamic>;
        if (expected['outcome'] == 'error') {
          expect(() => evaluateCase(fixtureCase), throwsArgumentError);
          return;
        }
        expect(expected['outcome'], 'value');
        expectValue(evaluateCase(fixtureCase), expected);
      });
    }
  });
}
