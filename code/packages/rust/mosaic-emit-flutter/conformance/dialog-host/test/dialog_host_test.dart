// _MosaicDialogHost, the widget every Flutter HostDialog lowers to (UI29-1
// §3.3), driven with the widget tester. tests/flutter_dialog_host.rs writes
// lib/dialog_host.dart from the emitter's own helper, unedited, plus a public
// alias (`MosaicDialogHost`) so this file can name the private class.
//
// What a component sees: `open` lives in its state, and its close event sets
// `open` back to false, as a Mosaic app's reducer does. Every way a dialog
// closes -- barrier, Escape, a control inside it that pops, the host -- must
// report exactly one close per open, and the dialog must never take another
// route with it or outlive its component.

import 'package:mosaic_dialog_host_conformance/dialog_host.dart';
import 'package:flutter/services.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

class Harness extends StatefulWidget {
  const Harness({super.key, this.initiallyOpen = false, this.barrierDismissible = true, this.show = true});
  final bool initiallyOpen;
  final bool barrierDismissible;
  final bool show;
  @override
  State<Harness> createState() => HarnessState();
}

class HarnessState extends State<Harness> {
  late bool open = widget.initiallyOpen;
  int closes = 0;
  void setOpen(bool value) => setState(() => open = value);

  @override
  Widget build(BuildContext context) => Column(children: [
        const Text('page'),
        if (widget.show)
          MosaicDialogHost(
            open: open,
            barrierDismissible: widget.barrierDismissible,
            onClose: () => setState(() {
              closes += 1;
              open = false;
            }),
            builder: (context) => const AlertDialog(content: Text('in the dialog')),
          ),
      ]);
}

Future<HarnessState> mount(WidgetTester tester, Widget harness) async {
  await tester.pumpWidget(MaterialApp(home: Scaffold(body: harness)));
  await tester.pumpAndSettle();
  return tester.state<HarnessState>(find.byType(Harness));
}

void main() {
  testWidgets('open on first build shows the dialog; a barrier tap closes it once', (tester) async {
    final state = await mount(tester, const Harness(initiallyOpen: true));
    expect(find.text('in the dialog'), findsOneWidget);
    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();
    expect(find.text('in the dialog'), findsNothing);
    expect(state.closes, 1);
    expect(state.open, isFalse);
  });

  testWidgets('the host closing it removes it and reports one close', (tester) async {
    final state = await mount(tester, const Harness());
    expect(find.text('in the dialog'), findsNothing);
    state.setOpen(true);
    await tester.pumpAndSettle();
    expect(find.text('in the dialog'), findsOneWidget);
    state.setOpen(false);
    await tester.pumpAndSettle();
    expect(find.text('in the dialog'), findsNothing);
    expect(state.closes, 1);
    expect(find.text('page'), findsOneWidget);
  });

  testWidgets('several rebuilds before the frame push one dialog', (tester) async {
    final state = await mount(tester, const Harness());
    // Two rebuilds that both see `open` with nothing pushed yet, each
    // scheduling an open for after its frame.
    state.setOpen(true);
    await tester.pump(Duration.zero);
    state.setOpen(true);
    state.setState(() {});
    await tester.pumpAndSettle();
    expect(find.text('in the dialog'), findsOneWidget);
    expect(find.byType(AlertDialog), findsOneWidget);
    // Close it: one pop leaves the page, not another dialog.
    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(state.closes, 1);
  });

  testWidgets('reopening after a close works, and closes count per open', (tester) async {
    final state = await mount(tester, const Harness());
    for (var i = 1; i <= 3; i++) {
      state.setOpen(true);
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      state.setOpen(false);
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(state.closes, i);
    }
  });

  testWidgets('a host close takes out its own route, not one pushed above it', (tester) async {
    final state = await mount(tester, const Harness(initiallyOpen: true));
    final navigator = tester.state<NavigatorState>(find.byType(Navigator).first);
    navigator.push(MaterialPageRoute<void>(builder: (_) => const Scaffold(body: Text('above'))));
    await tester.pumpAndSettle();
    expect(find.text('above'), findsOneWidget);
    state.setOpen(false);
    await tester.pumpAndSettle();
    expect(find.text('above'), findsOneWidget, reason: 'the page above the dialog stays');
    expect(state.closes, 1);
    navigator.pop();
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing, reason: 'the dialog was removed underneath');
    expect(find.text('page'), findsOneWidget);
    expect(state.closes, 1);
  });

  testWidgets('barrierDismissible: false keeps it open on a barrier tap', (tester) async {
    final state = await mount(tester, const Harness(initiallyOpen: true, barrierDismissible: false));
    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(state.closes, 0);
  });

  testWidgets('a control inside the dialog that pops closes it once', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: Scaffold(body: _PopHarness())));
    await tester.pumpAndSettle();
    final state = tester.state<_PopHarnessState>(find.byType(_PopHarness));
    await tester.tap(find.text('done'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(state.closes, 1);
  });

  testWidgets('a component that leaves the tree takes its dialog with it, with no close dispatched', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: Scaffold(body: Harness(initiallyOpen: true))));
    await tester.pumpAndSettle();
    final state = tester.state<HarnessState>(find.byType(Harness));
    expect(find.byType(AlertDialog), findsOneWidget);
    await tester.pumpWidget(const MaterialApp(home: Scaffold(body: Harness(initiallyOpen: true, show: false))));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(state.closes, 0);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Escape closes it once', (tester) async {
    final state = await mount(tester, const Harness(initiallyOpen: true));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(state.closes, 1);
  });
}

class _PopHarness extends StatefulWidget {
  const _PopHarness();
  @override
  State<_PopHarness> createState() => _PopHarnessState();
}

class _PopHarnessState extends State<_PopHarness> {
  bool open = true;
  int closes = 0;
  @override
  Widget build(BuildContext context) => MosaicDialogHost(
        open: open,
        onClose: () => setState(() {
          closes += 1;
          open = false;
        }),
        builder: (context) => AlertDialog(
          actions: [TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('done'))],
        ),
      );
}
