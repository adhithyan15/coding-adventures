import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { Complex, InvalidInputError } from "../src/index.js";

describe("Complex scalar operations", () => {
  it("constructs and initializes zero, one, i", () => {
    const c0 = new Complex();
    assert.equal(c0.re, 0);
    assert.equal(c0.im, 0);

    const zero = Complex.zero();
    assert.equal(zero.re, 0);
    assert.equal(zero.im, 0);

    const one = Complex.one();
    assert.equal(one.re, 1);
    assert.equal(one.im, 0);

    const i = Complex.i();
    assert.equal(i.re, 0);
    assert.equal(i.im, 1);
  });

  it("constructs from polar coordinates", () => {
    const c = Complex.fromPolar(2, Math.PI / 2);
    assert.ok(Math.abs(c.re) < 1e-12);
    assert.ok(Math.abs(c.im - 2) < 1e-12);

    const c2 = Complex.fromPolar(1, 0);
    assert.ok(c2.equals(Complex.one()));
  });

  it("performs basic arithmetic with Complex operands", () => {
    const a = new Complex(3, 4);
    const b = new Complex(1, -2);

    const sum = a.add(b);
    assert.equal(sum.re, 4);
    assert.equal(sum.im, 2);

    const diff = a.sub(b);
    assert.equal(diff.re, 2);
    assert.equal(diff.im, 6);

    const prod = a.mul(b);
    // (3 + 4i)(1 - 2i) = 3 - 6i + 4i + 8 = 11 - 2i
    assert.equal(prod.re, 11);
    assert.equal(prod.im, -2);

    const quot = prod.div(b);
    assert.ok(quot.equals(a, 1e-10));
  });

  it("performs arithmetic with number scalars", () => {
    const a = new Complex(3, 4);
    assert.ok(a.add(2).equals(new Complex(5, 4)));
    assert.ok(a.sub(2).equals(new Complex(1, 4)));
    assert.ok(a.mul(2).equals(new Complex(6, 8)));
    assert.ok(a.div(2).equals(new Complex(1.5, 2)));
    assert.ok(a.scale(3).equals(new Complex(9, 12)));
  });

  it("calculates conjugate, magnitude, phase, absSq, and neg", () => {
    const a = new Complex(3, 4);
    const conj = a.conj();
    assert.equal(conj.re, 3);
    assert.equal(conj.im, -4);
    assert.equal(a.abs(), 5);
    assert.equal(a.absSq(), 25);
    assert.equal(a.arg(), Math.atan2(4, 3));

    const neg = a.neg();
    assert.equal(neg.re, -3);
    assert.equal(neg.im, -4);
  });

  it("calculates complex exponential e^(a + bi)", () => {
    const c = new Complex(0, Math.PI);
    const exp = c.exp();
    // e^(i * pi) = -1
    assert.ok(Math.abs(exp.re - (-1)) < 1e-12);
    assert.ok(Math.abs(exp.im) < 1e-12);
  });

  it("supports clone, equals, and toString", () => {
    const a = new Complex(3, 4);
    const b = a.clone();
    assert.ok(a.equals(b));
    assert.equal(a.toString(), "3 + 4i");

    const negIm = new Complex(3, -4);
    assert.equal(negIm.toString(), "3 - 4i");
  });

  it("rejects non-finite inputs fail-closed", () => {
    assert.throws(() => new Complex(NaN, 0), InvalidInputError);
    assert.throws(() => new Complex(0, Infinity), InvalidInputError);
    assert.throws(() => Complex.fromPolar(-1, 0), InvalidInputError);
    assert.throws(() => Complex.fromPolar(1, NaN), InvalidInputError);
    assert.throws(() => new Complex(1, 1).add(NaN), InvalidInputError);
    assert.throws(() => new Complex(1, 1).sub(Infinity), InvalidInputError);
    assert.throws(() => new Complex(1, 1).mul(NaN), InvalidInputError);
    assert.throws(() => new Complex(1, 1).div(0), InvalidInputError);
    assert.throws(() => new Complex(1, 1).div(NaN), InvalidInputError);
    assert.throws(() => new Complex(1, 1).div(new Complex(0, 0)), InvalidInputError);
    assert.throws(() => new Complex(1, 1).scale(NaN), InvalidInputError);
  });
});
