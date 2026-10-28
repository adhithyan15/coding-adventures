/// One position in a level-order binary-tree representation.
///
/// `missing` means there is no node at the position. `value` means a node is
/// present, even when `Value` is itself optional and the payload is `nil`.
/// Keeping these states tagged avoids the classic ambiguity where a nullable
/// payload is accidentally decoded as a hole in the tree.
public enum BinaryTreeSlot<Value> {
  case missing
  case value(Value)

  fileprivate var isMissing: Bool {
    if case .missing = self {
      return true
    }
    return false
  }
}

extension BinaryTreeSlot: Equatable where Value: Equatable {}

/// Stable failures raised at the package's finite admission boundaries.
public enum BinaryTreeError: Error, Equatable, CustomStringConvertible {
  case tooManySlots
  case orphanSlot(index: Int)
  case sharedNode
  case cycle
  case positionalLimit
  case nodeNotFound

  public var description: String {
    switch self {
    case .tooManySlots:
      return "binary tree input exceeds the slot limit"
    case .orphanSlot(let index):
      return "binary tree slot \(index) has no parent"
    case .sharedNode:
      return "binary tree contains a shared node"
    case .cycle:
      return "binary tree contains a cycle"
    case .positionalLimit:
      return "binary tree position exceeds the projection limit"
    case .nodeNotFound:
      return "binary tree does not contain that node"
    }
  }
}

/// An immutable node whose identity is independent of its payload.
///
/// The node retains `value`, `left`, and `right` shallowly. It never copies or
/// compares caller-owned payload objects. Because this is a class, callers can
/// ask about an exact node even when several nodes carry equal values.
public final class BinaryTreeNode<Value> {
  public let value: Value
  public let left: BinaryTreeNode<Value>?
  public let right: BinaryTreeNode<Value>?

  public init(
    value: Value,
    left: BinaryTreeNode<Value>? = nil,
    right: BinaryTreeNode<Value>? = nil
  ) {
    self.value = value
    self.left = left
    self.right = right
  }
}

/// A finite immutable DT03 binary tree.
///
/// Every operation is iterative, so a deep but finite tree cannot exhaust the
/// process call stack. Construction validates that the supplied object graph
/// is a proper tree: each node identity occurs once. That rejects shared
/// children and also rejects a cycle should one arrive through foreign or
/// unsafe construction. Structural arrays and public outputs are capped at
/// `maximumSlots`, which is `2^20 - 1` positions.
public struct BinaryTree<Value> {
  /// The maximum admitted node count, positional slot count, and output size.
  public static var maximumSlots: Int { 1_048_575 }

  public let root: BinaryTreeNode<Value>?

  /// Construct an empty tree.
  public init() {
    root = nil
  }

  /// Construct a tree from an immutable supplied root.
  ///
  /// Validation completes before `self` is initialized, so a failure cannot
  /// publish a partially accepted tree.
  public init(root: BinaryTreeNode<Value>?) throws {
    try Self.validate(root)
    self.root = root
  }

  /// Construct from positional level-order slots.
  ///
  /// Missing positions are retained inside the representation, while trailing
  /// missing positions are canonicalized away. A present slot whose parent is
  /// missing is an orphan and is rejected before any tree is returned.
  public static func fromLevelOrder(_ slots: [BinaryTreeSlot<Value>]) throws -> Self {
    guard slots.count <= maximumSlots else {
      throw BinaryTreeError.tooManySlots
    }

    var canonicalCount = slots.count
    while canonicalCount > 0, slots[canonicalCount - 1].isMissing {
      canonicalCount -= 1
    }
    guard canonicalCount > 0 else {
      return Self()
    }

    for index in 1..<canonicalCount where !slots[index].isMissing {
      let parentIndex = (index - 1) / 2
      if slots[parentIndex].isMissing {
        throw BinaryTreeError.orphanSlot(index: index)
      }
    }

    var nodes = [BinaryTreeNode<Value>?](repeating: nil, count: canonicalCount)
    for index in stride(from: canonicalCount - 1, through: 0, by: -1) {
      guard case .value(let value) = slots[index] else {
        continue
      }
      let leftIndex = index * 2 + 1
      let rightIndex = leftIndex + 1
      nodes[index] = BinaryTreeNode(
        value: value,
        left: leftIndex < canonicalCount ? nodes[leftIndex] : nil,
        right: rightIndex < canonicalCount ? nodes[rightIndex] : nil
      )
    }

    return try Self(root: nodes[0])
  }

  /// Whether this exact node identity belongs to the tree.
  public func contains(_ target: BinaryTreeNode<Value>) -> Bool {
    guard let root else {
      return false
    }
    var stack = [root]
    while let node = stack.popLast() {
      if node === target {
        return true
      }
      if let right = node.right {
        stack.append(right)
      }
      if let left = node.left {
        stack.append(left)
      }
    }
    return false
  }

  /// Return the exact node's left child, rejecting a foreign node identity.
  public func leftChild(of node: BinaryTreeNode<Value>) throws -> BinaryTreeNode<Value>? {
    guard contains(node) else {
      throw BinaryTreeError.nodeNotFound
    }
    return node.left
  }

  /// Return the exact node's right child, rejecting a foreign node identity.
  public func rightChild(of node: BinaryTreeNode<Value>) throws -> BinaryTreeNode<Value>? {
    guard contains(node) else {
      throw BinaryTreeError.nodeNotFound
    }
    return node.right
  }

  /// The number of edges from the root to the exact node identity.
  public func depth(of target: BinaryTreeNode<Value>) throws -> Int {
    guard let root else {
      throw BinaryTreeError.nodeNotFound
    }
    var queue: [(node: BinaryTreeNode<Value>, depth: Int)] = [(root, 0)]
    var cursor = 0
    while cursor < queue.count {
      let current = queue[cursor]
      cursor += 1
      if current.node === target {
        return current.depth
      }
      if let left = current.node.left {
        queue.append((left, current.depth + 1))
      }
      if let right = current.node.right {
        queue.append((right, current.depth + 1))
      }
    }
    throw BinaryTreeError.nodeNotFound
  }

  /// Total number of nodes. The empty tree has size zero.
  public var size: Int {
    guard let root else {
      return 0
    }
    var count = 0
    var stack = [root]
    while let node = stack.popLast() {
      count += 1
      if let right = node.right {
        stack.append(right)
      }
      if let left = node.left {
        stack.append(left)
      }
    }
    return count
  }

  /// Longest root-to-leaf edge count. The empty tree has height `-1`.
  public var height: Int {
    guard let root else {
      return -1
    }
    var result = 0
    var stack: [(node: BinaryTreeNode<Value>, depth: Int)] = [(root, 0)]
    while let current = stack.popLast() {
      result = max(result, current.depth)
      if let right = current.node.right {
        stack.append((right, current.depth + 1))
      }
      if let left = current.node.left {
        stack.append((left, current.depth + 1))
      }
    }
    return result
  }

  /// Whether every node has either zero children or two children.
  public var isFull: Bool {
    guard let root else {
      return true
    }
    var stack = [root]
    while let node = stack.popLast() {
      switch (node.left, node.right) {
      case (nil, nil):
        continue
      case (.some(let left), .some(let right)):
        stack.append(right)
        stack.append(left)
      default:
        return false
      }
    }
    return true
  }

  /// Whether levels are filled left-to-right without a child after a gap.
  public var isComplete: Bool {
    guard let root else {
      return true
    }
    var queue = [root]
    var cursor = 0
    var sawGap = false
    while cursor < queue.count {
      let node = queue[cursor]
      cursor += 1

      if let left = node.left {
        if sawGap {
          return false
        }
        queue.append(left)
      } else {
        sawGap = true
      }

      if let right = node.right {
        if sawGap {
          return false
        }
        queue.append(right)
      } else {
        sawGap = true
      }
    }
    return true
  }

  /// Whether every internal node has two children and all leaves share a depth.
  public var isPerfect: Bool {
    guard let root else {
      return true
    }
    var leafDepth: Int?
    var stack: [(node: BinaryTreeNode<Value>, depth: Int)] = [(root, 0)]
    while let current = stack.popLast() {
      switch (current.node.left, current.node.right) {
      case (nil, nil):
        if let leafDepth, leafDepth != current.depth {
          return false
        }
        leafDepth = current.depth
      case (.some(let left), .some(let right)):
        stack.append((right, current.depth + 1))
        stack.append((left, current.depth + 1))
      default:
        return false
      }
    }
    return true
  }

  /// Left, root, right traversal.
  public func inorder() -> [Value] {
    var result: [Value] = []
    var stack: [BinaryTreeNode<Value>] = []
    var current = root
    while current != nil || !stack.isEmpty {
      while let node = current {
        stack.append(node)
        current = node.left
      }
      let node = stack.removeLast()
      result.append(node.value)
      current = node.right
    }
    return result
  }

  /// Root, left, right traversal.
  public func preorder() -> [Value] {
    guard let root else {
      return []
    }
    var result: [Value] = []
    var stack = [root]
    while let node = stack.popLast() {
      result.append(node.value)
      if let right = node.right {
        stack.append(right)
      }
      if let left = node.left {
        stack.append(left)
      }
    }
    return result
  }

  /// Left, right, root traversal.
  public func postorder() -> [Value] {
    guard let root else {
      return []
    }
    var result: [Value] = []
    var stack: [(node: BinaryTreeNode<Value>, expanded: Bool)] = [(root, false)]
    while let current = stack.popLast() {
      if current.expanded {
        result.append(current.node.value)
        continue
      }
      stack.append((current.node, true))
      if let right = current.node.right {
        stack.append((right, false))
      }
      if let left = current.node.left {
        stack.append((left, false))
      }
    }
    return result
  }

  /// Breadth-first, left-to-right traversal without missing positions.
  public func levelOrder() -> [Value] {
    guard let root else {
      return []
    }
    var result: [Value] = []
    var queue = [root]
    var cursor = 0
    while cursor < queue.count {
      let node = queue[cursor]
      cursor += 1
      result.append(node.value)
      if let left = node.left {
        queue.append(left)
      }
      if let right = node.right {
        queue.append(right)
      }
    }
    return result
  }

  /// Canonical positional level-order slots, including interior holes.
  ///
  /// No array is allocated until every checked positional index is known to
  /// fit within `maximumSlots`. The last element is always a present value.
  public func toLevelOrder() throws -> [BinaryTreeSlot<Value>] {
    guard let root else {
      return []
    }
    var positioned: [(index: Int, value: Value)] = []
    var queue: [(node: BinaryTreeNode<Value>, index: Int)] = [(root, 0)]
    var cursor = 0
    var maximumIndex = 0

    while cursor < queue.count {
      let current = queue[cursor]
      cursor += 1
      positioned.append((current.index, current.node.value))
      maximumIndex = max(maximumIndex, current.index)

      if let left = current.node.left {
        queue.append((left, try Self.childIndex(of: current.index, offset: 1)))
      }
      if let right = current.node.right {
        queue.append((right, try Self.childIndex(of: current.index, offset: 2)))
      }
    }

    var slots = Array(
      repeating: BinaryTreeSlot<Value>.missing,
      count: maximumIndex + 1
    )
    for item in positioned {
      slots[item.index] = .value(item.value)
    }
    return slots
  }

  private static func validate(_ root: BinaryTreeNode<Value>?) throws {
    guard let root else {
      return
    }
    var seen: Set<ObjectIdentifier> = []
    var active: Set<ObjectIdentifier> = []
    var stack: [(node: BinaryTreeNode<Value>, index: Int, leaving: Bool)] = [
      (root, 0, false)
    ]
    while let current = stack.popLast() {
      let identity = ObjectIdentifier(current.node)
      if current.leaving {
        active.remove(identity)
        continue
      }
      guard !active.contains(identity) else {
        throw BinaryTreeError.cycle
      }
      guard seen.insert(identity).inserted else {
        throw BinaryTreeError.sharedNode
      }
      guard seen.count <= maximumSlots else {
        throw BinaryTreeError.tooManySlots
      }
      active.insert(identity)
      stack.append((current.node, current.index, true))
      if let right = current.node.right {
        stack.append((right, try childIndex(of: current.index, offset: 2), false))
      }
      if let left = current.node.left {
        stack.append((left, try childIndex(of: current.index, offset: 1), false))
      }
    }
  }

  private static func childIndex(of index: Int, offset: Int) throws -> Int {
    let (doubled, multiplyOverflow) = index.multipliedReportingOverflow(by: 2)
    let (child, addOverflow) = doubled.addingReportingOverflow(offset)
    guard !multiplyOverflow, !addOverflow, child < maximumSlots else {
      throw BinaryTreeError.positionalLimit
    }
    return child
  }
}
