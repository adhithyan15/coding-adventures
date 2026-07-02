import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

final class EgSession extends Opaque {}

typedef _EgSessionNewNative = Pointer<EgSession> Function();
typedef _EgSessionNewDart = Pointer<EgSession> Function();
typedef _EgSessionFreeNative = Void Function(Pointer<EgSession>);
typedef _EgSessionFreeDart = void Function(Pointer<EgSession>);
typedef _EgStringFreeNative = Void Function(Pointer<Utf8>);
typedef _EgStringFreeDart = void Function(Pointer<Utf8>);
typedef _EgSnapshotNative = Pointer<Utf8> Function(Pointer<EgSession>);
typedef _EgSnapshotDart = Pointer<Utf8> Function(Pointer<EgSession>);
typedef _EgLoadSnapshotNative = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Utf8>,
);
typedef _EgLoadSnapshotDart = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Utf8>,
);
typedef _EgEngramAppPropsNative = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Utf8>,
  Uint64,
);
typedef _EgEngramAppPropsDart = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Utf8>,
  int,
);
typedef _EgHandleEngramAppEventNative = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  Uint64,
);
typedef _EgHandleEngramAppEventDart = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  int,
);

class MosaicHost {
  MosaicHost._(this._api, this._session);

  final _EngramCapi _api;
  final Pointer<EgSession> _session;
  bool _disposed = false;

  static MosaicHost? load() {
    final api = _EngramCapi.load();
    if (api == null) return null;

    final session = api.egSessionNewDemo();
    if (session == nullptr) return null;

    final host = MosaicHost._(api, session);
    host._hydrateSession();
    return host;
  }

  Map<String, Object?>? props() {
    if (_disposed) {
      return const <String, Object?>{'error': 'Engram Flutter MosaicHost disposed'};
    }
    return _withNativeUtf8(_deckId(), (deckIdPointer) {
      final json = _takeCString(
        _api.egEngramAppProps(_session, deckIdPointer, _nowMs()),
      );
      return _hostResponseFromJson(json);
    });
  }

  Map<String, Object?>? handleEvent(Map<String, Object?> event) {
    if (_disposed) {
      return const <String, Object?>{'error': 'Engram Flutter MosaicHost disposed'};
    }
    return _withNativeUtf8(jsonEncode(event), (eventPointer) {
      return _withNativeUtf8(_deckId(), (deckIdPointer) {
        final json = _takeCString(
          _api.egHandleEngramAppEvent(
            _session,
            eventPointer,
            deckIdPointer,
            _nowMs(),
          ),
        );
        final response = _hostResponseFromJson(json);
        if (response?['error'] == null) {
          _persistSnapshot();
        }
        return response;
      });
    });
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    if (_session != nullptr) {
      _api.egSessionFree(_session);
    }
  }

  String _takeCString(Pointer<Utf8> pointer) {
    if (pointer == nullptr) {
      return '{"ok":false,"error":"Engram native host returned null"}';
    }
    try {
      return pointer.toDartString();
    } finally {
      _api.egStringFree(pointer);
    }
  }

  void _hydrateSession() {
    final file = File(_snapshotPath());
    if (file.existsSync()) {
      try {
        final snapshot = file.readAsStringSync();
        if (_loadSnapshot(snapshot)) {
          return;
        }
      } catch (_) {}
    }
    _persistSnapshot();
  }

  bool _loadSnapshot(String snapshot) {
    return _withNativeUtf8(snapshot, (snapshotPointer) {
      final json = _takeCString(
        _api.egLoadSnapshot(_session, snapshotPointer),
      );
      return _jsonOk(json);
    });
  }

  void _persistSnapshot() {
    final json = _takeCString(_api.egSnapshot(_session));
    final Object? decoded;
    try {
      decoded = jsonDecode(json);
    } catch (_) {
      return;
    }
    if (decoded is! Map || decoded['ok'] == false || decoded['state'] == null) {
      return;
    }

    try {
      final file = File(_snapshotPath());
      file.parent.createSync(recursive: true);
      file.writeAsStringSync(jsonEncode(decoded['state']));
    } catch (_) {}
  }
}

class _EngramCapi {
  _EngramCapi._(DynamicLibrary library)
      : egSessionNewDemo = library.lookupFunction<_EgSessionNewNative, _EgSessionNewDart>(
          'eg_session_new_demo',
        ),
        egSessionFree =
            library.lookupFunction<_EgSessionFreeNative, _EgSessionFreeDart>(
          'eg_session_free',
        ),
        egStringFree =
            library.lookupFunction<_EgStringFreeNative, _EgStringFreeDart>(
          'eg_string_free',
        ),
        egSnapshot = library.lookupFunction<_EgSnapshotNative, _EgSnapshotDart>(
          'eg_snapshot',
        ),
        egLoadSnapshot =
            library.lookupFunction<_EgLoadSnapshotNative, _EgLoadSnapshotDart>(
          'eg_load_snapshot',
        ),
        egEngramAppProps = library
            .lookupFunction<_EgEngramAppPropsNative, _EgEngramAppPropsDart>(
          'eg_engram_app_props',
        ),
        egHandleEngramAppEvent = library.lookupFunction<
            _EgHandleEngramAppEventNative,
            _EgHandleEngramAppEventDart>('eg_handle_engram_app_event');

  final _EgSessionNewDart egSessionNewDemo;
  final _EgSessionFreeDart egSessionFree;
  final _EgStringFreeDart egStringFree;
  final _EgSnapshotDart egSnapshot;
  final _EgLoadSnapshotDart egLoadSnapshot;
  final _EgEngramAppPropsDart egEngramAppProps;
  final _EgHandleEngramAppEventDart egHandleEngramAppEvent;

  static _EngramCapi? load() {
    if (Platform.isIOS) {
      try {
        return _EngramCapi._(DynamicLibrary.process());
      } catch (_) {}
    }

    for (final candidate in _libraryCandidates()) {
      try {
        return _EngramCapi._(DynamicLibrary.open(candidate));
      } catch (_) {}
    }
    return null;
  }
}

Map<String, Object?> _hostResponseFromJson(String json) {
  if (json.trim().isEmpty) return const <String, Object?>{};

  final Object? decoded;
  try {
    decoded = jsonDecode(json);
  } catch (error) {
    return <String, Object?>{'error': 'Engram native host returned invalid JSON: $error'};
  }
  if (decoded is! Map) {
    return <String, Object?>{'error': 'Engram native host returned non-object JSON'};
  }
  if (decoded['ok'] == false) {
    return <String, Object?>{'error': decoded['error']};
  }

  final response = <String, Object?>{
    'props': _mosaicMap(decoded['props']),
  };
  final hostIntent = _mosaicMap(decoded['hostIntent']);
  if (hostIntent.isNotEmpty) {
    response['hostIntent'] = hostIntent;
  }
  return response;
}

bool _jsonOk(String json) {
  try {
    final decoded = jsonDecode(json);
    return decoded is Map && decoded['ok'] != false;
  } catch (_) {
    return false;
  }
}

Map<String, Object?> _mosaicMap(Object? value) {
  if (value is! Map) return const <String, Object?>{};
  final out = <String, Object?>{};
  for (final entry in value.entries) {
    final key = entry.key;
    if (key is String) {
      out[key] = entry.value;
    }
  }
  return out;
}

T _withNativeUtf8<T>(String value, T Function(Pointer<Utf8>) body) {
  final pointer = value.toNativeUtf8(allocator: calloc);
  try {
    return body(pointer);
  } finally {
    calloc.free(pointer);
  }
}

String _deckId() => Platform.environment['ENGRAM_DECK_ID'] ?? '';

int _nowMs() => DateTime.now().toUtc().millisecondsSinceEpoch;

String _snapshotPath() =>
    Platform.environment['ENGRAM_SNAPSHOT_PATH'] ??
    _joinPath(_joinPath(_homeDirectory(), '.engram'), 'mosaic-snapshot.v1.json');

String _homeDirectory() =>
    Platform.environment['HOME'] ??
    Platform.environment['USERPROFILE'] ??
    Directory.current.path;

Iterable<String> _libraryCandidates() sync* {
  final fileName = _nativeLibraryFileName();
  yield fileName;
  yield 'engram_capi';

  final roots = <String>{
    Directory.current.path,
    File(Platform.resolvedExecutable).parent.path,
  };
  for (final root in roots) {
    yield _joinPath(root, fileName);
  }
}

String _nativeLibraryFileName() {
  if (Platform.isWindows) return 'engram_capi.dll';
  if (Platform.isMacOS || Platform.isIOS) return 'libengram_capi.dylib';
  return 'libengram_capi.so';
}

String _joinPath(String root, String child) {
  if (root.endsWith(Platform.pathSeparator)) return '$root$child';
  return '$root${Platform.pathSeparator}$child';
}
