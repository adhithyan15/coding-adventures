/*
 * reactor_windows.c — the Winsock (WSAPoll) backend of reactor.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS uses reactor_mac.c /
 * kqueue and Linux reactor_linux.c / epoll via the shared `BUILD`). No OS
 * #ifdefs — the build chose this file. Links ws2_32 (WSAPoll).
 *
 * WSAPoll is Winsock's readiness poll: a growable array of {SOCKET, interest,
 * token}, rebuilt into a WSAPOLLFD array for each wait, with revents mapped back
 * to tokens. Like the kqueue/epoll backends it presents the same readiness
 * interface (one coalesced event per ready descriptor). The readiness bits are
 * POLLRDNORM / POLLWRNORM.
 */
#include "reactor/reactor.h"

#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>

#include <stdlib.h>

struct osp_entry {
    SOCKET fd;
    SHORT events;
    void *token;
};

struct osp_reactor {
    struct osp_entry *entries;
    size_t count;
    size_t cap;
};

static SHORT osp__to_wsa(int interest) {
    SHORT e = 0;
    if (interest & OSP_READABLE) {
        e = (SHORT)(e | POLLRDNORM);
    }
    if (interest & OSP_WRITABLE) {
        e = (SHORT)(e | POLLWRNORM);
    }
    return e;
}

static int osp__from_wsa(SHORT revents) {
    int e = 0;
    if (revents & (POLLRDNORM | POLLHUP | POLLERR)) {
        e |= OSP_READABLE;
    }
    if (revents & POLLWRNORM) {
        e |= OSP_WRITABLE;
    }
    return e;
}

static struct osp_entry *osp__find(struct osp_reactor *r, SOCKET fd) {
    size_t i;
    for (i = 0; i < r->count; i++) {
        if (r->entries[i].fd == fd) {
            return &r->entries[i];
        }
    }
    return NULL;
}

osp_status osp_reactor_create(osp_reactor **out) {
    struct osp_reactor *r;
    if (out == NULL) {
        return OSP_ERR_INVAL;
    }
    r = (struct osp_reactor *)malloc(sizeof(*r));
    if (r == NULL) {
        return OSP_ERR_NOMEM;
    }
    r->entries = NULL;
    r->count = 0;
    r->cap = 0;
    *out = r;
    return OSP_OK;
}

osp_status osp_reactor_add(osp_reactor *r, osp_fd fd, int interest, void *token) {
    struct osp_entry *e;
    SOCKET sock = (SOCKET)fd;
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    e = osp__find(r, sock);
    if (e != NULL) {
        e->events = osp__to_wsa(interest);
        e->token = token;
        return OSP_OK;
    }
    if (r->count == r->cap) {
        size_t ncap = (r->cap == 0) ? 8 : r->cap * 2;
        struct osp_entry *ne =
            (struct osp_entry *)realloc(r->entries, ncap * sizeof(*ne));
        if (ne == NULL) {
            return OSP_ERR_NOMEM;
        }
        r->entries = ne;
        r->cap = ncap;
    }
    r->entries[r->count].fd = sock;
    r->entries[r->count].events = osp__to_wsa(interest);
    r->entries[r->count].token = token;
    r->count++;
    return OSP_OK;
}

osp_status osp_reactor_del(osp_reactor *r, osp_fd fd) {
    struct osp_entry *e;
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    e = osp__find(r, (SOCKET)fd);
    if (e != NULL) {
        *e = r->entries[r->count - 1];
        r->count--;
    }
    return OSP_OK;
}

osp_status osp_reactor_wait(osp_reactor *r, osp_event *out_events, int max_events,
                            int timeout_ms, int *out_count) {
    WSAPOLLFD *pfds;
    int ready;
    int produced;
    size_t i;

    if (r == NULL || out_events == NULL || out_count == NULL || max_events <= 0) {
        return OSP_ERR_INVAL;
    }
    *out_count = 0;
    if (r->count == 0) {
        return OSP_OK;
    }
    pfds = (WSAPOLLFD *)malloc(r->count * sizeof(*pfds));
    if (pfds == NULL) {
        return OSP_ERR_NOMEM;
    }
    for (i = 0; i < r->count; i++) {
        pfds[i].fd = r->entries[i].fd;
        pfds[i].events = r->entries[i].events;
        pfds[i].revents = 0;
    }
    ready = WSAPoll(pfds, (ULONG)r->count, timeout_ms);
    if (ready == SOCKET_ERROR) {
        free(pfds);
        return OSP_ERR_OS;
    }
    produced = 0;
    for (i = 0; i < r->count && produced < max_events; i++) {
        if (pfds[i].revents != 0) {
            out_events[produced].token = r->entries[i].token;
            out_events[produced].events = osp__from_wsa(pfds[i].revents);
            produced++;
        }
    }
    free(pfds);
    *out_count = produced;
    return OSP_OK;
}

osp_status osp_reactor_destroy(osp_reactor *r) {
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    free(r->entries);
    free(r);
    return OSP_OK;
}
