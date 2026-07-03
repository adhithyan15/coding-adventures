// trampolines.dart — closure registry + bridge dispatch (WEB17)
//
// This module owns the Dart closure registries for handlers, before-filters,
// and after-hooks. Closures are stored keyed by a monotonically increasing
// integer ID. The integer is cast to a Pointer<Void> ctx and passed to
// conduit-capi as the opaque context for each registered callback.
//
// DISPATCH (bridge path)
// ──────────────────────
// Instead of NativeCallable trampolines (which do NOT work when called from
// a non-Dart OS thread), we use the conduit-dart-bridge crate's
// Dart_PostCObject_DL mechanism. When a request arrives:
//
//   1. The bridge posts a message [slot_id, ctx, type, req_ptr, …] to our
//      RawReceivePort (registered in ffi.dart → ensureBridgeInitialised()).
//   2. ffi._onBridgeMessage() extracts the message and calls dispatchHandler /
//      dispatchBefore / dispatchAfter here.
//   3. The dispatch functions look up the closure by ctx (the integer key),
//      call it, and return the native response pointer.
//   4. ffi._onBridgeMessage() calls conduitDartComplete(slot_id, responsePtr)
//      to wake the blocked Rust thread.
//
// REGISTRY DESIGN
// ───────────────
// Three separate maps are used (handler, before, after) sharing one global
// integer key space so that ctx_free can remove from any map by key alone.
// IDs start at 1 — ctx=0 (null pointer) means "no callback".

import 'dart:ffi';
import 'package:ffi/ffi.dart';
import 'ffi.dart' as ffi;
import 'request.dart';
import 'response.dart';

// ── Registry ─────────────────────────────────────────────────────────────────

int _nextId = 1;

final _handlerRegistry = <int, Response Function(Request)>{};
final _beforeRegistry = <int, Response? Function(Request)>{};
final _afterRegistry = <int, Response Function(Request, Response)>{};

Pointer<Void> allocHandler(Response Function(Request) fn) {
  final id = _nextId++;
  _handlerRegistry[id] = fn;
  return Pointer<Void>.fromAddress(id);
}

Pointer<Void> allocBefore(Response? Function(Request) fn) {
  final id = _nextId++;
  _beforeRegistry[id] = fn;
  return Pointer<Void>.fromAddress(id);
}

Pointer<Void> allocAfter(Response Function(Request, Response) fn) {
  final id = _nextId++;
  _afterRegistry[id] = fn;
  return Pointer<Void>.fromAddress(id);
}

/// Remove a closure from whichever registry owns it (called by the bridge on
/// ctx_free, so the bridge message handler can call this directly).
void freeCtx(int id) {
  _handlerRegistry.remove(id);
  _beforeRegistry.remove(id);
  _afterRegistry.remove(id);
}

// ── Error reporting ───────────────────────────────────────────────────────────

void _reportError(Object ex) {
  try {
    final raw = ex.toString();
    final safe = ffi.sanitizeForLog(raw);
    final ptr = safe.toNativeUtf8();
    try {
      ffi.conduitReportError(ptr);
    } finally {
      calloc.free(ptr);
    }
  } catch (_) {} // best-effort
}

// ── Bridge dispatch functions ─────────────────────────────────────────────────
//
// Called by ffi._onBridgeMessage() on Dart's event loop. These are the safe,
// event-loop-thread side of the request/response cycle.

/// Dispatch a route handler call. Returns the native ConduitResponse* or null.
Pointer<Void> dispatchHandler(int ctx, Pointer<Void> req) {
  final fn = _handlerRegistry[ctx];
  if (fn == null) {
    return Response.json(
      '{"error":"internal server error"}',
      status: 500,
    ).toNative();
  }
  try {
    return fn(Request.internal(req)).toNative();
  } on HaltException catch (e) {
    return e.response.toNative();
  } catch (ex) {
    _reportError(ex);
    try {
      return Response.json(
        '{"error":"internal server error"}',
        status: 500,
      ).toNative();
    } catch (_) {
      return nullptr;
    }
  }
}

/// Dispatch a before-filter call. Returns null to continue; non-null to halt.
Pointer<Void> dispatchBefore(int ctx, Pointer<Void> req) {
  final fn = _beforeRegistry[ctx];
  if (fn == null) return nullptr;
  try {
    final result = fn(Request.internal(req));
    if (result == null) return nullptr;
    return result.toNative();
  } on HaltException catch (e) {
    return e.response.toNative();
  } catch (ex) {
    _reportError(ex);
    return nullptr;
  }
}

/// Dispatch an after-hook call. Returns the (possibly replaced) response.
Pointer<Void> dispatchAfter(
  int ctx,
  Pointer<Void> req,
  Pointer<Void> current,
) {
  final fn = _afterRegistry[ctx];
  if (fn == null) return current;
  try {
    final currentResp = Response.fromNative(current);
    ffi.conduitResponseFree(current);
    return fn(Request.internal(req), currentResp).toNative();
  } on HaltException catch (e) {
    ffi.conduitResponseFree(current);
    return e.response.toNative();
  } catch (ex) {
    ffi.conduitResponseFree(current);
    _reportError(ex);
    try {
      return Response.json(
        '{"error":"internal server error"}',
        status: 500,
      ).toNative();
    } catch (_) {
      return Response.text('Internal Server Error', status: 500).toNative();
    }
  }
}
