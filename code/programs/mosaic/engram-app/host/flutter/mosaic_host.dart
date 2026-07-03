import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';
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
typedef _EgExportAnkiApkgNative = Pointer<Utf8> Function(Pointer<EgSession>);
typedef _EgExportAnkiApkgDart = Pointer<Utf8> Function(Pointer<EgSession>);
typedef _EgMergeAnkiApkgNative = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Uint8>,
  Uint64,
);
typedef _EgMergeAnkiApkgDart = Pointer<Utf8> Function(
  Pointer<EgSession>,
  Pointer<Uint8>,
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

  Future<Map<String, Object?>?> handleEvent(Map<String, Object?> event) async {
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
        return _handleHostIntent(response).then((handled) {
          if (handled?['error'] == null) {
            _persistSnapshot();
          }
          return handled;
        });
      });
    });
  }

  Future<Map<String, Object?>?> _handleHostIntent(
    Map<String, Object?>? response,
  ) async {
    if (response == null || response['error'] != null) {
      return response;
    }

    final hostIntent = _mosaicMap(response['hostIntent']);
    switch (hostIntent['type']) {
      case 'importAnki':
        return _importAnkiPackage(response, hostIntent);
      case 'exportAnki':
        return _exportAnkiPackage(response, hostIntent);
      default:
        return response;
    }
  }

  Future<Map<String, Object?>?> _importAnkiPackage(
    Map<String, Object?> response,
    Map<String, Object?> hostIntent,
  ) async {
    final XFile? file;
    try {
      file = await openFile(
        acceptedTypeGroups: _ankiTypeGroups(
          hostIntentExtensions(hostIntent, 'accept', const <String>[
            '.apkg',
            '.colpkg',
          ]),
        ),
        confirmButtonText: 'Import',
      );
    } catch (error) {
      return _hostResultResponse(
        response,
        hostIntent,
        'unsupported',
        error: error,
      );
    }

    if (file == null) {
      return _hostResultResponse(response, hostIntent, 'cancelled');
    }

    final path = _xFilePath(file);
    final Uint8List bytes;
    try {
      bytes = await file.readAsBytes();
    } catch (error) {
      return _hostResultResponse(
        response,
        hostIntent,
        'read-error',
        path: path,
        error: error,
      );
    }

    if (bytes.isEmpty) {
      return _hostResultResponse(
        response,
        hostIntent,
        'import-error',
        path: path,
        error: 'Anki package was empty',
      );
    }

    final json = _mergeAnkiApkg(bytes);
    if (!_jsonOk(json)) {
      return _hostResultResponse(
        response,
        hostIntent,
        'import-error',
        path: path,
        error: _jsonError(json),
      );
    }

    _persistSnapshot();
    final refreshed = props() ?? const <String, Object?>{};
    final hostResult = <String, Object?>{
      'status': 'imported',
      'path': path,
    };
    return _withHostStatusProps(<String, Object?>{
      ...refreshed,
      'hostIntent': hostIntent,
      'hostResult': hostResult,
    }, hostResult);
  }

  Future<Map<String, Object?>?> _exportAnkiPackage(
    Map<String, Object?> response,
    Map<String, Object?> hostIntent,
  ) async {
    final FileSaveLocation? location;
    try {
      location = await getSaveLocation(
        acceptedTypeGroups: _ankiTypeGroups(
          hostIntentExtensions(hostIntent, 'extensions', const <String>[
            '.apkg',
          ]),
        ),
        suggestedName: suggestedAnkiFileName(hostIntent),
        confirmButtonText: 'Export',
      );
    } catch (error) {
      return _hostResultResponse(
        response,
        hostIntent,
        'unsupported',
        error: error,
      );
    }

    if (location == null) {
      return _hostResultResponse(response, hostIntent, 'cancelled');
    }

    final outputPath = _ensureApkgExtension(location.path);
    final json = _takeCString(_api.egExportAnkiApkg(_session));
    if (!_jsonOk(json)) {
      return _hostResultResponse(
        response,
        hostIntent,
        'export-error',
        path: outputPath,
        error: _jsonError(json),
      );
    }

    final bytes = _jsonByteArray(json, 'apkg');
    if (bytes.isEmpty) {
      return _hostResultResponse(
        response,
        hostIntent,
        'export-error',
        path: outputPath,
        error: 'Engram native host returned an empty APKG',
      );
    }

    try {
      await XFile.fromData(
        bytes,
        mimeType: 'application/octet-stream',
        name: _baseName(outputPath),
      ).saveTo(outputPath);
    } catch (error) {
      return _hostResultResponse(
        response,
        hostIntent,
        'write-error',
        path: outputPath,
        error: error,
      );
    }

    return _hostResultResponse(
      response,
      hostIntent,
      'exported',
      path: outputPath,
    );
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

  String _mergeAnkiApkg(Uint8List bytes) {
    final data = calloc<Uint8>(bytes.length);
    try {
      data.asTypedList(bytes.length).setAll(0, bytes);
      return _takeCString(_api.egMergeAnkiApkg(_session, data, bytes.length));
    } finally {
      calloc.free(data);
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
            _EgHandleEngramAppEventDart>('eg_handle_engram_app_event'),
        egExportAnkiApkg = library
            .lookupFunction<_EgExportAnkiApkgNative, _EgExportAnkiApkgDart>(
          'eg_export_anki_apkg',
        ),
        egMergeAnkiApkg = library
            .lookupFunction<_EgMergeAnkiApkgNative, _EgMergeAnkiApkgDart>(
          'eg_merge_anki_apkg',
        );

  final _EgSessionNewDart egSessionNewDemo;
  final _EgSessionFreeDart egSessionFree;
  final _EgStringFreeDart egStringFree;
  final _EgSnapshotDart egSnapshot;
  final _EgLoadSnapshotDart egLoadSnapshot;
  final _EgEngramAppPropsDart egEngramAppProps;
  final _EgHandleEngramAppEventDart egHandleEngramAppEvent;
  final _EgExportAnkiApkgDart egExportAnkiApkg;
  final _EgMergeAnkiApkgDart egMergeAnkiApkg;

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

Map<String, Object?> _hostResultResponse(
  Map<String, Object?> response,
  Map<String, Object?> hostIntent,
  String status, {
  String? path,
  Object? error,
}) {
  final out = <String, Object?>{
    ...response,
    'hostIntent': hostIntent,
  };
  final hostResult = <String, Object?>{'status': status};
  if (path != null && path.isNotEmpty) {
    hostResult['path'] = path;
  }
  if (error != null) {
    hostResult['error'] = error.toString();
  }
  out['hostResult'] = hostResult;
  return _withHostStatusProps(out, hostResult);
}

Map<String, Object?> _withHostStatusProps(
  Map<String, Object?> response,
  Map<String, Object?> hostResult,
) {
  final statusProps = _hostStatusProps(hostResult);
  if (statusProps.isEmpty) return response;
  final props = <String, Object?>{
    ..._mosaicMap(response['props']),
    ...statusProps,
  };
  return <String, Object?>{
    ...response,
    'props': props,
  };
}

Map<String, Object?> _hostStatusProps(Map<String, Object?> hostResult) {
  final status = hostResult['status']?.toString() ?? '';
  if (status.isEmpty) return const <String, Object?>{};
  return <String, Object?>{
    'host-status-visible': true,
    'host-status-kind': status,
    'host-status-label': _hostStatusLabel(status),
    'host-status-message': _hostStatusMessage(hostResult, status),
  };
}

String _hostStatusLabel(String status) {
  switch (status) {
    case 'imported':
      return 'Import complete';
    case 'exported':
      return 'Export complete';
    case 'cancelled':
      return 'Import cancelled';
    case 'read-error':
    case 'import-error':
      return 'Import failed';
    case 'export-error':
    case 'write-error':
      return 'Export failed';
    case 'unsupported':
      return 'Host unavailable';
    default:
      return 'Host status';
  }
}

String _hostStatusMessage(Map<String, Object?> hostResult, String status) {
  final file = _hostResultFile(hostResult);
  final error = hostResult['error']?.toString() ?? '';
  switch (status) {
    case 'imported':
      return file.isEmpty ? 'Anki package imported.' : 'Imported $file.';
    case 'exported':
      return file.isEmpty ? 'Anki package exported.' : 'Saved $file.';
    case 'cancelled':
      return 'No Anki package was selected.';
    case 'read-error':
      return error.isNotEmpty
          ? 'Could not read ${file.isEmpty ? 'the selected file' : file}: $error'
          : 'Could not read ${file.isEmpty ? 'the selected file' : file}.';
    case 'import-error':
      return error.isNotEmpty
          ? 'Could not import ${file.isEmpty ? 'the selected package' : file}: $error'
          : 'Could not import ${file.isEmpty ? 'the selected package' : file}.';
    case 'export-error':
      return error.isNotEmpty
          ? 'Could not export Anki package: $error'
          : 'Could not export Anki package.';
    case 'write-error':
      return error.isNotEmpty
          ? 'Could not save ${file.isEmpty ? 'the Anki package' : file}: $error'
          : 'Could not save ${file.isEmpty ? 'the Anki package' : file}.';
    case 'unsupported':
      return 'This host does not support native Anki file dialogs yet.';
    default:
      return error.isNotEmpty ? error : (file.isEmpty ? status : file);
  }
}

String _hostResultFile(Map<String, Object?> hostResult) {
  final path = hostResult['path']?.toString() ?? '';
  return path.isEmpty ? '' : _baseName(path);
}

List<String> hostIntentExtensions(
  Map<String, Object?> hostIntent,
  String property,
  List<String> fallback,
) {
  final raw = hostIntent[property];
  if (raw is! List) {
    return fallback;
  }

  final extensions = raw
      .map((value) => value.toString().trim())
      .where((value) => value.isNotEmpty)
      .map((value) => value.startsWith('.') ? value : '.$value')
      .toList(growable: false);
  return extensions.isEmpty ? fallback : extensions;
}

List<XTypeGroup> _ankiTypeGroups(List<String> extensions) {
  return <XTypeGroup>[
    XTypeGroup(
      label: 'Anki packages',
      extensions: extensions
          .map((extension) => extension.startsWith('.') ? extension.substring(1) : extension)
          .toList(growable: false),
    ),
  ];
}

String suggestedAnkiFileName(Map<String, Object?> hostIntent) {
  final raw = hostIntent['deckId']?.toString().trim();
  final name = raw == null || raw.isEmpty ? 'engram-collection' : raw;
  final safe = name.replaceAll(RegExp(r'''[\/\\:*?"<>|]'''), '-');
  return safe.toLowerCase().endsWith('.apkg') ? safe : '$safe.apkg';
}

String _xFilePath(XFile file) => file.path.isEmpty ? file.name : file.path;

String _ensureApkgExtension(String path) =>
    path.toLowerCase().endsWith('.apkg') ? path : '$path.apkg';

String _baseName(String path) {
  final normalized = path.replaceAll('\\', '/');
  final index = normalized.lastIndexOf('/');
  return index < 0 ? normalized : normalized.substring(index + 1);
}

Uint8List _jsonByteArray(String json, String property) {
  final Object? decoded;
  try {
    decoded = jsonDecode(json);
  } catch (_) {
    return Uint8List(0);
  }
  if (decoded is! Map || decoded[property] is! List) {
    return Uint8List(0);
  }
  final values = decoded[property] as List;
  return Uint8List.fromList(
    values
        .whereType<num>()
        .map((value) => value.toInt() & 0xff)
        .toList(growable: false),
  );
}

String _jsonError(String json) {
  try {
    final decoded = jsonDecode(json);
    if (decoded is Map && decoded['error'] != null) {
      return decoded['error'].toString();
    }
  } catch (_) {}
  return 'Engram native host failed';
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
