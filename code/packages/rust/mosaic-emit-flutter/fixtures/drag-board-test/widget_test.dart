import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_drag_board/DragBoard.dart';

void main() {
  Widget app(List<DragBoardEvent> events) => MaterialApp(
        home: Scaffold(body: DragBoard(dispatch: events.add)),
      );

  testWidgets('pointer drag dispatches an accepted drop', (tester) async {
    final events = <DragBoardEvent>[];
    await tester.pumpWidget(app(events));

    final gesture =
        await tester.startGesture(tester.getCenter(find.text('Write spec')));
    await tester.pump();
    await gesture.moveTo(tester.getCenter(find.text('Done')));
    await tester.pump();
    await gesture.up();
    await tester.pump();

    expect(events.map((event) => event.mosaicName), contains('onDragStart'));
    expect(events.map((event) => event.mosaicName), contains('onDragEnter'));
    expect(events.map((event) => event.mosaicName), contains('onDropHover'));
    expect(events.map((event) => event.mosaicName), contains('onDrop'));
    expect(events.whereType<DragBoardEventDragEnd>().single.dropped, isTrue);
    final drop = events.whereType<DragBoardEventDrop>().single;
    expect(drop.key, 'task-a');
    expect(drop.kind, 'task');
    expect(drop.targetKey, 'done');
    expect(drop.position, isIn(['before', 'into', 'after']));
  });

  testWidgets('pointer drag rejects an unaccepted kind', (tester) async {
    final events = <DragBoardEvent>[];
    await tester.pumpWidget(app(events));

    final gesture =
        await tester.startGesture(tester.getCenter(find.text('Write spec')));
    await tester.pump();
    await gesture.moveTo(tester.getCenter(find.text('Rejected')));
    await tester.pump();
    await gesture.up();
    await tester.pump();

    expect(events.whereType<DragBoardEventDrop>(), isEmpty);
    expect(events.whereType<DragBoardEventDragEnd>().single.dropped, isFalse);
  });

  testWidgets('keyboard and semantics use the accepted drop path', (tester) async {
    final semantics = tester.ensureSemantics();
    final events = <DragBoardEvent>[];
    await tester.pumpWidget(app(events));

    final draggableText = find.text('Write spec');
    final semanticsNode = tester.getSemantics(draggableText);
    final semanticsData = semanticsNode.getSemanticsData();
    expect(semanticsData.hasAction(SemanticsAction.tap), isTrue);
    // Flutter 3.24-compatible semantics action injection.
    // ignore: deprecated_member_use
    tester.binding.pipelineOwner.semanticsOwner!
        .performAction(semanticsNode.id, SemanticsAction.tap);
    await tester.pump();
    expect(events.map((event) => event.mosaicName), ['onDragStart']);

    Focus.of(tester.element(draggableText)).requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pump();

    expect(events.map((event) => event.mosaicName).toList(), [
      'onDragStart',
      'onDragEnter',
      'onDropHover',
      'onDrop',
      'onDragEnd',
    ]);
    expect(events.whereType<DragBoardEventDragEnd>().single.dropped, isTrue);
    expect(events.whereType<DragBoardEventDrop>().single.mosaicPayload, {
      'key': 'task-a',
      'kind': 'task',
      'targetKey': 'done',
      'position': 'into',
    });
    semantics.dispose();
  });

  testWidgets('Escape cancels and reports dropped false', (tester) async {
    final events = <DragBoardEvent>[];
    await tester.pumpWidget(app(events));

    final draggableText = find.text('Write spec');
    Focus.of(tester.element(draggableText)).requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();

    expect(events.map((event) => event.mosaicName).toList(), [
      'onDragStart',
      'onDragEnter',
      'onDropHover',
      'onDragLeave',
      'onDragEnd',
    ]);
    expect(events.whereType<DragBoardEventDragEnd>().single.dropped, isFalse);
  });
}
