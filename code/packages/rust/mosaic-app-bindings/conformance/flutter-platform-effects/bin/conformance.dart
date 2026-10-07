// Behaviour of the Flutter platform library (UI87 §7.7) without a display or
// a Flutter engine: the dialogs are replaced by a fake that returns a chosen
// path (or cancels), and the host by a fake that records deferrals and
// answers, so the real open/save logic, limits and routing run as they would
// in an app -- the same cases as the Compose library's
// MosaicPlatformEffectsTest.kt, the SwiftUI harness's PlatformEffectsChecks
// and the XAML harness.
//
// Run on the plain Dart VM, after copying a generated Flutter project's
// lib/mosaic_host.dart and lib/mosaic_platform_effects_core.dart into lib/:
//
//   dart pub get
//   dart run bin/conformance.dart
//
// CI does this in the Flutter runtime lane, against TaskApp's generated
// project. `tests/flutter_platform_effects.rs` does it wherever `dart` is
// installed, against the files mosaic-app-bindings emits.

import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';

import 'package:mosaic_flutter_platform_effects_conformance/mosaic_host.dart';
import 'package:mosaic_flutter_platform_effects_conformance/mosaic_platform_effects_core.dart';

var _checks = 0;

void check(bool condition, String assertion) {
  _checks += 1;
  if (!condition) {
    throw StateError('Failed platform-effects assertion: $assertion');
  }
}

/// A dialog that answers with a fixed choice (null = the person cancelled),
/// or throws with a message that looks like a path -- which must never reach
/// the app.
final class FakeDialogs implements MosaicFileDialogs {
  FakeDialogs(this.choice, {this.throws = false});

  final String? choice;
  final bool throws;
  int opened = 0;
  List<String> lastExtensions = const <String>[];
  String? lastSuggestedName;

  @override
  Future<String?> chooseFileToOpen(List<String> extensions) async {
    opened += 1;
    lastExtensions = List<String>.of(extensions);
    if (throws) throw StateError('/home/someone/secret path');
    return choice;
  }

  @override
  Future<String?> chooseFileToSave(
    String suggestedName,
    List<String> extensions,
  ) async {
    opened += 1;
    lastExtensions = List<String>.of(extensions);
    lastSuggestedName = suggestedName;
    if (throws) throw StateError('/home/someone/secret path');
    return choice;
  }
}

/// One runtime's effect surface. The library answers after awaits (the
/// dialog, then the background isolate), so [answer] waits for one.
final class FakeHost implements MosaicPlatformEffectHost {
  @override
  MosaicEffectHandler? effectHandler;

  Set<int> waitingOn = <int>{};
  final List<int> deferred = <int>[];
  final Map<int, String> answers = <int, String>{};
  int completeAttempts = 0;

  /// Disposed, as the generated host is after a retried start: deferral is
  /// refused and an answer throws, which is what `MosaicHost` does once its
  /// runtime is disposed.
  bool closed = false;

  @override
  bool deferEffect(int id) {
    if (closed || !waitingOn.contains(id)) return false;
    deferred.add(id);
    return true;
  }

  @override
  void completeEffect(int id, Map<String, Object?> result) {
    completeAttempts += 1;
    if (closed) throw StateError('Mosaic application is closed');
    // Recorded as the JSON the real host would hand the runtime.
    answers[id] = jsonEncode(result);
    waitingOn.remove(id);
  }

  /// The answer for [id], waiting up to ten seconds for it.
  Future<String?> answer(int id) async {
    for (var tries = 0; tries < 2000 && !answers.containsKey(id); tries += 1) {
      await Future<void>.delayed(const Duration(milliseconds: 5));
    }
    return answers[id];
  }

  Future<void> waitForAttempts(int count) async {
    for (var tries = 0; tries < 2000 && completeAttempts < count; tries += 1) {
      await Future<void>.delayed(const Duration(milliseconds: 5));
    }
  }
}

String encoded(String text) => base64Encode(utf8.encode(text));

Map<String, Object?> payload(Map<String, Object?> fields) =>
    jsonDecode(jsonEncode(fields)) as Map<String, Object?>;

String? failure(Map<String, Object?> outcome) {
  final failed = outcome['failed'];
  return failed is Map ? failed['message'] as String? : null;
}

bool isCancelled(Map<String, Object?> outcome) =>
    outcome.length == 1 &&
    outcome['cancelled'] is Map &&
    (outcome['cancelled'] as Map).isEmpty;

Map<String, Object?>? okValue(Map<String, Object?> outcome) {
  final ok = outcome['ok'];
  return ok is Map<String, Object?> ? ok : null;
}

Future<String?> answeredFailure(FakeHost host, int id) async {
  final json = await host.answer(id);
  if (json == null) return null;
  final root = jsonDecode(json);
  return root is Map && root['failed'] is Map
      ? root['failed']['message'] as String?
      : null;
}

Future<String?> answeredName(FakeHost host, int id) async {
  final json = await host.answer(id);
  if (json == null) return null;
  final root = jsonDecode(json);
  return root is Map && root['ok'] is Map ? root['ok']['name'] as String? : null;
}

/// `\uXXXX` for every non-printable-ASCII UTF-16 unit, so a hostile name
/// reaches the library exactly as a JSON payload would carry it -- a lone
/// surrogate included. (The lesson from the XAML harness: an encoder asked to
/// write a malformed string "repairs" it, and the test then proves nothing.)
String jsonEscape(String text) {
  final out = StringBuffer();
  for (final unit in text.codeUnits) {
    if (unit >= 0x20 && unit <= 0x7E && unit != 0x22 && unit != 0x5C) {
      out.writeCharCode(unit);
    } else {
      out.write('\\u${unit.toRadixString(16).padLeft(4, '0')}');
    }
  }
  return out.toString();
}

String shown(String text) {
  final clipped = text.length > 40 ? '${text.substring(0, 40)}...' : text;
  final out = StringBuffer();
  for (final unit in clipped.codeUnits) {
    if (unit >= 0x20 && unit <= 0x7E) {
      out.writeCharCode(unit);
    } else {
      out.write('\\u${unit.toRadixString(16).toUpperCase().padLeft(4, '0')}');
    }
  }
  return out.toString();
}

bool leftovers(Directory directory) => directory
    .listSync()
    .any((entry) => entry.path.split('/').last.startsWith('.mosaic-save-'));

int mode(String path) => File(path).statSync().mode & 0xFFF;

void checkRouting() {
  // The app claims a kind: it goes to the app, even a standard one.
  check(
    mosaicRoutesToPlatform('files.save', {'files.save'}) == false,
    'claimed standard',
  );
  // A standard kind nobody claimed: the platform library.
  check(
    mosaicRoutesToPlatform('files.open', {'importAnki'}) == true,
    'unclaimed open',
  );
  check(mosaicRoutesToPlatform('files.save', null) == true, 'unclaimed save');
  // A non-standard kind: the app when it claimed nothing (the original
  // meaning), nobody when it named its kinds and this is not one.
  check(mosaicRoutesToPlatform('importAnki', null) == false, 'unclaimed custom');
  check(
    mosaicRoutesToPlatform('somethingElse', {'importAnki'}) == null,
    'unowned custom',
  );
}

Future<void> checkSave(Directory directory) async {
  final target = '${directory.path}/journal-2026-09-25.json';
  final dialogs = FakeDialogs(target);
  final outcome = await mosaicRunFilesSave(
    payload({
      'suggestedName': 'journal-2026-09-25.json',
      'accept': ['application/json'],
      'bytes': encoded('{"version":1}'),
    }),
    dialogs,
  );
  check(okValue(outcome)?['name'] == 'journal-2026-09-25.json', 'save ok name');
  check(okValue(outcome)!.length == 1, 'save answers only the name');
  check(File(target).readAsStringSync() == '{"version":1}', 'saved bytes');
  check(
    dialogs.lastExtensions.join(',') == 'json',
    'save dialog filtered to json',
  );
  check(
    dialogs.lastSuggestedName == 'journal-2026-09-25.json',
    'suggested name reached the dialog',
  );
  check(!leftovers(directory), 'no temporary left behind');

  // Saving over an existing file replaces it, in place, whole.
  final existing = '${directory.path}/existing.txt';
  File(existing).writeAsStringSync('a much longer old body that must not survive');
  check(
    okValue(
          await mosaicRunFilesSave(
            payload({'suggestedName': 'existing.txt', 'bytes': encoded('new')}),
            FakeDialogs(existing),
          ),
        ) !=
        null,
    'save over an existing file',
  );
  check(
    File(existing).readAsStringSync() == 'new',
    'replaced contents, nothing of the old left',
  );

  // A target that cannot be written is a failure without the path.
  final folder = '${directory.path}/a-folder.txt';
  Directory(folder).createSync();
  final blocked = await mosaicRunFilesSave(
    payload({'suggestedName': 'a-folder.txt', 'bytes': encoded('x')}),
    FakeDialogs(folder),
  );
  check(
    failure(blocked) == "couldn't save the file",
    'unwritable target: ${failure(blocked)}',
  );
  check(!leftovers(directory), 'a failed save leaves no temporary');

  if (Platform.isWindows) return;
  // POSIX only: saving over a private file keeps it private.
  final secret = '${directory.path}/secret.json';
  File(secret).writeAsStringSync('old');
  Process.runSync('chmod', ['600', secret]);
  check(
    okValue(
          await mosaicRunFilesSave(
            payload({'suggestedName': 'secret.json', 'bytes': encoded('new')}),
            FakeDialogs(secret),
          ),
        ) !=
        null,
    'save over a private file',
  );
  check(
    mode(secret) == 0x180,
    'replaced file keeps 0600, got ${mode(secret).toRadixString(8)}',
  );
  check(File(secret).readAsStringSync() == 'new', 'replaced private contents');

  // Only the rwx bits carry over: never setuid, setgid or sticky.
  final tool = '${directory.path}/tool.txt';
  File(tool).writeAsStringSync('old');
  Process.runSync('chmod', ['4755', tool]);
  check(mode(tool) == 0x9ED, 'the fixture is setuid 4755');
  check(
    okValue(
          await mosaicRunFilesSave(
            payload({'suggestedName': 'tool.txt', 'bytes': encoded('new')}),
            FakeDialogs(tool),
          ),
        ) !=
        null,
    'save over a setuid file',
  );
  check(
    mode(tool) == 0x1ED,
    'setuid is not copied, got ${mode(tool).toRadixString(8)}',
  );

  // A new file stays owner-only.
  final fresh = '${directory.path}/fresh.txt';
  await mosaicRunFilesSave(
    payload({'suggestedName': 'fresh.txt', 'bytes': encoded('x')}),
    FakeDialogs(fresh),
  );
  check(
    mode(fresh) == 0x180,
    'a new file is 0600, got ${mode(fresh).toRadixString(8)}',
  );

  // A chosen path that is a symlink: the link is replaced by the saved file
  // (rename semantics, as Compose and Qt), and the file it pointed at is
  // untouched.
  final pointedAt = '${directory.path}/pointed-at.txt';
  File(pointedAt).writeAsStringSync('keep me');
  final link = '${directory.path}/link.txt';
  Link(link).createSync(pointedAt);
  check(
    okValue(
          await mosaicRunFilesSave(
            payload({'suggestedName': 'link.txt', 'bytes': encoded('saved')}),
            FakeDialogs(link),
          ),
        ) !=
        null,
    'save over a chosen symlink',
  );
  check(
    FileSystemEntity.typeSync(link, followLinks: false) ==
            FileSystemEntityType.file &&
        File(link).readAsStringSync() == 'saved' &&
        File(pointedAt).readAsStringSync() == 'keep me',
    'the link is replaced, its target untouched',
  );
  check(
    mode(link) == 0x180,
    'a replaced link does not lend its 0777 bits, got ${mode(link).toRadixString(8)}',
  );
  check(!leftovers(directory), 'no temporary left after the POSIX saves');
}

Future<void> checkSaveRefusals(Directory directory) async {
  final cancelled = await mosaicRunFilesSave(
    payload({'suggestedName': 'a.json', 'bytes': encoded('x')}),
    FakeDialogs(null),
  );
  check(isCancelled(cancelled), 'a cancelled dialog is not a failure');

  final target = '${directory.path}/never.txt';
  for (final name in <String>[
    '', '.', '..', '../escape.json', 'dir/a.json', 'a\\b.json', 'a\u0000b',
    'bell\u0007.json', 'D:report.json', 'notes.json:stream',
    'invoice\u202Efdp.exe', 'trailing.', 'trailing ',
    // Tightened after the SwiftUI library's security review (UI87 §7):
    '.zshrc', ' leading.json', 'nbsp\u00A0', '\u3000ideographic.json',
    'line\u2028break.json', 'para\u2029break.json',
    'tag\u{E0001}.json', // LANGUAGE TAG, a format character outside the BMP
    'x' * 256, 'Invoice.pdf      .command',
    '.\u0301zshrc', 'Invoice.pdf\u2800\u2800.txt', 'x\u0D4E.',
    'a\uD800.json', 'a\uDC00.json', 'a\uE000.json',
    'Invoice.pdf${' \uFE00' * 30} x.html', '\uFE00.zshrc', 'notes.txt\uFE00.',
  ]) {
    check(!mosaicIsPlainFileName(name), 'plain name refused: ${shown(name)}');
    // Written as JSON by hand, every non-ASCII unit as `\uXXXX`: the point is
    // what arrives on the wire, lone surrogates included.
    final wire = jsonDecode(
      '{"suggestedName":"${jsonEscape(name)}","bytes":"${encoded('x')}"}',
    );
    final dialogs = FakeDialogs(target);
    final outcome = await mosaicRunFilesSave(wire, dialogs);
    check(
      failure(outcome) == 'suggestedName must be a plain file name',
      'refused name ${shown(name)}: ${failure(outcome)}',
    );
    check(dialogs.opened == 0, 'no dialog for refused name ${shown(name)}');
  }
  // No suggestedName at all, or not a string, is refused the same way.
  check(
    failure(
          await mosaicRunFilesSave(
            payload({'bytes': encoded('x')}),
            FakeDialogs(target),
          ),
        ) ==
        'suggestedName must be a plain file name',
    'missing suggestedName',
  );
  check(
    failure(
          await mosaicRunFilesSave(
            payload({'suggestedName': 7, 'bytes': encoded('x')}),
            FakeDialogs(target),
          ),
        ) ==
        'suggestedName must be a plain file name',
    'non-string suggestedName',
  );

  final mismatched = await mosaicRunFilesSave(
    payload({
      'suggestedName': 'notes.exe',
      'accept': ['application/json'],
      'bytes': encoded('{}'),
    }),
    FakeDialogs('${directory.path}/notes.exe'),
  );
  check(
    failure(mismatched) ==
        'suggestedName must end in an extension of an accepted type',
    'extension must match the accepted type',
  );
  check(
    !File('${directory.path}/notes.exe').existsSync(),
    'a mismatched name writes nothing',
  );

  // With no accepted type, a name that would run when opened is refused.
  for (final name in <String>[
    'run.command', 'open.terminal', 'site.webloc', 'setup.EXE', 'go.desktop',
    'a.ps1', 'a.j\u017F', 'img.iso', 'clip.scf', 'app.AppImage',
    'run\u0D4E.terminal',
  ]) {
    check(mosaicHasExecutableExtension(name), 'executable: ${shown(name)}');
    final dialogs = FakeDialogs('${directory.path}/$name');
    final outcome = await mosaicRunFilesSave(
      payload({'suggestedName': name, 'bytes': encoded('x')}),
      dialogs,
    );
    check(
      failure(outcome) == 'suggestedName must not end in an executable extension',
      'executable extension ${shown(name)}: ${failure(outcome)}',
    );
    check(dialogs.opened == 0, 'no dialog for executable ${shown(name)}');
    check(
      !File('${directory.path}/$name').existsSync(),
      'nothing written for ${shown(name)}',
    );
  }
  check(mosaicIsPlainFileName('journal.json'), 'an ordinary name passes');
  check(
    mosaicIsPlainFileName('caf\u00E9 menu.json'),
    'accented names still pass',
  );
  check(
    mosaicIsPlainFileName('\u2764\uFE0F list.txt'),
    "an emoji's own selector is fine",
  );
  check(mosaicIsPlainFileName('x' * 255), '255 UTF-16 units is the limit');
  check(
    !mosaicHasExecutableExtension('notes.txt') &&
        !mosaicHasExecutableExtension('README'),
    'an ordinary document is not executable',
  );

  // Dart's decoder would take the URL-safe alphabet and `%3D` padding; the
  // library refuses them, as Compose and SwiftUI do.
  for (final bad in <String>[
    'not base64!', '%%%', 'YQ', 'YQ==\n', 'YW Jj', 'YW-_', 'YQ%3D%3D',
  ]) {
    final outcome = await mosaicRunFilesSave(
      payload({'suggestedName': 'a.txt', 'bytes': bad}),
      FakeDialogs(target),
    );
    check(
      failure(outcome) == 'bytes must be base64 text',
      'invalid base64 ${shown(bad)}',
    );
  }
  check(
    failure(
          await mosaicRunFilesSave(
            payload({'suggestedName': 'a.txt'}),
            FakeDialogs(target),
          ),
        ) ==
        'bytes must be base64 text',
    'missing bytes',
  );

  final tooLarge = 'A' * ((mosaicMaxSaveBytes ~/ 3 + 2) * 4);
  final oversized = await mosaicRunFilesSave(
    payload({'suggestedName': 'a.txt', 'bytes': tooLarge}),
    FakeDialogs(target),
  );
  check(
    failure(oversized) == 'the file is larger than 16777216 bytes',
    'oversized save: ${failure(oversized)}',
  );
  check(!File(target).existsSync(), 'refusals write nothing');
}

void checkNameSafety() {
  // UI87 §3.1: Windows device names are never plain names, on any host.
  for (final name in <String>['CON', 'con.txt', 'Nul.json', 'COM1.json', 'lpt9', 'COM\u00B9.json', 'CON .txt', 'CONIN\$.log', 'aux.tar.gz']) {
    check(!mosaicIsPlainFileName(name), 'device name refused: ${shown(name)}');
  }
  for (final name in <String>['console.txt', 'CONFIG.json', 'aux-notes.txt', 'COM10.json', 'my.CON', 'nul report.json', 'CON\u0131N\$.txt']) {
    check(mosaicIsPlainFileName(name), 'not a device name: ${shown(name)}');
  }
  // Active content and non-ASCII extensions count as executable.
  for (final name in <String>['page.html', 'page.HTM', 'card.svg', 'archive.mht', 'shortcut.website', 'report.xlsm', 'deck.pptm', 'tool.py', 'invoice.\u0435x\u0435', 'setup.exe\u0301', 'macros.xlsb', 'addin.xla', 'link.iqy', 'sheet.slk', 'remote.rdp', 'app.pyzw', 'cache.pyc']) {
    check(mosaicHasExecutableExtension(name), 'executable: ${shown(name)}');
  }
  for (final name in <String>['notes.txt', 'data.xlsx', 'report.docx', 'photo.png']) {
    check(!mosaicHasExecutableExtension(name), 'not executable: ${shown(name)}');
  }
}

Future<void> checkOpen(Directory directory) async {
  final source = '${directory.path}/photo.PNG';
  File(source).writeAsBytesSync(<int>[1, 2, 3]);
  final dialogs = FakeDialogs(source);
  final outcome = await mosaicRunFilesOpen(
    payload({
      'accept': ['image/png', 'image/jpeg', 'image/png', 'x/unknown'],
    }),
    dialogs,
  );
  final value = okValue(outcome);
  check(value?['name'] == 'photo.PNG', 'open returns the name, never a path');
  check(value?['mimeType'] == 'image/png', 'mime type from the extension');
  check(value?['bytes'] == base64Encode(<int>[1, 2, 3]), 'open bytes');
  check(
    dialogs.lastExtensions.join(',') == 'png,jpg,jpeg',
    'extensions deduplicated in order, unknown dropped: ${dialogs.lastExtensions}',
  );

  // An unknown extension is application/octet-stream; no accept is any file.
  final unknown = '${directory.path}/notes.xyz';
  File(unknown).writeAsStringSync('x');
  final anyFile = FakeDialogs(unknown);
  check(
    okValue(await mosaicRunFilesOpen(null, anyFile))?['mimeType'] ==
        'application/octet-stream',
    'unknown extension',
  );
  check(anyFile.lastExtensions.isEmpty, 'no accept filters nothing');

  check(
    isCancelled(await mosaicRunFilesOpen(null, FakeDialogs(null))),
    'open cancelled',
  );
  check(
    failure(await mosaicRunFilesOpen(null, FakeDialogs(directory.path))) ==
        'that is not a regular file',
    'a directory is refused',
  );
  check(
    failure(
          await mosaicRunFilesOpen(
            null,
            FakeDialogs('${directory.path}/missing.png'),
          ),
        ) ==
        'that is not a regular file',
    'a missing file is refused',
  );

  if (!Platform.isWindows) {
    // A FIFO is refused before anything opens it (opening one would block).
    final fifo = '${directory.path}/pipe.png';
    if (Process.runSync('mkfifo', [fifo]).exitCode == 0) {
      check(
        failure(await mosaicRunFilesOpen(null, FakeDialogs(fifo))) ==
            'that is not a regular file',
        'a FIFO is refused',
      );
    }
    // A symlink the person chose is read, and named as chosen.
    final link = '${directory.path}/link.png';
    Link(link).createSync(source);
    final linked = okValue(await mosaicRunFilesOpen(null, FakeDialogs(link)));
    check(linked?['bytes'] == base64Encode(<int>[1, 2, 3]), 'chosen symlink');
    check(linked?['name'] == 'link.png', 'the chosen name, not the link target');
  }

  // Exactly the limit is read; one byte over is refused, bounded while reading.
  final atLimit = '${directory.path}/at-limit.bin';
  final atLimitHandle = File(atLimit).openSync(mode: FileMode.write);
  atLimitHandle.truncateSync(mosaicMaxOpenBytes);
  atLimitHandle.closeSync();
  check(
    okValue(await mosaicRunFilesOpen(null, FakeDialogs(atLimit))) != null,
    'a file of exactly the limit opens',
  );
  final large = '${directory.path}/large.bin';
  final largeHandle = File(large).openSync(mode: FileMode.write);
  largeHandle.truncateSync(mosaicMaxOpenBytes + 1);
  largeHandle.closeSync();
  check(
    failure(await mosaicRunFilesOpen(null, FakeDialogs(large))) ==
        'the selected file is larger than 52428800 bytes',
    'oversized open',
  );
}

Future<void> checkRouter(Directory directory) async {
  final target = '${directory.path}/routed.txt';
  final request = payload({'suggestedName': 'routed.txt', 'bytes': encoded('hi')});

  // A standard kind is deferred, then answered from the queued work.
  final queued = <void Function()>[];
  final host = FakeHost();
  final appCalls = <String>[];
  host.effectHandler = (id, kind, payload, delivery) => appCalls.add(kind);
  installMosaicPlatformRouter(
    host,
    appKinds: const <String>['importAnki'],
    dialogs: FakeDialogs(target),
    hasDialogs: true,
    runOnUi: queued.add,
  );
  final router = host.effectHandler;
  // Idempotent: a second install does not stack a second router.
  installMosaicPlatformRouter(
    host,
    appKinds: null,
    dialogs: FakeDialogs(null),
    hasDialogs: true,
    runOnUi: (work) => work(),
  );
  check(identical(host.effectHandler, router), 'a second install changes nothing');

  host.waitingOn = <int>{1, 2};
  host.effectHandler!(1, 'files.save', request, 'await');
  check(
    host.deferred.join(',') == '1',
    'a standard kind is deferred before any dialog',
  );
  check(
    !host.answers.containsKey(1) && queued.length == 1,
    'answered later, from the queued work',
  );
  // One file operation at a time: a second request while the first is open.
  host.effectHandler!(2, 'files.save', request, 'await');
  check(
    await answeredFailure(host, 2) == 'another file operation is in progress',
    'busy',
  );
  queued.removeAt(0)();
  check(await answeredName(host, 1) == 'routed.txt', 'deferred answer');
  check(File(target).readAsStringSync() == 'hi', 'the deferred save wrote the file');

  // The payload outlives the host's parsed update: answered from a copy, so
  // clearing the original after the handler returns changes nothing.
  final transient = payload({'suggestedName': 'copied.txt', 'bytes': encoded('copy')});
  host.waitingOn = <int>{9};
  host.effectHandler!(9, 'files.save', transient, 'await');
  transient.clear();
  queued.removeAt(0)();
  // The fake dialog's choice is `routed.txt`; what matters is that the request
  // was still readable after its original was cleared.
  check(
    await answeredName(host, 9) == 'routed.txt',
    'a deferred request survives its original',
  );
  check(File(target).readAsStringSync() == 'copy', 'and saves the copied bytes');
  check(
    !File('${directory.path}/copied.txt').existsSync(),
    "the dialog's choice, not the suggestion, is written",
  );

  // The app's claimed kind reaches the app; an unclaimed custom kind reaches
  // nobody (the host fails it); a notify needs no answer.
  host.effectHandler!(3, 'importAnki', null, 'await');
  host.effectHandler!(4, 'somethingElse', null, 'await');
  host.waitingOn = <int>{5};
  host.effectHandler!(5, 'files.open', null, 'notify');
  check(
    appCalls.join(',') == 'importAnki',
    'only the claimed kind reaches the app: $appCalls',
  );
  check(
    !host.answers.containsKey(4) && !host.answers.containsKey(5),
    'nothing answered for 4 and 5',
  );
  check(queued.isEmpty, 'a notify opens no dialog');

  // Not waiting on the id: no dialog, and the router is free again.
  host.waitingOn = <int>{};
  host.effectHandler!(6, 'files.open', null, 'await');
  check(
    queued.isEmpty && !host.answers.containsKey(6),
    'an id nobody awaits opens nothing',
  );
  host.waitingOn = <int>{7};
  host.effectHandler!(7, 'files.open', null, 'AWAIT');
  check(
    queued.length == 1,
    'the router is not left busy by a refused deferral (and AWAIT is an await)',
  );
  queued.removeAt(0)();
  check(await host.answer(7) != null, 'answered after the refused deferral');

  // A claimed standard kind reaches the app, not the library.
  final claimed = FakeHost();
  final claimedCalls = <String>[];
  claimed.effectHandler = (id, kind, payload, delivery) => claimedCalls.add(kind);
  installMosaicPlatformRouter(
    claimed,
    appKinds: const <String>['files.save'],
    dialogs: FakeDialogs(target),
    hasDialogs: true,
    runOnUi: (_) => throw StateError('no dialog for a claimed kind'),
  );
  claimed.waitingOn = <int>{1, 2};
  claimed.effectHandler!(1, 'files.save', request, 'await');
  check(
    claimedCalls.join(',') == 'files.save' && claimed.deferred.isEmpty,
    'a claimed standard kind goes to the app',
  );
  // The unclaimed one still reaches the library.
  final partlyQueue = <void Function()>[];
  final partly = FakeHost();
  final partlyCalls = <String>[];
  partly.effectHandler = (id, kind, payload, delivery) => partlyCalls.add(kind);
  installMosaicPlatformRouter(
    partly,
    appKinds: const <String>['files.save'],
    dialogs: FakeDialogs(null),
    hasDialogs: true,
    runOnUi: partlyQueue.add,
  );
  partly.waitingOn = <int>{1};
  partly.effectHandler!(1, 'files.open', null, 'await');
  check(
    partlyCalls.isEmpty && partlyQueue.length == 1,
    'the unclaimed standard kind reaches the library',
  );
  partlyQueue.removeAt(0)();
  check(
    await partly.answer(1) == '{"cancelled":{}}',
    'a cancelled dialog answers cancelled',
  );

  // An empty kinds list claims nothing: every custom kind goes to nobody.
  final none = FakeHost();
  final noneCalls = <String>[];
  none.effectHandler = (id, kind, payload, delivery) => noneCalls.add(kind);
  installMosaicPlatformRouter(
    none,
    appKinds: const <String>[],
    dialogs: FakeDialogs(null),
    hasDialogs: true,
  );
  none.effectHandler!(1, 'importAnki', null, 'await');
  check(noneCalls.isEmpty, 'an empty kinds list claims nothing');

  // No app handler and no kinds: a custom kind goes to nobody here.
  final bare = FakeHost();
  installMosaicPlatformRouter(
    bare,
    appKinds: null,
    dialogs: FakeDialogs(null),
    hasDialogs: true,
    runOnUi: (work) => work(),
  );
  bare.waitingOn = <int>{1};
  bare.effectHandler!(1, 'importAnki', null, 'await');
  check(
    !bare.answers.containsKey(1) && bare.deferred.isEmpty,
    "no handler: left for the host's sweep",
  );

  // Work the scheduler refuses (it throws): failed at once, because a
  // deferred effect is out of the host's sweep -- and not left busy.
  final refusing = FakeHost();
  installMosaicPlatformRouter(
    refusing,
    appKinds: null,
    dialogs: FakeDialogs(target),
    hasDialogs: true,
    runOnUi: (_) => throw StateError('no scheduler'),
  );
  refusing.waitingOn = <int>{1, 2};
  refusing.effectHandler!(1, 'files.open', null, 'await');
  check(
    await answeredFailure(refusing, 1) == 'the file dialog failed',
    'refused work fails the effect',
  );
  refusing.effectHandler!(2, 'files.save', request, 'await');
  check(
    await answeredFailure(refusing, 2) == 'the file dialog failed',
    'and does not leave the router busy',
  );

  // A dialog that throws: failed, without the exception's text (a path).
  final broken = FakeHost();
  installMosaicPlatformRouter(
    broken,
    appKinds: null,
    dialogs: FakeDialogs(null, throws: true),
    hasDialogs: true,
    runOnUi: (work) => work(),
  );
  broken.waitingOn = <int>{1, 2};
  broken.effectHandler!(1, 'files.open', null, 'await');
  check(
    await answeredFailure(broken, 1) == 'the file dialog failed',
    'a throwing dialog fails the effect',
  );
  broken.effectHandler!(2, 'files.save', request, 'await');
  check(
    await answeredFailure(broken, 2) == 'the file dialog failed',
    'and does not leave the router busy',
  );

  // No dialogs on this platform (Android, iOS): failed inline with the kind,
  // never a cancel nobody made, and nothing deferred.
  final phone = FakeHost();
  installMosaicPlatformRouter(
    phone,
    appKinds: null,
    dialogs: FakeDialogs(target),
    hasDialogs: false,
    runOnUi: (_) => throw StateError('no dialog without dialogs'),
  );
  phone.waitingOn = <int>{1};
  phone.effectHandler!(1, 'files.save', request, 'await');
  check(phone.deferred.isEmpty, 'nothing deferred without dialogs');
  check(
    await answeredFailure(phone, 1) ==
        'files.save is not available on this platform yet',
    'no dialogs: a clear failure',
  );

  // A host that refuses the answer (disposed underneath): nothing escapes.
  final closedQueue = <void Function()>[];
  final closed = FakeHost();
  installMosaicPlatformRouter(
    closed,
    appKinds: null,
    dialogs: FakeDialogs(null),
    hasDialogs: true,
    runOnUi: closedQueue.add,
  );
  closed.waitingOn = <int>{1};
  closed.effectHandler!(1, 'files.open', null, 'await');
  closed.closed = true;
  closedQueue.removeAt(0)();
  await closed.waitForAttempts(1);
  check(
    closed.completeAttempts == 1 && !closed.answers.containsKey(1),
    'an answer to a disposed host is dropped, not thrown',
  );

  // A retried start disposes the host and loads a new one while a dialog from
  // the old one is still open (UI87 §7.7). The old router holds the old host:
  // its late answer meets the disposed runtime and is dropped, and never
  // reaches the new runtime -- which reuses effect id 1 -- nor holds the new
  // router busy.
  final oldQueue = <void Function()>[];
  final oldHost = FakeHost();
  installMosaicPlatformRouter(
    oldHost,
    appKinds: null,
    dialogs: FakeDialogs(target),
    hasDialogs: true,
    runOnUi: oldQueue.add,
  );
  oldHost.waitingOn = <int>{1};
  oldHost.effectHandler!(1, 'files.save', request, 'await');
  check(
    oldHost.deferred.join(',') == '1' && oldQueue.length == 1,
    "the old runtime's dialog is open",
  );
  oldHost.closed = true;
  final newQueue = <void Function()>[];
  final newHost = FakeHost();
  installMosaicPlatformRouter(
    newHost,
    appKinds: null,
    dialogs: FakeDialogs(null),
    hasDialogs: true,
    runOnUi: newQueue.add,
  );
  newHost.waitingOn = <int>{1};
  newHost.effectHandler!(1, 'files.open', null, 'await');
  check(
    newHost.deferred.join(',') == '1' && newQueue.length == 1,
    "the new runtime's router is not busy with the old runtime's dialog",
  );
  oldQueue.removeAt(0)(); // the old dialog finishes, after the swap
  await oldHost.waitForAttempts(1);
  check(
    oldHost.completeAttempts == 1 && !oldHost.answers.containsKey(1),
    'the late answer went to the old runtime, which dropped it',
  );
  check(
    !newHost.answers.containsKey(1) && newHost.completeAttempts == 0,
    'a late answer from the old runtime never reaches the new one',
  );
  newQueue.removeAt(0)();
  check(
    await newHost.answer(1) == '{"cancelled":{}}',
    'the new runtime gets only its own answer',
  );

  // The adapter over the generated host. With no runtime behind it (the
  // permissive shell's fallback), there is no handler, nothing to defer, and
  // an answer is refused -- which the router drops.
  final noRuntime = MosaicHostEffects(const MosaicHost());
  check(noRuntime.effectHandler == null, 'no runtime: no handler');
  check(!noRuntime.deferEffect(1), 'no runtime: nothing to defer');
  var refused = false;
  try {
    noRuntime.completeEffect(1, mosaicFailed('x'));
  } on StateError {
    refused = true;
  }
  check(refused, 'no runtime: an answer is refused');
  installMosaicPlatformRouter(
    noRuntime,
    appKinds: null,
    dialogs: FakeDialogs(null),
  );
  check(noRuntime.effectHandler == null, 'installing on no runtime is a no-op');
}

/// The calls the save and the open make through dart:ffi (UI87 §7.7): the
/// `open(2)` flag tables, the mode rule, and -- on the running kernel -- that
/// the running ABI's table means what it says, that the temporary is created
/// exclusively and never through a link planted at its name, and that the
/// opened descriptor is typed before anything reads it.
Future<void> checkPosixCalls(Directory directory) async {
  String table(Abi abi) {
    final flags = mosaicOpenFlagsFor(abi);
    if (flags == null) return 'none';
    return <int>[
      flags.writeOnly,
      flags.create,
      flags.exclusive,
      flags.nonBlocking,
      flags.noFollow,
      flags.closeOnExec,
    ].map((value) => '0x${value.toRadixString(16)}').join(' ');
  }

  // O_WRONLY O_CREAT O_EXCL O_NONBLOCK O_NOFOLLOW O_CLOEXEC, as the headers
  // define them (arm64 Linux has its own O_NOFOLLOW).
  check(
    table(Abi.linuxX64) == '0x1 0x40 0x80 0x800 0x20000 0x80000',
    'Linux x64 open flags: ${table(Abi.linuxX64)}',
  );
  check(
    table(Abi.linuxArm64) == '0x1 0x40 0x80 0x800 0x8000 0x80000',
    'Linux arm64 open flags: ${table(Abi.linuxArm64)}',
  );
  for (final abi in <Abi>[Abi.macosX64, Abi.macosArm64]) {
    check(
      table(abi) == '0x1 0x200 0x800 0x4 0x100 0x1000000',
      '$abi open flags: ${table(abi)}',
    );
  }
  for (final abi in <Abi>[Abi.windowsX64, Abi.linuxIA32, Abi.androidArm64]) {
    check(table(abi) == 'none', 'no flag table for $abi');
  }
  check(MosaicOpenFlags.readOnly == 0, 'O_RDONLY is 0');

  // The mode rule, as the Qt library applies it.
  const me = 1000;
  check(mosaicReplacementMode(null, me) == 0x180, 'nothing there: 0600');
  check(
    mosaicReplacementMode((mode: 0x81A4, uid: me), me) == 0x1A4,
    'my 0644 file keeps 0644',
  );
  check(
    mosaicReplacementMode((mode: 0x81B4, uid: me), me) == 0x1B4,
    'my 0664 file keeps 0664',
  );
  check(
    mosaicReplacementMode((mode: 0x89ED, uid: me), me) == 0x1ED,
    'my setuid 4755 file becomes 0755',
  );
  check(
    mosaicReplacementMode((mode: 0x81B6, uid: 1001), me) == 0x180,
    "someone else's 0666 file becomes 0600",
  );
  check(
    mosaicReplacementMode((mode: 0x81B6, uid: null), me) == 0x1A4,
    'an unknown owner loses group and other write: 0666 -> 0644',
  );
  check(
    mosaicReplacementMode((mode: 0x8FFF, uid: null), me) == 0x1ED,
    'an unknown owner: 7777 -> 0755',
  );
  check(
    mosaicReplacementMode((mode: 0xA1FF, uid: me), me) == 0x180,
    'a link lends nothing: 0600',
  );
  check(
    mosaicReplacementMode((mode: 0x41ED, uid: me), me) == 0x180,
    'a directory lends nothing: 0600',
  );

  if (Platform.isWindows) return;
  final flags = mosaicOpenFlagsFor(Abi.current());
  check(flags != null, 'this POSIX ABI has a flag table: ${Abi.current()}');
  _checkFlagsOnThisKernel(directory, flags!);

  // A link planted at the temporary's name -- the attack a writable, shared
  // folder allows -- fails the save; its target is neither written,
  // truncated nor chmodded, and the link itself is left where it was.
  final victim = '${directory.path}/victim-bashrc';
  File(victim).writeAsStringSync('the victim keeps this');
  Process.runSync('chmod', ['644', victim]);
  final target = '${directory.path}/planted.txt';
  const planted = '.mosaic-save-planted.tmp';
  Link('${directory.path}/$planted').createSync(victim);
  final throughLink = mosaicWriteReplacing(
    target,
    utf8.encode('the app bytes'),
    temporaryName: planted,
  );
  check(
    failure(throughLink) == "couldn't save the file",
    'a link at the temporary name fails the save: $throughLink',
  );
  check(
    File(victim).readAsStringSync() == 'the victim keeps this',
    "the planted link's target is not written or truncated",
  );
  check(
    mode(victim) == 0x1A4,
    "the planted link's target keeps its mode, got ${mode(victim).toRadixString(8)}",
  );
  check(
    FileSystemEntity.typeSync(
          '${directory.path}/$planted',
          followLinks: false,
        ) ==
        FileSystemEntityType.link,
    'the planted link is left alone',
  );
  check(!File(target).existsSync(), 'nothing saved through a planted link');

  // A dangling link: O_CREAT through it would create the file it names.
  const dangling = '.mosaic-save-dangling.tmp';
  final wouldCreate = '${directory.path}/created-through-a-link';
  Link('${directory.path}/$dangling').createSync(wouldCreate);
  check(
    failure(
          mosaicWriteReplacing(
            target,
            utf8.encode('x'),
            temporaryName: dangling,
          ),
        ) ==
        "couldn't save the file",
    'a dangling link at the temporary name fails the save',
  );
  check(
    !File(wouldCreate).existsSync(),
    'nothing is created through a dangling link',
  );

  // O_EXCL: a regular file already at the name is refused, not truncated.
  const squatter = '.mosaic-save-squatter.tmp';
  File('${directory.path}/$squatter').writeAsStringSync("someone else's");
  check(
    failure(
          mosaicWriteReplacing(
            target,
            utf8.encode('x'),
            temporaryName: squatter,
          ),
        ) ==
        "couldn't save the file",
    'a file at the temporary name fails the save',
  );
  check(
    File('${directory.path}/$squatter').readAsStringSync() == "someone else's",
    'a file at the temporary name is not truncated',
  );
  Link('${directory.path}/$planted').deleteSync();
  Link('${directory.path}/$dangling').deleteSync();
  File('${directory.path}/$squatter').deleteSync();

  // The same call with nothing in the way saves, and leaves nothing behind.
  const fixed = '.mosaic-save-fixed.tmp';
  final saved = mosaicWriteReplacing(
    target,
    utf8.encode('saved'),
    temporaryName: fixed,
  );
  check(okValue(saved)?['name'] == 'planted.txt', 'a fixed-name save: $saved');
  check(File(target).readAsStringSync() == 'saved', 'fixed-name save bytes');
  check(
    !File('${directory.path}/$fixed').existsSync(),
    'the temporary is renamed away, not left behind',
  );

  // My own group-writable file keeps its bits.
  final shared = '${directory.path}/shared.txt';
  File(shared).writeAsStringSync('old');
  Process.runSync('chmod', ['664', shared]);
  check(
    okValue(mosaicWriteReplacing(shared, utf8.encode('new'))) != null,
    'save over my own 0664 file',
  );
  check(
    mode(shared) == 0x1B4,
    'my own 0664 file keeps 0664, got ${mode(shared).toRadixString(8)}',
  );

  // Someone else's file lends no bits (needs root to hand a file away). Its
  // group stays root's, so an owner read from the group field would match.
  if ((Process.runSync('id', ['-u']).stdout as String).trim() == '0') {
    final theirs = '${directory.path}/theirs.txt';
    File(theirs).writeAsStringSync('old');
    Process.runSync('chmod', ['666', theirs]);
    if (Process.runSync('chown', ['1000:0', theirs]).exitCode == 0) {
      check(
        okValue(mosaicWriteReplacing(theirs, utf8.encode('new'))) != null,
        "save over someone else's 0666 file",
      );
      check(
        mode(theirs) == 0x180,
        "someone else's 0666 file is replaced 0600, got ${mode(theirs).toRadixString(8)}",
      );
    }
  }

  // The descriptor is typed before it is read. Handed straight to the read,
  // as a swap after the path check would hand them: a FIFO (which must not
  // block -- run in an isolate with a deadline, so a regression fails rather
  // than hangs), a device and a directory are refused; a file is read.
  final fifo = '${directory.path}/swapped-in.fifo';
  if (Process.runSync('mkfifo', [fifo]).exitCode == 0) {
    final Map<String, Object?>? fromFifo;
    try {
      fromFifo = await Isolate.run(
        () => mosaicReadThroughDescriptor(fifo),
      ).timeout(const Duration(seconds: 20));
    } on TimeoutException {
      stderr.writeln('Failed platform-effects assertion: a FIFO blocked the read');
      exit(1);
    }
    check(
      fromFifo != null && failure(fromFifo) == 'that is not a regular file',
      'a FIFO on the descriptor is refused without blocking: $fromFifo',
    );
  }
  final device = mosaicReadThroughDescriptor('/dev/null');
  check(
    device != null && failure(device) == 'that is not a regular file',
    'a device on the descriptor is refused: $device',
  );
  final folder = mosaicReadThroughDescriptor(directory.path);
  check(
    folder != null && failure(folder) == 'that is not a regular file',
    'a directory on the descriptor is refused: $folder',
  );
  final read = mosaicReadThroughDescriptor(target);
  check(
    read != null && okValue(read)?['bytes'] == encoded('saved'),
    'a regular file is read through its descriptor: $read',
  );
}

/// The running ABI's flag table, checked against the running kernel with raw
/// `open(2)` calls: each flag does what the table says it does.
void _checkFlagsOnThisKernel(Directory directory, MosaicOpenFlags flags) {
  final libc = DynamicLibrary.process();
  final open = libc.lookupFunction<
    Int32 Function(Pointer<Utf8>, Int32, VarArgs<(Uint32,)>),
    int Function(Pointer<Utf8>, int, int)
  >('open');
  final fcntl = libc.lookupFunction<
    Int32 Function(Int32, Int32, VarArgs<(Int32,)>),
    int Function(int, int, int)
  >('fcntl');
  final close = libc.lookupFunction<Int32 Function(Int32), int Function(int)>(
    'close',
  );
  int openAt(String path, int openFlags) {
    final native = path.toNativeUtf8();
    try {
      return open(native, openFlags, 0x180);
    } finally {
      malloc.free(native);
    }
  }

  final existing = '${directory.path}/flags-existing';
  File(existing).writeAsStringSync('here');
  final link = '${directory.path}/flags-link';
  Link(link).createSync(existing);

  final followed = openAt(link, MosaicOpenFlags.readOnly);
  check(followed >= 0, 'a link opens without O_NOFOLLOW');
  close(followed);
  check(
    openAt(link, MosaicOpenFlags.readOnly | flags.noFollow) < 0,
    'O_NOFOLLOW (0x${flags.noFollow.toRadixString(16)}) refuses a link',
  );
  check(
    openAt(existing, flags.writeOnly | flags.create | flags.exclusive) < 0,
    'O_CREAT|O_EXCL refuses an existing name',
  );
  final fresh = '${directory.path}/flags-fresh';
  final created = openAt(fresh, flags.writeOnly | flags.create | flags.exclusive);
  check(created >= 0 && File(fresh).existsSync(), 'O_CREAT|O_EXCL creates');
  close(created);
  final cloexec = openAt(existing, MosaicOpenFlags.readOnly | flags.closeOnExec);
  // F_GETFD = 1 and FD_CLOEXEC = 1 on Linux and Apple platforms.
  check(
    cloexec >= 0 && fcntl(cloexec, 1, 0) & 1 == 1,
    'O_CLOEXEC (0x${flags.closeOnExec.toRadixString(16)}) sets FD_CLOEXEC',
  );
  close(cloexec);
  final fifo = '${directory.path}/flags-fifo';
  if (Process.runSync('mkfifo', [fifo]).exitCode == 0) {
    // Without O_NONBLOCK this open would wait for a writer forever.
    final nonBlocking = openAt(
      fifo,
      MosaicOpenFlags.readOnly | flags.nonBlocking,
    );
    check(nonBlocking >= 0, 'O_NONBLOCK opens a FIFO at once');
    close(nonBlocking);
    File(fifo).deleteSync();
  }
  Link(link).deleteSync();
  File(existing).deleteSync();
  File(fresh).deleteSync();
}

/// UI87 §7.7: where the dialog did not ask before replacing (Linux), the
/// library asks; a question answered "keep" is a cancel.
Future<void> checkConfirmReplacing(Directory directory) async {
  final existing = '${directory.path}${Platform.pathSeparator}taken.json';
  File(existing).writeAsStringSync('old');
  final fresh = '${directory.path}${Platform.pathSeparator}fresh.json';
  final asked = <String>[];
  Future<bool> answer(bool replace) async => replace;
  MosaicReplaceQuestion recording(bool replace) => (name) {
    asked.add(name);
    return answer(replace);
  };

  check(
    await mosaicConfirmReplacing(existing, dialogAsked: false, ask: recording(true)) ==
        existing,
    'replacing an existing file when the person agrees',
  );
  check(asked.length == 1 && asked.single == 'taken.json', 'asked by name, not path');
  check(
    await mosaicConfirmReplacing(existing, dialogAsked: false, ask: recording(false)) ==
        null,
    'keeping an existing file is a cancel',
  );
  asked.clear();
  check(
    await mosaicConfirmReplacing(fresh, dialogAsked: false, ask: recording(false)) == fresh,
    'a new name is not asked about',
  );
  check(
    await mosaicConfirmReplacing(existing, dialogAsked: true, ask: recording(false)) ==
        existing,
    'a dialog that asked is not asked again',
  );
  check(
    await mosaicConfirmReplacing(null, dialogAsked: false, ask: recording(true)) == null,
    'a cancelled dialog stays cancelled',
  );
  check(asked.isEmpty, 'no question for a new name, an asking dialog or a cancel');
  if (!Platform.isWindows) {
    final dangling = '${directory.path}/dangling.json';
    Link(dangling).createSync('${directory.path}/nowhere');
    check(
      await mosaicConfirmReplacing(dangling, dialogAsked: false, ask: recording(false)) ==
          null,
      'a dangling link is something to replace',
    );
    check(asked.single == 'dangling.json', 'asked about the link');
    Link(dangling).deleteSync();
  }
  // A question that cannot be asked fails the save; it never replaces.
  final save = await mosaicRunFilesSave(
    <String, Object?>{'suggestedName': 'taken.json', 'bytes': base64Encode(<int>[1])},
    _AskingDialogs(existing),
  ).then<Object?>((value) => value, onError: (Object error) => error);
  check(save is StateError, 'an unaskable question throws to the router');
  check(File(existing).readAsStringSync() == 'old', 'nothing replaced unasked');
  File(existing).deleteSync();
}

/// Dialogs that, like Linux's, chose an existing file without asking, and
/// have no window to ask through.
final class _AskingDialogs implements MosaicFileDialogs {
  _AskingDialogs(this.choice);
  final String choice;

  @override
  Future<String?> chooseFileToOpen(List<String> extensions) async => null;

  @override
  Future<String?> chooseFileToSave(String suggestedName, List<String> extensions) =>
      mosaicConfirmReplacing(
        choice,
        dialogAsked: false,
        ask: (_) => throw StateError('no window to ask whether to replace the file'),
      );
}

// ── Phones (UI89 §7.11) ──────────────────────────────────────────────────

/// The phone build's document plugin, answered by callbacks. Each records
/// what it was asked, so a check can see what reached the "picker".
/// The separator a request directory's paths use, as `Directory.createTemp`
/// makes them: `/` on phones and POSIX desktops, `\` on Windows.
final String sep = Platform.pathSeparator;

final class FakePhoneDocuments implements MosaicPhoneDocuments {
  FakePhoneDocuments(this.temporary, {this.open, this.save});

  final String temporary;
  final Future<String?> Function(String directory, List<String> mimeTypes, int limit)? open;
  final Future<String?> Function(String stagedPath, String mimeType)? save;

  int opened = 0;
  int exported = 0;
  List<String> lastMimeTypes = const <String>[];
  int lastLimit = 0;
  String? lastDirectory;
  String? lastStaged;
  String? stagedContents;
  String? lastMimeType;

  @override
  Future<String> temporaryDirectory() async => temporary;

  @override
  Future<String?> copyForOpening(
    String directory,
    List<String> mimeTypes,
    int limit,
  ) async {
    opened += 1;
    lastDirectory = directory;
    lastMimeTypes = List<String>.of(mimeTypes);
    lastLimit = limit;
    return open!(directory, mimeTypes, limit);
  }

  @override
  Future<String?> export(String stagedPath, String mimeType) async {
    exported += 1;
    lastStaged = stagedPath;
    lastMimeType = mimeType;
    stagedContents = File(stagedPath).readAsStringSync();
    return save!(stagedPath, mimeType);
  }
}

/// The request directories left under [temporary]'s `mosaic-files`.
List<String> requestDirectories(String temporary) {
  final root = Directory('$temporary/$mosaicPhoneFilesDirectoryName');
  if (!root.existsSync()) return const <String>[];
  return root.listSync().map((entry) => entry.path).toList();
}

Future<void> checkPhoneOpen(Directory directory) async {
  final temporary = Directory('${directory.path}/phone-open')..createSync();
  final temp = temporary.path;
  final accept = payload({
    'accept': <String>['application/json', 'text/x-unknown', 'application/json', 'text/plain'],
  });

  // A copy the plugin made is read, named, typed, and removed with its
  // directory.
  final documents = FakePhoneDocuments(temp, open: (dir, mimes, limit) async {
    final copy = '$dir${sep}deck.json';
    File(copy).writeAsStringSync('{"cards":1}');
    return copy;
  });
  final opened = await mosaicRunPhoneFilesOpen(accept, documents);
  check(okValue(opened)?['name'] == 'deck.json', 'phone open: the copy\'s name');
  check(okValue(opened)?['mimeType'] == 'application/json', 'phone open: its type');
  check(
    okValue(opened)?['bytes'] == encoded('{"cards":1}'),
    'phone open: its bytes',
  );
  check(
    documents.lastMimeTypes.join(',') == 'application/json,text/plain',
    'phone open: only known MIME types reach the picker, in order, once each',
  );
  check(documents.lastLimit == mosaicMaxOpenBytes, 'phone open: the 50 MiB limit');
  check(
    documents.lastDirectory!.startsWith('$temp${sep}$mosaicPhoneFilesDirectoryName${sep}'),
    'phone open: a request directory under mosaic-files',
  );
  check(requestDirectories(temp).isEmpty, 'phone open: the copy is removed');

  // The copy's name is the plugin's choice, so it is made ordinary too: a
  // provider's bidi-spoofed name is told as "document", typed by that name.
  final spoofed = await mosaicRunPhoneFilesOpen(
    accept,
    FakePhoneDocuments(temp, open: (dir, mimes, limit) async {
      final copy = '$dir${sep}invoice\u202Efdp.json';
      File(copy).writeAsStringSync('{}');
      return copy;
    }),
  );
  check(okValue(spoofed)?['name'] == 'document', 'phone open: a spoofed name');
  check(
    okValue(spoofed)?['mimeType'] == 'application/octet-stream',
    'phone open: typed by the name it is told',
  );

  // A temporary directory with a trailing separator, as iOS reports it.
  final trailing = FakePhoneDocuments('$temp${sep}', open: (dir, mimes, limit) async {
    check(!dir.contains('${sep}${sep}'), 'phone open: no doubled separator');
    final copy = '$dir${sep}deck.json';
    File(copy).writeAsStringSync('{}');
    return copy;
  });
  check(
    okValue(await mosaicRunPhoneFilesOpen(accept, trailing))?['name'] == 'deck.json',
    'phone open: a trailing separator in the temporary directory',
  );

  // Any document when nothing known is accepted.
  await mosaicRunPhoneFilesOpen(
    payload({'accept': <String>['text/x-unknown']}),
    FakePhoneDocuments(temp, open: (dir, mimes, limit) async {
      check(mimes.isEmpty, 'phone open: nothing known accepted is any document');
      return null;
    }),
  );

  // A cancel.
  final cancelled = await mosaicRunPhoneFilesOpen(
    accept,
    FakePhoneDocuments(temp, open: (dir, mimes, limit) async => null),
  );
  check(isCancelled(cancelled), 'phone open: a cancel');
  check(requestDirectories(temp).isEmpty, 'phone open: a cancel leaves nothing');

  // Exactly 50 MiB is read; one byte more is refused, whatever the plugin
  // copied.
  final atLimit = await mosaicRunPhoneFilesOpen(
    accept,
    FakePhoneDocuments(temp, open: (dir, mimes, limit) async {
      final copy = '$dir${sep}big.json';
      final handle = File(copy).openSync(mode: FileMode.writeOnly);
      handle.setPositionSync(mosaicMaxOpenBytes - 1);
      handle.writeByteSync(0x20);
      handle.closeSync();
      return copy;
    }),
  );
  check(okValue(atLimit) != null, 'phone open: exactly 50 MiB is read');
  final overLimit = await mosaicRunPhoneFilesOpen(
    accept,
    FakePhoneDocuments(temp, open: (dir, mimes, limit) async {
      final copy = '$dir${sep}big.json';
      final handle = File(copy).openSync(mode: FileMode.writeOnly);
      handle.setPositionSync(mosaicMaxOpenBytes);
      handle.writeByteSync(0x20);
      handle.closeSync();
      return copy;
    }),
  );
  check(
    failure(overLimit) ==
        'the selected file is larger than $mosaicMaxOpenBytes bytes',
    'phone open: an oversized copy is refused',
  );
  check(requestDirectories(temp).isEmpty, 'phone open: and removed');

  // The answered path is not trusted: a link, a path outside the request's
  // directory, `..`, a directory and a missing file are each refused unread.
  final secret = File('${directory.path}/phone-secret.txt')
    ..writeAsStringSync('private');
  // Links only where the harness makes them (as the desktop checks above):
  // Windows needs a privilege for a symbolic link, and both phones are POSIX.
  final hostile = <String, Future<String?> Function(String, List<String>, int)>{
    if (!Platform.isWindows)
      'a link to a private file': (dir, mimes, limit) async {
        Link('$dir${sep}deck.json').createSync(secret.path);
        return '$dir${sep}deck.json';
      },
    'a file outside the directory': (dir, mimes, limit) async => secret.path,
    'a path through ..': (dir, mimes, limit) async =>
        '$dir${sep}../${dir.split('/').last}/x',
    'the directory itself': (dir, mimes, limit) async => '$dir${sep}.',
    'a directory': (dir, mimes, limit) async {
      Directory('$dir${sep}folder').createSync();
      return '$dir${sep}folder';
    },
    'a missing file': (dir, mimes, limit) async => '$dir${sep}missing.json',
  };
  for (final MapEntry(key: label, value: answer) in hostile.entries) {
    final outcome = await mosaicRunPhoneFilesOpen(
      accept,
      FakePhoneDocuments(temp, open: answer),
    );
    check(
      failure(outcome) == "couldn't read the selected file",
      'phone open refuses $label',
    );
    check(
      !jsonEncode(outcome).contains('private'),
      'phone open reads nothing through $label',
    );
  }
  check(secret.readAsStringSync() == 'private', 'the private file is untouched');
  check(requestDirectories(temp).isEmpty, 'phone open: refusals leave nothing');

  // Each plugin code is a fixed message, never the platform's text.
  const openMessages = <String, String>{
    'busy': 'another file operation is in progress',
    'no_window': 'there is no window to show the file picker in',
    'activity_gone': 'the file picker closed with the app',
    'too_large': 'the selected file is larger than $mosaicMaxOpenBytes bytes',
    'stalled': 'the selected file stopped arriving',
    'unreadable': "couldn't read the selected file",
    'something_new': "couldn't read the selected file",
  };
  for (final MapEntry(key: code, value: message) in openMessages.entries) {
    final outcome = await mosaicRunPhoneFilesOpen(
      accept,
      FakePhoneDocuments(
        temp,
        open: (dir, mimes, limit) async =>
            throw MosaicPhoneDocumentsException(code),
      ),
    );
    check(failure(outcome) == message, 'phone open: code $code');
  }
  final missing = await mosaicRunPhoneFilesOpen(
    accept,
    FakePhoneDocuments(
      temp,
      open: (dir, mimes, limit) async =>
          throw StateError('/data/user/0/app/secret: no plugin'),
    ),
  );
  check(
    failure(missing) == "couldn't read the selected file",
    'phone open: a missing plugin is the generic failure, without its text',
  );
  check(requestDirectories(temp).isEmpty, 'phone open: failures leave nothing');
}

Future<void> checkPhoneSave(Directory directory) async {
  final temporary = Directory('${directory.path}/phone-save')..createSync();
  final temp = temporary.path;
  final request = payload({
    'suggestedName': 'deck.json',
    'bytes': encoded('{"cards":2}'),
    'accept': <String>['application/json'],
  });

  // The bytes are staged under the suggested name and handed to the picker;
  // the answer is the name it reports, and the staged file is removed.
  final documents = FakePhoneDocuments(
    temp,
    save: (staged, mime) async => 'deck (1).json',
  );
  final saved = await mosaicRunPhoneFilesSave(request, documents);
  check(okValue(saved)?['name'] == 'deck (1).json', 'phone save: the reported name');
  check(okValue(saved)!.length == 1, 'phone save answers only the name');
  check(documents.stagedContents == '{"cards":2}', 'phone save: the staged bytes');
  check(
    documents.lastStaged!.endsWith('/deck.json'),
    'phone save: staged under the suggested name',
  );
  check(documents.lastMimeType == 'application/json', 'phone save: the type');
  check(requestDirectories(temp).isEmpty, 'phone save: the staged file is removed');

  // The type: the name's own when accepted or when anything is, else the
  // first accepted.
  final anyType = FakePhoneDocuments(temp, save: (staged, mime) async => null);
  await mosaicRunPhoneFilesSave(
    payload({'suggestedName': 'notes.md', 'bytes': encoded('#')}),
    anyType,
  );
  check(anyType.lastMimeType == 'text/markdown', 'phone save: the name\'s own type');
  final firstType = FakePhoneDocuments(temp, save: (staged, mime) async => null);
  await mosaicRunPhoneFilesSave(
    payload({
      'suggestedName': 'notes.markdown',
      'bytes': encoded('#'),
      'accept': <String>['text/plain', 'text/markdown'],
    }),
    firstType,
  );
  check(firstType.lastMimeType == 'text/markdown', 'phone save: an accepted own type');

  // A reported name is told only when ordinary.
  final reportedNames = <String, String>{
    '/storage/emulated/0/Download/deck.json': 'deck.json',
    r'C:\docs\deck.json': 'deck.json',
    '': 'document',
    '.': 'document',
    '..': 'document',
    '   ': 'document',
    'evil\u202Egnp.json': 'document',
    'two\nlines.json': 'document',
    'nul\u0000.json': 'document',
    'c1\u0085.json': 'document',
    'para\u2029.json': 'document',
    'lone\uD800.json': 'document',
    'lone\uDC00.json': 'document',
    '${'é' * 128}.json': 'document',
    '${'a' * 256}': 'document',
    'family\u200D\u{1F468}.json': 'family\u200D\u{1F468}.json',
    'soft\u00ADhyphen.json': 'soft\u00ADhyphen.json',
    '${'a' * 250}.json': '${'a' * 250}.json',
  };
  for (final MapEntry(key: reported, value: told) in reportedNames.entries) {
    check(
      mosaicOrdinaryReportedName(reported) == told,
      'reported name ${shown(reported)} is told as ${shown(told)}',
    );
  }

  // A refused request never reaches the picker, and stages nothing.
  final refusing = FakePhoneDocuments(temp, save: (staged, mime) async => 'x');
  for (final bad in <Map<String, Object?>>[
    {'suggestedName': '../deck.json', 'bytes': encoded('x')},
    {'suggestedName': 'deck.exe', 'bytes': encoded('x')},
    {'suggestedName': 'deck.txt', 'bytes': encoded('x'), 'accept': <String>['application/json']},
    {'suggestedName': 'deck.json', 'bytes': '%%%'},
  ]) {
    final outcome = await mosaicRunPhoneFilesSave(payload(bad), refusing);
    check(failure(outcome) != null, 'phone save refuses ${bad['suggestedName']}');
  }
  check(refusing.exported == 0, 'phone save: no picker for a refused request');
  check(
    !Directory('$temp/$mosaicPhoneFilesDirectoryName').existsSync() ||
        requestDirectories(temp).isEmpty,
    'phone save: a refused request stages nothing',
  );

  // A cancel, and each code, leave nothing staged.
  final cancelled = await mosaicRunPhoneFilesSave(
    request,
    FakePhoneDocuments(temp, save: (staged, mime) async => null),
  );
  check(isCancelled(cancelled), 'phone save: a cancel');
  const saveMessages = <String, String>{
    'busy': 'another file operation is in progress',
    'no_window': 'there is no window to show the file picker in',
    'activity_gone': 'the file picker closed with the app',
    'too_large': 'the file is larger than $mosaicMaxSaveBytes bytes',
    'stalled': 'the file stopped saving',
    'unreadable': "couldn't save the file",
    'something_new': "couldn't save the file",
  };
  for (final MapEntry(key: code, value: message) in saveMessages.entries) {
    final outcome = await mosaicRunPhoneFilesSave(
      request,
      FakePhoneDocuments(
        temp,
        save: (staged, mime) async => throw MosaicPhoneDocumentsException(code),
      ),
    );
    check(failure(outcome) == message, 'phone save: code $code');
  }
  final thrown = await mosaicRunPhoneFilesSave(
    request,
    FakePhoneDocuments(
      temp,
      save: (staged, mime) async => throw StateError('/private/var/secret'),
    ),
  );
  check(
    failure(thrown) == "couldn't save the file",
    'phone save: anything else is the generic failure, without its text',
  );
  check(requestDirectories(temp).isEmpty, 'phone save: failures leave nothing');

  // A temporary directory that cannot be used fails the request, never
  // writes elsewhere.
  final blocked = '${directory.path}/phone-blocked';
  File(blocked).writeAsStringSync('not a directory');
  final noRoot = await mosaicRunPhoneFilesSave(
    request,
    FakePhoneDocuments(blocked, save: (staged, mime) async => 'x'),
  );
  check(failure(noRoot) == "couldn't save the file", 'phone save: no usable root');
  if (Platform.isWindows) return; // links: see checkPhoneOpen
  final linkedRoot = Directory('${directory.path}/phone-linked')..createSync();
  Link('${linkedRoot.path}/$mosaicPhoneFilesDirectoryName')
      .createSync(directory.path);
  final throughLink = await mosaicRunPhoneFilesSave(
    request,
    FakePhoneDocuments(linkedRoot.path, save: (staged, mime) async => 'x'),
  );
  check(
    failure(throughLink) == "couldn't save the file",
    'phone save: a linked mosaic-files is refused',
  );
}

Future<void> checkPhoneLeftovers(Directory directory) async {
  final temporary = Directory('${directory.path}/phone-sweep')..createSync();
  final root = Directory('${temporary.path}/$mosaicPhoneFilesDirectoryName')
    ..createSync();
  final old = Directory('${root.path}/request-old')..createSync();
  File('${old.path}/copy.json').writeAsStringSync('old');
  final young = Directory('${root.path}/request-young')..createSync();
  final outside = File('${directory.path}/phone-outside.txt')
    ..writeAsStringSync('keep');
  // Links where the harness makes them (see checkPhoneOpen). The old
  // directory's link reaches outside: the sweep must not follow it.
  final withLinks = !Platform.isWindows;
  final linked = Link('${root.path}/request-link');
  if (withLinks) {
    linked.createSync(outside.path);
    Link('${old.path}/escape').createSync(outside.path);
  }

  // Both directories were just made: a sweep now keeps them, and a sweep an
  // hour and a minute from now finds both old.
  final now = DateTime.now();
  final later = now.add(mosaicPhoneLeftoverAge + const Duration(minutes: 1));
  mosaicSweepPhoneLeftovers(root, now);
  check(old.existsSync() && young.existsSync(), 'a fresh sweep deletes nothing');
  mosaicSweepPhoneLeftovers(
    root,
    now.add(mosaicPhoneLeftoverAge - const Duration(minutes: 1)),
  );
  check(old.existsSync(), 'an entry younger than an hour is kept');
  mosaicSweepPhoneLeftovers(root, later);
  check(
    !old.existsSync() && !young.existsSync(),
    'a sweep deletes request directories unchanged for over an hour',
  );
  check(outside.readAsStringSync() == 'keep', 'without following its links');
  if (withLinks) {
    check(
      linked.existsSync() && outside.existsSync(),
      'a link in mosaic-files is left alone',
    );
  }

  // The router sweeps before its first request, and keeps what is young:
  // another engine's request in flight looks like this one.
  final inFlight = Directory('${root.path}/request-in-flight')..createSync();
  final queued = <void Function()>[];
  final host = FakeHost();
  var keptInFlight = false;
  final documents = FakePhoneDocuments(
    temporary.path,
    open: (dir, mimes, limit) async {
      keptInFlight = inFlight.existsSync();
      return null;
    },
  );
  installMosaicPlatformRouter(
    host,
    appKinds: null,
    dialogs: FakeDialogs(null),
    phoneDocuments: documents,
    runOnUi: queued.add,
  );
  host.waitingOn = <int>{1};
  host.effectHandler!(1, 'files.open', payload({}), 'await');
  queued.removeAt(0)();
  check(await host.answer(1) == '{"cancelled":{}}', 'the router asks the plugin');
  check(keptInFlight, 'the router\'s sweep keeps a young entry');
}

Future<void> checkPhoneRouter(Directory directory) async {
  final temporary = Directory('${directory.path}/phone-router')..createSync();
  final queued = <void Function()>[];
  final host = FakeHost();
  final pending = Completer<String?>();
  final documents = FakePhoneDocuments(
    temporary.path,
    save: (staged, mime) => pending.future,
  );
  final dialogs = FakeDialogs('${directory.path}/never.txt');
  installMosaicPlatformRouter(
    host,
    appKinds: null,
    dialogs: dialogs,
    phoneDocuments: documents,
    hasDialogs: false,
    runOnUi: queued.add,
  );
  final request = payload({'suggestedName': 'deck.json', 'bytes': encoded('x')});
  host.waitingOn = <int>{1, 2};
  host.effectHandler!(1, 'files.save', request, 'await');
  check(host.deferred.join(',') == '1', 'phone: deferred before any picker');
  queued.removeAt(0)();
  host.effectHandler!(2, 'files.save', request, 'await');
  check(
    await answeredFailure(host, 2) == 'another file operation is in progress',
    'phone: one file operation at a time',
  );
  for (var tries = 0; tries < 200 && documents.exported == 0; tries += 1) {
    await Future<void>.delayed(const Duration(milliseconds: 5));
  }
  pending.complete('deck.json');
  check(await answeredName(host, 1) == 'deck.json', 'phone: answered through the plugin');
  check(dialogs.opened == 0, 'phone: the desktop dialogs are never shown');
  check(documents.exported == 1, 'phone: one export');

  // Given a plugin, a phone has pickers; without one, it does not.
  final withoutPlugin = FakeHost();
  installMosaicPlatformRouter(
    withoutPlugin,
    appKinds: null,
    dialogs: dialogs,
    hasDialogs: false,
    runOnUi: (work) => work(),
  );
  withoutPlugin.waitingOn = <int>{3};
  withoutPlugin.effectHandler!(3, 'files.open', payload({}), 'await');
  check(
    await answeredFailure(withoutPlugin, 3) ==
        'files.open is not available on this platform yet',
    'phone without the plugin: still not available',
  );
}

Future<void> main() async {
  final directory = Directory.systemTemp.createTempSync(
    'mosaic-flutter-platform-effects-',
  );
  try {
    checkRouting();
    await checkSave(directory);
    await checkConfirmReplacing(directory);
    await checkPosixCalls(directory);
    await checkSaveRefusals(directory);
    checkNameSafety();
    await checkOpen(directory);
    await checkRouter(directory);
    await checkPhoneOpen(directory);
    await checkPhoneSave(directory);
    await checkPhoneLeftovers(directory);
    await checkPhoneRouter(directory);
  } finally {
    directory.deleteSync(recursive: true);
  }
  stdout.writeln(
    'Mosaic Flutter platform effects conformance passed ($_checks checks)',
  );
}
