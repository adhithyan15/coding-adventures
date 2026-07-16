/*
 * reactor_linux.c — the epoll backend of reactor (Linux).
 * ===========================================================================
 *
 * Compiled on Linux (named by the shared `BUILD` when platform_os is `linux`;
 * macOS uses reactor_mac.c (kqueue) and Windows reactor_windows.c (WSAPoll)).
 * No OS #ifdefs — the build chose this file.
 *
 * WHY epoll over poll. poll() rescans every registered descriptor on every wait
 * — O(n) per call. epoll registers interest once in the kernel and each wait
 * returns only the ready descriptors — O(ready). epoll also coalesces a fd's
 * read+write readiness into one event and carries the caller's token in
 * epoll_event.data.ptr, so — unlike kqueue — no per-wait de-duplication is
 * needed. A small {fd, interest, token} array is still kept as the registration
 * record: it decides ADD vs MOD on re-add and lets wait short-circuit when
 * nothing is registered, matching poll's "nothing to watch → return now".
 */
#include "reactor/reactor.h"

#include <errno.h>
#include <stdlib.h>
#include <sys/epoll.h>
#include <unistd.h>

struct osp_entry {
    int fd;
    int interest; /* OSP_READABLE | OSP_WRITABLE */
    void *token;
};

struct osp_reactor {
    int epfd;
    struct osp_entry *entries;
    size_t count;
    size_t cap;
};

/* Translate our interest bitmask into epoll event bits. */
static uint32_t osp__to_epoll(int interest) {
    uint32_t e = 0;
    if (interest & OSP_READABLE) {
        e |= EPOLLIN;
    }
    if (interest & OSP_WRITABLE) {
        e |= EPOLLOUT;
    }
    return e;
}

/* Translate epoll's returned events into our readiness bits. */
static int osp__from_epoll(uint32_t events) {
    int e = 0;
    /* EPOLLHUP/EPOLLERR surface as readable so a closed/broken peer is noticed
     * on the next read — parity with the poll backend. */
    if (events & (EPOLLIN | EPOLLHUP | EPOLLERR)) {
        e |= OSP_READABLE;
    }
    if (events & EPOLLOUT) {
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
    r->epfd = epoll_create1(EPOLL_CLOEXEC);
    if (r->epfd < 0) {
        free(r);
        return OSP_ERR_OS;
    }
    r->entries = NULL;
    r->count = 0;
    r->cap = 0;
    *out = r;
    return OSP_OK;
}

osp_status osp_reactor_add(osp_reactor *r, osp_fd fd, int interest, void *token) {
    struct osp_entry *e;
    struct epoll_event ev;
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    ev.events = osp__to_epoll(interest);
    ev.data.ptr = token;
    e = osp__find(r, fd);
    if (e != NULL) {
        /* Already registered → modify in the kernel, update the record. */
        if (epoll_ctl(r->epfd, EPOLL_CTL_MOD, fd, &ev) != 0) {
            return OSP_ERR_OS;
        }
        e->interest = interest;
        e->token = token;
        return OSP_OK;
    }
    if (epoll_ctl(r->epfd, EPOLL_CTL_ADD, fd, &ev) != 0) {
        return OSP_ERR_OS;
    }
    if (r->count == r->cap) {
        size_t ncap = (r->cap == 0) ? 8 : r->cap * 2;
        struct osp_entry *ne =
            (struct osp_entry *)realloc(r->entries, ncap * sizeof(*ne));
        if (ne == NULL) {
            /* Roll the kernel state back so record and kernel stay in sync. */
            epoll_ctl(r->epfd, EPOLL_CTL_DEL, fd, NULL);
            return OSP_ERR_NOMEM;
        }
        r->entries = ne;
        r->cap = ncap;
    }
    r->entries[r->count].fd = fd;
    r->entries[r->count].interest = interest;
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
        /* Best-effort kernel removal; the record is the source of truth. */
        epoll_ctl(r->epfd, EPOLL_CTL_DEL, fd, NULL);
        *e = r->entries[r->count - 1]; /* swap-remove */
        r->count--;
    }
    return OSP_OK; /* deleting an absent fd is not an error */
}

osp_status osp_reactor_wait(osp_reactor *r, osp_event *out_events, int max_events,
                            int timeout_ms, int *out_count) {
    struct epoll_event *evs;
    int n;
    int i;

    if (r == NULL || out_events == NULL || out_count == NULL || max_events <= 0) {
        return OSP_ERR_INVAL;
    }
    *out_count = 0;
    if (r->count == 0) {
        return OSP_OK; /* nothing to watch */
    }
    evs = (struct epoll_event *)malloc((size_t)max_events * sizeof(*evs));
    if (evs == NULL) {
        return OSP_ERR_NOMEM;
    }
    do {
        n = epoll_wait(r->epfd, evs, max_events, timeout_ms);
    } while (n < 0 && errno == EINTR);
    if (n < 0) {
        free(evs);
        return OSP_ERR_OS;
    }
    /* epoll already returns one event per ready fd, capped at max_events. */
    for (i = 0; i < n; i++) {
        out_events[i].token = evs[i].data.ptr;
        out_events[i].events = osp__from_epoll(evs[i].events);
    }
    free(evs);
    *out_count = n;
    return OSP_OK;
}

osp_status osp_reactor_destroy(osp_reactor *r) {
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    close(r->epfd);
    free(r->entries);
    free(r);
    return OSP_OK;
}
