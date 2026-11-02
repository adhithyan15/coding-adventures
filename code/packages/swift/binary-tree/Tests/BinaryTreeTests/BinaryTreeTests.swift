import Testing

@testable import BinaryTree

@Test func emptySingletonAndNullablePayloadsRemainDistinct() throws {
  let empty = BinaryTree<Int>()
  #expect(empty.root == nil)
  #expect(empty.size == 0)
  #expect(empty.height == -1)
  #expect(empty.isFull)
  #expect(empty.isComplete)
  #expect(empty.isPerfect)
  #expect(empty.inorder().isEmpty)
  #expect(empty.preorder().isEmpty)
  #expect(empty.postorder().isEmpty)
  #expect(empty.levelOrder().isEmpty)
  #expect(try empty.toLevelOrder().isEmpty)

  let nullable = BinaryTreeNode<Int?>(value: nil)
  let tree = try BinaryTree(root: nullable)
  #expect(tree.size == 1)
  #expect(tree.height == 0)
  #expect(tree.levelOrder().count == 1)
  #expect(tree.levelOrder()[0] == nil)
  #expect(try tree.toLevelOrder() == [.value(nil)])
  #expect(try BinaryTree<Int?>.fromLevelOrder([.missing]).root == nil)
}

@Test func sparseConstructionPreservesPositionAndTraversalOrder() throws {
  let tree = try BinaryTree.fromLevelOrder([
    .value("A"),
    .value("B"),
    .value("C"),
    .missing,
    .value("D"),
    .missing,
    .missing,
  ])

  #expect(tree.size == 4)
  #expect(tree.height == 2)
  #expect(tree.inorder() == ["B", "D", "A", "C"])
  #expect(tree.preorder() == ["A", "B", "D", "C"])
  #expect(tree.postorder() == ["D", "B", "C", "A"])
  #expect(tree.levelOrder() == ["A", "B", "C", "D"])
  #expect(
    try tree.toLevelOrder() == [
      .value("A"),
      .value("B"),
      .value("C"),
      .missing,
      .value("D"),
    ])
  #expect(!tree.isFull)
  #expect(!tree.isComplete)
  #expect(!tree.isPerfect)
}

@Test func shapePredicatesCoverIndependentTreeShapes() throws {
  let single = try BinaryTree(root: BinaryTreeNode(value: 1))
  #expect(single.isFull)
  #expect(single.isComplete)
  #expect(single.isPerfect)

  let leftOnly = try BinaryTree(
    root: BinaryTreeNode(value: 1, left: BinaryTreeNode(value: 2))
  )
  #expect(!leftOnly.isFull)
  #expect(leftOnly.isComplete)
  #expect(!leftOnly.isPerfect)

  let rightOnly = try BinaryTree(
    root: BinaryTreeNode(value: 1, right: BinaryTreeNode(value: 2))
  )
  #expect(!rightOnly.isFull)
  #expect(!rightOnly.isComplete)
  #expect(!rightOnly.isPerfect)

  let perfect = try BinaryTree(
    root: BinaryTreeNode(
      value: 1,
      left: BinaryTreeNode(
        value: 2,
        left: BinaryTreeNode(value: 4),
        right: BinaryTreeNode(value: 5)
      ),
      right: BinaryTreeNode(
        value: 3,
        left: BinaryTreeNode(value: 6),
        right: BinaryTreeNode(value: 7)
      )
    )
  )
  #expect(perfect.isFull)
  #expect(perfect.isComplete)
  #expect(perfect.isPerfect)
}

@Test func accessorsUseNodeIdentityRatherThanPayloadEquality() throws {
  let firstDuplicate = BinaryTreeNode(value: "same")
  let secondDuplicate = BinaryTreeNode(value: "same")
  let root = BinaryTreeNode(
    value: "root",
    left: BinaryTreeNode(value: "left", right: firstDuplicate),
    right: secondDuplicate
  )
  let tree = try BinaryTree(root: root)

  #expect(tree.contains(root))
  #expect(tree.contains(firstDuplicate))
  #expect(tree.contains(secondDuplicate))
  #expect(try tree.depth(of: root) == 0)
  #expect(try tree.depth(of: firstDuplicate) == 2)
  #expect(try tree.depth(of: secondDuplicate) == 1)
  #expect(try tree.rightChild(of: root.left!) === firstDuplicate)
  #expect(try tree.leftChild(of: root) === root.left)
  #expect(try tree.rightChild(of: root) === secondDuplicate)

  let outsider = BinaryTreeNode(value: "same")
  #expect(!tree.contains(outsider))
  #expect(throws: BinaryTreeError.nodeNotFound) {
    try tree.depth(of: outsider)
  }
  #expect(throws: BinaryTreeError.nodeNotFound) {
    try tree.leftChild(of: outsider)
  }
  #expect(throws: BinaryTreeError.nodeNotFound) {
    try tree.rightChild(of: outsider)
  }
}

@Test func malformedInputsFailBeforePublishingATree() throws {
  let shared = BinaryTreeNode(value: 2)
  let repeatedRoot = BinaryTreeNode(value: 1, left: shared, right: shared)
  #expect(throws: BinaryTreeError.sharedNode) {
    try BinaryTree(root: repeatedRoot)
  }

  #expect(throws: BinaryTreeError.orphanSlot(index: 3)) {
    try BinaryTree.fromLevelOrder([
      .value(1),
      .missing,
      .value(3),
      .value(4),
    ])
  }

  let canonical = try BinaryTree.fromLevelOrder([
    .value(1),
    .value(2),
    .missing,
    .missing,
    .missing,
  ])
  #expect(try canonical.toLevelOrder() == [.value(1), .value(2)])

  let oversized = Array(
    repeating: BinaryTreeSlot<Int>.missing,
    count: BinaryTree<Int>.maximumSlots + 1
  )
  #expect(throws: BinaryTreeError.tooManySlots) {
    try BinaryTree.fromLevelOrder(oversized)
  }
}

@Test func checkedPositionsRejectExponentiallyWideProjection() throws {
  var node: BinaryTreeNode<Int>? = nil
  for value in (0...20).reversed() {
    node = BinaryTreeNode(value: value, right: node)
  }
  #expect(throws: BinaryTreeError.positionalLimit) {
    try BinaryTree(root: node)
  }
}

@Test func wideTreeUsesIterativeAlgorithms() throws {
  let count = 4_095
  let slots = (0..<count).map(BinaryTreeSlot<Int>.value)
  let tree = try BinaryTree.fromLevelOrder(slots)

  #expect(tree.size == count)
  #expect(tree.height == 11)
  #expect(tree.preorder().first == 0)
  #expect(tree.preorder().last == count - 1)
  #expect(tree.inorder().count == count)
  #expect(tree.postorder().last == 0)
  #expect(tree.levelOrder().count == count)
  #expect(tree.isFull)
  #expect(tree.isComplete)
  #expect(tree.isPerfect)
  #expect(try tree.toLevelOrder() == slots)
}

@Test func payloadOwnershipIsShallowAndReturnedArraysAreIndependent() throws {
  final class Box {
    let value: Int

    init(_ value: Int) {
      self.value = value
    }
  }

  let box = Box(7)
  let tree = try BinaryTree(root: BinaryTreeNode(value: box))
  let first = tree.levelOrder()
  var second = tree.levelOrder()
  #expect(first[0] === box)
  #expect(second[0] === box)
  second.removeAll()
  #expect(tree.size == 1)
  #expect(tree.levelOrder()[0] === box)
}

@Test func stableErrorsHaveDeterministicDescriptions() {
  #expect(BinaryTreeError.tooManySlots.description == "binary tree input exceeds the slot limit")
  #expect(BinaryTreeError.orphanSlot(index: 9).description == "binary tree slot 9 has no parent")
  #expect(BinaryTreeError.sharedNode.description == "binary tree contains a shared node")
  #expect(BinaryTreeError.cycle.description == "binary tree contains a cycle")
  #expect(
    BinaryTreeError.positionalLimit.description
      == "binary tree position exceeds the projection limit")
  #expect(BinaryTreeError.nodeNotFound.description == "binary tree does not contain that node")
}
