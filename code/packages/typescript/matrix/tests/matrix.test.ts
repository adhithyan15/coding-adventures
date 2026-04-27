import { Matrix } from "../src/matrix";

describe("Matrix Operations", () => {
  it("should create zeros", () => {
    const z = Matrix.zeros(2, 3);
    expect(z.rows).toBe(2);
    expect(z.cols).toBe(3);
    expect(z.data[1][2]).toBe(0.0);
  });

  it("should add and subtract", () => {
    const A = new Matrix([[1.0, 2.0], [3.0, 4.0]]);
    const B = new Matrix([[5.0, 6.0], [7.0, 8.0]]);
    const C = A.add(B);
    expect(C.data).toEqual([[6.0, 8.0], [10.0, 12.0]]);
    
    const D = B.subtract(A);
    expect(D.data).toEqual([[4.0, 4.0], [4.0, 4.0]]);
    
    // Scalar operations
    const E = A.add(2.0);
    expect(E.data).toEqual([[3.0, 4.0], [5.0, 6.0]]);
    const F = A.subtract(1.0);
    expect(F.data).toEqual([[0.0, 1.0], [2.0, 3.0]]);
    
    expect(() => A.add(new Matrix([[1]]))).toThrow("Add dimension mismatch.");
  });

  it("should scale", () => {
    const A = new Matrix([[1.0, 2.0], [3.0, 4.0]]);
    const C = A.scale(2.0);
    expect(C.data).toEqual([[2.0, 4.0], [6.0, 8.0]]);
  });

  it("should transpose", () => {
    const A = new Matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    const C = A.transpose();
    expect(C.data).toEqual([[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]);
  });

  it("should execute dot products mathematically", () => {
    const A = new Matrix([[1.0, 2.0], [3.0, 4.0]]);
    const B = new Matrix([[5.0, 6.0], [7.0, 8.0]]);
    const C = A.dot(B);
    expect(C.data).toEqual([[19.0, 22.0], [43.0, 50.0]]);

    const D = new Matrix([1.0, 2.0, 3.0]); // 1D array natively!
    const E = new Matrix([[4.0], [5.0], [6.0]]);
    const F = D.dot(E);
    expect(F.data).toEqual([[32.0]]);
    
    expect(() => A.dot(E)).toThrow("Dot product inner dimensions strictly mismatch.");
  });
});
