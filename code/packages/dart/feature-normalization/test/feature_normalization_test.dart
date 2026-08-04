import 'package:coding_adventures_feature_normalization/feature_normalization.dart';
import 'package:test/test.dart';

const tolerance = 1e-9;

void main() {
  final rows = <List<double>>[
    [1000.0, 3.0, 1.0],
    [1500.0, 4.0, 0.0],
    [2000.0, 5.0, 1.0],
  ];

  group('shared reference vectors', () {
    test('standard scaling centers and scales columns', () {
      final scaler = fitStandardScaler(rows);
      expect(scaler.means[0], equals(1500.0));
      expect(scaler.means[1], equals(4.0));
      final transformed = transformStandard(rows, scaler);
      expect(transformed[0][0], closeTo(-1.224744871391589, tolerance));
      expect(transformed[1][0], closeTo(0.0, tolerance));
      expect(transformed[2][0], closeTo(1.224744871391589, tolerance));
    });

    test('min-max scaling maps columns to the unit range', () {
      expect(
        transformMinMax(rows, fitMinMaxScaler(rows)),
        equals([
          [0.0, 0.0, 1.0],
          [0.5, 0.5, 0.0],
          [1.0, 1.0, 1.0],
        ]),
      );
    });

    test('constant columns map to zero', () {
      final data = <List<double>>[
        [1.0, 7.0],
        [2.0, 7.0],
      ];
      expect(
        transformStandard(data, fitStandardScaler(data))[0][1],
        equals(0.0),
      );
      expect(transformMinMax(data, fitMinMaxScaler(data))[0][1], equals(0.0));
    });
  });

  group('validation and ownership', () {
    test('rejects empty and ragged matrices', () {
      expect(() => fitStandardScaler([]), throwsArgumentError);
      expect(() => fitMinMaxScaler([<double>[]]), throwsArgumentError);
      expect(
        () => fitStandardScaler([
          [1.0, 2.0],
          [3.0],
        ]),
        throwsArgumentError,
      );
    });

    test('rejects mismatched scaler widths', () {
      expect(
        () => transformStandard(rows, StandardScaler([0.0], [1.0])),
        throwsArgumentError,
      );
      expect(
        () => transformMinMax(rows, MinMaxScaler([0.0], [1.0])),
        throwsArgumentError,
      );
    });

    test('does not mutate inputs or expose mutable scaler storage', () {
      final input = <List<double>>[
        [1.0, 2.0],
        [3.0, 4.0],
      ];
      final scaler = fitStandardScaler(input);
      transformStandard(input, scaler)[0][0] = 99.0;
      expect(
        input,
        equals([
          [1.0, 2.0],
          [3.0, 4.0],
        ]),
      );
      expect(() => scaler.means[0] = 99.0, throwsUnsupportedError);
    });
  });
}
