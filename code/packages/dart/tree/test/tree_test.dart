/// Unit tests for `coding_adventures_tree`.
///
/// Mirrors the Rust reference's `tree_test.rs` case-for-case across its
/// eleven categories, then covers error value semantics, result-list
/// independence, error precedence, and name edge cases.
library;

import 'package:coding_adventures_tree/coding_adventures_tree.dart';
import 'package:test/test.dart';

//         A
//        / \
//       B   C
//      / \   \
//     D   E   F
//    /
//   G
Tree makeSampleTree() {
  final t = Tree('A');
  t.addChild('A', 'B');
  t.addChild('A', 'C');
  t.addChild('B', 'D');
  t.addChild('B', 'E');
  t.addChild('C', 'F');
  t.addChild('D', 'G');
  return t;
}

/// Expects [fn] to throw exactly [want] (compared with ==).
void expectTreeError(void Function() fn, TreeError want) {
  expect(fn, throwsA(want));
}

void main() {
  group('1. construction', () {
    test('create tree with root', () {
      expect(Tree('root').root, 'root');
    });

    test('new tree has size one', () {
      expect(Tree('root').size(), 1);
    });

    test('new tree root is leaf', () {
      expect(Tree('root').isLeaf('root'), isTrue);
    });

    test('new tree root is root', () {
      expect(Tree('root').isRoot('root'), isTrue);
    });

    test('new tree root has no parent', () {
      expect(Tree('root').parent('root'), isNull);
    });

    test('new tree root has no children', () {
      expect(Tree('root').children('root'), isEmpty);
    });

    test('new tree root has depth zero', () {
      expect(Tree('root').depth('root'), 0);
    });

    test('new tree height zero', () {
      expect(Tree('root').height(), 0);
    });

    test('new tree has root in nodes', () {
      expect(Tree('root').nodes(), contains('root'));
    });

    test('display', () {
      expect(Tree('root').toString(), 'Tree(root="root", size=1)');
    });
  });

  group('2. addChild', () {
    test('add one child', () {
      final t = Tree('root');
      t.addChild('root', 'child');
      expect(t.size(), 2);
    });

    test('child has correct parent', () {
      final t = Tree('root');
      t.addChild('root', 'child');
      expect(t.parent('child'), 'root');
    });

    test('parent has child in children list', () {
      final t = Tree('root');
      t.addChild('root', 'child');
      expect(t.children('root'), contains('child'));
    });

    test('add multiple children', () {
      final t = Tree('root');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      t.addChild('root', 'C');
      expect(t.children('root'), ['A', 'B', 'C']);
    });

    test('add child to non-root', () {
      final t = Tree('root');
      t.addChild('root', 'mid');
      t.addChild('mid', 'leaf');
      expect(t.parent('leaf'), 'mid');
    });

    test('build deep tree', () {
      final t = Tree('level0');
      for (var i = 1; i < 10; i++) {
        t.addChild('level${i - 1}', 'level$i');
      }
      expect(t.size(), 10);
      expect(t.depth('level9'), 9);
    });

    test('add child nonexistent parent', () {
      final t = Tree('root');
      expectTreeError(() => t.addChild('nonexistent', 'child'),
          const NodeNotFound('nonexistent'));
    });

    test('add duplicate child', () {
      final t = Tree('root');
      t.addChild('root', 'child');
      expectTreeError(
          () => t.addChild('root', 'child'), const DuplicateNode('child'));
    });

    test('add root as child', () {
      final t = Tree('root');
      expectTreeError(
          () => t.addChild('root', 'root'), const DuplicateNode('root'));
    });

    test('add child makes parent not leaf', () {
      final t = Tree('root');
      expect(t.isLeaf('root'), isTrue);
      t.addChild('root', 'child');
      expect(t.isLeaf('root'), isFalse);
    });

    test('new child is leaf', () {
      final t = Tree('root');
      t.addChild('root', 'child');
      expect(t.isLeaf('child'), isTrue);
    });
  });

  group('3. removeSubtree', () {
    test('remove leaf', () {
      final t = Tree('root');
      t.addChild('root', 'leaf');
      t.removeSubtree('leaf');
      expect(t.size(), 1);
      expect(t.hasNode('leaf'), isFalse);
    });

    test('remove subtree removes descendants', () {
      final t = makeSampleTree();
      t.removeSubtree('B');
      expect(t.size(), 3);
      expect(t.hasNode('B'), isFalse);
      expect(t.hasNode('D'), isFalse);
      expect(t.hasNode('E'), isFalse);
      expect(t.hasNode('G'), isFalse);
    });

    test('remove subtree preserves siblings', () {
      final t = makeSampleTree();
      t.removeSubtree('B');
      expect(t.hasNode('C'), isTrue);
      expect(t.hasNode('F'), isTrue);
      expect(t.children('A'), ['C']);
    });

    test('remove deep subtree', () {
      final t = makeSampleTree();
      t.removeSubtree('D');
      expect(t.size(), 5);
      expect(t.hasNode('D'), isFalse);
      expect(t.hasNode('G'), isFalse);
      expect(t.children('B'), ['E']);
    });

    test('remove root returns error', () {
      final t = Tree('root');
      expectTreeError(() => t.removeSubtree('root'), const RootRemoval());
    });

    test('remove nonexistent returns error', () {
      final t = Tree('root');
      expectTreeError(() => t.removeSubtree('nonexistent'),
          const NodeNotFound('nonexistent'));
    });

    test('remove then readd', () {
      final t = Tree('root');
      t.addChild('root', 'child');
      t.removeSubtree('child');
      t.addChild('root', 'child');
      expect(t.hasNode('child'), isTrue);
    });

    test('remove single child parent becomes leaf', () {
      final t = Tree('root');
      t.addChild('root', 'only_child');
      t.removeSubtree('only_child');
      expect(t.isLeaf('root'), isTrue);
    });
  });

  group('errors: display messages are specific', () {
    test('messages are exact', () {
      expect(const NodeNotFound('child').toString(),
          'node not found in tree: "child"');
      expect(const DuplicateNode('child').toString(),
          'node already exists in tree: "child"');
      expect(const RootRemoval().toString(), 'cannot remove the root node');
    });
  });

  group('4. queries: parent', () {
    test('parent of child', () {
      expect(makeSampleTree().parent('B'), 'A');
    });

    test('parent of grandchild', () {
      expect(makeSampleTree().parent('G'), 'D');
    });

    test('parent of root is none', () {
      expect(makeSampleTree().parent('A'), isNull);
    });

    test('parent nonexistent', () {
      expect(() => makeSampleTree().parent('Z'), throwsA(isA<TreeError>()));
    });
  });

  group('4. queries: children', () {
    test('children of root', () {
      expect(makeSampleTree().children('A'), ['B', 'C']);
    });

    test('children of internal', () {
      expect(makeSampleTree().children('B'), ['D', 'E']);
    });

    test('children of leaf', () {
      expect(makeSampleTree().children('G'), isEmpty);
    });

    test('children nonexistent', () {
      expect(() => makeSampleTree().children('Z'), throwsA(isA<TreeError>()));
    });
  });

  group('4. queries: siblings', () {
    test('siblings with sibling', () {
      expect(makeSampleTree().siblings('B'), ['C']);
    });

    test('siblings mutual', () {
      expect(makeSampleTree().siblings('C'), ['B']);
    });

    test('siblings only child', () {
      expect(makeSampleTree().siblings('F'), isEmpty);
    });

    test('siblings root', () {
      expect(makeSampleTree().siblings('A'), isEmpty);
    });

    test('siblings nonexistent', () {
      expect(() => makeSampleTree().siblings('Z'), throwsA(isA<TreeError>()));
    });

    test('siblings multiple', () {
      final t = Tree('root');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      t.addChild('root', 'C');
      t.addChild('root', 'D');
      expect(t.siblings('B'), ['A', 'C', 'D']);
    });
  });

  group('4. queries: leaf/root/depth/height', () {
    test('is leaf true', () {
      final t = makeSampleTree();
      expect(t.isLeaf('G'), isTrue);
      expect(t.isLeaf('E'), isTrue);
      expect(t.isLeaf('F'), isTrue);
    });

    test('is leaf false', () {
      final t = makeSampleTree();
      expect(t.isLeaf('A'), isFalse);
      expect(t.isLeaf('B'), isFalse);
    });

    test('is leaf nonexistent', () {
      expect(() => makeSampleTree().isLeaf('Z'), throwsA(isA<TreeError>()));
    });

    test('is root true', () {
      expect(makeSampleTree().isRoot('A'), isTrue);
    });

    test('is root false', () {
      expect(makeSampleTree().isRoot('B'), isFalse);
    });

    test('is root nonexistent', () {
      expect(() => makeSampleTree().isRoot('Z'), throwsA(isA<TreeError>()));
    });

    test('depth root', () {
      expect(makeSampleTree().depth('A'), 0);
    });

    test('depth level one', () {
      final t = makeSampleTree();
      expect(t.depth('B'), 1);
      expect(t.depth('C'), 1);
    });

    test('depth level two', () {
      final t = makeSampleTree();
      expect(t.depth('D'), 2);
      expect(t.depth('E'), 2);
      expect(t.depth('F'), 2);
    });

    test('depth level three', () {
      expect(makeSampleTree().depth('G'), 3);
    });

    test('depth nonexistent', () {
      expect(() => makeSampleTree().depth('Z'), throwsA(isA<TreeError>()));
    });

    test('height sample', () {
      expect(makeSampleTree().height(), 3);
    });

    test('height single node', () {
      expect(Tree('root').height(), 0);
    });

    test('height flat', () {
      final t = Tree('root');
      for (var i = 0; i < 5; i++) {
        t.addChild('root', 'child$i');
      }
      expect(t.height(), 1);
    });

    test('height deep chain', () {
      final t = Tree('0');
      for (var i = 1; i < 20; i++) {
        t.addChild('${i - 1}', '$i');
      }
      expect(t.height(), 19);
    });
  });

  group('4. queries: size/nodes/leaves/hasNode', () {
    test('size sample', () {
      expect(makeSampleTree().size(), 7);
    });

    test('size after add', () {
      final t = Tree('root');
      expect(t.size(), 1);
      t.addChild('root', 'A');
      expect(t.size(), 2);
    });

    test('nodes returns all', () {
      expect(makeSampleTree().nodes(), ['A', 'B', 'C', 'D', 'E', 'F', 'G']);
    });

    test('leaves sample', () {
      expect(makeSampleTree().leaves(), ['E', 'F', 'G']);
    });

    test('leaves single node', () {
      expect(Tree('root').leaves(), ['root']);
    });

    test('leaves flat', () {
      final t = Tree('root');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      t.addChild('root', 'C');
      expect(t.leaves(), ['A', 'B', 'C']);
    });

    test('has node true', () {
      expect(makeSampleTree().hasNode('A'), isTrue);
    });

    test('has node false', () {
      expect(makeSampleTree().hasNode('Z'), isFalse);
    });
  });

  group('5. traversals', () {
    test('preorder sample', () {
      expect(makeSampleTree().preorder(), ['A', 'B', 'D', 'G', 'E', 'C', 'F']);
    });

    test('preorder single node', () {
      expect(Tree('root').preorder(), ['root']);
    });

    test('preorder flat', () {
      final t = Tree('root');
      t.addChild('root', 'C');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      expect(t.preorder(), ['root', 'A', 'B', 'C']);
    });

    test('preorder deep chain', () {
      final t = Tree('A');
      t.addChild('A', 'B');
      t.addChild('B', 'C');
      expect(t.preorder(), ['A', 'B', 'C']);
    });

    test('postorder sample', () {
      expect(makeSampleTree().postorder(), ['G', 'D', 'E', 'B', 'F', 'C', 'A']);
    });

    test('postorder single node', () {
      expect(Tree('root').postorder(), ['root']);
    });

    test('postorder flat', () {
      final t = Tree('root');
      t.addChild('root', 'C');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      expect(t.postorder(), ['A', 'B', 'C', 'root']);
    });

    test('postorder deep chain', () {
      final t = Tree('A');
      t.addChild('A', 'B');
      t.addChild('B', 'C');
      expect(t.postorder(), ['C', 'B', 'A']);
    });

    test('level order sample', () {
      expect(
          makeSampleTree().levelOrder(), ['A', 'B', 'C', 'D', 'E', 'F', 'G']);
    });

    test('level order single node', () {
      expect(Tree('root').levelOrder(), ['root']);
    });

    test('level order flat', () {
      final t = Tree('root');
      t.addChild('root', 'C');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      expect(t.levelOrder(), ['root', 'A', 'B', 'C']);
    });

    test('level order deep chain', () {
      final t = Tree('A');
      t.addChild('A', 'B');
      t.addChild('B', 'C');
      expect(t.levelOrder(), ['A', 'B', 'C']);
    });

    test('traversals same length', () {
      final t = makeSampleTree();
      expect(t.preorder().length, 7);
      expect(t.postorder().length, 7);
      expect(t.levelOrder().length, 7);
    });

    test('traversals same elements', () {
      final t = makeSampleTree();
      final pre = t.preorder()..sort();
      final post = t.postorder()..sort();
      final level = t.levelOrder()..sort();
      expect(pre, equals(post));
      expect(pre, equals(level));
    });

    test('preorder root first', () {
      expect(makeSampleTree().preorder()[0], 'A');
    });

    test('postorder root last', () {
      final post = makeSampleTree().postorder();
      expect(post[post.length - 1], 'A');
    });

    test('level order root first', () {
      expect(makeSampleTree().levelOrder()[0], 'A');
    });
  });

  group('6. pathTo', () {
    test('path to root', () {
      expect(makeSampleTree().pathTo('A'), ['A']);
    });

    test('path to child', () {
      expect(makeSampleTree().pathTo('B'), ['A', 'B']);
    });

    test('path to grandchild', () {
      expect(makeSampleTree().pathTo('D'), ['A', 'B', 'D']);
    });

    test('path to deep node', () {
      expect(makeSampleTree().pathTo('G'), ['A', 'B', 'D', 'G']);
    });

    test('path to right branch', () {
      expect(makeSampleTree().pathTo('F'), ['A', 'C', 'F']);
    });

    test('path to nonexistent', () {
      expect(() => makeSampleTree().pathTo('Z'), throwsA(isA<TreeError>()));
    });

    test('path length equals depth plus one', () {
      final t = makeSampleTree();
      for (final node in t.nodes()) {
        expect(t.pathTo(node).length, t.depth(node) + 1);
      }
    });
  });

  group('7. lca', () {
    test('lca same node', () {
      expect(makeSampleTree().lca('D', 'D'), 'D');
    });

    test('lca siblings', () {
      expect(makeSampleTree().lca('D', 'E'), 'B');
    });

    test('lca parent child', () {
      expect(makeSampleTree().lca('B', 'D'), 'B');
    });

    test('lca child parent', () {
      expect(makeSampleTree().lca('D', 'B'), 'B');
    });

    test('lca cousins', () {
      expect(makeSampleTree().lca('D', 'F'), 'A');
    });

    test('lca root and leaf', () {
      expect(makeSampleTree().lca('A', 'G'), 'A');
    });

    test('lca deep nodes', () {
      expect(makeSampleTree().lca('G', 'E'), 'B');
    });

    test('lca leaves different subtrees', () {
      expect(makeSampleTree().lca('G', 'F'), 'A');
    });

    test('lca nonexistent a', () {
      expect(() => makeSampleTree().lca('Z', 'A'), throwsA(isA<TreeError>()));
    });

    test('lca nonexistent b', () {
      expect(() => makeSampleTree().lca('A', 'Z'), throwsA(isA<TreeError>()));
    });

    test('lca root with root', () {
      expect(makeSampleTree().lca('A', 'A'), 'A');
    });
  });

  group('8. subtree', () {
    test('subtree leaf', () {
      final sub = makeSampleTree().subtree('G');
      expect(sub.root, 'G');
      expect(sub.size(), 1);
    });

    test('subtree internal', () {
      final sub = makeSampleTree().subtree('B');
      expect(sub.root, 'B');
      expect(sub.size(), 4);
      expect(sub.hasNode('D'), isTrue);
      expect(sub.hasNode('E'), isTrue);
      expect(sub.hasNode('G'), isTrue);
    });

    test('subtree preserves structure', () {
      final sub = makeSampleTree().subtree('B');
      expect(sub.children('B'), ['D', 'E']);
      expect(sub.children('D'), ['G']);
      expect(sub.isLeaf('G'), isTrue);
      expect(sub.isLeaf('E'), isTrue);
    });

    test('subtree root', () {
      final t = makeSampleTree();
      final sub = t.subtree('A');
      expect(sub.size(), t.size());
      expect(sub.nodes(), t.nodes());
    });

    test('subtree does not modify original', () {
      final t = makeSampleTree();
      final origSize = t.size();
      t.subtree('B');
      expect(t.size(), origSize);
    });

    test('subtree nonexistent', () {
      expect(() => makeSampleTree().subtree('Z'), throwsA(isA<TreeError>()));
    });

    test('subtree is independent', () {
      final t = makeSampleTree();
      final sub = t.subtree('B');
      sub.addChild('E', 'new_node');
      expect(t.hasNode('new_node'), isFalse);
    });

    test('subtree right branch', () {
      final sub = makeSampleTree().subtree('C');
      expect(sub.root, 'C');
      expect(sub.size(), 2);
      expect(sub.children('C'), ['F']);
    });
  });

  group('9. toAscii', () {
    test('ascii single node', () {
      expect(Tree('root').toAscii(), 'root');
    });

    test('ascii root with one child', () {
      final t = Tree('root');
      t.addChild('root', 'child');
      expect(t.toAscii(), 'root\n└── child');
    });

    test('ascii root with two children', () {
      final t = Tree('root');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      expect(t.toAscii(), 'root\n├── A\n└── B');
    });

    test('ascii sample tree', () {
      const expected = 'A\n'
          '├── B\n'
          '│   ├── D\n'
          '│   │   └── G\n'
          '│   └── E\n'
          '└── C\n'
          '    └── F';
      expect(makeSampleTree().toAscii(), expected);
    });

    test('ascii deep chain', () {
      final t = Tree('A');
      t.addChild('A', 'B');
      t.addChild('B', 'C');
      expect(t.toAscii(), 'A\n└── B\n    └── C');
    });

    test('ascii wide tree', () {
      final t = Tree('root');
      t.addChild('root', 'A');
      t.addChild('root', 'B');
      t.addChild('root', 'C');
      t.addChild('root', 'D');
      expect(t.toAscii(), 'root\n├── A\n├── B\n├── C\n└── D');
    });
  });

  group('10. edge cases', () {
    test('single node traversals', () {
      final t = Tree('solo');
      expect(t.preorder(), ['solo']);
      expect(t.postorder(), ['solo']);
      expect(t.levelOrder(), ['solo']);
    });

    test('single node leaves', () {
      expect(Tree('solo').leaves(), ['solo']);
    });

    test('deep chain height', () {
      final t = Tree('n0');
      for (var i = 1; i < 100; i++) {
        t.addChild('n${i - 1}', 'n$i');
      }
      expect(t.height(), 99);
      expect(t.size(), 100);
    });

    test('wide tree height', () {
      final t = Tree('root');
      for (var i = 0; i < 100; i++) {
        t.addChild('root', 'child$i');
      }
      expect(t.height(), 1);
      expect(t.size(), 101);
    });

    test('balanced binary tree', () {
      final t = Tree('1');
      t.addChild('1', '2');
      t.addChild('1', '3');
      t.addChild('2', '4');
      t.addChild('2', '5');
      t.addChild('3', '6');
      t.addChild('3', '7');
      expect(t.size(), 7);
      expect(t.height(), 2);
      expect(t.leaves(), ['4', '5', '6', '7']);
    });

    test('node names with spaces', () {
      final t = Tree('my root');
      t.addChild('my root', 'my child');
      expect(t.parent('my child'), 'my root');
    });

    test('node names with special chars', () {
      final t = Tree('root:main');
      t.addChild('root:main', 'child.1');
      expect(t.hasNode('child.1'), isTrue);
    });

    test('path to single node', () {
      expect(Tree('solo').pathTo('solo'), ['solo']);
    });

    test('lca single node', () {
      expect(Tree('solo').lca('solo', 'solo'), 'solo');
    });

    test('subtree single node', () {
      final sub = Tree('solo').subtree('solo');
      expect(sub.root, 'solo');
      expect(sub.size(), 1);
    });

    test('remove and rebuild', () {
      final t = Tree('root');
      t.addChild('root', 'A');
      t.addChild('A', 'B');
      t.removeSubtree('A');
      t.addChild('root', 'A');
      t.addChild('A', 'C');
      expect(t.children('A'), ['C']);
      expect(t.hasNode('B'), isFalse);
    });
  });

  group('11. graph', () {
    test('graph has correct nodes', () {
      // The Dart graph already returns nodes sorted.
      expect(
          makeSampleTree().graph.nodes(), ['A', 'B', 'C', 'D', 'E', 'F', 'G']);
    });

    test('graph edge count', () {
      expect(makeSampleTree().graph.edges().length, 6);
    });

    test('graph has no cycles', () {
      expect(makeSampleTree().graph.hasCycle(), isFalse);
    });

    test('graph topological sort starts with root', () {
      expect(makeSampleTree().graph.topologicalSort()[0], 'A');
    });
  });

  group('EXTRA: errors and semantics', () {
    test('errors compare by value', () {
      const a = NodeNotFound('x');
      const b = NodeNotFound('x');
      const c = NodeNotFound('y');
      expect(a, equals(b));
      expect(a.hashCode, b.hashCode);
      expect(a == c, isFalse);
      expect(a == const DuplicateNode('x'), isFalse);
      expect(a == 'x', isFalse);
      expect(const DuplicateNode('x') == const DuplicateNode('x'), isTrue);
      expect(
          const DuplicateNode('x').hashCode, const DuplicateNode('x').hashCode);
      expect(const RootRemoval() == const RootRemoval(), isTrue);
      expect(const RootRemoval().hashCode, const RootRemoval().hashCode);
      expect(const RootRemoval() == const NodeNotFound('x'), isFalse);
    });

    test('addChild checks parent before duplicate', () {
      final t = Tree('root');
      t.addChild('root', 'taken');
      expectTreeError(
          () => t.addChild('ghost', 'taken'), const NodeNotFound('ghost'));
    });

    test('lca checks a before b', () {
      final t = makeSampleTree();
      expectTreeError(
          () => t.lca('ghost1', 'ghost2'), const NodeNotFound('ghost1'));
    });

    test('removeSubtree checks existence before root', () {
      final t = Tree('A');
      expectTreeError(
          () => t.removeSubtree('root'), const NodeNotFound('root'));
    });

    test('returned lists are independent copies', () {
      final t = makeSampleTree();
      t.children('A').add('X');
      t.nodes().clear();
      t.leaves().clear();
      t.preorder().clear();
      expect(t.size(), 7);
      expect(t.children('A'), ['B', 'C']);
    });

    test('empty and unicode names work', () {
      final t = Tree('');
      t.addChild('', 'café');
      expect(t.parent('café'), '');
      expect(t.depth('café'), 1);
    });

    test('toString shows root and size', () {
      expect(makeSampleTree().toString(), 'Tree(root="A", size=7)');
    });

    test('graph snapshot is detached from the tree', () {
      final t = makeSampleTree();
      final snapshot = t.graph;
      // Mutating the snapshot (even cyclically) cannot touch the tree.
      snapshot.addEdge('C', 'A');
      snapshot.removeNode('B');
      expect(snapshot.hasCycle(), isTrue);
      expect(t.size(), 7);
      expect(t.graph.hasCycle(), isFalse);
      expect(t.parent('B'), 'A');
      expect(t.depth('G'), 3);
    });

    test('postorder and ascii survive very deep chains', () {
      // The reference recurses here and overflows past ~20k depth; this
      // port iterates. 20,000 nodes also stress addChild linearly.
      final t = Tree('n0');
      for (var i = 1; i < 20000; i++) {
        t.addChild('n${i - 1}', 'n$i');
      }
      final post = t.postorder();
      expect(post.length, 20000);
      expect(post.first, 'n19999');
      expect(post.last, 'n0');
      final ascii = t.toAscii();
      expect(ascii.split('\n').length, 20000);
      expect(ascii.startsWith('n0\n'), isTrue);
    });
  });
}
