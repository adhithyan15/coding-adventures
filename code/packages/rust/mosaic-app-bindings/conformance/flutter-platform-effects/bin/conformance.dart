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
import 'dart:io';

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

Future<void> main() async {
  final directory = Directory.systemTemp.createTempSync(
    'mosaic-flutter-platform-effects-',
  );
  try {
    checkRouting();
    await checkSave(directory);
    await checkSaveRefusals(directory);
    await checkOpen(directory);
    await checkRouter(directory);
  } finally {
    directory.deleteSync(recursive: true);
  }
  stdout.writeln(
    'Mosaic Flutter platform effects conformance passed ($_checks checks)',
  );
}
