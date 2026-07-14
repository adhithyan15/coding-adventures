/*
 * reactor_posix.c — the POSIX (poll) backend of reactor (macOS + Linux).
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * reactor_windows.c via `BUILD_windows`). No OS #ifdefs — the build chose this
 * file. Uses only libc (poll).
 *
 * The reactor keeps a growable array of {fd, interest, token}. Each wait rebuilds
 * a parallel struct-pollfd array, calls poll() once, and maps any revents back to
 * the caller's tokens. poll() is O(n) per call; the epoll/kqueue upgrade behind
 * this same interface removes that cost, but poll() is correct and portable.
 */
#include "reactor/reactor.h"

#include <errno.h>
#include <poll.h>
#include <stdlib.h>

struct osp_entry {
    int fd;
    short events; /* POLLIN | POLLOUT */
    void *token;
};

struct osp_reactor {
    struct osp_entry *entries;
    size_t count;
    size_t cap;
};

/* Translate our interest bitmask into poll() event bits. */
static short osp__to_poll(int interest) {
    short e = 0;
    if (interest & OSP_READABLE) {
        e = (short)(e | POLLIN);
    }
    if (interest & OSP_WRITABLE) {
        e = (short)(e | POLLOUT);
    }
    return e;
}

/* Translate poll() revents back into our readiness bits. */
static int osp__from_poll(short revents) {
    int e = 0;
    /* POLLIN/POLLOUT for the normal cases; POLLHUP/POLLERR surface as readable so
     * the caller notices a closed/broken peer on its next read. */
    if (revents & (POLLIN | POLLHUP | POLLERR)) {
        e |= OSP_READABLE;
    }
    if (revents & POLLOUT) {
        e |= OSP_WRITABLE;
    }
    return e;
}

static struct osp_entry *osp__find(struct osp_reactor *r, int fd) {
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
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    /* Re-adding an existing fd updates it in place. */
    e = osp__find(r, fd);
    if (e != NULL) {
        e->events = osp__to_poll(interest);
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
    r->entries[r->count].fd = fd;
    r->entries[r->count].events = osp__to_poll(interest);
    r->entries[r->count].token = token;
    r->count++;
    return OSP_OK;
}

osp_status osp_reactor_del(osp_reactor *r, osp_fd fd) {
    struct osp_entry *e;
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    e = osp__find(r, fd);
    if (e != NULL) {
        /* swap-remove: move the last entry into this slot. */
        *e = r->entries[r->count - 1];
        r->count--;
    }
    return OSP_OK; /* deleting an absent fd is not an error */
}

osp_status osp_reactor_wait(osp_reactor *r, osp_event *out_events, int max_events,
                            int timeout_ms, int *out_count) {
    struct pollfd *pfds;
    int ready;
    int produced;
    size_t i;

    if (r == NULL || out_events == NULL || out_count == NULL || max_events <= 0) {
        return OSP_ERR_INVAL;
    }
    *out_count = 0;
    if (r->count == 0) {
        return OSP_OK; /* nothing to watch */
    }
    pfds = (struct pollfd *)malloc(r->count * sizeof(*pfds));
    if (pfds == NULL) {
        return OSP_ERR_NOMEM;
    }
    for (i = 0; i < r->count; i++) {
        pfds[i].fd = r->entries[i].fd;
        pfds[i].events = r->entries[i].events;
        pfds[i].revents = 0;
    }
    do {
        ready = poll(pfds, (nfds_t)r->count, timeout_ms);
    } while (ready < 0 && errno == EINTR);
    if (ready < 0) {
        free(pfds);
        return OSP_ERR_OS;
    }
    produced = 0;
    for (i = 0; i < r->count && produced < max_events; i++) {
        if (pfds[i].revents != 0) {
            out_events[produced].token = r->entries[i].token;
            out_events[produced].events = osp__from_poll(pfds[i].revents);
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
