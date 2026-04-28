export type Comparator<T> = (left: T, right: T) => number;

export class AVLNode<T> {
  constructor(
    readonly value: T,
    readonly left: AVLNode<T> | null = null,
    readonly right: AVLNode<T> | null = null,
    readonly height = 0,
    readonly size = 1,
  ) {}
}

export class AVLTree<T> {
  constructor(
    readonly root: AVLNode<T> | null = null,
    readonly compare: Comparator<T> = compareValues,
  ) {}

  static empty<T>(compare: Comparator<T> = compareValues): AVLTree<T> {
    return new AVLTree(null, compare);
  }

  static fromValues<T>(values: Iterable<T>, compare: Comparator<T> = compareValues): AVLTree<T> {
    let tree = AVLTree.empty(compare);
    for (const value of values) {
      tree = tree.insert(value);
    }
    return tree;
  }

  insert(value: T): AVLTree<T> {
    return new AVLTree(insertNode(this.root, value, this.compare), this.compare);
  }

  delete(value: T): AVLTree<T> {
    return new AVLTree(deleteNode(this.root, value, this.compare), this.compare);
  }

  search(value: T): AVLNode<T> | null {
    let current = this.root;
    while (current !== null) {
      const order = this.compare(value, current.value);
      if (order < 0) current = current.left;
      else if (order > 0) current = current.right;
      else return current;
    }
    return null;
  }

  contains(value: T): boolean {
    return this.search(value) !== null;
  }

  minValue(): T | null {
    let current = this.root;
    while (current?.left) current = current.left;
    return current?.value ?? null;
  }

  maxValue(): T | null {
    let current = this.root;
    while (current?.right) current = current.right;
    return current?.value ?? null;
  }

  predecessor(value: T): T | null {
    let current = this.root;
    let best: T | null = null;
    while (current !== null) {
      const order = this.compare(value, current.value);
      if (order <= 0) current = current.left;
      else {
        best = current.value;
        current = current.right;
      }
    }
    return best;
  }

  successor(value: T): T | null {
    let current = this.root;
    let best: T | null = null;
    while (current !== null) {
      const order = this.compare(value, current.value);
      if (order >= 0) current = current.right;
      else {
        best = current.value;
        current = current.left;
      }
    }
    return best;
  }

  kthSmallest(k: number): T | null {
    return kth(this.root, k);
  }

  rank(value: T): number {
    return rank(this.root, value, this.compare);
  }

  toSortedArray(): T[] {
    const out: T[] = [];
    inorder(this.root, out);
    return out;
  }

  isValidBst(): boolean {
    return validateBst(this.root, null, null, this.compare);
  }

  isValidAvl(): boolean {
    return validateAvl(this.root, null, null, this.compare) !== null;
  }

  balanceFactor(node: AVLNode<T> | null): number {
    return node === null ? 0 : heightOf(node.left) - heightOf(node.right);
  }

  height(): number {
    return heightOf(this.root);
  }

  size(): number {
    return sizeOf(this.root);
  }
}

export function compareValues<T>(left: T, right: T): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

function insertNode<T>(root: AVLNode<T> | null, value: T, compare: Comparator<T>): AVLNode<T> {
  if (root === null) return new AVLNode(value);
  const order = compare(value, root.value);
  if (order < 0) return rebalance(node(root.value, insertNode(root.left, value, compare), root.right));
  if (order > 0) return rebalance(node(root.value, root.left, insertNode(root.right, value, compare)));
  return root;
}

function deleteNode<T>(root: AVLNode<T> | null, value: T, compare: Comparator<T>): AVLNode<T> | null {
  if (root === null) return null;
  const order = compare(value, root.value);
  if (order < 0) return rebalance(node(root.value, deleteNode(root.left, value, compare), root.right));
  if (order > 0) return rebalance(node(root.value, root.left, deleteNode(root.right, value, compare)));
  if (root.left === null) return root.right;
  if (root.right === null) return root.left;
  const [newRight, successor] = extractMin(root.right);
  return rebalance(node(successor, root.left, newRight));
}

function extractMin<T>(root: AVLNode<T>): [AVLNode<T> | null, T] {
  if (root.left === null) return [root.right, root.value];
  const [newLeft, minimum] = extractMin(root.left);
  return [rebalance(node(root.value, newLeft, root.right)), minimum];
}

function rebalance<T>(root: AVLNode<T>): AVLNode<T> {
  const bf = balanceFactor(root);
  if (bf > 1) {
    const left = root.left && balanceFactor(root.left) < 0 ? rotateLeft(root.left) : root.left;
    return rotateRight(node(root.value, left, root.right));
  }
  if (bf < -1) {
    const right = root.right && balanceFactor(root.right) > 0 ? rotateRight(root.right) : root.right;
    return rotateLeft(node(root.value, root.left, right));
  }
  return root;
}

function rotateLeft<T>(root: AVLNode<T>): AVLNode<T> {
  if (root.right === null) return root;
  const newLeft = node(root.value, root.left, root.right.left);
  return node(root.right.value, newLeft, root.right.right);
}

function rotateRight<T>(root: AVLNode<T>): AVLNode<T> {
  if (root.left === null) return root;
  const newRight = node(root.value, root.left.right, root.right);
  return node(root.left.value, root.left.left, newRight);
}

function balanceFactor<T>(root: AVLNode<T>): number {
  return heightOf(root.left) - heightOf(root.right);
}

function kth<T>(root: AVLNode<T> | null, k: number): T | null {
  if (root === null || k <= 0) return null;
  const leftSize = sizeOf(root.left);
  if (k === leftSize + 1) return root.value;
  if (k <= leftSize) return kth(root.left, k);
  return kth(root.right, k - leftSize - 1);
}

function rank<T>(root: AVLNode<T> | null, value: T, compare: Comparator<T>): number {
  if (root === null) return 0;
  const order = compare(value, root.value);
  if (order < 0) return rank(root.left, value, compare);
  if (order > 0) return sizeOf(root.left) + 1 + rank(root.right, value, compare);
  return sizeOf(root.left);
}

function inorder<T>(root: AVLNode<T> | null, out: T[]): void {
  if (root === null) return;
  inorder(root.left, out);
  out.push(root.value);
  inorder(root.right, out);
}

function validateBst<T>(root: AVLNode<T> | null, min: T | null, max: T | null, compare: Comparator<T>): boolean {
  if (root === null) return true;
  if (min !== null && compare(root.value, min) <= 0) return false;
  if (max !== null && compare(root.value, max) >= 0) return false;
  return validateBst(root.left, min, root.value, compare) && validateBst(root.right, root.value, max, compare);
}

function validateAvl<T>(
  root: AVLNode<T> | null,
  min: T | null,
  max: T | null,
  compare: Comparator<T>,
): [number, number] | null {
  if (root === null) return [-1, 0];
  if (min !== null && compare(root.value, min) <= 0) return null;
  if (max !== null && compare(root.value, max) >= 0) return null;
  const left = validateAvl(root.left, min, root.value, compare);
  const right = validateAvl(root.right, root.value, max, compare);
  if (left === null || right === null) return null;
  const computedHeight = 1 + Math.max(left[0], right[0]);
  const computedSize = 1 + left[1] + right[1];
  if (root.height !== computedHeight || root.size !== computedSize || Math.abs(left[0] - right[0]) > 1) return null;
  return [computedHeight, computedSize];
}

function heightOf<T>(root: AVLNode<T> | null): number {
  return root?.height ?? -1;
}

function sizeOf<T>(root: AVLNode<T> | null): number {
  return root?.size ?? 0;
}

function node<T>(value: T, left: AVLNode<T> | null, right: AVLNode<T> | null): AVLNode<T> {
  return new AVLNode(value, left, right, 1 + Math.max(heightOf(left), heightOf(right)), 1 + sizeOf(left) + sizeOf(right));
}
