// Answer UI59's `files.open` effect on Flutter -- see
// code/specs/UI59-files-open-effect.md for the full contract.
//
// Everything else about the application -- props, events, snapshot, restore --
// goes through the standard Mosaic host. This is the one thing a generated
// host cannot do on the application's behalf, because only the host has a
// window to hang a dialog off (the same reason engram-app's Flutter host
// needs its own `engram_effects.dart`, which this mirrors).
//
// Installed by the generated `main.dart` through `[host_effects]`, immediately
// after the host is assigned.
//
// DEFERRED, like SwiftUI/Compose/Qt (well, Qt didn't need to) -- but for a
// different, Flutter-specific reason, matching engram-app's own Flutter
// handler exactly: `openFile` returns a `Future`, and `effectHandler` is a
// synchronous callback. There is no answer to give inline; `deferEffect` is
// not the better of two options, it is the only one that does not lose the
// effect. A Dart isolate is single-threaded, so unlike Qt/Compose there is
// nothing to marshal -- the eventual answer just arrives on a later turn of
// the event loop.

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';

import 'mosaic_host.dart';

// The three tagged outcomes UI59 §3 defines. Cancellation is not a failure:
// Escape in a file dialog is an ordinary thing for a person to do.
Map<String, Object?> _okOutcome(Map<String, Object?> value) =>
    <String, Object?>{'ok': value};

Map<String, Object?> _cancelledOutcome() => <String, Object?>{
  'cancelled': <String, Object?>{},
};

Map<String, Object?> _failedOutcome(String message) => <String, Object?>{
  'failed': <String, Object?>{'message': message},
};

// Common photo MIME types this app's own request asks for (see
// photo-picker-mosaic-app's ACCEPT_IMAGE_TYPES) mapped to the file extensions
// `file_selector`'s `XTypeGroup` actually wants -- it filters by extension,
// not MIME type (bare, no leading dot -- a leading dot makes the Linux and
// Windows pickers filter on `*..ext`, the same trap engram_effects.dart's own
// `_allowedExtensions` documents), so the handler owns this table rather than
// the app needing to know Flutter-specific filter syntax (UI59 §3), matching
// the table the XAML/Qt/Compose handlers each carry for the same reason.
const Map<String, List<String>> _mimeTypeExtensions = <String, List<String>>{
  'image/jpeg': <String>['jpg', 'jpeg'],
  'image/png': <String>['png'],
  'image/webp': <String>['webp'],
  'image/gif': <String>['gif'],
  'image/bmp': <String>['bmp'],
  'image/tiff': <String>['tif', 'tiff'],
};

// The reverse of the table above, for reporting the picked file's MIME type
// back to the app (UI59 §3's `mimeType` result field) -- an `XFile` carries
// no MIME type of its own on every platform, so this is recovered from the
// extension.
String _mimeTypeForExtension(String extension) {
  final lowered = extension.toLowerCase();
  for (final entry in _mimeTypeExtensions.entries) {
    if (entry.value.contains(lowered)) return entry.key;
  }
  return 'application/octet-stream';
}

// `accept` MIME types this host doesn't recognise are dropped rather than
// failing the request (UI59 §3); if nothing was recognised, an empty
// extension list (`file_selector` then shows "All Files").
List<String> _extensionsFromAccept(Object? payload) {
  final accept = payload is Map ? payload['accept'] : null;
  final declared = accept is List
      ? accept.whereType<String>().toList(growable: false)
      : const <String>[];
  return declared
      .expand((mimeType) => _mimeTypeExtensions[mimeType] ?? const <String>[])
      .toList(growable: false);
}

// A picked file is read fully into memory and base64-encoded (UI59 §3's
// `bytes` field is the whole file); without a cap, a caller picking an
// arbitrarily large file costs an arbitrarily large amount of host memory.
// 50 MiB matches the XAML/Qt/Compose handlers' own cap, for parity across
// backends.
const int _maxPickedFileBytes = 50 * 1024 * 1024;

// Reads at most `_maxPickedFileBytes` and returns null if the file ran over,
// rather than trusting `FileStat.size` checked once beforehand -- Engram's
// own Flutter handler (and the XAML handler's first cut) does exactly that
// single up-front check, which `/security-review` found is TOCTOU on the
// XAML PR (#15218): the file can grow between the check and the read, so a
// pre-check alone doesn't actually bound anything. Reading in bounded chunks
// here, the way the Qt (#15252) and Compose (#15329) handlers were hardened
// to from the start, means this handler never has that gap to begin with.
Future<Uint8List?> _readBounded(RandomAccessFile handle) async {
  final builder = BytesBuilder(copy: false);
  while (true) {
    final chunk = await handle.read(64 * 1024);
    if (chunk.isEmpty) return builder.toBytes();
    if (builder.length + chunk.length > _maxPickedFileBytes) return null;
    builder.add(chunk);
  }
}

// Returns the outcome rather than answering, so every path funnels to
// exactly one `completeEffect` at the call site -- same discipline as
// Engram's `_runImport`/`_runExport` (the effect has been deferred by then,
// so a path that forgets to answer does not degrade to "failed", it wedges
// the app permanently).
Future<Map<String, Object?>> _runPickPhoto(Object? payload) async {
  final XFile? file;
  try {
    file = await openFile(
      acceptedTypeGroups: <XTypeGroup>[
        XTypeGroup(label: 'Images', extensions: _extensionsFromAccept(payload)),
      ],
      confirmButtonText: 'Pick',
    );
  } on Object {
    // Not the dialog's own error text (`error.toString()`, what Engram's own
    // `_reason()` helper surfaces) -- a `FileSystemException`'s message
    // routinely embeds the full local filesystem path, and `failed.message`
    // is app-visible data (the same reasoning that moved the XAML, Qt, and
    // Compose handlers off raw exception messages, UI59 §4.2/§7.2/§10.2).
    return _failedOutcome('couldn\'t open the photo picker');
  }
  if (file == null) {
    return _cancelledOutcome();
  }

  // Opened through the SAME path that is then read, matching Engram's own
  // reasoning (`File.stat()` follows symlinks the same way a read does, so
  // the thing inspected and the thing read cannot be two different files).
  final handle = File(file.path);
  final RandomAccessFile opened;
  try {
    opened = await handle.open();
  } on Object {
    return _failedOutcome('couldn\'t read the selected file');
  }
  try {
    final stat = await handle.stat();
    if (stat.type != FileSystemEntityType.file) {
      return _failedOutcome('that is not a regular file');
    }

    final bytes = await _readBounded(opened);
    if (bytes == null) {
      return _failedOutcome(
        'the selected file is too large or couldn\'t be read (limit is $_maxPickedFileBytes bytes)',
      );
    }

    final extension = file.name.contains('.') ? file.name.split('.').last : '';
    return _okOutcome(<String, Object?>{
      'name': file.name,
      'mimeType': _mimeTypeForExtension(extension),
      'bytes': base64Encode(bytes),
    });
  } on Object {
    return _failedOutcome('couldn\'t read the selected file');
  } finally {
    await opened.close();
  }
}

/// Answer UI59's awaited `files.open` effect.
///
/// The generated `main.dart` is the only caller, and it passes the host it
/// just assigned.
void installPhotoPickerEffects(MosaicHost host) {
  host.effectHandler = (int id, String kind, Object? payload, String delivery) {
    if (delivery.toLowerCase() != 'await') return;
    if (kind != 'files.open') {
      // An awaited kind this host does not know is left alone on purpose --
      // the host's own sweep fails it with a reason the application shows
      // (same convention as Engram's Flutter handler).
      return;
    }

    // Ownership first, and synchronously -- before the first `await`, or the
    // settle that called this handler finishes with the effect not yet
    // deferred, and it has already been swept and failed by then.
    if (!host.deferEffect(id)) return;

    // Nothing may escape this future. The effect left the fail sweep the
    // moment `deferEffect` returned true, so an error that reaches the
    // zone's uncaught handler leaves the id awaited for the life of the
    // process. `Object`, not a narrower type: Dart lets anything be thrown,
    // and a missed answer here is permanent (same reasoning as Engram's
    // Flutter handler -- and, unlike the Compose handler's original
    // `catch (Exception)`, this was never narrower than "everything" to
    // begin with, so there was no separate OOM-wedge class of bug to find
    // here).
    () async {
      Map<String, Object?> outcome;
      try {
        outcome = await _runPickPhoto(payload);
      } on Object {
        outcome = _failedOutcome('couldn\'t pick a photo');
      }
      try {
        host.completeEffect(id, outcome);
      } on Object catch (error) {
        // The answer itself was refused -- a disposed host, or a runtime
        // that predates protocol 2. There is nothing left to answer with,
        // so report it where it can be seen rather than letting it vanish
        // (same convention as Engram's Flutter handler).
        stderr.writeln('PhotoPickerApp could not answer effect $id ($kind): $error');
      }
    }();
  };
}
