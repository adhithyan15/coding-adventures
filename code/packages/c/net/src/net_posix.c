/*
 * net_posix.c — the POSIX (BSD sockets) backend of net/tcp (macOS + Linux).
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * net_windows.c via `BUILD_windows`). No OS #ifdefs for behaviour — the build
 * chose this file. The only #ifdefs here are FEATURE-presence checks (not OS
 * names) for the two spellings of "don't raise SIGPIPE on a dead peer":
 * Linux offers the MSG_NOSIGNAL send flag, macOS the SO_NOSIGPIPE socket option.
 * Handling both means a send to a closed connection returns an error instead of
 * killing the process — a latent-crash guard even though the echo test never
 * trips it.
 */
#include "net/tcp.h"

#include <arpa/inet.h>
#include <errno.h>
#include <netinet/in.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

/* If the platform lacks the MSG_NOSIGNAL send flag (macOS), fall back to 0 and
 * rely on the SO_NOSIGPIPE socket option set at creation time instead. */
#ifndef MSG_NOSIGNAL
#define MSG_NOSIGNAL 0
#endif

struct osp_socket {
    int fd;
};

/* Suppress SIGPIPE on this socket where the OS uses a per-socket option. */
static void osp__suppress_sigpipe(int fd) {
#ifdef SO_NOSIGPIPE
    int on = 1;
    (void)setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &on, sizeof(on));
#else
    (void)fd; /* Linux uses the MSG_NOSIGNAL send flag instead. */
#endif
}

/* Fill a sockaddr_in from a numeric IPv4 string + port. */
static osp_status osp__make_addr(const char *host, unsigned short port,
                                 struct sockaddr_in *addr) {
    memset(addr, 0, sizeof(*addr));
    addr->sin_family = AF_INET;
    addr->sin_port = htons(port);
    if (inet_pton(AF_INET, host, &addr->sin_addr) != 1) {
        return OSP_ERR_INVAL; /* not a valid dotted-quad */
    }
    return OSP_OK;
}

/* Wrap an fd in a heap handle; close the fd and fail if allocation fails. */
static osp_status osp__wrap(int fd, osp_socket **out) {
    struct osp_socket *s = (struct osp_socket *)malloc(sizeof(*s));
    if (s == NULL) {
        close(fd);
        return OSP_ERR_NOMEM;
    }
    s->fd = fd;
    *out = s;
    return OSP_OK;
}

osp_status osp_net_init(void) {
    return OSP_OK; /* nothing to do on POSIX */
}

osp_status osp_net_shutdown(void) {
    return OSP_OK;
}

osp_status osp_tcp_listen(osp_socket **out, const char *host,
                          unsigned short port, int backlog) {
    struct sockaddr_in addr;
    int fd;
    int yes = 1;
    osp_status st;

    if (out == NULL || host == NULL) {
        return OSP_ERR_INVAL;
    }
    st = osp__make_addr(host, port, &addr);
    if (st != OSP_OK) {
        return st;
    }
    fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return OSP_ERR_OS;
    }
    osp__suppress_sigpipe(fd);
    /* SO_REUSEADDR lets the test rebind the port immediately after a prior run;
     * a failure here is non-fatal. */
    (void)setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));
    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) != 0) {
        close(fd);
        return OSP_ERR_OS;
    }
    if (listen(fd, backlog) != 0) {
        close(fd);
        return OSP_ERR_OS;
    }
    return osp__wrap(fd, out);
}

osp_status osp_tcp_local_port(osp_socket *s, unsigned short *out_port) {
    struct sockaddr_in addr;
    socklen_t len = sizeof(addr);

    if (s == NULL || out_port == NULL) {
        return OSP_ERR_INVAL;
    }
    if (getsockname(s->fd, (struct sockaddr *)&addr, &len) != 0) {
        return OSP_ERR_OS;
    }
    *out_port = ntohs(addr.sin_port);
    return OSP_OK;
}

osp_status osp_tcp_accept(osp_socket *listener, osp_socket **out_conn) {
    int fd;

    if (listener == NULL || out_conn == NULL) {
        return OSP_ERR_INVAL;
    }
    do {
        fd = accept(listener->fd, NULL, NULL);
    } while (fd < 0 && errno == EINTR);
    if (fd < 0) {
        return OSP_ERR_OS;
    }
    osp__suppress_sigpipe(fd);
    return osp__wrap(fd, out_conn);
}

osp_status osp_tcp_connect(osp_socket **out, const char *host,
                           unsigned short port) {
    struct sockaddr_in addr;
    int fd;
    osp_status st;

    if (out == NULL || host == NULL) {
        return OSP_ERR_INVAL;
    }
    st = osp__make_addr(host, port, &addr);
    if (st != OSP_OK) {
        return st;
    }
    fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return OSP_ERR_OS;
    }
    osp__suppress_sigpipe(fd);
    /* Blocking connect; on loopback the handshake completes before it returns. */
    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) != 0) {
        close(fd);
        return OSP_ERR_OS;
    }
    return osp__wrap(fd, out);
}

osp_status osp_socket_send(osp_socket *s, const void *buf, size_t len,
                           size_t *out_n) {
    const char *p = (const char *)buf;
    size_t off = 0;

    if (s == NULL || (buf == NULL && len > 0)) {
        return OSP_ERR_INVAL;
    }
    while (off < len) {
        ssize_t n = send(s->fd, p + off, len - off, MSG_NOSIGNAL);
        if (n < 0) {
            if (errno == EINTR) {
                continue;
            }
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
    ssize_t n;

    if (s == NULL || (buf == NULL && len > 0) || out_n == NULL) {
        return OSP_ERR_INVAL;
    }
    do {
        n = recv(s->fd, (char *)buf, len, 0);
    } while (n < 0 && errno == EINTR);
    if (n < 0) {
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
    if (close(s->fd) != 0) {
        st = OSP_ERR_OS;
    }
    free(s);
    return st;
}

osp_status osp_socket_fd(const osp_socket *s, uintptr_t *out) {
    if (s == NULL || out == NULL) {
        return OSP_ERR_INVAL;
    }
    *out = (uintptr_t)s->fd;
    return OSP_OK;
}
