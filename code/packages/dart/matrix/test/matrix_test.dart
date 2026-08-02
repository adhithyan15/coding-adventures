import 'dart:math' as math;

import 'package:coding_adventures_matrix/matrix.dart';
import 'package:test/test.dart';

void main() {
  group('construction', () {
    test('normalizes scalar, vector, and grid inputs', () {
      expect(
        Matrix(5).data,
        equals([
          [5.0],
        ]),
      );
      expect(
        Matrix([1, 2, 3]).data,
        equals([
          [1.0, 2.0, 3.0],
        ]),
      );
      expect(
        Matrix([
          [1, 2],
          [3, 4],
        ]).data,
        equals([
          [1.0, 2.0],
          [3.0, 4.0],
        ]),
      );
    });

    test('rejects empty, ragged, and unsupported inputs', () {
      expect(() => Matrix([]), throwsArgumentError);
      expect(() => Matrix([<num>[]]), throwsArgumentError);
      expect(
        () => Matrix([
          [1, 2],
          [3],
        ]),
        throwsArgumentError,
      );
      expect(() => Matrix('not numeric'), throwsArgumentError);
    });

    test('deep-copies inputs and exposes immutable data', () {
      final input = <List<double>>[
        [1.0, 2.0],
      ];
      final matrix = Matrix(input);
      input[0][0] = 99.0;
      expect(matrix.get(0, 0), equals(1.0));
      expect(() => matrix.data.add([3.0]), throwsUnsupportedError);
      expect(() => matrix.data[0][0] = 99.0, throwsUnsupportedError);
    });

    test('factories validate dimensions', () {
      expect(
        Matrix.zeros(2, 3).data,
        equals([
          [0.0, 0.0, 0.0],
          [0.0, 0.0, 0.0],
        ]),
      );
      expect(
        Matrix.identity(3).data,
        equals([
          [1.0, 0.0, 0.0],
          [0.0, 1.0, 0.0],
          [0.0, 0.0, 1.0],
        ]),
      );
      expect(
        Matrix.fromDiagonal([2, 3]).data,
        equals([
          [2.0, 0.0],
          [0.0, 3.0],
        ]),
      );
      expect(() => Matrix.zeros(0, 2), throwsArgumentError);
      expect(() => Matrix.identity(0), throwsArgumentError);
      expect(() => Matrix.fromDiagonal([]), throwsArgumentError);
    });
  });

  group('base arithmetic', () {
    final a = Matrix([
      [1, 2],
      [3, 4],
    ]);
    final b = Matrix([
      [5, 6],
      [7, 8],
    ]);

    test('adds and subtracts matrices and scalars', () {
      expect(
        (a + b).data,
        equals([
          [6.0, 8.0],
          [10.0, 12.0],
        ]),
      );
      expect(
        (b - a).data,
        equals([
          [4.0, 4.0],
          [4.0, 4.0],
        ]),
      );
      expect(
        (a + 2).data,
        equals([
          [3.0, 4.0],
          [5.0, 6.0],
        ]),
      );
      expect(
        (a - 1).data,
        equals([
          [0.0, 1.0],
          [2.0, 3.0],
        ]),
      );
      expect(() => a + Matrix([1]), throwsArgumentError);
      expect(() => a + 'bad', throwsArgumentError);
    });

    test('scales, transposes, and multiplies matrices', () {
      expect(
        (a * 2).data,
        equals([
          [2.0, 4.0],
          [6.0, 8.0],
        ]),
      );
      expect(
        a.scale(0.5).data,
        equals([
          [0.5, 1.0],
          [1.5, 2.0],
        ]),
      );
      expect(
        a.transpose().data,
        equals([
          [1.0, 3.0],
          [2.0, 4.0],
        ]),
      );
      expect(
        a.dot(b).data,
        equals([
          [19.0, 22.0],
          [43.0, 50.0],
        ]),
      );
      expect(() => Matrix([1, 2]).dot(Matrix([1, 2])), throwsArgumentError);
    });
  });

  group('extensions', () {
    final matrix = Matrix([
      [1, 2, 3],
      [4, 5, 6],
    ]);

    test('access and replacement stay immutable', () {
      expect(matrix.get(1, 2), equals(6.0));
      final replaced = matrix.set(0, 0, 99);
      expect(replaced.get(0, 0), equals(99.0));
      expect(matrix.get(0, 0), equals(1.0));
      expect(() => matrix.get(-1, 0), throwsRangeError);
      expect(() => matrix.set(2, 0, 1), throwsRangeError);
    });

    test('reductions match the shared vectors', () {
      final square = Matrix([
        [1, 2],
        [3, 4],
      ]);
      expect(square.sum(), equals(10.0));
      expect(square.mean(), equals(2.5));
      expect(
        square.sumRows().data,
        equals([
          [3.0],
          [7.0],
        ]),
      );
      expect(
        square.sumCols().data,
        equals([
          [4.0, 6.0],
        ]),
      );
      expect(square.min(), equals(1.0));
      expect(square.max(), equals(4.0));
      expect(square.argmin(), equals(const MatrixIndex(0, 0)));
      expect(square.argmax(), equals(const MatrixIndex(1, 1)));
    });

    test('element-wise math preserves shape', () {
      final values = Matrix([
        [1, 4],
        [9, 16],
      ]);
      expect(
        values.map((value) => value + 1).data,
        equals([
          [2.0, 5.0],
          [10.0, 17.0],
        ]),
      );
      expect(
        values.sqrt().data,
        equals([
          [1.0, 2.0],
          [3.0, 4.0],
        ]),
      );
      expect(
        Matrix([
          [-1, 2],
          [-3, 4],
        ]).abs().data,
        equals([
          [1.0, 2.0],
          [3.0, 4.0],
        ]),
      );
      expect(
        Matrix([
          [1, 2],
          [3, 4],
        ]).pow(2).data,
        equals([
          [1.0, 4.0],
          [9.0, 16.0],
        ]),
      );
      expect(
        () => Matrix([
          [-1],
        ]).sqrt(),
        throwsArgumentError,
      );
    });

    test('shape operations use row-major order', () {
      expect(
        matrix.flatten().data,
        equals([
          [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        ]),
      );
      expect(
        matrix.reshape(3, 2).data,
        equals([
          [1.0, 2.0],
          [3.0, 4.0],
          [5.0, 6.0],
        ]),
      );
      expect(
        matrix.row(1).data,
        equals([
          [4.0, 5.0, 6.0],
        ]),
      );
      expect(
        matrix.col(1).data,
        equals([
          [2.0],
          [5.0],
        ]),
      );
      expect(
        matrix.slice(0, 2, 1, 3).data,
        equals([
          [2.0, 3.0],
          [5.0, 6.0],
        ]),
      );
      expect(() => matrix.reshape(4, 2), throwsArgumentError);
      expect(() => matrix.slice(1, 1, 0, 1), throwsArgumentError);
      expect(() => matrix.slice(-1, 1, 0, 1), throwsRangeError);
    });

    test('exact and tolerant comparisons follow ML03', () {
      final same = Matrix([
        [1, 2, 3],
        [4, 5, 6],
      ]);
      expect(matrix.equals(same), isTrue);
      expect(matrix == same, isTrue);
      expect(
        matrix.close(
          Matrix([
            [1, 2, 3],
            [4, 5, 6.0000000001],
          ]),
        ),
        isTrue,
      );
      expect(
        matrix.close(
          Matrix([
            [1, 2, 3],
            [4, 5, 6.1],
          ]),
        ),
        isFalse,
      );
      expect(matrix.close(same, tolerance: -1), isFalse);
      expect(
          Matrix([
            [double.nan]
          ]).close(Matrix([
            [double.nan]
          ])),
          isFalse);
      expect(
        Matrix([
          [double.infinity]
        ]).close(Matrix([
          [double.infinity]
        ])),
        isTrue,
      );
      expect(
        Matrix([
          [double.infinity]
        ]).close(Matrix([
          [double.negativeInfinity]
        ])),
        isFalse,
      );
      expect(
        Matrix.identity(2)
            .dot(
              Matrix([
                [2, 3],
                [4, 5],
              ]),
            )
            .data,
        equals([
          [2.0, 3.0],
          [4.0, 5.0],
        ]),
      );
      expect(
        Matrix([
          [1, 4],
          [9, 16],
        ]).sqrt().pow(2).close(
              Matrix([
                [1, 4],
                [9, 16],
              ]),
            ),
        isTrue,
      );
      expect(math.sqrt(matrix.get(0, 0)), equals(1.0));
    });
  });
}
