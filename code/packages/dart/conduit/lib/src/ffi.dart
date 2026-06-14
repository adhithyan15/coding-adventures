// ffi.dart — DynamicLibrary bindings for conduit-capi + conduit-dart-bridge (WEB17)
//
// LIBRARY LOADING
// ───────────────
// Two dylibs are loaded:
//
//   conduitLib  (CONDUIT_CAPI_PATH)
//       All HTTP server API: conduit_app_*, conduit_server_*, conduit_request_*,
//       conduit_response_*, conduit_string_free, conduit_last_error, etc.
//
//   bridgeLib  (CONDUIT_DART_BRIDGE_PATH, or derived from CONDUIT_CAPI_PATH)
//       Dart-specific cross-thread bridge: conduit_dart_init, conduit_dart_set_port,
//       conduit_dart_handler_fn, conduit_dart_before_fn, conduit_dart_after_fn,
//       conduit_dart_ctx_free_fn, conduit_dart_complete.
//
// WHY TWO LIBRARIES
// ─────────────────
// conduit-capi dispatches request handlers from a Rust background OS thread.
// Dart's NativeCallable.isolateLocal is only safe when called from the creating
// isolate's thread — calling from an independent OS thread crashes with
// "Cannot invoke native callback outside an isolate."
//
// The conduit-dart-bridge crate uses Dart_PostCObject_DL (Dart's thread-safe
// C API) to post request messages to Dart's event loop and block the Rust
// thread on a Condvar until Dart responds. This is the correct Dart-idiomatic
// approach for receiving callbacks from native threads with non-void returns.
//
// BRIDGE CALLBACK SCHEME
// ──────────────────────
// 1. Dart calls initBridge() once: registers a RawReceivePort and initialises
//    the Dart DL API in the bridge library.
// 2. Each route/filter/hook is registered with conduit-capi using:
//    - handler: bridgeHandlerFn (a C ptr from conduit_dart_handler_fn())
//    - ctx:     an integer key (pointer-cast int) from our Dart closure registry
//    - ctx_free: bridgeCtxFreeFn (from conduit_dart_ctx_free_fn())
// 3. When a request arrives, the bridge posts [slot_id, ctx, type, req_ptr, …]
//    to the Dart port and blocks on a Condvar.
// 4. handleBridgeMessage() fires on Dart's event loop: looks up the closure
//    by ctx, calls it, then calls conduitDartComplete(slot_id, resp_ptr)
//    which signals the Condvar, unblocking the Rust thread.

import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';

import 'trampolines.dart' as tramp;

// ── Library loading ──────────────────────────────────────────────────────────

/// Singleton conduit-capi library handle.
final DynamicLibrary conduitLib = _loadConduitCapi();

/// Singleton bridge library handle.
final DynamicLibrary bridgeLib = _loadBridge();

DynamicLibrary _loadConduitCapi() {
  final envPath = Platform.environment['CONDUIT_CAPI_PATH'];
  if (envPath != null && envPath.isNotEmpty) {
    if (!_isAbsolute(envPath)) {
      throw ArgumentError(
        'CONDUIT_CAPI_PATH must be an absolute path, got: $envPath',
      );
    }
    return DynamicLibrary.open(envPath);
  }
  if (Platform.isMacOS) return DynamicLibrary.open('libconduit_capi.dylib');
  if (Platform.isWindows) return DynamicLibrary.open('conduit_capi.dll');
  return DynamicLibrary.open('libconduit_capi.so');
}

DynamicLibrary _loadBridge() {
  // Prefer an explicit path; otherwise derive from CONDUIT_CAPI_PATH.
  final explicit = Platform.environment['CONDUIT_DART_BRIDGE_PATH'];
  if (explicit != null && explicit.isNotEmpty) {
    if (!_isAbsolute(explicit)) {
      throw ArgumentError(
        'CONDUIT_DART_BRIDGE_PATH must be an absolute path, got: $explicit',
      );
    }
    return DynamicLibrary.open(explicit);
  }
  // Derive from CONDUIT_CAPI_PATH if set.
  final capiPath = Platform.environment['CONDUIT_CAPI_PATH'];
  if (capiPath != null && capiPath.isNotEmpty) {
    final dir = _dirname(capiPath);
    if (Platform.isMacOS) {
      return DynamicLibrary.open('${dir}libconduit_dart_bridge.dylib');
    }
    if (Platform.isWindows) {
      return DynamicLibrary.open('${dir}conduit_dart_bridge.dll');
    }
    return DynamicLibrary.open('${dir}libconduit_dart_bridge.so');
  }
  // Fall back to OS name-based search.
  if (Platform.isMacOS) return DynamicLibrary.open('libconduit_dart_bridge.dylib');
  if (Platform.isWindows) return DynamicLibrary.open('conduit_dart_bridge.dll');
  return DynamicLibrary.open('libconduit_dart_bridge.so');
}

bool _isAbsolute(String p) {
  if (p.isEmpty) return false;
  if (p.startsWith('/')) return true;
  if (Platform.isWindows && p.length >= 3 && RegExp(r'^[A-Za-z]:[/\\]').hasMatch(p)) return true;
  return false;
}

String _dirname(String path) {
  final i = path.lastIndexOf('/');
  return i >= 0 ? path.substring(0, i + 1) : '';
}

// ── Bridge initialisation ─────────────────────────────────────────────────────
//
// Called once at startup (from a lazy initialiser triggered by the first
// Application() constructor call). Sets up the DL API and the receive port.

RawReceivePort? _bridgePort;

void ensureBridgeInitialised() {
  if (_bridgePort != null) return;

  // 1. Initialise the Dart DL API in the bridge library.
  final initFn = bridgeLib.lookupFunction<
      IntPtr Function(Pointer<Void>),
      int Function(Pointer<Void>)>('conduit_dart_init');
  final rc = initFn(NativeApi.initializeApiDLData);
  if (rc != 0) {
    throw StateError('conduit_dart_init failed with code $rc');
  }

  // 2. Create a RawReceivePort. Its listener fires on Dart's event loop
  //    when the bridge posts a message from a Rust background thread.
  _bridgePort = RawReceivePort(_onBridgeMessage);

  // 3. Tell the bridge which port to post to.
  final setPortFn = bridgeLib.lookupFunction<
      Void Function(Int64),
      void Function(int)>('conduit_dart_set_port');
  setPortFn(_bridgePort!.sendPort.nativePort);
}

// ── Bridge message handler ─────────────────────────────────────────────────────
//
// Fired on Dart's event loop when the bridge posts a request from a Rust
// background thread. The message is a List with 5 elements:
//   [slot_id (int), ctx (int), type (int), req_ptr (int), current_resp_ptr (int)]
//
// type 0 = route handler
// type 1 = before-filter
// type 2 = after-hook
// type 255 = ctx_free (fire-and-forget, no response needed)

void _onBridgeMessage(dynamic message) {
  if (message is! List || message.length < 5) return;
  final slotId = message[0] as int;
  final ctx = message[1] as int;
  final type = message[2] as int;
  final reqAddr = message[3] as int;
  final respAddr = message[4] as int;

  if (type == 255) {
    // ctx_free — just clean up the Dart closure registry.
    tramp.freeCtx(ctx);
    return;
  }

  // Use try/finally so conduitDartComplete is ALWAYS called — even if an
  // unexpected exception escapes before reaching the switch. Without the
  // guarantee, the blocked Rust thread is permanently parked.
  Pointer<Void> responsePtr = nullptr;
  try {
    final reqPtr = Pointer<Void>.fromAddress(reqAddr);
    switch (type) {
      case 0: // handler
        responsePtr = tramp.dispatchHandler(ctx, reqPtr);
      case 1: // before-filter
        responsePtr = tramp.dispatchBefore(ctx, reqPtr);
      case 2: // after-hook
        final currentRespPtr = Pointer<Void>.fromAddress(respAddr);
        responsePtr = tramp.dispatchAfter(ctx, reqPtr, currentRespPtr);
      default:
        responsePtr = nullptr;
    }
  } catch (_) {
    responsePtr = nullptr;
  } finally {
    conduitDartComplete(slotId, responsePtr);
  }
}

// ── Error channels ────────────────────────────────────────────────────────────

typedef _NativeReportError = Void Function(Pointer<Utf8> msg);
typedef _DartReportError = void Function(Pointer<Utf8> msg);

typedef _NativeLastError = Pointer<Utf8> Function();
typedef _DartLastError = Pointer<Utf8> Function();

final void Function(Pointer<Utf8>) conduitReportError =
    conduitLib.lookupFunction<_NativeReportError, _DartReportError>(
  'conduit_capi_report_error',
);

final Pointer<Utf8> Function() conduitLastError =
    conduitLib.lookupFunction<_NativeLastError, _DartLastError>(
  'conduit_last_error',
);

// ── App lifecycle ─────────────────────────────────────────────────────────────

typedef _NativeAppNew = Pointer<Void> Function();
typedef _DartAppNew = Pointer<Void> Function();

typedef _NativeAppFree = Void Function(Pointer<Void> app);
typedef _DartAppFree = void Function(Pointer<Void> app);

typedef _NativeAppSetSetting = Void Function(
  Pointer<Void> app,
  Pointer<Utf8> key,
  Pointer<Utf8> value,
);
typedef _DartAppSetSetting = void Function(
  Pointer<Void> app,
  Pointer<Utf8> key,
  Pointer<Utf8> value,
);

typedef _NativeAppGetSetting = Pointer<Utf8> Function(
  Pointer<Void> app,
  Pointer<Utf8> key,
);
typedef _DartAppGetSetting = Pointer<Utf8> Function(
  Pointer<Void> app,
  Pointer<Utf8> key,
);

typedef _NativeAppAddRoute = Void Function(
  Pointer<Void> app,
  Pointer<Utf8> method,
  Pointer<Utf8> pattern,
  Pointer<NativeFunction<HandlerNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);
typedef _DartAppAddRoute = void Function(
  Pointer<Void> app,
  Pointer<Utf8> method,
  Pointer<Utf8> pattern,
  Pointer<NativeFunction<HandlerNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);

typedef _NativeAppAddBefore = Void Function(
  Pointer<Void> app,
  Pointer<NativeFunction<BeforeNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);
typedef _DartAppAddBefore = void Function(
  Pointer<Void> app,
  Pointer<NativeFunction<BeforeNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);

typedef _NativeAppAddAfter = Void Function(
  Pointer<Void> app,
  Pointer<NativeFunction<AfterNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);
typedef _DartAppAddAfter = void Function(
  Pointer<Void> app,
  Pointer<NativeFunction<AfterNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);

typedef _NativeAppSetSpecial = Void Function(
  Pointer<Void> app,
  Pointer<NativeFunction<HandlerNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);
typedef _DartAppSetSpecial = void Function(
  Pointer<Void> app,
  Pointer<NativeFunction<HandlerNative>> handler,
  Pointer<Void> ctx,
  Pointer<NativeFunction<CtxFreeNative>> ctxFree,
);

final Pointer<Void> Function() conduitAppNew =
    conduitLib.lookupFunction<_NativeAppNew, _DartAppNew>('conduit_app_new');

final void Function(Pointer<Void>) conduitAppFree =
    conduitLib.lookupFunction<_NativeAppFree, _DartAppFree>('conduit_app_free');

final void Function(Pointer<Void>, Pointer<Utf8>, Pointer<Utf8>)
    conduitAppSetSetting =
    conduitLib.lookupFunction<_NativeAppSetSetting, _DartAppSetSetting>(
  'conduit_app_set_setting',
);

final Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>)
    conduitAppGetSetting =
    conduitLib.lookupFunction<_NativeAppGetSetting, _DartAppGetSetting>(
  'conduit_app_get_setting',
);

final void Function(
  Pointer<Void>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<NativeFunction<HandlerNative>>,
  Pointer<Void>,
  Pointer<NativeFunction<CtxFreeNative>>,
) conduitAppAddRoute =
    conduitLib.lookupFunction<_NativeAppAddRoute, _DartAppAddRoute>(
  'conduit_app_add_route',
);

final void Function(
  Pointer<Void>,
  Pointer<NativeFunction<BeforeNative>>,
  Pointer<Void>,
  Pointer<NativeFunction<CtxFreeNative>>,
) conduitAppAddBefore =
    conduitLib.lookupFunction<_NativeAppAddBefore, _DartAppAddBefore>(
  'conduit_app_add_before',
);

final void Function(
  Pointer<Void>,
  Pointer<NativeFunction<AfterNative>>,
  Pointer<Void>,
  Pointer<NativeFunction<CtxFreeNative>>,
) conduitAppAddAfter =
    conduitLib.lookupFunction<_NativeAppAddAfter, _DartAppAddAfter>(
  'conduit_app_add_after',
);

final void Function(
  Pointer<Void>,
  Pointer<NativeFunction<HandlerNative>>,
  Pointer<Void>,
  Pointer<NativeFunction<CtxFreeNative>>,
) conduitAppSetNotFound =
    conduitLib.lookupFunction<_NativeAppSetSpecial, _DartAppSetSpecial>(
  'conduit_app_set_not_found',
);

final void Function(
  Pointer<Void>,
  Pointer<NativeFunction<HandlerNative>>,
  Pointer<Void>,
  Pointer<NativeFunction<CtxFreeNative>>,
) conduitAppSetErrorHandler =
    conduitLib.lookupFunction<_NativeAppSetSpecial, _DartAppSetSpecial>(
  'conduit_app_set_error_handler',
);

// ── Server ────────────────────────────────────────────────────────────────────

typedef _NativeServerBind = Pointer<Void> Function(
  Pointer<Utf8> host,
  Uint16 port,
  Pointer<Void> app,
);
typedef _DartServerBind = Pointer<Void> Function(
  Pointer<Utf8> host,
  int port,
  Pointer<Void> app,
);

typedef _NativeServerServe = Int32 Function(Pointer<Void> srv);
typedef _DartServerServe = int Function(Pointer<Void> srv);

typedef _NativeServerLocalPort = Uint16 Function(Pointer<Void> srv);
typedef _DartServerLocalPort = int Function(Pointer<Void> srv);

typedef _NativeServerRunning = Int32 Function(Pointer<Void> srv);
typedef _DartServerRunning = int Function(Pointer<Void> srv);

typedef _NativeServerVoid = Void Function(Pointer<Void> srv);
typedef _DartServerVoid = void Function(Pointer<Void> srv);

final Pointer<Void> Function(Pointer<Utf8>, int, Pointer<Void>)
    conduitServerBind =
    conduitLib.lookupFunction<_NativeServerBind, _DartServerBind>(
  'conduit_server_bind',
);

final int Function(Pointer<Void>) conduitServerServe =
    conduitLib.lookupFunction<_NativeServerServe, _DartServerServe>(
  'conduit_server_serve',
);

final int Function(Pointer<Void>) conduitServerServeBackground =
    conduitLib.lookupFunction<_NativeServerServe, _DartServerServe>(
  'conduit_server_serve_background',
);

final void Function(Pointer<Void>) conduitServerStop =
    conduitLib.lookupFunction<_NativeServerVoid, _DartServerVoid>(
  'conduit_server_stop',
);

final int Function(Pointer<Void>) conduitServerLocalPort =
    conduitLib.lookupFunction<_NativeServerLocalPort, _DartServerLocalPort>(
  'conduit_server_local_port',
);

final int Function(Pointer<Void>) conduitServerRunning =
    conduitLib.lookupFunction<_NativeServerRunning, _DartServerRunning>(
  'conduit_server_running',
);

final void Function(Pointer<Void>) conduitServerFree =
    conduitLib.lookupFunction<_NativeServerVoid, _DartServerVoid>(
  'conduit_server_free',
);

// ── Request accessors ─────────────────────────────────────────────────────────

typedef _NativeReqStr = Pointer<Utf8> Function(Pointer<Void> req);
typedef _DartReqStr = Pointer<Utf8> Function(Pointer<Void> req);

typedef _NativeReqParam = Pointer<Utf8> Function(
  Pointer<Void> req,
  Pointer<Utf8> name,
);
typedef _DartReqParam = Pointer<Utf8> Function(
  Pointer<Void> req,
  Pointer<Utf8> name,
);

typedef _NativeReqBody = Pointer<Uint8> Function(
  Pointer<Void> req,
  Pointer<IntPtr> outLen,
);
typedef _DartReqBody = Pointer<Uint8> Function(
  Pointer<Void> req,
  Pointer<IntPtr> outLen,
);

final Pointer<Utf8> Function(Pointer<Void>) conduitRequestMethod =
    conduitLib.lookupFunction<_NativeReqStr, _DartReqStr>(
  'conduit_request_method',
);
final Pointer<Utf8> Function(Pointer<Void>) conduitRequestPath =
    conduitLib.lookupFunction<_NativeReqStr, _DartReqStr>(
  'conduit_request_path',
);
final Pointer<Utf8> Function(Pointer<Void>) conduitRequestQueryString =
    conduitLib.lookupFunction<_NativeReqStr, _DartReqStr>(
  'conduit_request_query_string',
);
final Pointer<Utf8> Function(Pointer<Void>) conduitRequestContentType =
    conduitLib.lookupFunction<_NativeReqStr, _DartReqStr>(
  'conduit_request_content_type',
);
final Pointer<Utf8> Function(Pointer<Void>) conduitRequestRemoteAddr =
    conduitLib.lookupFunction<_NativeReqStr, _DartReqStr>(
  'conduit_request_remote_addr',
);
final Pointer<Utf8> Function(Pointer<Void>) conduitRequestError =
    conduitLib.lookupFunction<_NativeReqStr, _DartReqStr>(
  'conduit_request_error',
);

final Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>)
    conduitRequestParam =
    conduitLib.lookupFunction<_NativeReqParam, _DartReqParam>(
  'conduit_request_param',
);
final Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>) conduitRequestQuery =
    conduitLib.lookupFunction<_NativeReqParam, _DartReqParam>(
  'conduit_request_query',
);
final Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>)
    conduitRequestHeader =
    conduitLib.lookupFunction<_NativeReqParam, _DartReqParam>(
  'conduit_request_header',
);

final Pointer<Uint8> Function(Pointer<Void>, Pointer<IntPtr>)
    conduitRequestBody =
    conduitLib.lookupFunction<_NativeReqBody, _DartReqBody>(
  'conduit_request_body',
);

// ── Response builder / reader ─────────────────────────────────────────────────

typedef _NativeRespNew = Pointer<Void> Function(
  Uint16 status,
  Pointer<Uint8> body,
  IntPtr bodyLen,
);
typedef _DartRespNew = Pointer<Void> Function(
  int status,
  Pointer<Uint8> body,
  int bodyLen,
);

typedef _NativeRespSetHeader = Void Function(
  Pointer<Void> resp,
  Pointer<Utf8> name,
  Pointer<Utf8> value,
);
typedef _DartRespSetHeader = void Function(
  Pointer<Void> resp,
  Pointer<Utf8> name,
  Pointer<Utf8> value,
);

typedef _NativeRespStatus = Uint16 Function(Pointer<Void> resp);
typedef _DartRespStatus = int Function(Pointer<Void> resp);

typedef _NativeRespBody = Pointer<Uint8> Function(
  Pointer<Void> resp,
  Pointer<IntPtr> outLen,
);
typedef _DartRespBody = Pointer<Uint8> Function(
  Pointer<Void> resp,
  Pointer<IntPtr> outLen,
);

typedef _NativeRespCount = IntPtr Function(Pointer<Void> resp);
typedef _DartRespCount = int Function(Pointer<Void> resp);

typedef _NativeRespHeaderAt = Pointer<Utf8> Function(
  Pointer<Void> resp,
  IntPtr i,
);
typedef _DartRespHeaderAt = Pointer<Utf8> Function(Pointer<Void> resp, int i);

typedef _NativeRespFree = Void Function(Pointer<Void> resp);
typedef _DartRespFree = void Function(Pointer<Void> resp);

typedef _NativeStrFree = Void Function(Pointer<Utf8> s);
typedef _DartStrFree = void Function(Pointer<Utf8> s);

final Pointer<Void> Function(int, Pointer<Uint8>, int) conduitResponseNew =
    conduitLib.lookupFunction<_NativeRespNew, _DartRespNew>(
  'conduit_response_new',
);

final void Function(Pointer<Void>, Pointer<Utf8>, Pointer<Utf8>)
    conduitResponseSetHeader =
    conduitLib.lookupFunction<_NativeRespSetHeader, _DartRespSetHeader>(
  'conduit_response_set_header',
);

final int Function(Pointer<Void>) conduitResponseStatus =
    conduitLib.lookupFunction<_NativeRespStatus, _DartRespStatus>(
  'conduit_response_status',
);

final Pointer<Uint8> Function(Pointer<Void>, Pointer<IntPtr>)
    conduitResponseBody =
    conduitLib.lookupFunction<_NativeRespBody, _DartRespBody>(
  'conduit_response_body',
);

final int Function(Pointer<Void>) conduitResponseHeaderCount =
    conduitLib.lookupFunction<_NativeRespCount, _DartRespCount>(
  'conduit_response_header_count',
);

final Pointer<Utf8> Function(Pointer<Void>, int) conduitResponseHeaderName =
    conduitLib.lookupFunction<_NativeRespHeaderAt, _DartRespHeaderAt>(
  'conduit_response_header_name',
);

final Pointer<Utf8> Function(Pointer<Void>, int) conduitResponseHeaderValue =
    conduitLib.lookupFunction<_NativeRespHeaderAt, _DartRespHeaderAt>(
  'conduit_response_header_value',
);

final void Function(Pointer<Void>) conduitResponseFree =
    conduitLib.lookupFunction<_NativeRespFree, _DartRespFree>(
  'conduit_response_free',
);

final void Function(Pointer<Utf8>) conduitStringFree =
    conduitLib.lookupFunction<_NativeStrFree, _DartStrFree>(
  'conduit_string_free',
);

// ── Bridge function pointers ──────────────────────────────────────────────────
//
// These C function pointers are passed to conduit-capi for route/filter/hook
// registration instead of NativeCallable trampolines. When called from the
// Rust background thread, they post to Dart's event loop via Dart_PostCObject_DL
// and block until Dart delivers a response.

typedef _BridgeFnPtrGetter = Pointer<Void> Function();

Pointer<NativeFunction<HandlerNative>> get bridgeHandlerFn {
  final getter = bridgeLib.lookupFunction<_BridgeFnPtrGetter, _BridgeFnPtrGetter>(
    'conduit_dart_handler_fn',
  );
  return getter().cast<NativeFunction<HandlerNative>>();
}

Pointer<NativeFunction<BeforeNative>> get bridgeBeforeFn {
  final getter = bridgeLib.lookupFunction<_BridgeFnPtrGetter, _BridgeFnPtrGetter>(
    'conduit_dart_before_fn',
  );
  return getter().cast<NativeFunction<BeforeNative>>();
}

Pointer<NativeFunction<AfterNative>> get bridgeAfterFn {
  final getter = bridgeLib.lookupFunction<_BridgeFnPtrGetter, _BridgeFnPtrGetter>(
    'conduit_dart_after_fn',
  );
  return getter().cast<NativeFunction<AfterNative>>();
}

Pointer<NativeFunction<CtxFreeNative>> get bridgeCtxFreeFn {
  final getter = bridgeLib.lookupFunction<_BridgeFnPtrGetter, _BridgeFnPtrGetter>(
    'conduit_dart_ctx_free_fn',
  );
  return getter().cast<NativeFunction<CtxFreeNative>>();
}

// Complete a bridge request slot from the Dart side (signals the Rust Condvar).
typedef _NativeComplete = Void Function(Uint64 slotId, Pointer<Void> response);
typedef _DartComplete = void Function(int slotId, Pointer<Void> response);

final void Function(int, Pointer<Void>) conduitDartComplete =
    bridgeLib.lookupFunction<_NativeComplete, _DartComplete>(
  'conduit_dart_complete',
);

// ── Callback function type aliases ────────────────────────────────────────────

typedef HandlerNative = Pointer<Void> Function(Pointer<Void>, Pointer<Void>);
typedef BeforeNative = Pointer<Void> Function(Pointer<Void>, Pointer<Void>);
typedef AfterNative = Pointer<Void> Function(
  Pointer<Void>,
  Pointer<Void>,
  Pointer<Void>,
);
typedef CtxFreeNative = Void Function(Pointer<Void>);

// ── Helpers ───────────────────────────────────────────────────────────────────

String cstrToDart(Pointer<Utf8> ptr) {
  if (ptr == nullptr) return '';
  return ptr.toDartString();
}

String? cstrToDartNullable(Pointer<Utf8> ptr) {
  if (ptr == nullptr) return null;
  return ptr.toDartString();
}

String sanitizeForLog(String s, {int maxLen = 512}) {
  // Restrict to printable ASCII (0x20–0x7e) only.
  // Unicode directional overrides and other control categories (≥ 0x80) can
  // forge or corrupt log lines in terminal renderers and log parsers.
  final buf = StringBuffer();
  for (final c in s.runes) {
    if (c >= 0x20 && c <= 0x7e) {
      if (buf.length >= maxLen) break;
      buf.writeCharCode(c);
    }
  }
  return buf.toString();
}
