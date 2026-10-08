import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:coding_adventures_affine2d/affine2d.dart';
import 'package:coding_adventures_point2d/point2d.dart';
import 'package:test/test.dart';

const epsilon = 1e-12;

Point point(List<dynamic> pair) =>
    Point((pair[0] as num).toDouble(), (pair[1] as num).toDouble());

Affine2D matrix(List<dynamic> values) => Affine2D(
  (values[0] as num).toDouble(),
  (values[1] as num).toDouble(),
  (values[2] as num).toDouble(),
  (values[3] as num).toDouble(),
  (values[4] as num).toDouble(),
  (values[5] as num).toDouble(),
);

void nearPoint(Point expected, Point actual) {
  expect(actual.x, closeTo(expected.x, epsilon));
  expect(actual.y, closeTo(expected.y, epsilon));
}

void nearMatrix(List<double> expected, List<double> actual) {
  expect(actual, hasLength(6));
  for (var i = 0; i < 6; i++) {
    expect(actual[i], closeTo(expected[i], epsilon), reason: 'component $i');
  }
}

void main() {
  test('G2D01 immutable factories and predicates', () {
    const p = Point(1, 2);
    nearPoint(p, Affine2D.identity().applyToPoint(p));
    expect(Affine2D.identity().isIdentity(), isTrue);
    expect(Affine2D.translate(5, 7).isTranslationOnly(), isTrue);
    expect(Affine2D.translate(0.001, 0).isIdentity(), isFalse);
    expect(Affine2D.rotate(0.1).isTranslationOnly(), isFalse);
    nearPoint(const Point(6, 9), Affine2D.translate(5, 7).applyToPoint(p));
    nearPoint(const Point(2, 6), Affine2D.scale(2, 3).applyToPoint(p));
    nearPoint(const Point(2, 4), Affine2D.scaleUniform(2).applyToPoint(p));
    nearPoint(
      const Point(0, 1),
      Affine2D.rotate(math.pi / 2).applyToPoint(const Point(1, 0)),
    );
    nearPoint(
      const Point(5, 6),
      Affine2D.rotateAround(
        const Point(5, 5),
        math.pi / 2,
      ).applyToPoint(const Point(6, 5)),
    );
    nearPoint(const Point(3, 2), Affine2D.skewX(math.pi / 4).applyToPoint(p));
    nearPoint(const Point(1, 3), Affine2D.skewY(math.pi / 4).applyToPoint(p));
    expect(Affine2D.scale(2, 3).determinant(), closeTo(6, epsilon));
    expect(Affine2D.rotate(0.4).determinant(), closeTo(1, 1e-10));
    expect(Affine2D.scale(0, 1).invert(), isNull);
    final translated = Affine2D.translate(3, 5);
    expect(translated, const Affine2D(1, 0, 0, 1, 3, 5));
    expect(translated == Affine2D.identity(), isFalse);
    expect(translated.hashCode, const Affine2D(1, 0, 0, 1, 3, 5).hashCode);
    nearMatrix(
      Affine2D.identity().toArray(),
      translated.multiply(translated.invert()!).toArray(),
    );
  });

  test('consumes all and only five G2D01 neutral cases', () {
    final corpus =
        jsonDecode(
              File(
                '../../../specs/fixtures/geometry2d-v1/cases.json',
              ).readAsStringSync(),
            )
            as Map<String, dynamic>;
    expect(corpus['version'], 1);
    expect(corpus['absolute_tolerance'], epsilon);
    final seen = <String>{};
    for (final raw in corpus['cases'] as List<dynamic>) {
      final fixture = raw as Map<String, dynamic>;
      final operation = fixture['operation'] as String;
      if (!operation.startsWith('affine-')) continue;
      final id = fixture['id'] as String;
      expect(seen.add(id), isTrue, reason: 'duplicate fixture $id');
      switch (operation) {
        case 'affine-compose':
          final actual = matrix(
            fixture['first'] as List<dynamic>,
          ).multiply(matrix(fixture['second'] as List<dynamic>));
          nearMatrix(
            matrix(fixture['expected_matrix'] as List<dynamic>).toArray(),
            actual.toArray(),
          );
          nearPoint(
            point(fixture['expected_point'] as List<dynamic>),
            actual.applyToPoint(point(fixture['point'] as List<dynamic>)),
          );
          break;
        case 'affine-invert':
          final actual = matrix(fixture['matrix'] as List<dynamic>).invert();
          final expected = fixture['expected_inverse'];
          if (expected == null) {
            expect(actual, isNull);
          } else {
            expect(actual, isNotNull);
            nearMatrix(
              matrix(expected as List<dynamic>).toArray(),
              actual!.toArray(),
            );
          }
          break;
        case 'affine-vector':
          nearPoint(
            point(fixture['expected'] as List<dynamic>),
            matrix(
              fixture['matrix'] as List<dynamic>,
            ).applyToVector(point(fixture['vector'] as List<dynamic>)),
          );
          break;
        default:
          fail('unknown affine operation: $operation');
      }
    }
    expect(seen, {
      'affine-compose-order-a',
      'affine-compose-order-b',
      'affine-invert-nonsingular',
      'affine-invert-singular',
      'affine-vector-translation',
    });
  });
}
