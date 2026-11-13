// net_compat.h — the loopback-socket calls the end-to-end tests need, spelled
// once for both BSD sockets (Linux, macOS) and Winsock (Windows).
//
// WHY THIS EXISTS
// ---------------
// The E2E tests drive a real Conduit server with a hand-rolled HTTP/1.0 client.
// On Unix that client is plain BSD sockets: <sys/socket.h>, <arpa/inet.h>,
// close(), and an `int` file descriptor. Windows has none of those headers.
// It has Winsock instead, and the differences are not just spelling:
//
//     Unix (BSD sockets)               Windows (Winsock 2)
//     -------------------------------  ----------------------------------------
//     <sys/socket.h> <arpa/inet.h>     <winsock2.h> <ws2tcpip.h>
//     socket() returns int, -1 = fail  socket() returns SOCKET (an unsigned
//                                      handle), INVALID_SOCKET = fail
//     close(fd)                        closesocket(s)   (close() is for CRT fds)
//     SO_RCVTIMEO takes a `timeval`    SO_RCVTIMEO takes a DWORD of MILLISECONDS
//     setsockopt(.., const void*)      setsockopt(.., const char*)
//     send/recv take size_t, give      send/recv take int, give int
//       ssize_t
//     nothing to initialise            WSAStartup() before the first socket call
//                                      and WSACleanup() after the last
//
// The receive timeout is the sharp one: passing Windows a `timeval` is accepted
// silently and read as a DWORD, i.e. a timeout of "5 milliseconds" (the low word
// of the struct is tv_sec = 5), which would make every request flaky. So the
// timeout goes through set_recv_timeout_seconds() and nothing else.
//
// Everything below is a thin inline wrapper. On Unix each one collapses to the
// exact system call the tests made before this header existed.
//
// This header lives in tests/ (not include/) because it is test scaffolding,
// not part of the Conduit API.

#ifndef CONDUIT_NET_COMPAT_H
#define CONDUIT_NET_COMPAT_H

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>

#ifdef _WIN32
// winsock2.h must come before anything that pulls in <windows.h>, otherwise the
// old winsock 1 declarations are included first and clash with it.
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <unistd.h>
#endif

namespace netcompat {

#ifdef _WIN32
using Socket = SOCKET;
constexpr Socket kInvalidSocket = INVALID_SOCKET;
#else
using Socket = int;
constexpr Socket kInvalidSocket = -1;
#endif

#ifdef _WIN32
// Winsock must be started once per process before any socket call. The static
// local in ensure_started() makes that "once" lazy and thread-safe, and its
// destructor runs WSACleanup() when the process exits. (The Rust side keeps its
// own WSAStartup/WSACleanup count: Winsock reference-counts them, so the two
// do not interfere.)
struct WinsockSession {
    WinsockSession() {
        WSADATA data;
        const int rc = WSAStartup(MAKEWORD(2, 2), &data);
        if (rc != 0) {
            throw std::runtime_error("WSAStartup failed with code " + std::to_string(rc));
        }
    }
    ~WinsockSession() { WSACleanup(); }
    WinsockSession(const WinsockSession&) = delete;
    WinsockSession& operator=(const WinsockSession&) = delete;
};
#endif

// Make sure the socket layer is usable. A no-op on Unix.
inline void ensure_started() {
#ifdef _WIN32
    static const WinsockSession session;
    (void)session;
#endif
}

// A new IPv4 TCP socket, or kInvalidSocket.
inline Socket open_tcp() {
    ensure_started();
    return ::socket(AF_INET, SOCK_STREAM, 0);
}

inline void close_socket(Socket s) {
#ifdef _WIN32
    ::closesocket(s);
#else
    ::close(s);
#endif
}

// Bound `recv` to `seconds`, so a wedged server fails the test instead of
// hanging it. See the SO_RCVTIMEO row in the table at the top of this file.
inline void set_recv_timeout_seconds(Socket s, int seconds) {
#ifdef _WIN32
    const DWORD millis = static_cast<DWORD>(seconds) * 1000U;
    ::setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<const char*>(&millis),
                 static_cast<int>(sizeof(millis)));
#else
    timeval tv{};
    tv.tv_sec = seconds;
    tv.tv_usec = 0;
    ::setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
#endif
}

// Connect `s` to 127.0.0.1:`port`. Returns true on success.
inline bool connect_loopback(Socket s, std::uint16_t port) {
    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    // inet_pton (not the legacy inet_addr, which MSVC flags as deprecated).
    if (::inet_pton(AF_INET, "127.0.0.1", &addr.sin_addr) != 1) return false;
    return ::connect(s, reinterpret_cast<const sockaddr*>(&addr),
                     static_cast<int>(sizeof(addr))) == 0;
}

// send()/recv() with one signature on both platforms. Both return the byte
// count, or a negative number on error (recv: 0 means the peer closed).
inline long long send_bytes(Socket s, const char* data, std::size_t len) {
#ifdef _WIN32
    return ::send(s, data, static_cast<int>(len), 0);
#else
    return ::send(s, data, len, 0);
#endif
}

inline long long recv_bytes(Socket s, char* buf, std::size_t len) {
#ifdef _WIN32
    return ::recv(s, buf, static_cast<int>(len), 0);
#else
    return ::recv(s, buf, len, 0);
#endif
}

}  // namespace netcompat

#endif  // CONDUIT_NET_COMPAT_H
