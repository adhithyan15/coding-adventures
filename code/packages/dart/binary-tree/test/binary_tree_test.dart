import 'package:coding_adventures_binary_tree/binary_tree.dart';
import 'package:test/test.dart';

BinaryTreeSlot<T> value<T>(T item) => BinaryTreeSlot<T>.value(item);
BinaryTreeSlot<T> missing<T>() => BinaryTreeSlot<T>.missing();

void main() {
  group('construction and positional representation', () {
    test('empty tree has the neutral empty contract', () {
      final tree = BinaryTree<int>.empty();
      expect(tree.root, isNull);
      expect(tree.size, 0);
      expect(tree.height, -1);
      expect(tree.isEmpty, isTrue);
      expect(tree.isFull, isTrue);
      expect(tree.isComplete, isTrue);
      expect(tree.isPerfect, isTrue);
      expect(tree.inorder(), isEmpty);
      expect(tree.preorder(), isEmpty);
      expect(tree.postorder(), isEmpty);
      expect(tree.levelOrder(), isEmpty);
      expect(tree.toArray(), isEmpty);
      expect(BinaryTree<int>.fromRoot(null).isEmpty, isTrue);
      expect(
        () => tree.depthOf(const BinaryTreeNode<int>(1)),
        throwsA(
          isA<BinaryTreeException>()
              .having(
                (e) => e.code,
                'code',
                BinaryTreeErrorCode.nodeNotFound,
              )
              .having((e) => e.toString(), 'text', contains('nodeNotFound')),
        ),
      );
    });

    test('sparse positions round trip and trailing gaps are canonical', () {
      final tree = BinaryTree<int>.fromLevelOrder([
        value(1),
        value(2),
        value(3),
        missing(),
        value(4),
        missing(),
        missing(),
        missing(),
      ]);
      expect(tree.toArray(), [
        value(1),
        value(2),
        value(3),
        missing(),
        value(4),
      ]);
      expect(
        BinaryTree<int>.fromLevelOrder(tree.toArray()).toArray(),
        tree.toArray(),
      );
    });

    test('missing and present null payload are distinct', () {
      final tree = BinaryTree<String?>.fromLevelOrder([
        value<String?>(null),
        missing<String?>(),
        value<String?>('right'),
      ]);
      expect(tree.size, 2);
      expect(tree.root!.value, isNull);
      expect(tree.toArray(), [
        value<String?>(null),
        missing<String?>(),
        value<String?>('right'),
      ]);
    });

    test('orphan and oversized projections fail with stable codes', () {
      expect(
        () => BinaryTree<int>.fromLevelOrder([missing(), value(2)]),
        throwsA(
          isA<BinaryTreeException>().having(
            (e) => e.code,
            'code',
            BinaryTreeErrorCode.orphanSlot,
          ),
        ),
      );
      expect(
        () => BinaryTree<int>.fromLevelOrder([
          value(1),
          missing(),
          value(3),
          missing(),
          value(5),
        ]),
        throwsA(
          isA<BinaryTreeException>().having(
            (e) => e.code,
            'code',
            BinaryTreeErrorCode.orphanSlot,
          ),
        ),
      );
      expect(
        () => BinaryTree<int>.fromLevelOrder(
          List.filled(maxBinaryTreeSlots + 1, missing()),
        ),
        throwsA(
          isA<BinaryTreeException>().having(
            (e) => e.code,
            'code',
            BinaryTreeErrorCode.tooManySlots,
          ),
        ),
      );
      expect(missing<int>().hashCode, missing<int>().hashCode);
      expect(value(1).hashCode, value(1).hashCode);
      expect(missing<int>().toString(), contains('missing'));
      expect(value(1).toString(), contains('value(1)'));
    });
  });

  group('identity and validation', () {
    test('duplicate payloads do not confuse identity access', () {
      final left = BinaryTreeNode<int>(7);
      final right = BinaryTreeNode<int>(7);
      final root = BinaryTreeNode<int>(7, left: left, right: right);
      final tree = BinaryTree<int>.fromRoot(root);
      expect(tree.contains(left), isTrue);
      expect(tree.leftChild(root), same(left));
      expect(tree.rightChild(root), same(right));
      expect(tree.depthOf(right), 1);
      expect(tree.contains(const BinaryTreeNode<int>(7)), isFalse);
      expect(
        () => tree.depthOf(const BinaryTreeNode<int>(7)),
        throwsA(
          isA<BinaryTreeException>().having(
            (e) => e.code,
            'code',
            BinaryTreeErrorCode.nodeNotFound,
          ),
        ),
      );
    });

    test('shared supplied nodes are rejected atomically', () {
      const child = BinaryTreeNode<int>(2);
      const root = BinaryTreeNode<int>(1, left: child, right: child);
      expect(
        () => BinaryTree<int>.fromRoot(root),
        throwsA(
          isA<BinaryTreeException>().having(
            (e) => e.code,
            'code',
            BinaryTreeErrorCode.sharedNode,
          ),
        ),
      );
    });

    test('a position beyond the hard projection ceiling is rejected', () {
      var node = const BinaryTreeNode<int>(20);
      for (var value = 19; value >= 0; value--) {
        node = BinaryTreeNode<int>(value, left: node);
      }
      expect(
        () => BinaryTree<int>.fromRoot(node),
        throwsA(
          isA<BinaryTreeException>().having(
            (e) => e.code,
            'code',
            BinaryTreeErrorCode.positionalLimit,
          ),
        ),
      );
    });

    test('payload ownership is shallow and arrays are returned by copy', () {
      final payload = <int>[1];
      final tree = BinaryTree<List<int>>.fromRoot(BinaryTreeNode(payload));
      payload.add(2);
      expect(tree.root!.value, [1, 2]);
      final first = tree.toArray();
      first[0] = missing();
      expect(tree.toArray(), [value(payload)]);
    });
  });

  group('traversal and shape', () {
    final tree = BinaryTree<int>.fromLevelOrder([
      value(1),
      value(2),
      value(3),
      value(4),
      value(5),
      missing(),
      value(6),
    ]);

    test('all traversals are deterministic and iterative', () {
      expect(tree.inorder(), [4, 2, 5, 1, 3, 6]);
      expect(tree.preorder(), [1, 2, 4, 5, 3, 6]);
      expect(tree.postorder(), [4, 5, 2, 6, 3, 1]);
      expect(tree.levelOrder(), [1, 2, 3, 4, 5, 6]);
      expect(tree.size, 6);
      expect(tree.height, 2);
    });

    test('full complete and perfect remain separate properties', () {
      expect(tree.isFull, isFalse);
      expect(tree.isComplete, isFalse);
      expect(tree.isPerfect, isFalse);

      final perfect = BinaryTree<int>.fromLevelOrder(
        List.generate(7, (i) => value(i)),
      );
      expect(perfect.isFull, isTrue);
      expect(perfect.isComplete, isTrue);
      expect(perfect.isPerfect, isTrue);

      final leftOnly = BinaryTree<int>.fromLevelOrder([value(1), value(2)]);
      expect(leftOnly.isFull, isFalse);
      expect(leftOnly.isComplete, isTrue);
      expect(leftOnly.isPerfect, isFalse);

      final rightOnly = BinaryTree<int>.fromLevelOrder([
        value(1),
        missing(),
        value(2),
      ]);
      expect(rightOnly.isFull, isFalse);
      expect(rightOnly.isComplete, isFalse);
      expect(rightOnly.isPerfect, isFalse);
    });

    test('single node is full complete and perfect', () {
      final single = BinaryTree<int>.fromRoot(const BinaryTreeNode(1));
      expect(single.height, 0);
      expect(single.size, 1);
      expect(single.isFull, isTrue);
      expect(single.isComplete, isTrue);
      expect(single.isPerfect, isTrue);
    });
  });
}
