export class Matrix {
  data: number[][];
  rows: number;
  cols: number;

  constructor(data: number | number[] | number[][]) {
    if (typeof data === "number") {
      this.data = [[data]];
    } else if (Array.isArray(data) && data.length > 0 && typeof data[0] === "number") {
      this.data = [(data as number[])];
    } else if (Array.isArray(data)) {
      this.data = data as number[][];
    } else {
      this.data = [];
    }
    this.rows = this.data.length;
    this.cols = this.rows > 0 ? this.data[0].length : 0;
  }

  static zeros(rows: number, cols: number): Matrix {
    return new Matrix(Array.from({ length: rows }, () => Array(cols).fill(0.0)));
  }

  add(other: Matrix | number): Matrix {
    if (typeof other === "number") {
      return new Matrix(this.data.map(row => row.map(val => val + other)));
    }
    if (this.rows !== other.rows || this.cols !== other.cols) throw new Error("Add dimension mismatch.");
    return new Matrix(this.data.map((row, i) => row.map((val, j) => val + other.data[i][j])));
  }

  subtract(other: Matrix | number): Matrix {
    if (typeof other === "number") {
      return new Matrix(this.data.map(row => row.map(val => val - other)));
    }
    if (this.rows !== other.rows || this.cols !== other.cols) throw new Error("Subtract dimension mismatch.");
    return new Matrix(this.data.map((row, i) => row.map((val, j) => val - other.data[i][j])));
  }

  scale(scalar: number): Matrix {
    return new Matrix(this.data.map(row => row.map(val => val * scalar)));
  }

  transpose(): Matrix {
    if (this.rows === 0) return new Matrix([]);
    return new Matrix(this.data[0].map((_, colIndex) => this.data.map(row => row[colIndex])));
  }

  dot(other: Matrix): Matrix {
    if (this.cols !== other.rows) throw new Error("Dot product inner dimensions strictly mismatch.");
    const c = Matrix.zeros(this.rows, other.cols);
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < other.cols; j++) {
        for (let k = 0; k < this.cols; k++) {
          c.data[i][j] += this.data[i][k] * other.data[k][j];
        }
      }
    }
    return c;
  }
}
