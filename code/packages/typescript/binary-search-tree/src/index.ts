export type Comparator<T> = (left: T, right: T) => number;

export class BSTNode<T> {
  value: T;
  left: BSTNode<T> | null;
  right: BSTNode<T> | null;
  size: number;

  constructor(
    value: T,
    left: BSTNode<T> | null = null,
    right: BSTNode<T> | null = null,
    size = 1 + nodeSize(left) + nodeSize(right),
  ) {
    this.value = value;
    this.left = left;
    this.right = right;
    this.size = size;
  }
}

export class BinarySearchTree<T> {
  readonly root: BSTNode<T> | null;
  readonly compare: Comparator<T>;

  constructor(root: BSTNode<T> | null = null, compare: Comparator<T> = compareValues) {
    this.root = root;
    this.compare = compare;
  }

  static empty<T>(compare: Comparator<T> = compareValues): BinarySearchTree<T> {
    return new BinarySearchTree(null, compare);
  }

  static fromSortedArray<T>(
    values: Iterable<T>,
    compare: Comparator<T> = compareValues,
  ): BinarySearchTree<T> {
    return new BinarySearchTree(buildBalanced(Array.from(values)), compare);
  }

  insert(value: T): BinarySearchTree<T> {
    return new BinarySearchTree(insertNode(this.root, value, this.compare), this.compare);
  }

  delete(value: T): BinarySearchTree<T> {
    return new BinarySearchTree(deleteNode(this.root, value, this.compare), this.compare);
  }

  search(value: T): BSTNode<T> | null {
    let current = this.root;
    while (current !== null) {
      const order = this.compare(value, current.value);
      if (order < 0) {
        current = current.left;
      } else if (order > 0) {
        current = current.right;
      } else {
        return current;
      }
    }
    return null;
  }

  contains(value: T): boolean {
    return this.search(value) !== null;
  }

  minValue(): T | null {
    let current = this.root;
    while (current?.left !== null && current?.left !== undefined) {
      current = current.left;
    }
    return current?.value ?? null;
  }

  maxValue(): T | null {
    let current = this.root;
    while (current?.right !== null && current?.right !== undefined) {
      current = current.right;
    }
    return current?.value ?? null;
  }

  predecessor(value: T): T | null {
    let current = this.root;
    let best: T | null = null;
    while (current !== null) {
      const order = this.compare(value, current.value);
      if (order <= 0) {
        current = current.left;
      } else {
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
      if (order >= 0) {
        current = current.right;
      } else {
        best = current.value;
        current = current.left;
      }
    }
    return best;
  }

  kthSmallest(k: number): T | null {
    return kthSmallest(this.root, k);
  }

  rank(value: T): number {
    return rank(this.root, value, this.compare);
  }

  toSortedArray(): T[] {
    const out: T[] = [];
    inorder(this.root, out);
    return out;
  }

  isValid(): boolean {
    return validate(this.root, null, null, this.compare) !== null;
  }

  height(): number {
    return height(this.root);
  }

  size(): number {
    return nodeSize(this.root);
  }

  toString(): string {
    return `BinarySearchTree(root=${String(this.root?.value ?? null)}, size=${this.size()})`;
  }
}

export function compareValues<T>(left: T, right: T): number {
  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }
  return 0;
}

function insertNode<T>(
  root: BSTNode<T> | null,
  value: T,
  compare: Comparator<T>,
): BSTNode<T> {
  if (root === null) {
    return new BSTNode(value);
  }
  const order = compare(value, root.value);
  if (order < 0) {
    return withChildren(root, insertNode(root.left, value, compare), root.right);
  }
  if (order > 0) {
    return withChildren(root, root.left, insertNode(root.right, value, compare));
  }
  return root;
}

function deleteNode<T>(
  root: BSTNode<T> | null,
  value: T,
  compare: Comparator<T>,
): BSTNode<T> | null {
  if (root === null) {
    return null;
  }
  const order = compare(value, root.value);
  if (order < 0) {
    return withChildren(root, deleteNode(root.left, value, compare), root.right);
  }
  if (order > 0) {
    return withChildren(root, root.left, deleteNode(root.right, value, compare));
  }
  if (root.left === null) {
    return root.right;
  }
  if (root.right === null) {
    return root.left;
  }

  const [newRight, successor] = extractMin(root.right);
  return new BSTNode(successor, root.left, newRight);
}

function extractMin<T>(root: BSTNode<T>): [BSTNode<T> | null, T] {
  if (root.left === null) {
    return [root.right, root.value];
  }
  const [newLeft, minimum] = extractMin(root.left);
  return [withChildren(root, newLeft, root.right), minimum];
}

function kthSmallest<T>(root: BSTNode<T> | null, k: number): T | null {
  if (root === null || k <= 0) {
    return null;
  }
  const leftSize = nodeSize(root.left);
  if (k === leftSize + 1) {
    return root.value;
  }
  if (k <= leftSize) {
    return kthSmallest(root.left, k);
  }
  return kthSmallest(root.right, k - leftSize - 1);
}

function rank<T>(
  root: BSTNode<T> | null,
  value: T,
  compare: Comparator<T>,
): number {
  if (root === null) {
    return 0;
  }
  const order = compare(value, root.value);
  if (order < 0) {
    return rank(root.left, value, compare);
  }
  if (order > 0) {
    return nodeSize(root.left) + 1 + rank(root.right, value, compare);
  }
  return nodeSize(root.left);
}

function inorder<T>(root: BSTNode<T> | null, out: T[]): void {
  if (root === null) {
    return;
  }
  inorder(root.left, out);
  out.push(root.value);
  inorder(root.right, out);
}

function validate<T>(
  root: BSTNode<T> | null,
  minimum: T | null,
  maximum: T | null,
  compare: Comparator<T>,
): [number, number] | null {
  if (root === null) {
    return [-1, 0];
  }
  if (minimum !== null && compare(root.value, minimum) <= 0) {
    return null;
  }
  if (maximum !== null && compare(root.value, maximum) >= 0) {
    return null;
  }

  const left = validate(root.left, minimum, root.value, compare);
  const right = validate(root.right, root.value, maximum, compare);
  if (left === null || right === null) {
    return null;
  }
  const nodeHeight = 1 + Math.max(left[0], right[0]);
  const size = 1 + left[1] + right[1];
  if (root.size !== size) {
    return null;
  }
  return [nodeHeight, size];
}

function height<T>(root: BSTNode<T> | null): number {
  if (root === null) {
    return -1;
  }
  return 1 + Math.max(height(root.left), height(root.right));
}

function buildBalanced<T>(values: T[]): BSTNode<T> | null {
  if (values.length === 0) {
    return null;
  }
  const mid = Math.floor(values.length / 2);
  return new BSTNode(
    values[mid]!,
    buildBalanced(values.slice(0, mid)),
    buildBalanced(values.slice(mid + 1)),
  );
}

function nodeSize<T>(root: BSTNode<T> | null): number {
  return root?.size ?? 0;
}

function withChildren<T>(
  root: BSTNode<T>,
  left: BSTNode<T> | null,
  right: BSTNode<T> | null,
): BSTNode<T> {
  return new BSTNode(root.value, left, right);
}
