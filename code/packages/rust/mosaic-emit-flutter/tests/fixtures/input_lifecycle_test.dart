import 'package:flutter/material.dart' hide Typography;
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_typography/typography.dart';
import 'package:mosaic_typography/editors.dart';

void main() {
  testWidgets('scale and controlled echoes retain caret, composition and focus', (tester) async {
    var content = 'Workbook';
    var size = 18.0;
    late StateSetter update;
    await tester.pumpWidget(MaterialApp(home: Scaffold(body:
      StatefulBuilder(builder: (context, setState) {
        update = setState;
        return Typography(textSize: size, content: content, dispatch: (event) {
          if (event is TypographyEventEdit) {
            setState(() { content = event.value; });
          }
        });
      }))));
    final finder = find.byType(TextField).first;
    await tester.tap(finder);
    final controller = tester.widget<TextField>(finder).controller!;
    final composing = const TextEditingValue(text: 'Workbooks',
      selection: TextSelection.collapsed(offset: 3),
      composing: TextRange(start: 0, end: 4));
    tester.testTextInput.updateEditingValue(composing);
    await tester.pump();
    expect(content, 'Workbooks');
    expect(controller.value, composing);
    update(() { size = 27; });
    await tester.pump();
    expect(tester.widget<TextField>(finder).controller, same(controller));
    expect(controller.value, composing);
    expect(tester.widget<EditableText>(find.byType(EditableText).first).focusNode.hasFocus, isTrue);

    // External replacement clamps the selection and clears stale composition.
    controller.selection = const TextSelection(baseOffset: 8, extentOffset: 2);
    update(() { content = 'Hi'; });
    await tester.pump();
    expect(controller.text, 'Hi');
    expect(controller.selection, const TextSelection(baseOffset: 2, extentOffset: 2));
    expect(controller.value.composing, TextRange.empty);
    expect(tester.takeException(), isNull);

    await tester.pumpWidget(const SizedBox.shrink());
    expect(() => controller.addListener(() {}), throwsFlutterError);
  });

  testWidgets('legacy input retains local edits on presentation-only rebuild', (tester) async {
    Future<void> mount(double size) => tester.pumpWidget(MaterialApp(home: Scaffold(body:
      Typography(textSize: size, content: 'Workbook', dispatch: (_) {}))));
    await mount(18);
    final finder = find.byType(TextField).at(1);
    await tester.enterText(finder, 'Local edit');
    final controller = tester.widget<TextField>(finder).controller!;
    controller.selection = const TextSelection.collapsed(offset: 3);
    await mount(36);
    expect(tester.widget<TextField>(finder).controller, same(controller));
    expect(controller.text, 'Local edit');
    expect(controller.selection.baseOffset, 3);
  });

  testWidgets('table row editors own independent disposable controllers', (tester) async {
    Future<void> mount(List<String> rows) => tester.pumpWidget(MaterialApp(home: Scaffold(body:
      Editors(rows: rows, dispatch: (_) {}))));
    await mount(['First', 'Second']);
    final first = tester.widget<TextField>(find.byType(TextField).at(0)).controller!;
    final second = tester.widget<TextField>(find.byType(TextField).at(1)).controller!;
    expect(first, isNot(same(second)));
    first.selection = const TextSelection.collapsed(offset: 2);
    second.selection = const TextSelection.collapsed(offset: 4);
    await mount(['First', 'Second', 'Third']);
    expect(tester.widget<TextField>(find.byType(TextField).at(0)).controller, same(first));
    expect(tester.widget<TextField>(find.byType(TextField).at(1)).controller, same(second));
    expect(first.selection.baseOffset, 2);
    expect(second.selection.baseOffset, 4);
    await mount(['First', 'X']);
    expect(first.text, 'First');
    expect(second.text, 'X');
    expect(second.selection.baseOffset, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(() => first.addListener(() {}), throwsFlutterError);
    expect(() => second.addListener(() {}), throwsFlutterError);
    expect(tester.takeException(), isNull);
  });
}
