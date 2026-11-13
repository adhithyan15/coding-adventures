// Smoke test: launch the real demo app on an OS-assigned port and hit a few
// routes. Loopback-socket client (BSD sockets on Unix, Winsock on Windows — see
// net_compat.h) + watchdog thread.
#include <atomic>
#include <chrono>
#include <iostream>
#include <string>
#include <thread>

#include "app.hpp"
#include "net_compat.h"

namespace {

struct Result {
    int status = 0;
    std::string body;
};

bool get(uint16_t port, const std::string& path, Result& out) {
    out = Result{};
    netcompat::Socket fd = netcompat::open_tcp();
    if (fd == netcompat::kInvalidSocket) return false;
    netcompat::set_recv_timeout_seconds(fd, 5);
    if (!netcompat::connect_loopback(fd, port)) {
        netcompat::close_socket(fd);
        return false;
    }
    std::string req = "GET " + path + " HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if (netcompat::send_bytes(fd, req.data(), req.size()) < 0) {
        netcompat::close_socket(fd);
        return false;
    }
    std::string raw;
    char buf[4096];
    long long n;
    while ((n = netcompat::recv_bytes(fd, buf, sizeof(buf))) > 0)
        raw.append(buf, static_cast<size_t>(n));
    netcompat::close_socket(fd);
    auto sep = raw.find("\r\n\r\n");
    if (sep == std::string::npos) return false;
    out.body = raw.substr(sep + 4);
    size_t sp1 = raw.find(' ');
    size_t sp2 = raw.find(' ', sp1 + 1);
    if (sp1 != std::string::npos) out.status = std::stoi(raw.substr(sp1 + 1, sp2 - sp1 - 1));
    return true;
}

bool request(uint16_t port, const std::string& path, Result& out) {
    for (int i = 0; i < 100; ++i) {
        if (get(port, path, out)) return true;
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    return false;
}

int failures = 0;
void check(bool cond, const std::string& what) {
    if (cond) {
        std::cout << "[PASS] " << what << '\n';
    } else {
        std::cerr << "[FAIL] " << what << '\n';
        ++failures;
    }
}

}  // namespace

int main() {
    conduit::Server server = make_app().bind("127.0.0.1", 0);
    if (!server.serveBackground()) {
        std::cerr << "failed to start server\n";
        return 1;
    }
    uint16_t port = server.localPort();

    std::atomic<bool> done{false};
    std::thread watchdog([&] {
        for (int i = 0; i < 300 && !done.load(); ++i)
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
        if (!done.load()) server.stop();
    });

    Result r;
    check(request(port, "/", r) && r.status == 200, "GET / -> 200");
    check(r.body.find("Hello from Conduit") != std::string::npos, "greeting body");
    check(request(port, "/hello/Ada", r) && r.status == 200, "GET /hello/:name -> 200");
    check(r.body.find("Hello Ada") != std::string::npos, "route param interpolated");
    check(request(port, "/nope", r) && r.status == 404, "unknown route -> 404");

    done.store(true);
    server.stop();
    watchdog.join();

    std::cout << failures << " failures\n";
    return failures == 0 ? 0 : 1;
}
