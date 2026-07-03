// Package conduit is a Sinatra/Express-style web framework for Go.
//
// It hosts the Rust web-core HTTP engine (WEB08 facade) through the reusable
// conduit-capi C ABI via cgo. Your handlers are ordinary Go funcs; routing,
// lifecycle hooks, and HTTP I/O run in Rust. This is the WEB14 port in the
// cross-language Conduit family and the third consumer of conduit-capi (after
// Swift and C++).
//
//	app := conduit.New()
//	app.Before(func(req *conduit.Request) *conduit.Response {
//	    if req.Path() == "/down" { conduit.Halt(503, "maintenance") }
//	    return nil
//	})
//	app.Get("/", func(*conduit.Request) conduit.Response { return conduit.HTML("<h1>Hi</h1>") })
//	server, _ := app.Bind("127.0.0.1", 3000)
//	server.Serve() // blocks until stopped
package conduit

/*
#cgo CFLAGS: -I${SRCDIR}/../../rust/conduit-capi/include
// Link the STATIC archive by full path (not -lconduit_capi): on Linux, ld
// prefers the sibling libconduit_capi.so when both sit in the search path,
// yielding binaries that fail to load the .so at runtime. Naming the .a forces
// static linking. The trailing libs are the Rust staticlib's native deps.
#cgo darwin LDFLAGS: ${SRCDIR}/../../rust/target/release/libconduit_capi.a -liconv
#cgo linux  LDFLAGS: ${SRCDIR}/../../rust/target/release/libconduit_capi.a -lpthread -ldl -lm -lrt -lutil

#include <stdlib.h>
#include "conduit_capi.h"

// Forward declarations of the Go-exported trampolines (defined below via
// //export). cgo generates these with non-const ConduitRequest*, so we cast to
// the ABI's const-pointer typedefs in the shims.
ConduitResponse* goConduitHandler(void* ctx, ConduitRequest* req);
ConduitResponse* goConduitBefore(void* ctx, ConduitRequest* req);
ConduitResponse* goConduitAfter(void* ctx, ConduitRequest* req, ConduitResponse* cur);
void goConduitFree(void* ctx);

// C shims: bind each trampoline + cast a uintptr handle to the opaque ctx. This
// keeps all function-pointer/void* juggling on the C side so Go only ever passes
// a clean C.uintptr_t (no go vet "misuse of unsafe.Pointer" from uintptr->ptr).
static void conduitGoAddRoute(ConduitApp* a, const char* m, const char* p, uintptr_t h) {
    conduit_app_add_route(a, m, p, (ConduitHandler)goConduitHandler, (void*)h, (ConduitCtxFree)goConduitFree);
}
static void conduitGoAddBefore(ConduitApp* a, uintptr_t h) {
    conduit_app_add_before(a, (ConduitHandler)goConduitBefore, (void*)h, (ConduitCtxFree)goConduitFree);
}
static void conduitGoAddAfter(ConduitApp* a, uintptr_t h) {
    conduit_app_add_after(a, (ConduitAfter)goConduitAfter, (void*)h, (ConduitCtxFree)goConduitFree);
}
static void conduitGoSetNotFound(ConduitApp* a, uintptr_t h) {
    conduit_app_set_not_found(a, (ConduitHandler)goConduitHandler, (void*)h, (ConduitCtxFree)goConduitFree);
}
static void conduitGoSetError(ConduitApp* a, uintptr_t h) {
    conduit_app_set_error_handler(a, (ConduitHandler)goConduitHandler, (void*)h, (ConduitCtxFree)goConduitFree);
}
*/
import "C"

import (
	"fmt"
	"runtime/cgo"
	"strings"
	"unsafe"
)

// HandlerFunc handles a request and returns a response (routes, not-found, error).
type HandlerFunc func(*Request) Response

// BeforeFunc runs before routing; return a *Response to short-circuit, or nil to
// continue. Calling Halt short-circuits too.
type BeforeFunc func(*Request) *Response

// AfterFunc transforms the response after the handler runs (return it unchanged
// to merely observe).
type AfterFunc func(*Request, Response) Response

// Header is a response header name/value pair.
type Header struct{ Name, Value string }

// ── Response ─────────────────────────────────────────────────────────────────

// Response is an HTTP response. Build one directly or with the helpers below.
// The native side clamps the status to 100–599 and drops headers whose
// name/value carry CR/LF/control bytes.
type Response struct {
	Status  int
	Headers []Header
	Body    []byte
}

// HTML returns a text/html response (default 200).
func HTML(body string, status ...int) Response {
	return Response{Status: opt(status, 200), Body: []byte(body),
		Headers: []Header{{"content-type", "text/html; charset=utf-8"}}}
}

// JSON returns an application/json response (default 200).
func JSON(body string, status ...int) Response {
	return Response{Status: opt(status, 200), Body: []byte(body),
		Headers: []Header{{"content-type", "application/json"}}}
}

// Text returns a text/plain response (default 200).
func Text(body string, status ...int) Response {
	return Response{Status: opt(status, 200), Body: []byte(body),
		Headers: []Header{{"content-type", "text/plain; charset=utf-8"}}}
}

// Respond returns a response with an arbitrary status, body, and headers.
func Respond(status int, body string, headers ...Header) Response {
	return Response{Status: status, Body: []byte(body), Headers: headers}
}

// Redirect returns a redirect (default 302). It returns an error if the location
// contains CR or LF (response-splitting guard).
func Redirect(location string, status ...int) (Response, error) {
	if strings.ContainsAny(location, "\r\n") {
		return Response{}, fmt.Errorf("redirect location must not contain CR or LF")
	}
	return Response{Status: opt(status, 302), Headers: []Header{{"location", location}}}, nil
}

func opt(s []int, def int) int {
	if len(s) > 0 {
		return s[0]
	}
	return def
}

// toC builds an owned *C.ConduitResponse for handing back to the engine.
func (r Response) toC() *C.ConduitResponse {
	status := r.Status
	if status < 100 {
		status = 100
	} else if status > 599 {
		status = 599
	}
	var bptr *C.uint8_t
	if len(r.Body) > 0 {
		bptr = (*C.uint8_t)(unsafe.Pointer(&r.Body[0]))
	}
	cr := C.conduit_response_new(C.uint16_t(status), bptr, C.size_t(len(r.Body)))
	if cr == nil {
		return nil
	}
	for _, h := range r.Headers {
		cn, cv := C.CString(h.Name), C.CString(h.Value)
		C.conduit_response_set_header(cr, cn, cv)
		C.free(unsafe.Pointer(cn))
		C.free(unsafe.Pointer(cv))
	}
	return cr
}

// responseFromC reads a response back out of a *C.ConduitResponse (for after
// hooks). It does not free p.
func responseFromC(p *C.ConduitResponse) Response {
	var r Response
	r.Status = int(C.conduit_response_status(p))
	var n C.size_t
	if bp := C.conduit_response_body(p, &n); bp != nil && n > 0 {
		r.Body = C.GoBytes(unsafe.Pointer(bp), C.int(n))
	}
	hc := C.conduit_response_header_count(p)
	for i := C.size_t(0); i < hc; i++ {
		hn := C.conduit_response_header_name(p, i)
		hv := C.conduit_response_header_value(p, i)
		if hn != nil && hv != nil {
			r.Headers = append(r.Headers, Header{C.GoString(hn), C.GoString(hv)})
		}
	}
	return r
}

// ── Halt — Sinatra-style non-local exit ──────────────────────────────────────

type haltPanic struct{ resp Response }

// Halt stops handling the current request and returns status/body. It panics
// with an internal value that the dispatch trampoline recovers.
func Halt(status int, body string) {
	panic(haltPanic{Text(body, status)})
}

// ── Request ──────────────────────────────────────────────────────────────────

// Request is a read-only view of the incoming request, valid only inside the
// handler it is passed to.
type Request struct{ ptr *C.ConduitRequest }

func (r *Request) Method() string      { return C.GoString(C.conduit_request_method(r.ptr)) }
func (r *Request) Path() string        { return C.GoString(C.conduit_request_path(r.ptr)) }
func (r *Request) QueryString() string { return C.GoString(C.conduit_request_query_string(r.ptr)) }
func (r *Request) ContentType() string { return C.GoString(C.conduit_request_content_type(r.ptr)) }
func (r *Request) RemoteAddr() string  { return C.GoString(C.conduit_request_remote_addr(r.ptr)) }

// Error is the failure message inside an error handler ("" otherwise).
func (r *Request) Error() string { return C.GoString(C.conduit_request_error(r.ptr)) }

// Body returns the raw request body bytes.
func (r *Request) Body() []byte {
	var n C.size_t
	bp := C.conduit_request_body(r.ptr, &n)
	if bp == nil || n == 0 {
		return nil
	}
	return C.GoBytes(unsafe.Pointer(bp), C.int(n))
}

// BodyString returns the request body as a string.
func (r *Request) BodyString() string { return string(r.Body()) }

// Param returns a named route parameter (:name) and whether it was present.
func (r *Request) Param(name string) (string, bool) {
	cn := C.CString(name)
	defer C.free(unsafe.Pointer(cn))
	return cstrOpt(C.conduit_request_param(r.ptr, cn))
}

// Query returns a query-string value and whether it was present.
func (r *Request) Query(name string) (string, bool) {
	cn := C.CString(name)
	defer C.free(unsafe.Pointer(cn))
	return cstrOpt(C.conduit_request_query(r.ptr, cn))
}

// Header returns a request header (case-insensitive) and whether it was present.
func (r *Request) Header(name string) (string, bool) {
	cn := C.CString(name)
	defer C.free(unsafe.Pointer(cn))
	return cstrOpt(C.conduit_request_header(r.ptr, cn))
}

// cstrOpt converts a possibly-nil C string to (value, present).
func cstrOpt(p *C.char) (string, bool) {
	if p == nil {
		return "", false
	}
	return C.GoString(p), true
}

// ── Trampolines (exported to C; recover panics so none cross the cgo boundary) ─

//export goConduitHandler
func goConduitHandler(ctx unsafe.Pointer, creq *C.ConduitRequest) *C.ConduitResponse {
	fn := cgo.Handle(uintptr(ctx)).Value().(HandlerFunc)
	return runHandler(fn, &Request{ptr: creq})
}

func runHandler(fn HandlerFunc, req *Request) (out *C.ConduitResponse) {
	defer func() {
		if rec := recover(); rec != nil {
			out = recoverDispatch(rec)
		}
	}()
	return fn(req).toC()
}

//export goConduitBefore
func goConduitBefore(ctx unsafe.Pointer, creq *C.ConduitRequest) *C.ConduitResponse {
	fn := cgo.Handle(uintptr(ctx)).Value().(BeforeFunc)
	return runBefore(fn, &Request{ptr: creq})
}

func runBefore(fn BeforeFunc, req *Request) (out *C.ConduitResponse) {
	defer func() {
		if rec := recover(); rec != nil {
			out = recoverDispatch(rec)
		}
	}()
	if resp := fn(req); resp != nil {
		return resp.toC()
	}
	return nil // continue
}

//export goConduitAfter
func goConduitAfter(ctx unsafe.Pointer, creq *C.ConduitRequest, cur *C.ConduitResponse) *C.ConduitResponse {
	fn := cgo.Handle(uintptr(ctx)).Value().(AfterFunc)
	return runAfter(fn, &Request{ptr: creq}, cur)
}

func runAfter(fn AfterFunc, req *Request, cur *C.ConduitResponse) (out *C.ConduitResponse) {
	defer func() {
		if recover() != nil {
			out = C.conduit_response_new(500, nil, 0) // non-allocating fallback
		}
	}()
	// responseFromC must deep-copy all data (body via C.GoBytes, headers via
	// C.GoString) before we free cur. No slice in the returned Response may
	// point into cur's memory — that would be a use-after-free once cur is freed.
	r := responseFromC(cur)
	C.conduit_response_free(cur)
	return fn(req, r).toC()
}

//export goConduitFree
func goConduitFree(ctx unsafe.Pointer) {
	cgo.Handle(uintptr(ctx)).Delete()
}

// recoverDispatch turns a recovered panic (always non-nil; callers guard) into a
// response: a Halt becomes its response; any other value is reported as an error
// (so the engine routes through the error handler) and yields a nil response.
//
// The panic value may originate from user-controlled input (a handler that panics
// with fmt.Sprintf("bad: %s", userInput)). We strip control characters and cap
// length before passing to conduit_capi_report_error to prevent log injection.
func recoverDispatch(rec any) *C.ConduitResponse {
	if h, ok := rec.(haltPanic); ok {
		return h.resp.toC()
	}
	raw := fmt.Sprint(rec)
	safe := strings.Map(func(r rune) rune {
		if r < 0x20 || r == 0x7f {
			return -1
		}
		return r
	}, raw)
	if len(safe) > 512 {
		safe = safe[:512]
	}
	msg := C.CString(safe)
	C.conduit_capi_report_error(msg)
	C.free(unsafe.Pointer(msg))
	return nil
}

// ── Application ──────────────────────────────────────────────────────────────

// Application registers routes and hooks, then Bind returns a Server. Every
// registration method returns the Application so calls chain.
type Application struct {
	app      *C.ConduitApp
	consumed bool
}

// New creates an empty application.
func New() *Application {
	return &Application{app: C.conduit_app_new()}
}

// Route registers a handler for an arbitrary method.
func (a *Application) Route(method, pattern string, h HandlerFunc) *Application {
	handle := cgo.NewHandle(h)
	cm, cp := C.CString(method), C.CString(pattern)
	C.conduitGoAddRoute(a.app, cm, cp, C.uintptr_t(handle))
	C.free(unsafe.Pointer(cm))
	C.free(unsafe.Pointer(cp))
	return a
}

func (a *Application) Get(p string, h HandlerFunc) *Application    { return a.Route("GET", p, h) }
func (a *Application) Post(p string, h HandlerFunc) *Application   { return a.Route("POST", p, h) }
func (a *Application) Put(p string, h HandlerFunc) *Application    { return a.Route("PUT", p, h) }
func (a *Application) Delete(p string, h HandlerFunc) *Application { return a.Route("DELETE", p, h) }
func (a *Application) Patch(p string, h HandlerFunc) *Application  { return a.Route("PATCH", p, h) }

// Before registers a before-filter.
func (a *Application) Before(h BeforeFunc) *Application {
	C.conduitGoAddBefore(a.app, C.uintptr_t(cgo.NewHandle(h)))
	return a
}

// After registers a transforming after-hook.
func (a *Application) After(h AfterFunc) *Application {
	C.conduitGoAddAfter(a.app, C.uintptr_t(cgo.NewHandle(h)))
	return a
}

// NotFound registers a custom not-found handler.
func (a *Application) NotFound(h HandlerFunc) *Application {
	C.conduitGoSetNotFound(a.app, C.uintptr_t(cgo.NewHandle(h)))
	return a
}

// OnError registers a custom error handler (called when a handler panics).
func (a *Application) OnError(h HandlerFunc) *Application {
	C.conduitGoSetError(a.app, C.uintptr_t(cgo.NewHandle(h)))
	return a
}

// Set stores an application setting.
func (a *Application) Set(key, value string) *Application {
	ck, cv := C.CString(key), C.CString(value)
	C.conduit_app_set_setting(a.app, ck, cv)
	C.free(unsafe.Pointer(ck))
	C.free(unsafe.Pointer(cv))
	return a
}

// GetSetting reads an application setting and whether it was present.
func (a *Application) GetSetting(key string) (string, bool) {
	ck := C.CString(key)
	defer C.free(unsafe.Pointer(ck))
	v := C.conduit_app_get_setting(a.app, ck)
	if v == nil {
		return "", false
	}
	defer C.conduit_string_free(v)
	return C.GoString(v), true
}

// Bind binds host:port and returns a Server. It consumes the application (the
// native side moves it into the server), so call it last.
func (a *Application) Bind(host string, port uint16) (*Server, error) {
	ch := C.CString(host)
	defer C.free(unsafe.Pointer(ch))
	srv := C.conduit_server_bind(ch, C.uint16_t(port), a.app)
	a.consumed = true // bind frees the app on success AND failure
	if srv == nil {
		return nil, fmt.Errorf("conduit bind failed: %s", C.GoString(C.conduit_last_error()))
	}
	return &Server{srv: srv}, nil
}

// Free releases an application that was never bound. After Bind, this is a no-op.
func (a *Application) Free() {
	if a.app != nil && !a.consumed {
		C.conduit_app_free(a.app)
		a.app = nil
	}
}

// ── Server ───────────────────────────────────────────────────────────────────

// Server is a bound Conduit server.
type Server struct{ srv *C.ConduitServer }

// Serve runs the request loop in the foreground until stopped (blocks).
func (s *Server) Serve() bool { return C.conduit_server_serve(s.srv) == 0 }

// ServeBackground runs the request loop on a dedicated OS thread.
func (s *Server) ServeBackground() bool { return C.conduit_server_serve_background(s.srv) == 0 }

// Stop stops a running server (and joins its background thread, if any).
func (s *Server) Stop() { C.conduit_server_stop(s.srv) }

// LocalPort is the bound port (useful after binding to port 0).
func (s *Server) LocalPort() uint16 { return uint16(C.conduit_server_local_port(s.srv)) }

// Running reports whether the server is currently running.
func (s *Server) Running() bool { return C.conduit_server_running(s.srv) != 0 }

// Close frees the native server. Safe to call once after you are done with it.
func (s *Server) Close() {
	if s.srv != nil {
		C.conduit_server_free(s.srv)
		s.srv = nil
	}
}
