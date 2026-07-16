/*
 * tcp_runtime.c — the reactor-driven TCP server, built on net + reactor.
 * ===========================================================================
 *
 * OS-agnostic: `net` owns the per-OS socket calls and `reactor` owns the per-OS
 * readiness mechanism (kqueue/epoll/WSAPoll), so this file has no #ifdefs. It is
 * the glue that turns "one blocking socket at a time" into "one thread watching
 * many sockets."
 *
 * CONNECTION IDENTITY. Each accepted connection is a heap-allocated node whose
 * pointer is used verbatim as the reactor's token — so a wait result maps back
 * to its connection in O(1). The nodes must be individually heap-allocated (not
 * slots in a growable array) because the array of node POINTERS reallocs, which
 * would invalidate any token pointing into it. The listener is registered with
 * the runtime pointer itself as a sentinel token, distinct from every node.
 */
#include "tcp_runtime/tcp_runtime.h"

#include "net/tcp.h"
#include "reactor/reactor.h"

#include <stdlib.h>

/* Per-read scratch. A reply larger than this is truncated (a phase-one limit;
 * streaming/mailbox output is a documented follow-up). */
#define TCP_RT_BUFSZ 8192u
/* Ready descriptors serviced per poll step before returning to the caller. */
#define TCP_RT_MAX_EVENTS 64

struct tcp_conn {
    osp_socket *sock;
    osp_fd fd; /* cached for osp_reactor_del (the socket may be closing) */
    uint64_t id;
};

struct tcp_runtime {
    osp_socket *listener;
    osp_fd listener_fd;
    osp_reactor *reactor;
    tcp_handler handler;
    void *user;
    uint64_t next_id;
    struct tcp_conn **conns; /* array of stable node pointers */
    size_t count;
    size_t cap;
    volatile int stopped;
};

/* Register a freshly accepted socket as a tracked connection. On any failure the
 * caller still owns `sock` and must close it. */
static osp_status tcp__add_conn(struct tcp_runtime *rt, osp_socket *sock) {
    struct tcp_conn *node;
    uintptr_t fdv;
    osp_status st;

    node = (struct tcp_conn *)malloc(sizeof(*node));
    if (node == NULL) {
        return OSP_ERR_NOMEM;
    }
    st = osp_socket_fd(sock, &fdv);
    if (st != OSP_OK) {
        free(node);
        return st;
    }
    node->sock = sock;
    node->fd = (osp_fd)fdv;
    node->id = rt->next_id++;

    if (rt->count == rt->cap) {
        size_t ncap = (rt->cap == 0) ? 8 : rt->cap * 2;
        struct tcp_conn **na =
            (struct tcp_conn **)realloc(rt->conns, ncap * sizeof(*na));
        if (na == NULL) {
            free(node);
            return OSP_ERR_NOMEM;
        }
        rt->conns = na;
        rt->cap = ncap;
    }
    /* Register before recording, so a failed registration leaves no dangling
     * node in the array. */
    st = osp_reactor_add(rt->reactor, node->fd, OSP_READABLE, node);
    if (st != OSP_OK) {
        free(node);
        return st;
    }
    rt->conns[rt->count++] = node;
    return OSP_OK;
}

/* Stop watching, close, and free a tracked connection. */
static void tcp__remove_conn(struct tcp_runtime *rt, struct tcp_conn *node) {
    size_t i;
    osp_reactor_del(rt->reactor, node->fd);
    osp_socket_close(node->sock);
    for (i = 0; i < rt->count; i++) {
        if (rt->conns[i] == node) {
            rt->conns[i] = rt->conns[rt->count - 1]; /* swap-remove */
            rt->count--;
            break;
        }
    }
    free(node);
}

osp_status tcp_runtime_bind(tcp_runtime **out, const char *host,
                            unsigned short port, tcp_handler handler,
                            void *user) {
    struct tcp_runtime *rt;
    osp_status st;
    uintptr_t lfd;

    if (out == NULL || host == NULL || handler == NULL) {
        return OSP_ERR_INVAL;
    }
    st = osp_net_init();
    if (st != OSP_OK) {
        return st;
    }
    rt = (struct tcp_runtime *)calloc(1, sizeof(*rt));
    if (rt == NULL) {
        osp_net_shutdown();
        return OSP_ERR_NOMEM;
    }
    rt->handler = handler;
    rt->user = user;
    rt->next_id = 1;

    st = osp_tcp_listen(&rt->listener, host, port, 128);
    if (st != OSP_OK) {
        free(rt);
        osp_net_shutdown();
        return st;
    }
    st = osp_socket_fd(rt->listener, &lfd);
    if (st != OSP_OK) {
        osp_socket_close(rt->listener);
        free(rt);
        osp_net_shutdown();
        return st;
    }
    rt->listener_fd = (osp_fd)lfd;

    st = osp_reactor_create(&rt->reactor);
    if (st != OSP_OK) {
        osp_socket_close(rt->listener);
        free(rt);
        osp_net_shutdown();
        return st;
    }
    /* The runtime pointer is the listener's sentinel token — distinct from every
     * connection node pointer. */
    st = osp_reactor_add(rt->reactor, rt->listener_fd, OSP_READABLE, rt);
    if (st != OSP_OK) {
        osp_reactor_destroy(rt->reactor);
        osp_socket_close(rt->listener);
        free(rt);
        osp_net_shutdown();
        return st;
    }
    *out = rt;
    return OSP_OK;
}

osp_status tcp_runtime_local_port(tcp_runtime *rt, unsigned short *out_port) {
    if (rt == NULL || out_port == NULL) {
        return OSP_ERR_INVAL;
    }
    return osp_tcp_local_port(rt->listener, out_port);
}

osp_status tcp_runtime_poll(tcp_runtime *rt, int timeout_ms, int *out_handled) {
    osp_event events[TCP_RT_MAX_EVENTS];
    char rbuf[TCP_RT_BUFSZ];
    char wbuf[TCP_RT_BUFSZ];
    int count = 0;
    int i;
    osp_status st;

    if (rt == NULL || out_handled == NULL) {
        return OSP_ERR_INVAL;
    }
    *out_handled = 0;
    st = osp_reactor_wait(rt->reactor, events, TCP_RT_MAX_EVENTS, timeout_ms,
                          &count);
    if (st != OSP_OK) {
        return st;
    }
    for (i = 0; i < count; i++) {
        if (events[i].token == (void *)rt) {
            /* listener readable → accept one pending connection (level-triggered,
             * so any further pending connections re-report next poll). */
            osp_socket *conn = NULL;
            if (osp_tcp_accept(rt->listener, &conn) == OSP_OK) {
                if (tcp__add_conn(rt, conn) != OSP_OK) {
                    osp_socket_close(conn); /* could not track it */
                }
            }
        } else {
            struct tcp_conn *node = (struct tcp_conn *)events[i].token;
            size_t n = 0;
            osp_status rst = osp_socket_recv(node->sock, rbuf, sizeof(rbuf), &n);
            if (rst != OSP_OK || n == 0) {
                tcp__remove_conn(rt, node); /* error or orderly peer close */
            } else {
                tcp_action a =
                    rt->handler(node->id, rbuf, n, wbuf, sizeof(wbuf), rt->user);
                if (a.write_len > 0) {
                    /* Clamp defensively — we only own `sizeof wbuf` bytes. */
                    size_t wl = (a.write_len > sizeof(wbuf)) ? sizeof(wbuf)
                                                             : a.write_len;
                    (void)osp_socket_send(node->sock, wbuf, wl, NULL);
                }
                if (a.close) {
                    tcp__remove_conn(rt, node);
                }
            }
        }
    }
    *out_handled = count;
    return OSP_OK;
}

osp_status tcp_runtime_serve(tcp_runtime *rt) {
    if (rt == NULL) {
        return OSP_ERR_INVAL;
    }
    while (!rt->stopped) {
        int handled = 0;
        osp_status st = tcp_runtime_poll(rt, 100, &handled);
        if (st != OSP_OK) {
            return st;
        }
    }
    return OSP_OK;
}

void tcp_runtime_stop(tcp_runtime *rt) {
    if (rt != NULL) {
        rt->stopped = 1;
    }
}

osp_status tcp_runtime_destroy(tcp_runtime *rt) {
    size_t i;
    if (rt == NULL) {
        return OSP_ERR_INVAL;
    }
    for (i = 0; i < rt->count; i++) {
        osp_reactor_del(rt->reactor, rt->conns[i]->fd);
        osp_socket_close(rt->conns[i]->sock);
        free(rt->conns[i]);
    }
    free(rt->conns);
    osp_reactor_destroy(rt->reactor);
    osp_socket_close(rt->listener);
    free(rt);
    osp_net_shutdown();
    return OSP_OK;
}
