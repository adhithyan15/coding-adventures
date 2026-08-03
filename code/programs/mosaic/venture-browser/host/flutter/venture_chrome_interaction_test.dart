import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_venture_chrome/main.dart';
import 'package:mosaic_venture_chrome/mosaic_host.dart';

class RecordingMosaicHost extends MosaicHost {
  RecordingMosaicHost({required this.navigationDisabled});

  final bool navigationDisabled;
  final List<Map<String, Object?>> events = <Map<String, Object?>>[];

  @override
  FutureOr<Map<String, Object?>?> props() => <String, Object?>{
    'props': <String, Object?>{
      'address': 'http://venture.test/start',
      'page-title': 'Venture Flutter Acceptance',
      'status-text': 'Ready',
      'back-disabled': true,
      'forward-disabled': true,
      'navigation-disabled': navigationDisabled,
      'content-surface': const Text('Flutter host surface'),
    },
  };

  @override
  FutureOr<Map<String, Object?>?> handleEvent(Map<String, Object?> event) {
    events.add(Map<String, Object?>.from(event));
    if (event['event'] == 'onNavigate') {
      return <String, Object?>{
        'props': <String, Object?>{
          'address': 'http://venture.test/next',
          'page-title': 'Venture Flutter Acceptance',
          'status-text': 'Navigated through MosaicHost',
          'back-disabled': false,
          'forward-disabled': true,
          'navigation-disabled': false,
          'content-surface': const Text('Flutter host surface'),
        },
      };
    }
    return null;
  }
}

Future<void> pumpVentureShell(
  WidgetTester tester,
  RecordingMosaicHost host,
) async {
  await tester.binding.setSurfaceSize(const Size(1400, 900));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(MosaicApp(mosaicHost: host));
  await tester.pumpAndSettle();
  expect(find.text('Venture Flutter Acceptance'), findsOneWidget);
  expect(find.text('Flutter host surface'), findsOneWidget);
}

void main() {
  testWidgets('disabled native controls suppress Mosaic dispatch', (
    WidgetTester tester,
  ) async {
    final host = RecordingMosaicHost(navigationDisabled: true);
    await pumpVentureShell(tester, host);

    expect(tester.widget<TextField>(find.byType(TextField)).readOnly, isTrue);
    for (final label in <String>['Back', 'Forward', 'Reload', 'Go']) {
      final button = tester.widget<ElevatedButton>(
        find.widgetWithText(ElevatedButton, label),
      );
      expect(button.onPressed, isNull, reason: '$label must be disabled');
      await tester.tap(find.text(label));
    }
    await tester.pump();
    expect(host.events, isEmpty);
  });

  testWidgets('address edit, Return, and Go cross the Mosaic host seam', (
    WidgetTester tester,
  ) async {
    final host = RecordingMosaicHost(navigationDisabled: false);
    await pumpVentureShell(tester, host);

    final input = find.byType(TextField);
    expect(tester.widget<TextField>(input).readOnly, isFalse);
    await tester.enterText(input, 'http://venture.test/next');
    await tester.pump();
    expect(host.events.last, <String, Object?>{
      'event': 'onAddressChange',
      'value': 'http://venture.test/next',
    });

    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(host.events.last['event'], 'onNavigate');
    expect(find.text('Navigated through MosaicHost'), findsOneWidget);

    await tester.tap(find.text('Go'));
    await tester.pumpAndSettle();
    expect(
      host.events.where((event) => event['event'] == 'onNavigate').length,
      2,
    );
  });
}
