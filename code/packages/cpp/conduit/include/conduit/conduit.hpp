// ============================================================================
// conduit.hpp — a Sinatra/Express-style web framework for C++.
// ============================================================================
//
// A header-only C++ wrapper over the reusable `conduit-capi` C ABI (which
// exposes the Rust web-core engine / WEB08 facade). C++ handlers are
// std::function closures; routing, lifecycle hooks, and HTTP I/O run in Rust.
//
//   #include "conduit/conduit.hpp"
//   using namespace conduit;
//
//   Application app;
//   app.before([](const Request& req) -> std::optional<Response> {
//       if (req.path() == "/down") halt(503, "maintenance");
//       return std::nullopt;
//   });
//   app.get("/", [](const Request&) { return Response::html("<h1>Hi</h1>"); });
//   app.get("/hello/:name", [](const Request& req) {
//       return Response::json("{\"hi\":\"" + req.param("name").value_or("") + "\"}");
//   });
//   Server server = app.bind("127.0.0.1", 3000);
//   server.serve();   // blocks until stopped
//
// Link against libconduit_capi.a and add the C ABI's include dir to the path.

#ifndef CONDUIT_HPP
#define CONDUIT_HPP

#include <cstdint>
#include <functional>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "conduit_capi.h"

namespace conduit {

using Header = std::pair<std::string, std::string>;

// ── Response ─────────────────────────────────────────────────────────────────

/// An HTTP response: status, ordered headers, body. Build directly or with the
/// Sinatra-style helpers. The native side clamps the status to 100–599 and drops
/// any header whose name/value carries CR/LF/control bytes.
class Response {
public:
    int status = 200;
    std::vector<Header> headers;
    std::string body;

    Response() = default;
    Response(int s, std::string b, std::vector<Header> h = {})
        : status(s), headers(std::move(h)), body(std::move(b)) {}

    static Response html(std::string body, int status = 200) {
        return Response(status, std::move(body), {{"content-type", "text/html; charset=utf-8"}});
    }
    static Response json(std::string body, int status = 200) {
        return Response(status, std::move(body), {{"content-type", "application/json"}});
    }
    static Response text(std::string body, int status = 200) {
        return Response(status, std::move(body), {{"content-type", "text/plain; charset=utf-8"}});
    }
    static Response respond(int status, std::string body = "", std::vector<Header> headers = {}) {
        return Response(status, std::move(body), std::move(headers));
    }
    /// A redirect (default 302). Throws std::invalid_argument if the location
    /// contains CR or LF (response-splitting guard).
    static Response redirect(const std::string& location, int status = 302) {
        for (char c : location) {
            if (c == '\r' || c == '\n') {
                throw std::invalid_argument("redirect location must not contain CR or LF");
            }
        }
        return Response(status, "", {{"location", location}});
    }

    /// Build an owned ConduitResponse* for handing back to the engine.
    ConduitResponse* toC() const {
        int clamped = status < 100 ? 100 : (status > 599 ? 599 : status);
        const uint8_t* bptr =
            body.empty() ? nullptr : reinterpret_cast<const uint8_t*>(body.data());
        ConduitResponse* r =
            conduit_response_new(static_cast<uint16_t>(clamped), bptr, body.size());
        if (!r) return nullptr;
        for (const auto& h : headers) {
            conduit_response_set_header(r, h.first.c_str(), h.second.c_str());
        }
        return r;
    }

    /// Read a response back out of a ConduitResponse* (used by after-hooks).
    /// Does not free `p`.
    static Response fromC(const ConduitResponse* p) {
        Response r;
        r.status = conduit_response_status(p);
        size_t len = 0;
        const uint8_t* b = conduit_response_body(p, &len);
        if (b && len) r.body.assign(reinterpret_cast<const char*>(b), len);
        size_t n = conduit_response_header_count(p);
        for (size_t i = 0; i < n; ++i) {
            const char* hn = conduit_response_header_name(p, i);
            const char* hv = conduit_response_header_value(p, i);
            if (hn && hv) r.headers.emplace_back(hn, hv);
        }
        return r;
    }
};

// ── Halt — Sinatra-style non-local exit ──────────────────────────────────────

struct Halt {
    Response response;
    Halt(int status, std::string body = "") : response(Response::text(std::move(body), status)) {}
    explicit Halt(Response r) : response(std::move(r)) {}
};

/// Immediately stop handling the current request and return status/body.
[[noreturn]] inline void halt(int status, std::string body = "") {
    throw Halt(status, std::move(body));
}

// ── Request ──────────────────────────────────────────────────────────────────

/// A read-only view of the request, valid only inside the handler.
class Request {
public:
    explicit Request(const ConduitRequest* p) : p_(p) {}

    std::string method() const { return str(conduit_request_method(p_)); }
    std::string path() const { return str(conduit_request_path(p_)); }
    std::string queryString() const { return str(conduit_request_query_string(p_)); }
    std::string contentType() const { return str(conduit_request_content_type(p_)); }
    std::string remoteAddr() const { return str(conduit_request_remote_addr(p_)); }
    std::string error() const { return str(conduit_request_error(p_)); }

    std::string body() const {
        size_t len = 0;
        const uint8_t* b = conduit_request_body(p_, &len);
        return (b && len) ? std::string(reinterpret_cast<const char*>(b), len) : std::string();
    }

    std::optional<std::string> param(const std::string& name) const {
        return opt(conduit_request_param(p_, name.c_str()));
    }
    std::optional<std::string> query(const std::string& name) const {
        return opt(conduit_request_query(p_, name.c_str()));
    }
    std::optional<std::string> header(const std::string& name) const {
        return opt(conduit_request_header(p_, name.c_str()));
    }

private:
    const ConduitRequest* p_;
    static std::string str(const char* s) { return s ? std::string(s) : std::string(); }
    static std::optional<std::string> opt(const char* s) {
        return s ? std::optional<std::string>(s) : std::nullopt;
    }
};

using Handler = std::function<Response(const Request&)>;
using BeforeHandler = std::function<std::optional<Response>(const Request&)>;
using AfterHandler = std::function<Response(const Request&, Response)>;

// ── Trampolines (C linkage so function-pointer types match the C ABI exactly;
//    inline so the header stays single-include-safe across translation units) ──

extern "C" {

inline ConduitResponse* conduit_cpp_handler_tramp(void* ctx, const ConduitRequest* req) {
    auto* fn = static_cast<Handler*>(ctx);
    Request r(req);
    try {
        return (*fn)(r).toC();
    } catch (const Halt& h) {
        return h.response.toC();
    } catch (const std::exception& e) {
        conduit_capi_report_error(e.what());
        return nullptr;
    } catch (...) {
        conduit_capi_report_error("unknown error");
        return nullptr;
    }
}

inline ConduitResponse* conduit_cpp_before_tramp(void* ctx, const ConduitRequest* req) {
    auto* fn = static_cast<BeforeHandler*>(ctx);
    Request r(req);
    try {
        std::optional<Response> resp = (*fn)(r);
        return resp ? resp->toC() : nullptr;  // nullptr = continue
    } catch (const Halt& h) {
        return h.response.toC();
    } catch (const std::exception& e) {
        conduit_capi_report_error(e.what());
        return nullptr;
    } catch (...) {
        conduit_capi_report_error("unknown error");
        return nullptr;
    }
}

inline ConduitResponse* conduit_cpp_after_tramp(void* ctx, const ConduitRequest* req,
                                                ConduitResponse* current) {
    auto* fn = static_cast<AfterHandler*>(ctx);
    // Everything that can allocate (fromC, the user hook, toC's header loop) must
    // be inside the try so no C++ exception unwinds across this extern "C" frame.
    // `current` is freed exactly once: on the happy path after reading it, or in
    // the catch if fromC threw before we got there.
    try {
        Request r(req);
        Response cur = Response::fromC(current);  // may throw (allocates)
        conduit_response_free(current);
        current = nullptr;
        return (*fn)(r, std::move(cur)).toC();
    } catch (...) {
        if (current) conduit_response_free(current);
        // Build the fallback with a single C call — no C++ allocation that could
        // itself throw out of this handler under OOM.
        return conduit_response_new(500, nullptr, 0);
    }
}

inline void conduit_cpp_free_handler(void* ctx) { delete static_cast<Handler*>(ctx); }
inline void conduit_cpp_free_before(void* ctx) { delete static_cast<BeforeHandler*>(ctx); }
inline void conduit_cpp_free_after(void* ctx) { delete static_cast<AfterHandler*>(ctx); }

}  // extern "C"

// ── Server ───────────────────────────────────────────────────────────────────

class Server {
public:
    explicit Server(ConduitServer* s) : s_(s) {}
    ~Server() { if (s_) conduit_server_free(s_); }

    Server(Server&& o) noexcept : s_(o.s_) { o.s_ = nullptr; }
    Server& operator=(Server&& o) noexcept {
        if (this != &o) {
            if (s_) conduit_server_free(s_);
            s_ = o.s_;
            o.s_ = nullptr;
        }
        return *this;
    }
    Server(const Server&) = delete;
    Server& operator=(const Server&) = delete;

    /// Serve in the foreground until stopped (blocks). Returns false on failure.
    bool serve() { return s_ && conduit_server_serve(s_) == 0; }
    /// Serve on a dedicated OS thread. Returns false if it could not start.
    bool serveBackground() { return s_ && conduit_server_serve_background(s_) == 0; }
    void stop() { if (s_) conduit_server_stop(s_); }
    uint16_t localPort() const { return s_ ? conduit_server_local_port(s_) : 0; }
    bool running() const { return s_ && conduit_server_running(s_) != 0; }

private:
    ConduitServer* s_;
};

// ── Application ──────────────────────────────────────────────────────────────

/// Register routes and hooks, then bind() to get a Server. Registration methods
/// return *this so calls chain. Handlers are std::function closures.
class Application {
public:
    Application() : app_(conduit_app_new()) {}
    ~Application() { if (app_ && !consumed_) conduit_app_free(app_); }

    Application(const Application&) = delete;
    Application& operator=(const Application&) = delete;

    // Movable so an Application can be returned by value (e.g. a make_app()
    // factory). The moved-from object is left empty/consumed.
    Application(Application&& o) noexcept : app_(o.app_), consumed_(o.consumed_) {
        o.app_ = nullptr;
        o.consumed_ = true;
    }
    Application& operator=(Application&& o) noexcept {
        if (this != &o) {
            if (app_ && !consumed_) conduit_app_free(app_);
            app_ = o.app_;
            consumed_ = o.consumed_;
            o.app_ = nullptr;
            o.consumed_ = true;
        }
        return *this;
    }

    Application& route(const std::string& method, const std::string& pattern, Handler h) {
        conduit_app_add_route(app_, method.c_str(), pattern.c_str(), &conduit_cpp_handler_tramp,
                              new Handler(std::move(h)), &conduit_cpp_free_handler);
        return *this;
    }
    Application& get(const std::string& p, Handler h) { return route("GET", p, std::move(h)); }
    Application& post(const std::string& p, Handler h) { return route("POST", p, std::move(h)); }
    Application& put(const std::string& p, Handler h) { return route("PUT", p, std::move(h)); }
    /// DELETE route (named `del` because `delete` is a keyword).
    Application& del(const std::string& p, Handler h) { return route("DELETE", p, std::move(h)); }
    Application& patch(const std::string& p, Handler h) { return route("PATCH", p, std::move(h)); }

    /// Before-filter: return a Response to short-circuit, std::nullopt to continue
    /// (halt(...) short-circuits too).
    Application& before(BeforeHandler h) {
        conduit_app_add_before(app_, &conduit_cpp_before_tramp,
                               new BeforeHandler(std::move(h)), &conduit_cpp_free_before);
        return *this;
    }
    /// Transforming after-hook: receives the request and current response, returns
    /// the response to send (return it unchanged to merely observe).
    Application& after(AfterHandler h) {
        conduit_app_add_after(app_, &conduit_cpp_after_tramp,
                              new AfterHandler(std::move(h)), &conduit_cpp_free_after);
        return *this;
    }
    Application& notFound(Handler h) {
        conduit_app_set_not_found(app_, &conduit_cpp_handler_tramp,
                                  new Handler(std::move(h)), &conduit_cpp_free_handler);
        return *this;
    }
    Application& onError(Handler h) {
        conduit_app_set_error_handler(app_, &conduit_cpp_handler_tramp,
                                      new Handler(std::move(h)), &conduit_cpp_free_handler);
        return *this;
    }

    Application& set(const std::string& key, const std::string& value) {
        conduit_app_set_setting(app_, key.c_str(), value.c_str());
        return *this;
    }
    std::optional<std::string> getSetting(const std::string& key) {
        char* v = conduit_app_get_setting(app_, key.c_str());
        if (!v) return std::nullopt;
        std::string s(v);
        conduit_string_free(v);
        return s;
    }

    /// Bind host:port and return a Server. Consumes the application (the native
    /// side moves it into the server), so call this last. Throws on bind failure.
    Server bind(const std::string& host = "127.0.0.1", uint16_t port = 3000) {
        ConduitServer* s = conduit_server_bind(host.c_str(), port, app_);
        consumed_ = true;  // bind frees the app on success AND failure
        if (!s) {
            throw std::runtime_error(std::string("conduit bind failed: ") + conduit_last_error());
        }
        return Server(s);
    }

private:
    ConduitApp* app_;
    bool consumed_ = false;
};

}  // namespace conduit

#endif  // CONDUIT_HPP
