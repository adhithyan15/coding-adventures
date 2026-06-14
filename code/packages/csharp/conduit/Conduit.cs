// Conduit.cs — C# P/Invoke binding for the conduit-capi C ABI (WEB15)
//
// Conduit is a Sinatra/Express-style web framework. This file wraps the Rust
// `conduit-capi` shared library via .NET's Platform Invocation Services (P/Invoke).
//
// ARCHITECTURE OVERVIEW
// ─────────────────────
//   Program.cs  →  Application (C# builder)
//                  │  registers routes/hooks as C# lambdas
//                  ▼
//               Trampolines (static [UnmanagedCallersOnly] methods)
//                  │  convert calls across the managed/unmanaged boundary
//                  ▼
//               Native (P/Invoke [DllImport] declarations)
//                  │
//                  ▼
//               libconduit_capi.so / .dylib  (Rust cdylib, WEB12)
//
// THE DELEGATE LIFETIME PROBLEM
// ──────────────────────────────
// .NET's GC can move or collect objects at any time. If we pass a managed
// delegate's function pointer to C and the delegate is later collected, the
// C code holds a dangling pointer — a crash waiting to happen.
//
// Solution: GCHandle.Alloc(closure) creates a *strong root* that prevents GC
// collection. We pass GCHandle.ToIntPtr() as the opaque `void* ctx` to C.
// When C calls `ctx_free`, our static trampoline recovers the handle and calls
// .Free() — releasing the root so the GC may eventually reclaim the closure.
//
// This is the same pattern as Go's cgo.Handle in WEB14.
//
//   C# managed heap                 C native heap
//  ┌──────────────┐                ┌──────────────────────┐
//  │  lambda fn   │◄── GCHandle ──►│ ctx (opaque IntPtr)  │
//  └──────────────┘  (strong root) └──────────────────────┘
//         ▲                                   │
//         └───────────────────────────────────┘
//               trampoline recovers via
//          GCHandle.FromIntPtr(ctx).Target

using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace CodingAdventures.Conduit;

// ── Native P/Invoke layer ────────────────────────────────────────────────────
//
// All [DllImport] declarations live here. This class is internal — users never
// touch it directly. The static constructor installs a NativeLibrary resolver so
// we can locate libconduit_capi.so/.dylib without requiring it on LD_LIBRARY_PATH.

internal static class Native
{
    private const string Lib = "conduit_capi";

    static Native()
    {
        // Install our resolver *before* any P/Invoke fires. .NET calls this
        // delegate whenever it needs to load a native library by DllImport name.
        NativeLibrary.SetDllImportResolver(typeof(Native).Assembly, Resolve);
    }

    // Check CONDUIT_CAPI_PATH first (set by tools/run-tests.sh to the exact
    // path of the built .so/.dylib). Fall through to OS default search otherwise
    // (LD_LIBRARY_PATH, DYLD_LIBRARY_PATH, RPATH, etc.).
    private static IntPtr Resolve(string name, Assembly asm, DllImportSearchPath? paths)
    {
        if (name != Lib) return IntPtr.Zero;

        var env = Environment.GetEnvironmentVariable("CONDUIT_CAPI_PATH");
        if (env is { Length: > 0 })
        {
            // No File.Exists pre-check — that would introduce a TOCTOU race
            // (an attacker could replace the file between check and load).
            // NativeLibrary.Load throws DllNotFoundException with a clear message
            // if the path does not exist or is not a valid shared library.
            // CONDUIT_CAPI_PATH is a build-time variable set by run-tests.sh and
            // must not be influenced by untrusted user input.
            return NativeLibrary.Load(env);
        }

        return NativeLibrary.Load(name, asm, paths);
    }

    // ── Error channels ───────────────────────────────────────────────────────

    // Store a thread-local error message before returning NULL from a handler.
    [DllImport(Lib)]
    internal static extern void conduit_capi_report_error(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string msg);

    // Retrieve the last error (valid until the next conduit call on this thread).
    [DllImport(Lib)]
    internal static extern IntPtr conduit_last_error();

    // ── App lifecycle ────────────────────────────────────────────────────────

    [DllImport(Lib)] internal static extern IntPtr conduit_app_new();
    [DllImport(Lib)] internal static extern void   conduit_app_free(IntPtr app);

    [DllImport(Lib)]
    internal static extern void conduit_app_set_setting(
        IntPtr app,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string key,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string value);

    // Returns an owned char* — caller must conduit_string_free it.
    [DllImport(Lib)]
    internal static extern IntPtr conduit_app_get_setting(
        IntPtr app,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string key);

    [DllImport(Lib)]
    internal static extern void conduit_app_add_route(
        IntPtr app,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string method,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string pattern,
        IntPtr handler, IntPtr ctx, IntPtr ctxFree);

    [DllImport(Lib)]
    internal static extern void conduit_app_add_before(
        IntPtr app, IntPtr handler, IntPtr ctx, IntPtr ctxFree);

    [DllImport(Lib)]
    internal static extern void conduit_app_add_after(
        IntPtr app, IntPtr handler, IntPtr ctx, IntPtr ctxFree);

    [DllImport(Lib)]
    internal static extern void conduit_app_set_not_found(
        IntPtr app, IntPtr handler, IntPtr ctx, IntPtr ctxFree);

    [DllImport(Lib)]
    internal static extern void conduit_app_set_error_handler(
        IntPtr app, IntPtr handler, IntPtr ctx, IntPtr ctxFree);

    // ── Server ───────────────────────────────────────────────────────────────

    // Consumes `app` on both success and failure. Returns NULL on error.
    [DllImport(Lib)]
    internal static extern IntPtr conduit_server_bind(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string host,
        ushort port,
        IntPtr app);

    [DllImport(Lib)] internal static extern int    conduit_server_serve(IntPtr srv);
    [DllImport(Lib)] internal static extern int    conduit_server_serve_background(IntPtr srv);
    [DllImport(Lib)] internal static extern void   conduit_server_stop(IntPtr srv);
    [DllImport(Lib)] internal static extern ushort conduit_server_local_port(IntPtr srv);
    [DllImport(Lib)] internal static extern int    conduit_server_running(IntPtr srv);
    [DllImport(Lib)] internal static extern void   conduit_server_free(IntPtr srv);

    // ── Request accessors ────────────────────────────────────────────────────
    //
    // All return borrowed const char* valid only for the duration of the handler
    // call. Marshal.PtrToStringUTF8 copies the bytes into a managed string
    // immediately, which is exactly what we need.

    [DllImport(Lib)] internal static extern IntPtr conduit_request_method(IntPtr req);
    [DllImport(Lib)] internal static extern IntPtr conduit_request_path(IntPtr req);
    [DllImport(Lib)] internal static extern IntPtr conduit_request_query_string(IntPtr req);
    [DllImport(Lib)] internal static extern IntPtr conduit_request_content_type(IntPtr req);
    [DllImport(Lib)] internal static extern IntPtr conduit_request_remote_addr(IntPtr req);
    [DllImport(Lib)] internal static extern IntPtr conduit_request_error(IntPtr req);

    // Returns pointer + length — body is NOT null-terminated.
    [DllImport(Lib)]
    internal static extern IntPtr conduit_request_body(IntPtr req, out nuint outLen);

    [DllImport(Lib)]
    internal static extern IntPtr conduit_request_param(
        IntPtr req, [MarshalAs(UnmanagedType.LPUTF8Str)] string name);

    [DllImport(Lib)]
    internal static extern IntPtr conduit_request_query(
        IntPtr req, [MarshalAs(UnmanagedType.LPUTF8Str)] string name);

    [DllImport(Lib)]
    internal static extern IntPtr conduit_request_header(
        IntPtr req, [MarshalAs(UnmanagedType.LPUTF8Str)] string name);

    // ── Response builder / reader ────────────────────────────────────────────

    [DllImport(Lib)]
    internal static extern IntPtr conduit_response_new(
        ushort status, IntPtr body, nuint bodyLen);

    [DllImport(Lib)]
    internal static extern void conduit_response_set_header(
        IntPtr resp,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string name,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string value);

    [DllImport(Lib)] internal static extern ushort conduit_response_status(IntPtr resp);
    [DllImport(Lib)] internal static extern IntPtr conduit_response_body(IntPtr resp, out nuint outLen);
    [DllImport(Lib)] internal static extern nuint  conduit_response_header_count(IntPtr resp);
    [DllImport(Lib)] internal static extern IntPtr conduit_response_header_name(IntPtr resp, nuint i);
    [DllImport(Lib)] internal static extern IntPtr conduit_response_header_value(IntPtr resp, nuint i);
    [DllImport(Lib)] internal static extern void   conduit_response_free(IntPtr resp);
    [DllImport(Lib)] internal static extern void   conduit_string_free(IntPtr s);

    // ── Helpers ──────────────────────────────────────────────────────────────

    internal static string? CStr(IntPtr p) =>
        p == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(p);

    internal static string CStrNotNull(IntPtr p) => CStr(p) ?? "";
}

// ── Delegate types ────────────────────────────────────────────────────────────

/// <summary>Handler function for routes, not-found handlers, and error handlers.</summary>
public delegate Response HandlerFunc(Request req);

/// <summary>
/// Before-filter function. Return null to continue to the next filter/route;
/// return a Response to short-circuit immediately.
/// </summary>
public delegate Response? BeforeFunc(Request req);

/// <summary>
/// After-hook function. Receives the current response (read from native), returns
/// a (possibly modified) Response that replaces it.
/// </summary>
public delegate Response AfterFunc(Request req, Response current);

// ── Trampolines ───────────────────────────────────────────────────────────────
//
// [UnmanagedCallersOnly] declares a static method as directly callable from C
// code using the C calling convention (cdecl). When Rust calls a handler function
// pointer, execution enters one of these methods.
//
// Rules enforced by the compiler:
//   1. Must be static — no 'this' in C.
//   2. Parameters must be blittable: IntPtr, ushort, nuint, bool, etc.
//   3. Exceptions must not escape — always catch and handle inside.
//
// Function pointer addresses are captured once in the static constructor and
// cached as IntPtr fields for Application to pass to conduit_app_add_route etc.

// [ExcludeFromCodeCoverage] is mandatory here: coverlet instruments IL by injecting
// managed calls, which violates the [UnmanagedCallersOnly] contract (those methods
// must not be entered via managed call sites). Without this attribute, running
// `dotnet test` with coverage enabled crashes the test host.
[System.Diagnostics.CodeAnalysis.ExcludeFromCodeCoverage]
internal static unsafe class Trampolines
{
    internal static readonly IntPtr Handler;
    internal static readonly IntPtr BeforeHandler;
    internal static readonly IntPtr After;
    internal static readonly IntPtr CtxFree;

    static Trampolines()
    {
        // The & operator on an [UnmanagedCallersOnly] method yields a typed native
        // function pointer. Cast to IntPtr for opaque storage and passing to C.
        Handler       = (IntPtr)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr>)&HandlerFn;
        BeforeHandler = (IntPtr)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr>)&BeforeFn;
        After         = (IntPtr)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr, IntPtr>)&AfterFn;
        CtxFree       = (IntPtr)(delegate* unmanaged[Cdecl]<IntPtr, void>)&CtxFreeFn;
    }

    // ── Route / not-found / error handler trampoline ─────────────────────────
    //
    // ctx holds a GCHandle to a HandlerFunc delegate.
    // Returns a new ConduitResponse* (owned by Rust), or IntPtr.Zero on error.
    //
    // WHY we return a 500 JSON response directly instead of calling
    // conduit_capi_report_error + returning NULL:
    //   When a C handler returns NULL after calling conduit_capi_report_error,
    //   conduit-capi returns the raw error string as the response body directly
    //   (as a plain-text 500), bypassing the C# error handler registered via
    //   conduit_app_set_error_handler. To prevent error details from leaking to
    //   clients and to ensure correct JSON format, we build the 500 ourselves.
    //   The native OnError handler is still registered and will be called for
    //   conduit-internal errors that conduit-capi routes there directly.

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static IntPtr HandlerFn(IntPtr ctx, IntPtr req)
    {
        // A null ctx means the handle was never registered or has already been freed —
        // this indicates a bug in the native layer. Return 500 rather than crashing.
        if (ctx == IntPtr.Zero)
            return Response.Json("{\"error\":\"internal server error\"}", 500).ToNative();

        try
        {
            var fn = (HandlerFunc)GCHandle.FromIntPtr(ctx).Target!;
            return fn(new Request(req)).ToNative();
        }
        catch (HaltException h) { return h.Response.ToNative(); }
        catch (Exception ex)
        {
            ReportError(ex);  // log server-side
            // Return a safe 500 — never reflect exception details to clients.
            try { return Response.Json("{\"error\":\"internal server error\"}", 500).ToNative(); }
            catch { return IntPtr.Zero; }
        }
    }

    // ── Before-filter trampoline ──────────────────────────────────────────────
    //
    // ctx holds a GCHandle to a BeforeFunc delegate (stored directly — no wrapper).
    // Returns IntPtr.Zero = continue; non-zero = short-circuit with that response.

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static IntPtr BeforeFn(IntPtr ctx, IntPtr req)
    {
        // Null ctx = no filter registered; NULL return = continue to next filter/route.
        if (ctx == IntPtr.Zero) return IntPtr.Zero;

        try
        {
            var fn     = (BeforeFunc)GCHandle.FromIntPtr(ctx).Target!;
            var result = fn(new Request(req));
            return result is null ? IntPtr.Zero : result.ToNative();
        }
        catch (HaltException h) { return h.Response.ToNative(); }
        catch (Exception ex)
        {
            // On a before-filter exception, log it and continue (return NULL = continue).
            // A before-filter failure is not fatal — let the request reach a route.
            ReportError(ex);
            return IntPtr.Zero; // NULL = continue to next filter/route
        }
    }

    // ── After-hook trampoline ─────────────────────────────────────────────────
    //
    // ctx holds a GCHandle to an AfterFunc delegate.
    // `current` is OWNED by this call — we must either free it or return it.

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static IntPtr AfterFn(IntPtr ctx, IntPtr req, IntPtr current)
    {
        // Null ctx = no after-hook registered; pass `current` through unchanged
        // rather than crashing. We own `current` and must return something valid.
        if (ctx == IntPtr.Zero) return current;

        try
        {
            // Read the native response into a managed object, then release it.
            var currentResp = Response.FromNative(current);
            Native.conduit_response_free(current);

            var fn = (AfterFunc)GCHandle.FromIntPtr(ctx).Target!;
            return fn(new Request(req), currentResp).ToNative();
        }
        catch (HaltException h)
        {
            Native.conduit_response_free(current);
            return h.Response.ToNative();
        }
        catch (Exception ex)
        {
            Native.conduit_response_free(current);
            ReportError(ex);
            // After-hooks must return a valid response (NULL is UB here).
            try { return Response.Json("{\"error\":\"internal server error\"}", 500).ToNative(); }
            catch { return Response.Text("Internal Server Error", 500).ToNative(); }
        }
    }

    // ── GCHandle destructor ───────────────────────────────────────────────────
    //
    // Called by Rust when the owning app/server is freed. Releases the strong GC
    // root so the closure can be collected.

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static void CtxFreeFn(IntPtr ctx)
    {
        try { GCHandle.FromIntPtr(ctx).Free(); }
        catch { /* double-free or already freed — ignore */ }
    }

    // ── Error sanitisation ────────────────────────────────────────────────────
    //
    // Strip ASCII control characters and cap at 512 bytes before sending to the
    // native error channel. Prevents log injection and runaway allocations.

    internal static void ReportError(Exception ex)
    {
        try
        {
            var raw  = ex.Message ?? "unknown error";
            var safe = new StringBuilder(Math.Min(raw.Length, 512));
            foreach (var c in raw)
            {
                if (c < '\x20' || c == '\x7f') continue;
                if (safe.Length >= 512) break;
                safe.Append(c);
            }
            Native.conduit_capi_report_error(safe.ToString());
        }
        catch { /* best-effort */ }
    }
}

// ── Response ─────────────────────────────────────────────────────────────────
//
// An immutable bundle of (status, headers, body). Built via static factory methods.
// The native ConduitResponse* is only created when a handler actually returns, in
// ToNative().

public sealed class Response
{
    private readonly int    _status;
    private readonly string _body;
    private readonly List<(string Name, string Value)> _headers;

    private Response(int status, string body, List<(string, string)>? headers = null)
    {
        _status  = status;
        _body    = body;
        _headers = headers ?? new List<(string, string)>();
    }

    // ── Factory methods ───────────────────────────────────────────────────────

    /// <summary>HTML response (content-type: text/html; charset=utf-8).</summary>
    public static Response Html(string body, int status = 200) =>
        new(status, body, new List<(string, string)>
        {
            ("content-type", "text/html; charset=utf-8")
        });

    /// <summary>JSON response (content-type: application/json).</summary>
    public static Response Json(string body, int status = 200) =>
        new(status, body, new List<(string, string)>
        {
            ("content-type", "application/json")
        });

    /// <summary>Plain-text response (content-type: text/plain; charset=utf-8).</summary>
    public static Response Text(string body, int status = 200) =>
        new(status, body, new List<(string, string)>
        {
            ("content-type", "text/plain; charset=utf-8")
        });

    /// <summary>Arbitrary status + body + optional extra headers.</summary>
    public static Response Respond(int status, string body,
        params (string Name, string Value)[] headers) =>
        new(status, body, new List<(string, string)>(headers));

    /// <summary>
    /// HTTP redirect. Throws ArgumentException if the location contains CR or LF —
    /// belt-and-suspenders on top of conduit-capi's own header-injection defence.
    /// </summary>
    public static Response Redirect(string location, int status = 302)
    {
        if (location.Contains('\r') || location.Contains('\n'))
            throw new ArgumentException(
                "Redirect location must not contain CR or LF.", nameof(location));

        return new(status, "", new List<(string, string)>
        {
            ("location", location)
        });
    }

    // ── Accessors (useful in after-hooks) ─────────────────────────────────────

    public int    Status  => _status;
    public string Body    => _body;
    public IReadOnlyList<(string Name, string Value)> Headers => _headers;

    /// <summary>Return a new Response with an additional header appended.</summary>
    public Response WithHeader(string name, string value)
    {
        var h = new List<(string, string)>(_headers) { (name, value) };
        return new Response(_status, _body, h);
    }

    // ── Conversion to/from native ─────────────────────────────────────────────

    // Creates a ConduitResponse* — ownership transfers to Rust when returned from
    // a handler. For responses not returned from a handler, call
    // Native.conduit_response_free() to avoid a leak.
    internal IntPtr ToNative()
    {
        // Guard against out-of-range status codes before the narrowing cast to
        // ushort: an unchecked (ushort)70000 silently wraps to 4464, producing
        // a nonsensical wire status with no error. HTTP status codes live in
        // [100, 599]; we widen to [100, 999] for custom/non-standard codes.
        if (_status < 100 || _status > 999)
            throw new InvalidOperationException(
                $"HTTP status code {_status} is out of the valid range [100, 999].");

        byte[] body = Encoding.UTF8.GetBytes(_body);

        // GCHandle.Pinned keeps the byte array address stable while conduit_response_new
        // copies the bytes into the native buffer. We pin a 1-byte placeholder when
        // body is empty to ensure AddrOfPinnedObject() returns a valid (non-null) ptr.
        var pin = GCHandle.Alloc(
            body.Length > 0 ? body : new byte[] { 0 },
            GCHandleType.Pinned);
        try
        {
            var resp = Native.conduit_response_new(
                (ushort)_status,
                pin.AddrOfPinnedObject(),
                (nuint)body.Length);

            foreach (var (n, v) in _headers)
                Native.conduit_response_set_header(resp, n, v);

            return resp;
        }
        finally
        {
            pin.Free();
        }
    }

    // Reads a native ConduitResponse* into a managed Response, copying all data.
    // The caller is responsible for freeing the native pointer afterwards.
    internal static Response FromNative(IntPtr ptr)
    {
        var status  = (int)Native.conduit_response_status(ptr);

        var bodyPtr = Native.conduit_response_body(ptr, out var bodyLen);
        string body;
        if (bodyPtr == IntPtr.Zero || bodyLen == 0)
        {
            body = "";
        }
        else
        {
            // Guard against a rogue native layer returning a body length that
            // overflows .NET's Array.MaxLength (2_147_483_591 on current runtimes).
            if (bodyLen > (nuint)Array.MaxLength)
                throw new InvalidOperationException(
                    $"Native response body length {bodyLen} exceeds Array.MaxLength.");

            var bytes = new byte[(int)bodyLen];
            Marshal.Copy(bodyPtr, bytes, 0, (int)bodyLen);
            body = Encoding.UTF8.GetString(bytes);
        }

        var count   = (int)Native.conduit_response_header_count(ptr);
        var headers = new List<(string, string)>(count);
        for (int i = 0; i < count; i++)
        {
            var n = Native.CStrNotNull(Native.conduit_response_header_name(ptr, (nuint)i));
            var v = Native.CStrNotNull(Native.conduit_response_header_value(ptr, (nuint)i));
            headers.Add((n, v));
        }

        return new Response(status, body, headers);
    }
}

// ── HaltException ─────────────────────────────────────────────────────────────
//
// Throw this from inside any handler to immediately short-circuit with a specific
// response — equivalent to Sinatra's `halt` or an early `return` in Express.
//
// Example:
//   app.Before(req => {
//       if (!HasApiKey(req))
//           throw new HaltException(Response.Text("Unauthorized", 401));
//       return null;
//   });

public sealed class HaltException : Exception
{
    public Response Response { get; }
    public HaltException(Response response) : base("halt") => Response = response;
}

// ── Request ───────────────────────────────────────────────────────────────────
//
// A read-only view of an HTTP request, valid only for the duration of one handler
// invocation. DANGER: Do NOT store a Request and use it after the handler returns
// — the native ConduitRequest* is freed immediately after. Copy any data you need.

public sealed class Request
{
    private readonly IntPtr _ptr;

    internal Request(IntPtr ptr) => _ptr = ptr;

    /// <summary>HTTP method in uppercase: "GET", "POST", "PUT", "DELETE", etc.</summary>
    public string Method => Native.CStrNotNull(Native.conduit_request_method(_ptr));

    /// <summary>URL path without query string: "/api/users/42".</summary>
    public string Path => Native.CStrNotNull(Native.conduit_request_path(_ptr));

    /// <summary>Raw query string without the leading '?': "q=hello&page=2".</summary>
    public string QueryString => Native.CStrNotNull(Native.conduit_request_query_string(_ptr));

    /// <summary>Content-Type header value, or "" if absent.</summary>
    public string ContentType => Native.CStrNotNull(Native.conduit_request_content_type(_ptr));

    /// <summary>Remote address as "IP:port": "127.0.0.1:54321".</summary>
    public string RemoteAddr => Native.CStrNotNull(Native.conduit_request_remote_addr(_ptr));

    /// <summary>
    /// Non-empty only inside an OnError handler — the message stored by the
    /// failing route via conduit_capi_report_error (or a panic message from Rust).
    /// </summary>
    public string Error => Native.CStrNotNull(Native.conduit_request_error(_ptr));

    /// <summary>Named route parameter from the URL pattern, or null if absent.</summary>
    public string? Param(string name) =>
        Native.CStr(Native.conduit_request_param(_ptr, name));

    /// <summary>Query string value for the given key, or null if absent.</summary>
    public string? Query(string name) =>
        Native.CStr(Native.conduit_request_query(_ptr, name));

    /// <summary>Request header value (case-insensitive lookup), or null if absent.</summary>
    public string? Header(string name) =>
        Native.CStr(Native.conduit_request_header(_ptr, name));

    /// <summary>Raw request body bytes. Empty array if no body.</summary>
    public byte[] Body()
    {
        var ptr = Native.conduit_request_body(_ptr, out var len);
        if (ptr == IntPtr.Zero || len == 0) return Array.Empty<byte>();

        // Guard against a rogue native layer returning a body length that
        // overflows .NET's Array.MaxLength.
        if (len > (nuint)Array.MaxLength)
            throw new InvalidOperationException(
                $"Native request body length {len} exceeds Array.MaxLength.");

        var bytes = new byte[(int)len];
        Marshal.Copy(ptr, bytes, 0, (int)len);
        return bytes;
    }

    /// <summary>Request body decoded as UTF-8 text.</summary>
    public string BodyString() => Encoding.UTF8.GetString(Body());
}

// ── Application ───────────────────────────────────────────────────────────────
//
// The builder object. Register routes and hooks, then call Bind() to obtain a
// Server.
//
// CRITICAL: Bind() CONSUMES the native ConduitApp* (the Rust function moves it
// out even on failure). Do not call any method on this Application object after
// Bind() — it will throw InvalidOperationException.
//
// Settings captured with GetSetting() MUST be read before calling Bind():
//
//   var app = new Application();
//   app.Set("title", "MyApp");
//   var title = app.GetSetting("title");   // ← read settings HERE
//   app.Get("/", req => Response.Html($"<h1>{title}</h1>"));  // capture string
//   using var server = app.Bind();         // ← app consumed here

public sealed class Application : IDisposable
{
    private IntPtr _app;
    private bool   _consumed;

    // List of GCHandle roots we have allocated. Even though Rust will call
    // ctx_free for each when the server is freed, we keep a copy here so that
    // Dispose() can clean up handles in the case where Bind() never succeeded.
    private readonly List<GCHandle> _handles = new();

    public Application() => _app = Native.conduit_app_new();

    // ── Settings ──────────────────────────────────────────────────────────────

    /// <summary>Store a string setting (must be called before Bind).</summary>
    public Application Set(string key, string value)
    {
        CheckAlive();
        Native.conduit_app_set_setting(_app, key, value);
        return this;
    }

    /// <summary>
    /// Retrieve a setting stored with Set(). Returns null if the key is absent.
    /// Must be called before Bind() — the ConduitApp* is consumed on Bind().
    /// </summary>
    public string? GetSetting(string key)
    {
        CheckAlive();
        var ptr = Native.conduit_app_get_setting(_app, key);
        if (ptr == IntPtr.Zero) return null;
        var s = Marshal.PtrToStringUTF8(ptr);
        Native.conduit_string_free(ptr);
        return s;
    }

    // ── Route registration (fluent) ───────────────────────────────────────────

    public Application Get(string pattern, HandlerFunc handler)    => Route("GET",    pattern, handler);
    public Application Post(string pattern, HandlerFunc handler)   => Route("POST",   pattern, handler);
    public Application Put(string pattern, HandlerFunc handler)    => Route("PUT",    pattern, handler);
    public Application Delete(string pattern, HandlerFunc handler) => Route("DELETE", pattern, handler);
    public Application Patch(string pattern, HandlerFunc handler)  => Route("PATCH",  pattern, handler);

    public Application Route(string method, string pattern, HandlerFunc handler)
    {
        CheckAlive();
        var handle = GCHandle.Alloc(handler);
        _handles.Add(handle);
        Native.conduit_app_add_route(
            _app, method, pattern,
            Trampolines.Handler, GCHandle.ToIntPtr(handle), Trampolines.CtxFree);
        return this;
    }

    // ── Filters and hooks ─────────────────────────────────────────────────────

    /// <summary>
    /// Add a before-filter. Return null to continue to the next middleware/route;
    /// return a Response to short-circuit and send that response immediately.
    /// </summary>
    public Application Before(BeforeFunc filter)
    {
        CheckAlive();
        // Store the BeforeFunc delegate directly in the GCHandle (no wrapper needed).
        // BeforeFn casts GCHandle.Target directly to BeforeFunc.
        var handle = GCHandle.Alloc(filter);
        _handles.Add(handle);
        Native.conduit_app_add_before(
            _app,
            Trampolines.BeforeHandler, GCHandle.ToIntPtr(handle), Trampolines.CtxFree);
        return this;
    }

    /// <summary>Add an after-hook. Receives current Response; return modified Response.</summary>
    public Application After(AfterFunc hook)
    {
        CheckAlive();
        var handle = GCHandle.Alloc(hook);
        _handles.Add(handle);
        Native.conduit_app_add_after(
            _app,
            Trampolines.After, GCHandle.ToIntPtr(handle), Trampolines.CtxFree);
        return this;
    }

    /// <summary>Custom handler for requests that match no route (default: 404).</summary>
    public Application NotFound(HandlerFunc handler)
    {
        CheckAlive();
        var handle = GCHandle.Alloc(handler);
        _handles.Add(handle);
        Native.conduit_app_set_not_found(
            _app,
            Trampolines.Handler, GCHandle.ToIntPtr(handle), Trampolines.CtxFree);
        return this;
    }

    /// <summary>Custom error handler. Inspect req.Error for the failure message.</summary>
    public Application OnError(HandlerFunc handler)
    {
        CheckAlive();
        var handle = GCHandle.Alloc(handler);
        _handles.Add(handle);
        Native.conduit_app_set_error_handler(
            _app,
            Trampolines.Handler, GCHandle.ToIntPtr(handle), Trampolines.CtxFree);
        return this;
    }

    // ── Bind ──────────────────────────────────────────────────────────────────

    /// <summary>
    /// Bind to host:port and return a Server. Consumes this Application object.
    ///
    /// Use port 0 to let the OS assign an ephemeral port; read it back via
    /// Server.LocalPort.
    /// </summary>
    public Server Bind(string host = "127.0.0.1", ushort port = 3000)
    {
        CheckAlive();
        _consumed = true; // mark before the call so Dispose skips conduit_app_free

        var srv = Native.conduit_server_bind(host, port, _app);
        if (srv == IntPtr.Zero)
        {
            // Log the raw native error to stderr (server-controlled output only) so
            // operators can diagnose bind failures without leaking internals to callers.
            var rawErr = Native.CStrNotNull(Native.conduit_last_error());
            Console.Error.WriteLine($"[conduit] conduit_server_bind failed: {rawErr}");

            // Throw a sanitized message. "Failed to bind" is enough for callers to act on;
            // the details are in the log above where they can't reach end-users.
            throw new InvalidOperationException(
                $"Failed to bind conduit server on {host}:{port}. See stderr for details.");
        }

        return new Server(srv);
    }

    // ── IDisposable ───────────────────────────────────────────────────────────

    public void Dispose()
    {
        if (!_consumed && _app != IntPtr.Zero)
        {
            Native.conduit_app_free(_app);
            _app = IntPtr.Zero;
        }

        // Free any handles we still hold. Rust calls ctx_free for routes that
        // were registered on a bound server, but if Bind() never succeeded these
        // handles would leak without this cleanup.
        foreach (var h in _handles)
        {
            if (h.IsAllocated) { try { h.Free(); } catch { /* ignore */ } }
        }
        _handles.Clear();
    }

    private void CheckAlive()
    {
        if (_consumed)
            throw new InvalidOperationException(
                "Application has already been passed to Bind(). " +
                "Create a new Application to register more routes.");
    }
}

// ── Server ────────────────────────────────────────────────────────────────────
//
// Owns the native ConduitServer* handle. Always dispose via `using` so the OS
// socket is released promptly.
//
//   using var server = app.Bind("127.0.0.1", 0);
//   server.ServeBackground();
//   // … test code …
//   // server.Dispose() called automatically at end of `using` block

public sealed class Server : IDisposable
{
    private IntPtr _srv;

    internal Server(IntPtr srv) => _srv = srv;

    /// <summary>
    /// Block the calling thread and serve requests until Stop() is called.
    /// Returns 0 on clean shutdown.
    /// </summary>
    public int Serve()
    {
        CheckAlive();
        return Native.conduit_server_serve(_srv);
    }

    /// <summary>
    /// Start serving in a background OS thread. Returns immediately.
    /// Returns 0 on success.
    /// </summary>
    public int ServeBackground()
    {
        CheckAlive();
        return Native.conduit_server_serve_background(_srv);
    }

    /// <summary>Signal the server to stop accepting new connections.</summary>
    public void Stop()
    {
        if (_srv != IntPtr.Zero)
            Native.conduit_server_stop(_srv);
    }

    /// <summary>
    /// The actual port the OS assigned. Useful when you passed port 0 to Bind().
    /// </summary>
    public ushort LocalPort
    {
        get { CheckAlive(); return Native.conduit_server_local_port(_srv); }
    }

    /// <summary>True if the server background thread is currently running.</summary>
    public bool IsRunning =>
        _srv != IntPtr.Zero && Native.conduit_server_running(_srv) != 0;

    public void Dispose()
    {
        if (_srv != IntPtr.Zero)
        {
            Native.conduit_server_stop(_srv);
            Native.conduit_server_free(_srv);
            _srv = IntPtr.Zero;
        }
    }

    private void CheckAlive()
    {
        if (_srv == IntPtr.Zero)
            throw new ObjectDisposedException(nameof(Server));
    }
}
