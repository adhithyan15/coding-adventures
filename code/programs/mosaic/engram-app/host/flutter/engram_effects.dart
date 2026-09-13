// Answer Engram's Anki file-dialog effects on Flutter.
//
// Everything else about the application -- props, events, snapshot, restore --
// goes through the standard Mosaic host. This is the one thing a generated host
// cannot do on the application's behalf, because only the host has a window to
// hang a dialog off.
//
// Installed by the generated `main.dart` through `[host_effects]`, immediately
// after the host is assigned.
//
// DEFERRED, like SwiftUI and Compose -- but for a different reason, and with
// nothing to marshal.
//
// Those two defer because the host holds a lock across the handler call, and
// they marshal because their answer would otherwise land on the wrong thread.
// Neither applies here, and the generated host says so itself: "a Dart isolate
// is single-threaded, so an answer never arrives concurrently with a settle --
// it arrives on a later turn of the event loop. That removes the deadlock those
// hosts have to warn about, and it removes the need to marshal a deferred
// answer anywhere."
//
// The reason here is simpler and inescapable: `openFile` and `getSaveLocation`
// return `Future`s, and `effectHandler` is a synchronous callback. There is no
// answer to give inline. `deferEffect` is not the better of two options, it is
// the only one that does not lose the effect.
//
// Which makes the answering discipline load-bearing in the same way. Once
// `deferEffect` returns true the id is out of the runtime's fail sweep, so a
// path that forgets to answer does not degrade to "failed" -- it wedges the app
// permanently, because the runtime gates snapshot AND restore on nothing being
// pending. Every `_run*` below therefore RETURNS an outcome rather than
// answering, so all paths funnel to exactly one `completeEffect` at the call
// site, and the whole future is wrapped so a throw cannot skip it.

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';

import 'mosaic_host.dart';

// The three tagged outcomes the protocol defines. Cancellation is not a
// failure: Escape in a file dialog is an ordinary thing for a person to do, and
// the application says "Import cancelled." rather than "Import failed."
Map<String, Object?> _okOutcome([
  Map<String, Object?> value = const <String, Object?>{},
]) => <String, Object?>{'ok': value};

Map<String, Object?> _cancelledOutcome() => <String, Object?>{
  'cancelled': <String, Object?>{},
};

Map<String, Object?> _failedOutcome(String message) => <String, Object?>{
  'failed': <String, Object?>{'message': message},
};

// The largest package this host will read into memory.
//
// Aligned with what the engine will actually accept: the package layer refuses
// a collection past 256 MiB on native targets, so a larger file cannot import
// whatever this does. Checked BEFORE the read, because the point is to never
// make the allocation -- one import costs several times the file's size in
// flight (the bytes, their base64, that base64 as a String, the JSON envelope,
// and the runtime's own copy), and the application's own cap sits at the far
// end of all of it.
const int _maxImportBytes = 256 * 1024 * 1024;

// The shape an extension has to have before it reaches the picker.
//
// `^` and `$`, and NOT the `\A`/`\z` the Kotlin and Swift handlers use. This is
// the fifth regex engine asked the anchor question and the first where copying
// the previous answer would have been actively wrong, so it was measured on
// Dart 3.9.4 rather than carried over:
//
//   * `^...$` with no `multiLine` refuses a trailing LF, CRLF, CR, NEL, U+2028
//     and U+2029 -- all six. That matches Rust and beats PCRE2 (concedes LF),
//     ICU (concedes all) and Java (concedes five of the six).
//   * With `multiLine: true` it concedes LF, CRLF, CR, U+2028 and U+2029 but
//     NOT NEL, which is a seventh distinct answer. This RegExp is built without
//     that flag, deliberately.
//   * `\A` and `\z` do not fail here, which is the trap. Dart's RegExp is
//     ECMAScript-derived, where both are IDENTITY ESCAPES: `\A` is a literal
//     `A` and `\z` is a literal `z`. `RegExp(r'\A\.?[A-Za-z0-9_-]{1,16}\z')`
//     compiles, rejects `"apkg"` and `".apkg"`, accepts `"A.apkgz"`, and is not
//     anchored at all -- `firstMatch("xxAapkgzyy")` returns `Aapkgz`. Every
//     real extension would be refused, `_allowedExtensions` would fall back
//     forever, and the payload's list would be silently ignored.
//
// An extension is accepted only if it looks like one. The payload comes from
// this application rather than a user today, but a filter widened to everything
// is not something to leave to that staying true.
final RegExp _extensionShape = RegExp(r'^\.?[A-Za-z0-9_-]{1,16}$');

List<String> _allowedExtensions(
  Object? payload,
  String key,
  List<String> fallback,
) {
  final declared = (payload is Map ? payload[key] : null);
  final source = declared is List
      ? declared.whereType<String>().toList(growable: false)
      : const <String>[];
  final candidates = source.isEmpty ? fallback : source;
  final accepted = candidates
      .where(_extensionShape.hasMatch)
      // `file_selector` wants bare extensions; a leading dot makes the Linux
      // and Windows pickers filter on `*..apkg`.
      .map((extension) => extension.startsWith('.') ? extension.substring(1) : extension)
      .toList(growable: false);
  // Everything was refused, so fall back rather than hand the picker an empty
  // extension list, which shows the person a dialog matching nothing.
  return accepted.isEmpty
      ? fallback
            .map((e) => e.startsWith('.') ? e.substring(1) : e)
            .toList(growable: false)
      : accepted;
}

List<XTypeGroup> _ankiTypeGroups(List<String> extensions) => <XTypeGroup>[
  XTypeGroup(label: 'Anki packages', extensions: extensions),
];

// The bytes travel, not the path.
//
// The application built this package and sent it out in the payload, so the
// host only has to write it. That is the arrangement sandboxing forces
// elsewhere in this family of hosts, and keeping all five the same is worth
// more than shaving a copy on the platforms that would allow it.
Future<Map<String, Object?>> _runExport(Object? payload) async {
  final fields = payload is Map ? payload : const <Object?, Object?>{};

  // Forced to a bare filename. A suggestion of `../.ssh/authorized_keys` would
  // otherwise open the dialog in a different directory with only the basename
  // visible. Nothing sends this key today; that is not a reason to trust it.
  //
  // Separators are normalised first: `lastIndexOf('/')` alone would keep the
  // whole of `..\\..\\evil` on Windows, where the picker treats `\` as a
  // separator too.
  final rawSuggestion = fields['suggestedName']?.toString() ?? '';
  final normalised = rawSuggestion.replaceAll('\\', '/');
  final separator = normalised.lastIndexOf('/');
  final bare = separator < 0
      ? normalised
      : normalised.substring(separator + 1);
  final suggested = (bare.isEmpty || bare == '.' || bare == '..')
      ? 'engram.apkg'
      : bare;

  final FileSaveLocation? location;
  try {
    location = await getSaveLocation(
      acceptedTypeGroups: _ankiTypeGroups(
        _allowedExtensions(payload, 'extensions', const <String>['.apkg']),
      ),
      suggestedName: suggested,
      confirmButtonText: 'Export',
    );
  } on Object catch (error) {
    return _failedOutcome(_reason(error, 'the export dialog could not open'));
  }
  if (location == null) {
    return _cancelledOutcome();
  }

  final target = location.path.toLowerCase().endsWith('.apkg')
      ? location.path
      : '${location.path}.apkg';

  final encoded = fields['apkg'];
  if (encoded is! String || encoded.isEmpty) {
    return _failedOutcome('the export carried no package');
  }
  // Strict decoding. `base64Decode` rejects a character outside the alphabet
  // rather than skipping it, which matters because silently discarding one
  // would write a corrupt `.apkg` that only fails later, inside Anki, where
  // nothing points back here.
  final Uint8List decoded;
  try {
    decoded = base64Decode(encoded);
  } on FormatException {
    return _failedOutcome('the export package was not valid base64');
  }
  // A zip, which is what an `.apkg` is. Not an is-it-empty check: padding-only
  // input decodes SUCCESSFULLY to a byte or two, so an emptiness test would
  // pass it and write a file Anki cannot open.
  if (decoded.length < 4 ||
      decoded[0] != 0x50 ||
      decoded[1] != 0x4B ||
      decoded[2] != 0x03 ||
      decoded[3] != 0x04) {
    return _failedOutcome('the export package was not a valid Anki package');
  }

  // Written beside the target and renamed into place, so a failure partway
  // leaves no truncated `.apkg` behind looking like a real one. The temp file
  // is a sibling rather than one in the system temp directory, because `rename`
  // is only atomic within a filesystem and the person may well have picked an
  // external disk.
  final temp = File('$target.part');
  try {
    await temp.writeAsBytes(decoded, flush: true);
    await temp.rename(target);
  } on Object catch (error) {
    try {
      if (await temp.exists()) await temp.delete();
    } on Object {
      // The write already failed; the app learns nothing useful from a second
      // failure cleaning up after it, and swallowing this cannot lose the
      // original reason, which is reported below.
    }
    return _failedOutcome(_reason(error, 'the export could not be written'));
  }
  return _okOutcome();
}

Future<Map<String, Object?>> _runImport(Object? payload) async {
  final XFile? file;
  try {
    file = await openFile(
      acceptedTypeGroups: _ankiTypeGroups(
        _allowedExtensions(payload, 'accept', const <String>[
          '.apkg',
          '.colpkg',
        ]),
      ),
      confirmButtonText: 'Import',
    );
  } on Object catch (error) {
    return _failedOutcome(_reason(error, 'the import dialog could not open'));
  }
  if (file == null) {
    return _cancelledOutcome();
  }

  // Sized up before it is opened, through the SAME path that is then read.
  //
  // `File.stat()` follows symlinks, as `readAsBytes` does, so the thing
  // inspected and the thing read cannot be two different files -- the mismatch
  // the SwiftUI handler had to reason its way out of.
  //
  // The regular-file test is not a restatement of the size test: a read on a
  // fifo or a character device never reaches EOF, and neither reports a
  // meaningful size.
  final handle = File(file.path);
  final FileStat stat;
  try {
    stat = await handle.stat();
  } on Object catch (error) {
    return _failedOutcome(_reason(error, 'that file could not be read'));
  }
  if (stat.type != FileSystemEntityType.file) {
    return _failedOutcome('that is not a regular file');
  }
  if (stat.size <= 0) {
    return _failedOutcome('that file is empty');
  }
  if (stat.size > _maxImportBytes) {
    return _failedOutcome('that package is too large to open');
  }

  final Uint8List bytes;
  try {
    bytes = await handle.readAsBytes();
  } on Object catch (error) {
    return _failedOutcome(_reason(error, 'that file could not be read'));
  }
  if (bytes.isEmpty) {
    return _failedOutcome('that file is empty');
  }

  // The application decodes and merges. Reading the file is the host's whole
  // job here, for the same reason the export writes it.
  return _okOutcome(<String, Object?>{'apkg': base64Encode(bytes)});
}

// A message for the person, never an empty string.
//
// `error.toString()` on a `FileSystemException` already reads reasonably
// ("Cannot open file, path = '...' (OS Error: ...)"), but an arbitrary thrown
// object can stringify to nothing at all, and an empty `message` in a `failed`
// outcome shows the person a status line with no status in it.
String _reason(Object error, String fallback) {
  final text = error.toString().trim();
  return text.isEmpty ? fallback : text;
}

/// Answer Engram's awaited file-dialog effects.
///
/// The generated `main.dart` is the only caller, and it passes the host it just
/// assigned.
void installEngramEffects(MosaicHost host) {
  host.effectHandler = (int id, String kind, Object? payload, String delivery) {
    // Only the awaited kinds are answered. `openCard` arrives as a `Notify` and
    // is deliberately not handled: nothing is waiting on it, and answering an
    // effect the runtime is not awaiting is refused anyway.
    if (delivery.toLowerCase() != 'await') return;

    // Exactly the two kinds `effect_for_intent` mints as `Await`.
    // `host_intent_for_event` emits one other intent, `openCard`, and it is the
    // `Notify` excluded above. A branch for any other kind would be unreachable
    // code that reads to the next person as shipped behaviour.
    if (kind != 'importAnki' && kind != 'exportAnki') {
      // An awaited kind this host does not know is left alone on purpose. The
      // host's own sweep then fails it with a reason the application shows,
      // which is a better outcome than this file inventing one -- and it is
      // what keeps a new effect kind from silently doing nothing while
      // appearing handled.
      return;
    }

    // Ownership first, and synchronously. `deferEffect` returns false for an id
    // the runtime is not actually waiting on, and in that case the right move
    // is to do nothing at all rather than open a dialog whose answer would be
    // refused.
    //
    // It has to happen before the first `await`: the settle that called this
    // handler finishes at the end of this function, and an effect not yet
    // deferred by then has already been swept and failed.
    if (!host.deferEffect(id)) return;

    // Nothing may escape this future. The effect left the fail sweep the moment
    // `deferEffect` returned true, so an error that reaches the zone's uncaught
    // handler does not degrade the outcome -- it leaves the id awaited for the
    // life of the process.
    //
    // The `_run*` functions guard their own dialogs and I/O, so this is the
    // backstop for what they cannot: a plugin that throws outside those spans,
    // or an `Error` from the framework itself. `Object`, not `Exception`, since
    // Dart lets anything be thrown and a missed answer here is permanent.
    () async {
      Map<String, Object?> outcome;
      try {
        outcome = kind == 'importAnki'
            ? await _runImport(payload)
            : await _runExport(payload);
      } on Object catch (error) {
        outcome = _failedOutcome(_reason(error, 'the dialog could not be opened'));
      }
      try {
        host.completeEffect(id, outcome);
      } on Object catch (error) {
        // The answer itself was refused -- a disposed host, or a runtime that
        // predates protocol 2. There is nothing left to answer with, so report
        // it where it can be seen rather than letting it vanish.
        stderr.writeln('Engram could not answer effect $id ($kind): $error');
      }
    }();
  };
}
