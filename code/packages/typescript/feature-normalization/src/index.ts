export const VERSION = "0.1.0";

export type Matrix = number[][];

export interface StandardScaler {
  means: number[];
  standardDeviations: number[];
}

export interface MinMaxScaler {
  minimums: number[];
  maximums: number[];
}

function validateMatrix(rows: Matrix): number {
  if (rows.length === 0 || rows[0].length === 0) {
    throw new Error("matrix must have at least one row and one column");
  }
  const width = rows[0].length;
  for (const row of rows) {
    if (row.length !== width) {
      throw new Error("all rows must have the same number of columns");
    }
  }
  return width;
}

export function fitStandardScaler(rows: Matrix): StandardScaler {
  const width = validateMatrix(rows);
  const means = Array(width).fill(0);
  for (const row of rows) {
    row.forEach((value, col) => {
      means[col] += value;
    });
  }
  for (let col = 0; col < width; col++) means[col] /= rows.length;

  const standardDeviations = Array(width).fill(0);
  for (const row of rows) {
    row.forEach((value, col) => {
      const diff = value - means[col];
      standardDeviations[col] += diff * diff;
    });
  }
  for (let col = 0; col < width; col++) {
    standardDeviations[col] = Math.sqrt(standardDeviations[col] / rows.length);
  }
  return { means, standardDeviations };
}

export function transformStandard(rows: Matrix, scaler: StandardScaler): Matrix {
  const width = validateMatrix(rows);
  if (width !== scaler.means.length || width !== scaler.standardDeviations.length) {
    throw new Error("matrix width must match scaler width");
  }
  return rows.map(row => row.map((value, col) => (
    scaler.standardDeviations[col] === 0
      ? 0
      : (value - scaler.means[col]) / scaler.standardDeviations[col]
  )));
}

export function fitMinMaxScaler(rows: Matrix): MinMaxScaler {
  const width = validateMatrix(rows);
  const minimums = [...rows[0]];
  const maximums = [...rows[0]];
  for (const row of rows.slice(1)) {
    for (let col = 0; col < width; col++) {
      minimums[col] = Math.min(minimums[col], row[col]);
      maximums[col] = Math.max(maximums[col], row[col]);
    }
  }
  return { minimums, maximums };
}

export function transformMinMax(rows: Matrix, scaler: MinMaxScaler): Matrix {
  const width = validateMatrix(rows);
  if (width !== scaler.minimums.length || width !== scaler.maximums.length) {
    throw new Error("matrix width must match scaler width");
  }
  return rows.map(row => row.map((value, col) => {
    const span = scaler.maximums[col] - scaler.minimums[col];
    return span === 0 ? 0 : (value - scaler.minimums[col]) / span;
  }));
}
