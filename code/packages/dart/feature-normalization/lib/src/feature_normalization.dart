import 'dart:math' as math;

class StandardScaler {
  StandardScaler(List<double> means, List<double> standardDeviations)
      : means = List<double>.unmodifiable(means),
        standardDeviations = List<double>.unmodifiable(standardDeviations);

  final List<double> means;
  final List<double> standardDeviations;
}

class MinMaxScaler {
  MinMaxScaler(List<double> minimums, List<double> maximums)
      : minimums = List<double>.unmodifiable(minimums),
        maximums = List<double>.unmodifiable(maximums);

  final List<double> minimums;
  final List<double> maximums;
}

StandardScaler fitStandardScaler(List<List<double>> rows) {
  final width = _validateMatrix(rows);
  final means = List.generate(
    width,
    (column) =>
        rows.fold(0.0, (total, row) => total + row[column]) / rows.length,
  );
  final standardDeviations = List.generate(width, (column) {
    final variance = rows.fold(0.0, (total, row) {
          final difference = row[column] - means[column];
          return total + difference * difference;
        }) /
        rows.length;
    return math.sqrt(variance);
  });
  return StandardScaler(means, standardDeviations);
}

List<List<double>> transformStandard(
  List<List<double>> rows,
  StandardScaler scaler,
) {
  final width = _validateMatrix(rows);
  if (scaler.means.length != width ||
      scaler.standardDeviations.length != width) {
    throw ArgumentError('matrix width must match standard scaler width');
  }
  return rows.map((row) {
    return List.generate(width, (column) {
      final standardDeviation = scaler.standardDeviations[column];
      return standardDeviation == 0.0
          ? 0.0
          : (row[column] - scaler.means[column]) / standardDeviation;
    });
  }).toList();
}

MinMaxScaler fitMinMaxScaler(List<List<double>> rows) {
  final width = _validateMatrix(rows);
  final minimums = List.generate(
    width,
    (column) => rows.map((row) => row[column]).reduce(math.min),
  );
  final maximums = List.generate(
    width,
    (column) => rows.map((row) => row[column]).reduce(math.max),
  );
  return MinMaxScaler(minimums, maximums);
}

List<List<double>> transformMinMax(
  List<List<double>> rows,
  MinMaxScaler scaler,
) {
  final width = _validateMatrix(rows);
  if (scaler.minimums.length != width || scaler.maximums.length != width) {
    throw ArgumentError('matrix width must match min-max scaler width');
  }
  return rows.map((row) {
    return List.generate(width, (column) {
      final span = scaler.maximums[column] - scaler.minimums[column];
      return span == 0.0 ? 0.0 : (row[column] - scaler.minimums[column]) / span;
    });
  }).toList();
}

int _validateMatrix(List<List<double>> rows) {
  if (rows.isEmpty || rows.first.isEmpty) {
    throw ArgumentError('matrix must have at least one row and one column');
  }
  final width = rows.first.length;
  if (rows.any((row) => row.length != width)) {
    throw ArgumentError('all matrix rows must have the same width');
  }
  return width;
}
