// Conduit.fs — F# P/Invoke binding for the conduit-capi C ABI (WEB16)
//
// CodingAdventures.Conduit.FSharp is a Sinatra/Express-style web framework.
// This module wraps the Rust `conduit-capi` shared library via .NET P/Invoke,
// presenting a functional, pipe-friendly API idiomatic to F#.
//
// ARCHITECTURE
// ────────────
//   Program.fs  →  Application module  (builder, pipeline-compose)
//                  │  registers routes/hooks as F# lambdas
//                  ▼
//               Trampolines module  (internal)
//                  │  Marshal.GetFunctionPointerForDelegate
//                  │  [UnmanagedFunctionPointer(Cdecl)]
//                  ▼
//               Native module  (internal)
//                  │  [<DllImport>] declarations
//                  ▼
//               libconduit_capi.so / .dylib  (Rust cdylib, WEB12)
//
// F# doesn't have C# 9's `delegate*` syntax in an idiomatic form, so we use
// the classic .NET delegate approach:
//
//   1. Declare a delegate type marked [<UnmanagedFunctionPointer(Cdecl)>].
//   2. Create a static module-level delegate instance (never GC'd; module
//      statics live for the process lifetime on first access).
//   3. Obtain the native function pointer via Marshal.GetFunctionPointerForDelegate.
//
// DELEGATE LIFETIME
// ─────────────────
// F# closures are heap-allocated objects. The GC could collect them while Rust
// still holds function-pointer references to them — a use-after-free waiting to
// happen. We prevent this with GCHandle.Alloc(fn) strong roots, exactly as in
// the C# port (WEB15) and Go's cgo.Handle pattern (WEB14).
//
//   F# managed heap                    C native heap
//  ┌──────────────────┐               ┌────────────────────────────┐
//  │  F# lambda (fn)  │◄─ GCHandle ──►│ ctx  (opaque nativeint)    │
//  └──────────────────┘  (strong root) └────────────────────────────┘
//          ▲                                         │
//          └─────────────────────────────────────────┘
//                trampoline recovers via
//           GCHandle.FromIntPtr(ctx).Target :?> _

module CodingAdventures.Conduit.FSharp

open System
open System.IO
open System.Net
open System.Reflection
open System.Runtime.InteropServices
open System.Text

// ── Native P/Invoke layer ────────────────────────────────────────────────────
//
// All [<DllImport>] declarations live here. The module constructor installs a
// NativeLibrary resolver so callers don't need DYLD_LIBRARY_PATH / LD_LIBRARY_PATH.

[<AutoOpen>]
module internal Native =

    // The unmanaged library name without platform prefix/suffix.
    [<Literal>]
    let private Lib = "conduit_capi"

    // NativeLibrary resolver: check CONDUIT_CAPI_PATH first (set by run-tests.sh
    // to the exact path of the built .so/.dylib), then fall back to OS search.
    //
    // No File.Exists pre-check — that would introduce a TOCTOU race.
    // NativeLibrary.Load throws DllNotFoundException with a clear message if the
    // path is absent or not a valid shared library.
    //
    // The path MUST be absolute. Relative paths resolve against the process
    // working directory and create a path-traversal window; we reject them early
    // so operators get a clear error instead of a subtle load from the wrong place.
    let private resolve (name: string) (asm: Assembly) (paths: DllImportSearchPath Nullable) =
        if name <> Lib then nativeint 0
        else
            let env = Environment.GetEnvironmentVariable "CONDUIT_CAPI_PATH"
            if not (String.IsNullOrEmpty env) then
                if not (Path.IsPathRooted env) then
                    raise (InvalidOperationException
                        "CONDUIT_CAPI_PATH must be an absolute path.")
                NativeLibrary.Load env
            else
                NativeLibrary.Load(name, asm, paths)

    do  // module constructor — runs once before any DllImport fires
        NativeLibrary.SetDllImportResolver(Assembly.GetExecutingAssembly(), resolve)

    // ── Error channels ───────────────────────────────────────────────────────

    [<DllImport(Lib)>]
    extern void conduit_capi_report_error([<MarshalAs(UnmanagedType.LPUTF8Str)>] string msg)

    [<DllImport(Lib)>]
    extern nativeint conduit_last_error()

    // ── App lifecycle ────────────────────────────────────────────────────────

    [<DllImport(Lib)>] extern nativeint conduit_app_new()
    [<DllImport(Lib)>] extern void      conduit_app_free(nativeint app)

    [<DllImport(Lib)>]
    extern void conduit_app_set_setting(
        nativeint app,
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string key,
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string value)

    [<DllImport(Lib)>]
    extern nativeint conduit_app_get_setting(
        nativeint app,
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string key)

    [<DllImport(Lib)>]
    extern void conduit_app_add_route(
        nativeint app,
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string method,
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string pattern,
        nativeint handler, nativeint ctx, nativeint ctxFree)

    [<DllImport(Lib)>]
    extern void conduit_app_add_before(nativeint app, nativeint handler, nativeint ctx, nativeint ctxFree)

    [<DllImport(Lib)>]
    extern void conduit_app_add_after(nativeint app, nativeint handler, nativeint ctx, nativeint ctxFree)

    [<DllImport(Lib)>]
    extern void conduit_app_set_not_found(nativeint app, nativeint handler, nativeint ctx, nativeint ctxFree)

    [<DllImport(Lib)>]
    extern void conduit_app_set_error_handler(nativeint app, nativeint handler, nativeint ctx, nativeint ctxFree)

    // ── Server ───────────────────────────────────────────────────────────────

    // conduit_server_bind consumes `app` on both success and failure.
    [<DllImport(Lib)>]
    extern nativeint conduit_server_bind(
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string host,
        uint16 port,
        nativeint app)

    [<DllImport(Lib)>] extern int    conduit_server_serve(nativeint srv)
    [<DllImport(Lib)>] extern int    conduit_server_serve_background(nativeint srv)
    [<DllImport(Lib)>] extern void   conduit_server_stop(nativeint srv)
    [<DllImport(Lib)>] extern uint16 conduit_server_local_port(nativeint srv)
    [<DllImport(Lib)>] extern int    conduit_server_running(nativeint srv)
    [<DllImport(Lib)>] extern void   conduit_server_free(nativeint srv)

    // ── Request accessors ────────────────────────────────────────────────────
    //
    // All return borrowed const char* valid only during a handler call.
    // Marshal.PtrToStringUTF8 copies the bytes into a managed string immediately.

    [<DllImport(Lib)>] extern nativeint conduit_request_method(nativeint req)
    [<DllImport(Lib)>] extern nativeint conduit_request_path(nativeint req)
    [<DllImport(Lib)>] extern nativeint conduit_request_query_string(nativeint req)
    [<DllImport(Lib)>] extern nativeint conduit_request_content_type(nativeint req)
    [<DllImport(Lib)>] extern nativeint conduit_request_remote_addr(nativeint req)
    [<DllImport(Lib)>] extern nativeint conduit_request_error(nativeint req)

    // Body is NOT null-terminated; returns pointer + length.
    [<DllImport(Lib)>]
    extern nativeint conduit_request_body(nativeint req, [<Out>] unativeint& outLen)

    [<DllImport(Lib)>]
    extern nativeint conduit_request_param(
        nativeint req, [<MarshalAs(UnmanagedType.LPUTF8Str)>] string name)

    [<DllImport(Lib)>]
    extern nativeint conduit_request_query(
        nativeint req, [<MarshalAs(UnmanagedType.LPUTF8Str)>] string name)

    [<DllImport(Lib)>]
    extern nativeint conduit_request_header(
        nativeint req, [<MarshalAs(UnmanagedType.LPUTF8Str)>] string name)

    // ── Response builder / reader ────────────────────────────────────────────

    [<DllImport(Lib)>]
    extern nativeint conduit_response_new(uint16 status, nativeint body, unativeint bodyLen)

    [<DllImport(Lib)>]
    extern void conduit_response_set_header(
        nativeint resp,
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string name,
        [<MarshalAs(UnmanagedType.LPUTF8Str)>] string value)

    [<DllImport(Lib)>] extern uint16  conduit_response_status(nativeint resp)
    [<DllImport(Lib)>] extern nativeint conduit_response_body(nativeint resp, [<Out>] unativeint& outLen)
    [<DllImport(Lib)>] extern unativeint   conduit_response_header_count(nativeint resp)
    [<DllImport(Lib)>] extern nativeint conduit_response_header_name(nativeint resp, unativeint i)
    [<DllImport(Lib)>] extern nativeint conduit_response_header_value(nativeint resp, unativeint i)
    [<DllImport(Lib)>] extern void    conduit_response_free(nativeint resp)
    [<DllImport(Lib)>] extern void    conduit_string_free(nativeint s)

    // ── Helpers ──────────────────────────────────────────────────────────────

    let cstr (p: nativeint) : string option =
        if p = 0n then None
        else Some (Marshal.PtrToStringUTF8 p)

    let cstrNotNull (p: nativeint) : string =
        cstr p |> Option.defaultValue ""

    // Strip ASCII control characters and cap at 512 characters — used when
    // writing native-sourced strings to stderr to prevent log-injection attacks.
    let sanitizeForLog (s: string) : string =
        let sb = StringBuilder(min s.Length 512)
        for c in s do
            if c >= '\x20' && c <> '\x7f' then
                if sb.Length < 512 then sb.Append c |> ignore
        sb.ToString()


// ── Response ─────────────────────────────────────────────────────────────────
//
// An immutable bundle of (status, headers, body). Build with the Response module
// factory functions, or modify with `Response.withHeader`.
//
// Status codes live in [100, 999]: the HTTP-registered range is [100, 599]; we
// allow up to 999 for experimental / proprietary codes.

/// An HTTP response: status code, body text, and optional headers.
type Response = {
    Status  : int
    Body    : string
    Headers : (string * string) list
}

/// Thrown inside a handler to immediately short-circuit with a specific response.
///
/// Equivalent to Sinatra's `halt` or an early `return` in Express.
///   `raise (HaltException (Response.text "Unauthorized" |> Response.withStatus 401))`
exception HaltException of Response

/// Factory functions and helpers for building HTTP responses.
module Response =

    let private make status body headers =
        if status < 100 || status > 999 then
            invalidArg "status"
                $"HTTP status code {status} is outside the valid range [100, 999]."
        { Status = status; Body = body; Headers = headers }

    // ── Factory methods ───────────────────────────────────────────────────────
    //
    // F# module-level `let` bindings cannot have optional parameters (FS0718).
    // Use `|> Response.withStatus 201` to override the default status:
    //
    //   Response.html "<p>Created</p>" |> Response.withStatus 201
    //   Response.json "{\"error\":\"gone\"}" |> Response.withStatus 410

    /// HTML response with status 200 (content-type: text/html; charset=utf-8).
    let html (body: string) =
        make 200 body ["content-type", "text/html; charset=utf-8"]

    /// JSON response with status 200 (content-type: application/json).
    let json (body: string) =
        make 200 body ["content-type", "application/json"]

    /// Plain-text response with status 200 (content-type: text/plain; charset=utf-8).
    let text (body: string) =
        make 200 body ["content-type", "text/plain; charset=utf-8"]

    /// Arbitrary status + body + explicit header list.
    let respond (status: int) (body: string) (headers: (string * string) list) =
        make status body headers

    /// HTTP 302 redirect. Raises ArgumentException if location contains CR or LF —
    /// belt-and-suspenders on top of conduit-capi's header-injection defence.
    /// Use `|> Response.withStatus 301` to change the redirect code.
    let redirect (location: string) =
        if location.Contains '\r' || location.Contains '\n' then
            raise (ArgumentException(
                "Redirect location must not contain CR or LF.", "location"))
        make 302 "" ["location", location]

    /// Return a new Response with an additional header appended.
    let withHeader (name: string) (value: string) (resp: Response) =
        { resp with Headers = resp.Headers @ [(name, value)] }

    /// Return a new Response with the status code replaced.
    /// Raises ArgumentException if status is outside [100, 999].
    let withStatus (status: int) (resp: Response) =
        if status < 100 || status > 999 then
            invalidArg "status"
                $"HTTP status code {status} is outside the valid range [100, 999]."
        { resp with Status = status }

    // ── Conversion to/from native ─────────────────────────────────────────────

    // Creates a ConduitResponse* — ownership transfers to Rust.
    let internal toNative (resp: Response) : nativeint =
        // Guard before narrowing cast: (uint16)70000 silently wraps.
        if resp.Status < 100 || resp.Status > 999 then
            invalidOp $"HTTP status code {resp.Status} is outside the valid range [100, 999]."

        let body = Encoding.UTF8.GetBytes resp.Body

        // GCHandle.Pinned keeps the byte array address stable while
        // conduit_response_new copies into the native buffer. We use a
        // 1-byte placeholder when body is empty to get a valid (non-null) ptr.
        let arr  = if body.Length > 0 then body else [| 0uy |]
        let pin  = GCHandle.Alloc(arr, GCHandleType.Pinned)
        try
            let ptr = pin.AddrOfPinnedObject()
            let r   = conduit_response_new(uint16 resp.Status, ptr, unativeint body.Length)
            for (n, v) in resp.Headers do
                conduit_response_set_header(r, n, v)
            r
        finally
            pin.Free()

    // Reads a native ConduitResponse* into a managed Response, copying all data.
    // Caller is responsible for freeing the native pointer afterwards.
    let internal fromNative (ptr: nativeint) : Response =
        let status = int (conduit_response_status ptr)

        let bodyStr =
            let mutable len = unativeint 0u
            let bodyPtr = conduit_response_body(ptr, &len)
            if bodyPtr = 0n || len = unativeint 0u then ""
            else
                // Guard against a rogue native layer returning an overflowed length.
                if len > unativeint Array.MaxLength then
                    invalidOp $"Native response body length {len} exceeds Array.MaxLength."
                let bytes = Array.zeroCreate<byte> (int len)
                Marshal.Copy(bodyPtr, bytes, 0, int len)
                Encoding.UTF8.GetString bytes

        let count = int (conduit_response_header_count ptr)
        let headers =
            [ for i in 0 .. count - 1 do
                let n = cstrNotNull (conduit_response_header_name(ptr, unativeint i))
                let v = cstrNotNull (conduit_response_header_value(ptr, unativeint i))
                yield (n, v) ]

        { Status = status; Body = bodyStr; Headers = headers }


// ── Request ───────────────────────────────────────────────────────────────────
//
// A read-only view of an HTTP request, valid only for the duration of one handler
// call. DANGER: Do not store a Request value and use it after the handler returns
// — the native ConduitRequest* is freed immediately. Copy any data you need.

/// A read-only view of an HTTP request (valid only inside the handler call).
type Request internal (ptr: nativeint) =

    /// HTTP method in uppercase: "GET", "POST", "PUT", "DELETE", etc.
    member _.Method      = cstrNotNull (conduit_request_method ptr)

    /// URL path without query string: "/api/users/42".
    member _.Path        = cstrNotNull (conduit_request_path ptr)

    /// Raw query string without the leading '?': "q=hello&page=2".
    member _.QueryString = cstrNotNull (conduit_request_query_string ptr)

    /// Content-Type header value, or "" if absent.
    member _.ContentType = cstrNotNull (conduit_request_content_type ptr)

    /// Remote address as "IP:port": "127.0.0.1:54321".
    member _.RemoteAddr  = cstrNotNull (conduit_request_remote_addr ptr)

    /// Non-empty only inside an OnError handler — the Rust error message.
    member _.Error       = cstrNotNull (conduit_request_error ptr)

    /// Named route parameter from the URL pattern, or None if absent.
    member _.Param (name: string) = cstr (conduit_request_param(ptr, name))

    /// Query string value for the given key, or None if absent.
    member _.Query (name: string) = cstr (conduit_request_query(ptr, name))

    /// Request header value (case-insensitive lookup), or None if absent.
    member _.Header (name: string) = cstr (conduit_request_header(ptr, name))

    /// Raw request body bytes. Empty array if no body.
    member _.Body() : byte[] =
        let mutable len = unativeint 0u
        let bodyPtr = conduit_request_body(ptr, &len)
        if bodyPtr = 0n || len = unativeint 0u then [||]
        else
            // Guard against a rogue native layer returning an overflowed length.
            if len > unativeint Array.MaxLength then
                invalidOp $"Native request body length {len} exceeds Array.MaxLength."
            let bytes = Array.zeroCreate<byte> (int len)
            Marshal.Copy(bodyPtr, bytes, 0, int len)
            bytes

    /// Request body decoded as UTF-8 text.
    member this.BodyString() = Encoding.UTF8.GetString(this.Body())


// ── Trampolines ───────────────────────────────────────────────────────────────
//
// These are static module-level delegate instances (process-lifetime) that bridge
// Rust's C function-pointer calls into F# managed closures via GCHandle-boxed ctx.
//
// Pattern for each trampoline:
//   1. Declare the delegate type with [UnmanagedFunctionPointer(Cdecl)].
//   2. Implement the actual logic in a private module function.
//   3. Create a single static delegate instance wrapping that function.
//   4. Expose Marshal.GetFunctionPointerForDelegate as an IntPtr field.
//
// The static delegate instances (handlerDel, beforeDel, etc.) live for the
// process lifetime and thus never need their own GCHandles.

module internal Trampolines =

    // ── Delegate type declarations ────────────────────────────────────────────

    [<UnmanagedFunctionPointer(CallingConvention.Cdecl)>]
    type private HandlerDel = delegate of nativeint * nativeint -> nativeint

    [<UnmanagedFunctionPointer(CallingConvention.Cdecl)>]
    type private BeforeDel = delegate of nativeint * nativeint -> nativeint

    [<UnmanagedFunctionPointer(CallingConvention.Cdecl)>]
    type private AfterDel = delegate of nativeint * nativeint * nativeint -> nativeint

    [<UnmanagedFunctionPointer(CallingConvention.Cdecl)>]
    type private CtxFreeDel = delegate of nativeint -> unit

    // ── Error sanitisation ────────────────────────────────────────────────────
    //
    // Routes managed exception messages through sanitizeForLog before forwarding
    // to the native error channel. Prevents log injection and runaway allocations.

    let private reportError (ex: exn) =
        try
            let raw  = ex.Message |> Option.ofObj |> Option.defaultValue "unknown error"
            conduit_capi_report_error (sanitizeForLog raw)
        with _ -> ()  // best-effort

    // ── Route / not-found / error handler trampoline ─────────────────────────
    //
    // ctx  = GCHandle to a (Request -> Response) function
    // req  = ConduitRequest*
    // →      ConduitResponse* (owned by Rust) or 0n on fatal error
    //
    // We return a full 500 JSON response rather than returning 0n (NULL) here
    // because conduit-capi would then forward the raw error string as plain-text
    // 500, bypassing the F# OnError handler and leaking internal details.

    let private handlerImpl (ctx: nativeint) (req: nativeint) : nativeint =
        // Null ctx = native bug; return 500 rather than crashing into native code.
        if ctx = 0n then
            Response.json "{\"error\":\"internal server error\"}" |> Response.withStatus 500 |> Response.toNative
        else
            try
                let fn  = GCHandle.FromIntPtr(ctx).Target :?> (Request -> Response)
                fn (Request req) |> Response.toNative
            with
            | HaltException r -> Response.toNative r
            | ex ->
                reportError ex
                try  Response.json "{\"error\":\"internal server error\"}" |> Response.withStatus 500 |> Response.toNative
                with _ -> 0n

    // ── Before-filter trampoline ──────────────────────────────────────────────
    //
    // ctx  = GCHandle to a (Request -> Response option) function
    // →      0n = continue to next filter/route
    //        non-zero ConduitResponse* = short-circuit immediately

    let private beforeImpl (ctx: nativeint) (req: nativeint) : nativeint =
        // Null ctx = no filter registered; NULL = continue to next filter/route.
        if ctx = 0n then 0n
        else
            try
                let fn = GCHandle.FromIntPtr(ctx).Target :?> (Request -> Response option)
                match fn (Request req) with
                | None   -> 0n
                | Some r -> Response.toNative r
            with
            | HaltException r -> Response.toNative r
            | ex ->
                // On exception, log and continue — a filter failure is not fatal.
                reportError ex
                0n  // NULL = continue to next filter/route

    // ── After-hook trampoline ─────────────────────────────────────────────────
    //
    // ctx     = GCHandle to a (Request -> Response -> Response) function
    // current = OWNED ConduitResponse* — we must return a valid pointer.
    // →        ConduitResponse* for the final response

    let private afterImpl (ctx: nativeint) (req: nativeint) (current: nativeint) : nativeint =
        // Null ctx = no hook registered; pass current through unchanged.
        if ctx = 0n then current
        else
            try
                let currentResp = Response.fromNative current
                conduit_response_free current

                let fn = GCHandle.FromIntPtr(ctx).Target :?> (Request -> Response -> Response)
                fn (Request req) currentResp |> Response.toNative
            with
            | HaltException r ->
                conduit_response_free current
                Response.toNative r
            | ex ->
                conduit_response_free current
                reportError ex
                // After-hooks must return a valid non-null pointer.
                try  Response.json "{\"error\":\"internal server error\"}" |> Response.withStatus 500 |> Response.toNative
                with _ -> Response.text "Internal Server Error" |> Response.withStatus 500 |> Response.toNative

    // ── GCHandle destructor trampoline ────────────────────────────────────────
    //
    // Called by Rust when the owning server is freed. Releases the strong GC root.

    let private ctxFreeImpl (ctx: nativeint) =
        try GCHandle.FromIntPtr(ctx).Free()
        with _ -> ()  // double-free or invalid — ignore

    // ── Static delegate instances (process-lifetime) ──────────────────────────

    let private handlerDel  = HandlerDel  handlerImpl
    let private beforeDel   = BeforeDel   beforeImpl
    let private afterDel    = AfterDel    afterImpl
    let private ctxFreeDel  = CtxFreeDel  ctxFreeImpl

    // ── Exported function pointers ────────────────────────────────────────────

    let Handler  = Marshal.GetFunctionPointerForDelegate handlerDel
    let Before   = Marshal.GetFunctionPointerForDelegate beforeDel
    let After    = Marshal.GetFunctionPointerForDelegate afterDel
    let CtxFree  = Marshal.GetFunctionPointerForDelegate ctxFreeDel


// ── Application ───────────────────────────────────────────────────────────────
//
// Mutable builder. Fluent configuration, then call Application.bind to obtain a
// Server. After bind the Application is consumed and must not be used again.
//
// Idiomatic F# usage — pipe the builder through configuration functions:
//
//   let server =
//       Application.create()
//       |> Application.set "app_name" "hello"
//       |> Application.get "/" (fun req -> Response.html "<h1>Hi</h1>")
//       |> Application.bind "127.0.0.1" 3000us

/// Internal mutable state for the builder (all user-visible operations go through the Application module).
type Application internal (ptr: nativeint) =
    let mutable _ptr  = ptr
    let mutable _used = false
    let _handles      = System.Collections.Generic.List<GCHandle>()

    member internal _.Ptr
        with get() =
            if _used then invalidOp "Application has already been consumed by bind()."
            if _ptr = 0n then invalidOp "Application native pointer is null."
            _ptr

    member internal _.Alloc<'T when 'T : not struct>(fn: 'T) : nativeint =
        let h = GCHandle.Alloc(fn)
        _handles.Add h
        GCHandle.ToIntPtr h

    member internal _.MarkConsumed() = _used <- true; _ptr <- 0n

    interface IDisposable with
        member _.Dispose() =
            if not _used && _ptr <> 0n then
                conduit_app_free _ptr
                _ptr <- 0n
            // Free any handles that didn't transfer to Rust (i.e. if Bind failed).
            for h in _handles do
                try h.Free() with _ -> ()
            _handles.Clear()

// ── Server ────────────────────────────────────────────────────────────────────
//
// Returned by Application.bind. All operations delegate to the native conduit_server_*
// functions. Implements IDisposable — wrapping in `use` is the idiomatic pattern.

/// A bound conduit server. Dispose to stop the server and free native resources.
type Server internal (ptr: nativeint) =

    let mutable _ptr  = ptr
    let mutable _freed = false

    let checkAlive () =
        if _freed then invalidOp "Server has already been disposed."
        if _ptr = 0n then invalidOp "Server native pointer is null."

    /// The TCP port the server is listening on. Useful when you bound with port 0.
    member _.LocalPort =
        checkAlive()
        conduit_server_local_port _ptr

    /// True once the Tokio accept-loop is live. Returns false (not throws) if already disposed.
    member _.IsRunning =
        if _freed || _ptr = 0n then false
        else conduit_server_running _ptr <> 0

    /// Block the current thread indefinitely, serving requests until the process
    /// exits or Stop() is called from another thread.
    member _.Serve() =
        checkAlive()
        conduit_server_serve _ptr |> ignore

    /// Start the Tokio accept-loop on a background OS thread and return immediately.
    member _.ServeBackground() =
        checkAlive()
        conduit_server_serve_background _ptr |> ignore

    /// Signal the server to stop accepting new connections.
    member _.Stop() =
        checkAlive()
        conduit_server_stop _ptr

    interface IDisposable with
        member _.Dispose() =
            if not _freed && _ptr <> 0n then
                conduit_server_free _ptr
                _ptr  <- 0n
                _freed <- true


/// Functional builder for configuring and binding a conduit web server.
///
/// Pipe an Application through configuration functions, then call `Application.bind`
/// to start listening:
///
///   Application.create()
///   |> Application.set "version" "1.0"
///   |> Application.get "/hello" (fun req -> Response.text "hi")
///   |> Application.bind "127.0.0.1" 3000us
module Application =

    /// Create a new Application builder.
    let create () =
        let p = conduit_app_new()
        if p = 0n then invalidOp "conduit_app_new() returned null — native library failed to initialise."
        new Application(p)

    // ── Settings ──────────────────────────────────────────────────────────────

    /// Store a string setting. Must be called before bind().
    let set (key: string) (value: string) (app: Application) =
        conduit_app_set_setting(app.Ptr, key, value)
        app

    /// Retrieve a setting stored with set(). Returns None if the key is absent.
    /// Must be called before bind() — the native app is consumed by bind().
    let getSetting (key: string) (app: Application) : string option =
        let ptr = conduit_app_get_setting(app.Ptr, key)
        if ptr = 0n then None
        else
            let s = Marshal.PtrToStringUTF8 ptr
            conduit_string_free ptr
            Option.ofObj s

    // ── Route registration ────────────────────────────────────────────────────

    let private addRoute (meth: string) (pattern: string) (handler: Request -> Response) (app: Application) =
        let ctx = app.Alloc handler
        conduit_app_add_route(app.Ptr, meth, pattern, Trampolines.Handler, ctx, Trampolines.CtxFree)
        app

    /// Register a GET route.
    let get    p h app = addRoute "GET"    p h app
    /// Register a POST route.
    let post   p h app = addRoute "POST"   p h app
    /// Register a PUT route.
    let put    p h app = addRoute "PUT"    p h app
    /// Register a DELETE route.
    let delete p h app = addRoute "DELETE" p h app
    /// Register a PATCH route.
    let patch  p h app = addRoute "PATCH"  p h app

    /// Register a route with an explicit HTTP method string.
    let route (meth: string) p h app = addRoute meth p h app

    // ── Filters and hooks ─────────────────────────────────────────────────────

    /// Add a before-filter. Return None to continue; return Some response to
    /// short-circuit immediately and send that response.
    let before (filter: Request -> Response option) (app: Application) =
        let ctx = app.Alloc filter
        conduit_app_add_before(app.Ptr, Trampolines.Before, ctx, Trampolines.CtxFree)
        app

    /// Add an after-hook. Receives the current response and returns the (possibly
    /// modified) replacement.
    let after (hook: Request -> Response -> Response) (app: Application) =
        let ctx = app.Alloc hook
        conduit_app_add_after(app.Ptr, Trampolines.After, ctx, Trampolines.CtxFree)
        app

    // ── Special handlers ──────────────────────────────────────────────────────

    /// Register a custom 404 handler. Runs when no route matches.
    let notFound (handler: Request -> Response) (app: Application) =
        let ctx = app.Alloc handler
        conduit_app_set_not_found(app.Ptr, Trampolines.Handler, ctx, Trampolines.CtxFree)
        app

    /// Register a custom error handler. Runs when a route handler throws an
    /// unhandled exception. Use req.Error to read the sanitised error message.
    let onError (handler: Request -> Response) (app: Application) =
        let ctx = app.Alloc handler
        conduit_app_set_error_handler(app.Ptr, Trampolines.Handler, ctx, Trampolines.CtxFree)
        app

    // ── Bind ──────────────────────────────────────────────────────────────────

    /// Consume the Application, bind to the given host and port, and return a
    /// Server. The Application must not be used after this call.
    ///
    /// Use port 0 to let the OS pick an ephemeral port; read it back via
    /// server.LocalPort after the call.
    ///
    /// Raises InvalidOperationException on failure (e.g. port already in use).
    let bind (host: string) (port: uint16) (app: Application) =
        let ptr = app.Ptr
        app.MarkConsumed()
        let srv = conduit_server_bind(host, port, ptr)
        if srv = 0n then
            let rawErr = cstrNotNull (conduit_last_error()) |> sanitizeForLog
            eprintfn $"[conduit] conduit_server_bind failed: {rawErr}"
            raise (InvalidOperationException(
                $"Failed to bind conduit server on {host}:{port}. See stderr for details."))
        new Server(srv)
