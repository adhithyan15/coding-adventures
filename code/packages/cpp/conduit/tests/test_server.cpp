// ============================================================================
// End-to-end: build a real Conduit app, serve it on a background thread, and
// drive it with a tiny blocking HTTP/1.0 client over a POSIX socket. A watchdog
// thread stops the server after a deadline so a wedged run fails fast; sockets
// carry a receive timeout for the same reason.
// ============================================================================
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>

#include <atomic>
#include <chrono>
#include <stdexcept>
#include <string>
#include <thread>

#include "conduit/conduit.hpp"
#include "conduit_test.h"

using namespace conduit;

namespace {

struct HttpResult {
    int status = 0;
    std::string body;
    std::vector<std::pair<std::string, std::string>> headers;
    std::string headerValue(const std::string& name) const {
        for (auto& h : headers)
            if (h.first == name) return h.second;
        return "";
    }
};

// One blocking HTTP/1.0 request. Returns false if the connection couldn't be
// made (e.g. the server isn't accepting yet — callers retry).
bool httpRequest(uint16_t port, const std::string& method, const std::string& path,
                 const std::string& body, const std::string& contentType, HttpResult& out) {
    out = HttpResult{};  // reset — callers reuse the same out across requests
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return false;

    timeval tv{5, 0};
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = inet_addr("127.0.0.1");

    if (connect(fd, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) != 0) {
        close(fd);
        return false;
    }

    std::string req = method + " " + path + " HTTP/1.0\r\nHost: 127.0.0.1\r\n";
    if (!body.empty()) {
        req += "Content-Type: " + (contentType.empty() ? std::string("text/plain") : contentType) +
               "\r\nContent-Length: " + std::to_string(body.size()) + "\r\n";
    }
    req += "Connection: close\r\n\r\n" + body;

    if (send(fd, req.data(), req.size(), 0) < 0) {
        close(fd);
        return false;
    }

    std::string raw;
    char buf[4096];
    for (;;) {
        ssize_t n = recv(fd, buf, sizeof(buf), 0);
        if (n <= 0) break;
        raw.append(buf, static_cast<size_t>(n));
    }
    close(fd);

    auto sep = raw.find("\r\n\r\n");
    if (sep == std::string::npos) return false;
    std::string head = raw.substr(0, sep);
    out.body = raw.substr(sep + 4);

    size_t lineEnd = head.find("\r\n");
    std::string statusLine = head.substr(0, lineEnd);
    size_t sp1 = statusLine.find(' ');
    size_t sp2 = statusLine.find(' ', sp1 + 1);
    if (sp1 != std::string::npos) {
        out.status = std::stoi(statusLine.substr(sp1 + 1, sp2 - sp1 - 1));
    }
    size_t pos = (lineEnd == std::string::npos) ? head.size() : lineEnd + 2;
    while (pos < head.size()) {
        size_t eol = head.find("\r\n", pos);
        if (eol == std::string::npos) eol = head.size();
        std::string line = head.substr(pos, eol - pos);
        size_t colon = line.find(": ");
        if (colon != std::string::npos) {
            std::string k = line.substr(0, colon);
            for (auto& c : k) c = static_cast<char>(::tolower(c));
            out.headers.emplace_back(k, line.substr(colon + 2));
        }
        pos = eol + 2;
    }
    return true;
}

// Connect-retry wrapper: the background reactor may need a moment after bind.
bool request(uint16_t port, const std::string& method, const std::string& path,
             HttpResult& out, const std::string& body = "", const std::string& ct = "") {
    for (int i = 0; i < 100; ++i) {
        if (httpRequest(port, method, path, body, ct, out)) return true;
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    return false;
}

}  // namespace

CONDUIT_TEST(full_dispatch) {
    Application app;
    app.set("app_name", "conduit-test");

    app.before([](const Request& req) -> std::optional<Response> {
        if (req.path() == "/down") halt(503, "maintenance");
        return std::nullopt;
    });

    // Transforming after-hook: stamp a header (full response round-trip).
    app.after([](const Request&, Response resp) {
        resp.headers.emplace_back("x-served-by", "conduit-cpp");
        return resp;
    });

    app.get("/", [](const Request&) { return Response::html("<h1>OK</h1>"); });
    app.get("/hello/:name", [](const Request& req) {
        return Response::json("{\"hi\":\"" + req.param("name").value_or("") + "\"}");
    });
    app.post("/echo", [](const Request& req) {
        std::string ct = req.contentType().empty() ? "text/plain" : req.contentType();
        return Response::respond(200, req.body(), {{"content-type", ct}});
    });
    app.get("/q", [](const Request& req) {
        return Response::text("a=" + req.query("a").value_or(""));
    });
    app.get("/boom", [](const Request&) -> Response { throw std::runtime_error("explode"); });
    app.get("/redir", [](const Request&) { return Response::redirect("/", 302); });
    app.notFound([](const Request& req) { return Response::text("no route: " + req.path(), 404); });
    app.onError([](const Request&) { return Response::json("{\"error\":\"server\"}", 500); });

    Server server = app.bind("127.0.0.1", 0);
    CONDUIT_ASSERT(server.serveBackground());
    uint16_t port = server.localPort();
    CONDUIT_ASSERT(port > 0);

    // Watchdog: force the server down after a deadline so nothing hangs.
    std::atomic<bool> done{false};
    std::thread watchdog([&] {
        for (int i = 0; i < 300 && !done.load(); ++i)
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
        if (!done.load()) server.stop();
    });
    // Always tear the watchdog down — even if an assertion throws — so the
    // joinable std::thread isn't destroyed mid-flight (which would std::terminate
    // and mask the real failure).
    auto cleanup = [&] {
        done.store(true);
        server.stop();
        if (watchdog.joinable()) watchdog.join();
    };

    try {
        HttpResult r;

        CONDUIT_ASSERT(request(port, "GET", "/", r));
        CONDUIT_ASSERT_EQ(r.status, 200);
        CONDUIT_ASSERT_EQ(r.body, std::string("<h1>OK</h1>"));
        CONDUIT_ASSERT_EQ(r.headerValue("x-served-by"), std::string("conduit-cpp"));

        CONDUIT_ASSERT(request(port, "GET", "/hello/world", r));
        CONDUIT_ASSERT_EQ(r.status, 200);
        CONDUIT_ASSERT_EQ(r.body, std::string("{\"hi\":\"world\"}"));

        CONDUIT_ASSERT(request(port, "POST", "/echo", r, "ping-pong", "application/octet-stream"));
        CONDUIT_ASSERT_EQ(r.status, 200);
        CONDUIT_ASSERT_EQ(r.body, std::string("ping-pong"));
        CONDUIT_ASSERT(r.headerValue("content-type").find("octet-stream") != std::string::npos);

        CONDUIT_ASSERT(request(port, "GET", "/q?a=42", r));
        CONDUIT_ASSERT_EQ(r.body, std::string("a=42"));

        CONDUIT_ASSERT(request(port, "GET", "/down", r));
        CONDUIT_ASSERT_EQ(r.status, 503);
        CONDUIT_ASSERT_EQ(r.body, std::string("maintenance"));

        CONDUIT_ASSERT(request(port, "GET", "/boom", r));
        CONDUIT_ASSERT_EQ(r.status, 500);
        CONDUIT_ASSERT_EQ(r.body, std::string("{\"error\":\"server\"}"));

        CONDUIT_ASSERT(request(port, "GET", "/nope", r));
        CONDUIT_ASSERT_EQ(r.status, 404);
        CONDUIT_ASSERT_EQ(r.body, std::string("no route: /nope"));

        CONDUIT_ASSERT(request(port, "GET", "/redir", r));
        CONDUIT_ASSERT_EQ(r.status, 302);
        CONDUIT_ASSERT_EQ(r.headerValue("location"), std::string("/"));
    } catch (...) {
        cleanup();
        throw;
    }
    cleanup();
}

CONDUIT_MAIN()
