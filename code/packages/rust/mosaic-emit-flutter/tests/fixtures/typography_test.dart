import 'package:flutter/material.dart' hide Typography;
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_typography/typography.dart';

void main() {
  testWidgets('live typography preserves styles, fallback and interaction', (tester) async {
    final events = <TypographyEvent>[];
    Future<void> mount(double size) async {
      await tester.pumpWidget(MaterialApp(home: Scaffold(body:
        DefaultTextStyle(style: const TextStyle(fontSize: 19), child:
          Typography(textSize: size, content: 'Workbook', dispatch: events.add)))));
      await tester.pump();
    }

    double renderedSize(String label) {
      final richText = tester.widget<RichText>(find.descendant(
        of: find.text(label), matching: find.byType(RichText)).first);
      return richText.text.style!.fontSize!;
    }

    for (final size in [24.0, 36.0, 0.0, -1.0, double.nan, double.infinity, 27.0]) {
      await mount(size);
      final valid = size.isFinite && size > 0;
      expect(renderedSize('Title'), valid ? size : 18);
      expect(renderedSize('Action'), valid ? size : 14);
      expect(renderedSize('Inherited'), valid ? size : 19);
      final inputs = tester.widgetList<TextField>(find.byType(TextField)).toList();
      expect(inputs[0].style!.fontSize, valid ? size : 16);
      expect(inputs[1].style!.fontSize, valid ? size : 15);
      expect(inputs[0].style!.fontFamily, 'monospace');
      final title = tester.widget<Text>(find.text('Title'));
      expect(title.style!.color, const Color(0xFF123456));
      expect(title.style!.fontFamily, 'monospace');
      expect(tester.takeException(), isNull);
    }
    await tester.enterText(find.byType(TextField).first, 'Edited');
    expect(events.whereType<TypographyEventEdit>().last.value, 'Edited');
    await tester.tap(find.text('Action'));
    expect(events.whereType<TypographyEventAction>(), hasLength(1));
    expect(tester.takeException(), isNull);
  });
}
