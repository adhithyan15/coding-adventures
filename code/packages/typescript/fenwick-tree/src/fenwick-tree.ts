export class FenwickError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FenwickError";
  }
}

export class IndexOutOfRangeError extends FenwickError {
  constructor(message: string) {
    super(message);
    this.name = "IndexOutOfRangeError";
  }
}

export class EmptyTreeError extends FenwickError {
  constructor(message: string) {
    super(message);
    this.name = "EmptyTreeError";
  }
}

export class FenwickTree {
  private readonly _n: number;
  private readonly _bit: number[];

  constructor(n: number) {
    if (!Number.isInteger(n) || n < 0) {
      throw new FenwickError(`Size must be a non-negative integer, got ${n}`);
    }
    this._n = n;
    this._bit = Array.from({ length: n + 1 }, () => 0);
  }

  static fromList(values: readonly number[]): FenwickTree {
    const tree = new FenwickTree(values.length);
    for (let i = 1; i <= tree._n; i++) {
      tree._bit[i] += values[i - 1];
      const parent = i + (i & -i);
      if (parent <= tree._n) {
        tree._bit[parent] += tree._bit[i];
      }
    }
    return tree;
  }

  update(index: number, delta: number): void {
    this._checkIndex(index);
    let current = index;
    while (current <= this._n) {
      this._bit[current] += delta;
      current += current & -current;
    }
  }

  prefixSum(index: number): number {
    if (!Number.isInteger(index) || index < 0 || index > this._n) {
      throw new IndexOutOfRangeError(
        `prefixSum index ${index} out of range [0, ${this._n}]`
      );
    }

    let total = 0;
    let current = index;
    while (current > 0) {
      total += this._bit[current];
      current -= current & -current;
    }
    return total;
  }

  rangeSum(left: number, right: number): number {
    if (left > right) {
      throw new FenwickError(`left (${left}) must be <= right (${right})`);
    }
    this._checkIndex(left);
    this._checkIndex(right);
    if (left === 1) {
      return this.prefixSum(right);
    }
    return this.prefixSum(right) - this.prefixSum(left - 1);
  }

  pointQuery(index: number): number {
    this._checkIndex(index);
    return this.rangeSum(index, index);
  }

  findKth(k: number): number {
    if (this._n === 0) {
      throw new EmptyTreeError("findKth called on empty tree");
    }
    if (k <= 0) {
      throw new FenwickError(`k must be positive, got ${k}`);
    }

    let idx = 0;
    let remaining = k;
    let bitMask = 1;
    while ((bitMask << 1) <= this._n) {
      bitMask <<= 1;
    }

    while (bitMask !== 0) {
      const nextIdx = idx + bitMask;
      if (nextIdx <= this._n && this._bit[nextIdx] < remaining) {
        idx = nextIdx;
        remaining -= this._bit[nextIdx];
      }
      bitMask >>= 1;
    }

    const result = idx + 1;
    if (result > this._n) {
      throw new FenwickError("k exceeds total sum of the tree");
    }
    return result;
  }

  get length(): number {
    return this._n;
  }

  toString(): string {
    return `FenwickTree(n=${this._n}, bit=${JSON.stringify(this._bit.slice(1))})`;
  }

  private _checkIndex(index: number): void {
    if (!Number.isInteger(index) || index < 1 || index > this._n) {
      throw new IndexOutOfRangeError(
        `Index ${index} out of range [1, ${this._n}]`
      );
    }
  }
}
