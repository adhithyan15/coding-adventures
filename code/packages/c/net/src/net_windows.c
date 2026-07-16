/*
 * net_windows.c — the Winsock2 backend of net/tcp.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS/Linux use net_posix.c via
 * the shared `BUILD`). No OS #ifdefs — the build chose this file. Links ws2_32.
 *
 * Winsock mirrors BSD sockets closely, with a few Windows-isms this file hides:
 *   - the library must be started (WSAStartup) and stopped (WSACleanup);
 *   - a socket is a SOCKET handle (not an int fd), and failure is INVALID_SOCKET
 *     / SOCKET_ERROR rather than a negative int;
 *   - send/recv take an `int` length and a `char *` buffer, so sizes are clamped
 *     to INT_MAX per call;
 *   - the socket is closed with closesocket, not close.
 * There is no SIGPIPE on Windows, so the POSIX MSG_NOSIGNAL / SO_NOSIGPIPE
 * handling has no counterpart here.
 */
#include "net/tcp.h"

#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <ws2tcpip.h> /* inet_pton */

#include <limits.h> /* INT_MAX */
#include <stdlib.h>

struct osp_socket {
    SOCKET sock;
};

static osp_status osp__make_addr(const char *host, unsigned short port,
                                 struct sockaddr_in *addr) {
    ZeroMemory(addr, sizeof(*addr));
    addr->sin_family = AF_INET;
    addr->sin_port = htons(port);
    if (inet_pton(AF_INET, host, &addr->sin_addr) != 1) {
        return OSP_ERR_INVAL;
    }
    return OSP_OK;
}

static osp_status osp__wrap(SOCKET sock, osp_socket **out) {
    struct osp_socket *s = (struct osp_socket *)malloc(sizeof(*s));
    if (s == NULL) {
        closesocket(sock);
        return OSP_ERR_NOMEM;
    }
    s->sock = sock;
    *out = s;
    return OSP_OK;
}

osp_status osp_net_init(void) {
    WSADATA wsa;
    if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) {
        return OSP_ERR_OS;
    }
    return OSP_OK;
}

osp_status osp_net_shutdown(void) {
    if (WSACleanup() != 0) {
        return OSP_ERR_OS;
    }
    return OSP_OK;
}

osp_status osp_tcp_listen(osp_socket **out, const char *host,
                          unsigned short port, int backlog) {
    struct sockaddr_in addr;
    SOCKET sock;
    BOOL yes = TRUE;
    osp_status st;

    if (out == NULL || host == NULL) {
        return OSP_ERR_INVAL;
    }
    st = osp__make_addr(host, port, &addr);
    if (st != OSP_OK) {
        return st;
    }
    sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (sock == INVALID_SOCKET) {
        return OSP_ERR_OS;
    }
    (void)setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, (const char *)&yes,
                     sizeof(yes));
    if (bind(sock, (struct sockaddr *)&addr, sizeof(addr)) == SOCKET_ERROR) {
        closesocket(sock);
        return OSP_ERR_OS;
    }
    if (listen(sock, backlog) == SOCKET_ERROR) {
        closesocket(sock);
        return OSP_ERR_OS;
    }
    return osp__wrap(sock, out);
}

osp_status osp_tcp_local_port(osp_socket *s, unsigned short *out_port) {
    struct sockaddr_in addr;
    int len = (int)sizeof(addr);

    if (s == NULL || out_port == NULL) {
        return OSP_ERR_INVAL;
    }
    if (getsockname(s->sock, (struct sockaddr *)&addr, &len) == SOCKET_ERROR) {
        return OSP_ERR_OS;
    }
    *out_port = ntohs(addr.sin_port);
    return OSP_OK;
}

osp_status osp_tcp_accept(osp_socket *listener, osp_socket **out_conn) {
    SOCKET sock;

    if (listener == NULL || out_conn == NULL) {
        return OSP_ERR_INVAL;
    }
    sock = accept(listener->sock, NULL, NULL);
    if (sock == INVALID_SOCKET) {
        return OSP_ERR_OS;
    }
    return osp__wrap(sock, out_conn);
}

osp_status osp_tcp_connect(osp_socket **out, const char *host,
                           unsigned short port) {
    struct sockaddr_in addr;
    SOCKET sock;
    osp_status st;

    if (out == NULL || host == NULL) {
        return OSP_ERR_INVAL;
    }
    st = osp__make_addr(host, port, &addr);
    if (st != OSP_OK) {
        return st;
    }
    sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (sock == INVALID_SOCKET) {
        return OSP_ERR_OS;
    }
    if (connect(sock, (struct sockaddr *)&addr, sizeof(addr)) == SOCKET_ERROR) {
        closesocket(sock);
        return OSP_ERR_OS;
    }
    return osp__wrap(sock, out);
}

osp_status osp_socket_send(osp_socket *s, const void *buf, size_t len,
                           size_t *out_n) {
    const char *p = (const char *)buf;
    size_t off = 0;

    if (s == NULL || (buf == NULL && len > 0)) {
        return OSP_ERR_INVAL;
    }
    while (off < len) {
        size_t remain = len - off;
        int chunk = (remain > (size_t)INT_MAX) ? INT_MAX : (int)remain;
        int n = send(s->sock, p + off, chunk, 0);
        if (n == SOCKET_ERROR) {
            return OSP_ERR_OS;
        }
        off += (size_t)n;
    }
    if (out_n != NULL) {
        *out_n = off;
    }
    return OSP_OK;
}

osp_status osp_socket_recv(osp_socket *s, void *buf, size_t len, size_t *out_n) {
    int chunk;
    int n;

    if (s == NULL || (buf == NULL && len > 0) || out_n == NULL) {
        return OSP_ERR_INVAL;
    }
    chunk = (len > (size_t)INT_MAX) ? INT_MAX : (int)len;
    n = recv(s->sock, (char *)buf, chunk, 0);
    if (n == SOCKET_ERROR) {
        return OSP_ERR_OS;
    }
    *out_n = (size_t)n; /* 0 => peer closed */
    return OSP_OK;
}

osp_status osp_socket_close(osp_socket *s) {
    osp_status st = OSP_OK;
    if (s == NULL) {
        return OSP_ERR_INVAL;
    }
    if (closesocket(s->sock) == SOCKET_ERROR) {
        st = OSP_ERR_OS;
    }
    free(s);
    return st;
}

osp_status osp_socket_fd(const osp_socket *s, uintptr_t *out) {
    if (s == NULL || out == NULL) {
        return OSP_ERR_INVAL;
    }
    *out = (uintptr_t)s->sock;
    return OSP_OK;
}
