export class BinaryTreeNode<T> {
  value: T;
  left: BinaryTreeNode<T> | null;
  right: BinaryTreeNode<T> | null;

  constructor(
    value: T,
    left: BinaryTreeNode<T> | null = null,
    right: BinaryTreeNode<T> | null = null,
  ) {
    this.value = value;
    this.left = left;
    this.right = right;
  }
}

export class BinaryTree<T> {
  readonly root: BinaryTreeNode<T> | null;

  constructor(root: BinaryTreeNode<T> | null = null) {
    this.root = root;
  }

  static withRoot<T>(root: BinaryTreeNode<T> | null): BinaryTree<T> {
    return new BinaryTree(root);
  }

  static singleton<T>(value: T): BinaryTree<T> {
    return new BinaryTree(new BinaryTreeNode(value));
  }

  static fromLevelOrder<T>(
    values: Iterable<T | null | undefined>,
  ): BinaryTree<T> {
    const items = Array.from(values);
    return new BinaryTree(buildFromLevelOrder(items, 0));
  }

  find(value: T): BinaryTreeNode<T> | null {
    return find(this.root, value);
  }

  leftChild(value: T): BinaryTreeNode<T> | null {
    return this.find(value)?.left ?? null;
  }

  rightChild(value: T): BinaryTreeNode<T> | null {
    return this.find(value)?.right ?? null;
  }

  isFull(): boolean {
    return isFull(this.root);
  }

  isComplete(): boolean {
    return isComplete(this.root);
  }

  isPerfect(): boolean {
    return isPerfect(this.root);
  }

  height(): number {
    return height(this.root);
  }

  size(): number {
    return size(this.root);
  }

  inorder(): T[] {
    const out: T[] = [];
    inorder(this.root, out);
    return out;
  }

  preorder(): T[] {
    const out: T[] = [];
    preorder(this.root, out);
    return out;
  }

  postorder(): T[] {
    const out: T[] = [];
    postorder(this.root, out);
    return out;
  }

  levelOrder(): T[] {
    if (this.root === null) {
      return [];
    }

    const out: T[] = [];
    const queue: BinaryTreeNode<T>[] = [this.root];
    for (let index = 0; index < queue.length; index += 1) {
      const node = queue[index]!;
      out.push(node.value);
      if (node.left !== null) {
        queue.push(node.left);
      }
      if (node.right !== null) {
        queue.push(node.right);
      }
    }
    return out;
  }

  toArray(): Array<T | null> {
    const treeHeight = this.height();
    if (treeHeight < 0) {
      return [];
    }

    const out = Array<T | null>((1 << (treeHeight + 1)) - 1).fill(null);
    fillArray(this.root, 0, out);
    return out;
  }

  toAscii(): string {
    if (this.root === null) {
      return "";
    }

    const lines: string[] = [];
    renderAscii(this.root, "", true, lines);
    return lines.join("\n");
  }

  toString(): string {
    return `BinaryTree(root=${String(this.root?.value ?? null)}, size=${this.size()})`;
  }
}

export function find<T>(
  root: BinaryTreeNode<T> | null,
  value: T,
): BinaryTreeNode<T> | null {
  if (root === null) {
    return null;
  }
  if (Object.is(root.value, value)) {
    return root;
  }
  return find(root.left, value) ?? find(root.right, value);
}

export function isFull<T>(root: BinaryTreeNode<T> | null): boolean {
  if (root === null) {
    return true;
  }
  if (root.left === null && root.right === null) {
    return true;
  }
  if (root.left === null || root.right === null) {
    return false;
  }
  return isFull(root.left) && isFull(root.right);
}

export function isComplete<T>(root: BinaryTreeNode<T> | null): boolean {
  const queue: Array<BinaryTreeNode<T> | null> = [root];
  let seenNull = false;

  for (let index = 0; index < queue.length; index += 1) {
    const node = queue[index] ?? null;
    if (node === null) {
      seenNull = true;
      continue;
    }
    if (seenNull) {
      return false;
    }
    queue.push(node.left);
    queue.push(node.right);
  }

  return true;
}

export function isPerfect<T>(root: BinaryTreeNode<T> | null): boolean {
  const treeHeight = height(root);
  if (treeHeight < 0) {
    return size(root) === 0;
  }
  return size(root) === (1 << (treeHeight + 1)) - 1;
}

export function height<T>(root: BinaryTreeNode<T> | null): number {
  if (root === null) {
    return -1;
  }
  return 1 + Math.max(height(root.left), height(root.right));
}

export function size<T>(root: BinaryTreeNode<T> | null): number {
  if (root === null) {
    return 0;
  }
  return 1 + size(root.left) + size(root.right);
}

function buildFromLevelOrder<T>(
  values: Array<T | null | undefined>,
  index: number,
): BinaryTreeNode<T> | null {
  if (index >= values.length) {
    return null;
  }

  const value = values[index];
  if (value === null || value === undefined) {
    return null;
  }

  return new BinaryTreeNode(
    value,
    buildFromLevelOrder(values, 2 * index + 1),
    buildFromLevelOrder(values, 2 * index + 2),
  );
}

function inorder<T>(root: BinaryTreeNode<T> | null, out: T[]): void {
  if (root === null) {
    return;
  }
  inorder(root.left, out);
  out.push(root.value);
  inorder(root.right, out);
}

function preorder<T>(root: BinaryTreeNode<T> | null, out: T[]): void {
  if (root === null) {
    return;
  }
  out.push(root.value);
  preorder(root.left, out);
  preorder(root.right, out);
}

function postorder<T>(root: BinaryTreeNode<T> | null, out: T[]): void {
  if (root === null) {
    return;
  }
  postorder(root.left, out);
  postorder(root.right, out);
  out.push(root.value);
}

function fillArray<T>(
  root: BinaryTreeNode<T> | null,
  index: number,
  out: Array<T | null>,
): void {
  if (root === null || index >= out.length) {
    return;
  }
  out[index] = root.value;
  fillArray(root.left, 2 * index + 1, out);
  fillArray(root.right, 2 * index + 2, out);
}

function renderAscii<T>(
  node: BinaryTreeNode<T>,
  prefix: string,
  isTail: boolean,
  lines: string[],
): void {
  lines.push(`${prefix}${isTail ? "`-- " : "|-- "}${String(node.value)}`);

  const children = [node.left, node.right].filter(
    (child): child is BinaryTreeNode<T> => child !== null,
  );
  const nextPrefix = `${prefix}${isTail ? "    " : "|   "}`;
  children.forEach((child, index) => {
    renderAscii(child, nextPrefix, index + 1 === children.length, lines);
  });
}
