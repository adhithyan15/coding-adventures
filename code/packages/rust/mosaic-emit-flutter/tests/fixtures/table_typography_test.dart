import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_typography/NativeSheet.dart';
import 'package:mosaic_typography/FallbackSheet.dart';

void main() {
  for (final native in [true, false]) {
    testWidgets('${native ? "native" : "fallback"} table typography and scope', (tester) async {
      Future<void> mount(double size, {int rows = 1}) async {
        final data = List.generate(rows, (i) => ['Row $i']);
        final sheet = native
            ? NativeSheet(textSize: size, headers: const ['Header'], rows: data, dispatch: (_) {})
            : FallbackSheet(textSize: size, headers: const ['Header'], rows: data, dispatch: (_) {});
        await tester.pumpWidget(MaterialApp(home: Scaffold(body:
          DefaultTextStyle(style: const TextStyle(fontSize: 19), child: sheet))));
        await tester.pump();
      }
      TextStyle style(String label) => tester.widget<RichText>(find.descendant(
        of: find.text(label), matching: find.byType(RichText)).first).text.style!;
      await mount(18);
      expect(find.byType(DataTable), native ? findsOneWidget : findsNothing);
      await tester.tap(find.byType(TextField).first);
      final controller = tester.widget<TextField>(find.byType(TextField).first).controller!;
      controller.selection = const TextSelection.collapsed(offset: 2);
      for (final size in [27.0, 36.0, 0.0, -1.0, double.nan, double.infinity, 24.0]) {
        await mount(size);
        final expected = size.isFinite && size > 0 ? size : 14;
        expect(style(native ? 'Header' : 'Cell').fontSize, expected);
        expect(style(native ? 'Header' : 'Cell').fontFamily, 'monospace');
        final inputs = tester.widgetList<TextField>(find.byType(TextField)).toList();
        expect(inputs.first.style!.fontSize, expected);
        expect(inputs.first.style!.fontFamily, 'monospace');
        expect(inputs.first.controller, same(controller));
        expect(controller.selection.baseOffset, 2);
        expect(tester.widget<EditableText>(find.byType(EditableText).first).focusNode.hasFocus, isTrue);
        expect(style('Outside').fontSize, 19);
        if (!native) {
          expect(style('Own').fontSize, 21);
          expect(inputs[1].style!.fontSize, 21);
          expect(style('Fixed').fontSize, 22);
          expect(style('Fixed').fontFamily, 'serif');
          expect(inputs[2].style!.fontFamily, 'serif');
          expect(inputs[2].style!.fontSize, 22);
          expect(style('Explicit').fontSize, 25);
        }
        expect(tester.takeException(), isNull);
      }
      if (native) {
        await mount(30, rows: 2);
        final inputs = tester.widgetList<TextField>(find.byType(TextField)).toList();
        expect(inputs, hasLength(2));
        expect(inputs[1].style!.fontSize, 30);
        expect(inputs[1].controller!.text, 'Row 1');
      }
    });
  }
}
