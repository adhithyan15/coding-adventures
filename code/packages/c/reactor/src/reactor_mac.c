/*
 * reactor_mac.c — the kqueue backend of reactor (macOS / BSD).
 * ===========================================================================
 *
 * Compiled on macOS (named by the shared `BUILD` when platform_os is `mac`;
 * Linux uses reactor_linux.c (epoll) and Windows reactor_windows.c (WSAPoll)).
 * No OS #ifdefs — the build chose this file.
 *
 * WHY kqueue over poll. poll() rescans every registered descriptor on every
 * wait — O(n) per call. kqueue registers interest once in the kernel and each
 * wait returns only the descriptors that are actually ready — O(ready). Same
 * readiness interface, better asymptotics for many idle connections.
 *
 * A single fd can be registered under two filters (EVFILT_READ, EVFILT_WRITE),
 * so one wait may hand back two kevents for the same fd. To keep the poll
 * backend's contract — one coalesced osp_event per ready descriptor — we dedup
 * by ident within each wait. We also keep a tiny {fd, interest, token} array as
 * the registration record: it lets add compute the precise filter delta (so
 * narrowing interest removes the now-unwanted filter) and lets wait short-circuit
 * when nothing is registered, matching poll's "nothing to watch → return now".
 */
#include "reactor/reactor.h"

#include <errno.h>
#include <stdlib.h>
#include <sys/event.h>
#include <sys/time.h>
#include <sys/types.h>
#include <unistd.h>

struct osp_entry {
    int fd;
    int interest; /* OSP_READABLE | OSP_WRITABLE */
    void *token;
};

struct osp_reactor {
    int kq;
    struct osp_entry *entries;
    size_t count;
    size_t cap;
};

static struct osp_entry *osp__find(struct osp_reactor *r, int fd) {
    size_t i;
    for (i = 0; i < r->count; i++) {
        if (r->entries[i].fd == fd) {
            return &r->entries[i];
        }
    }
    return NULL;
}

/* Apply one filter change. EV_DELETE of an absent filter (ENOENT) is not an
 * error; any other failure is OSP_ERR_OS. */
static osp_status osp__kq_change(int kq, int fd, int16_t filter, uint16_t flags,
                                 void *token) {
    struct kevent ch;
    EV_SET(&ch, (uintptr_t)fd, filter, flags, 0, 0, token);
    if (kevent(kq, &ch, 1, NULL, 0, NULL) == 0) {
        return OSP_OK;
    }
    if ((flags & EV_DELETE) != 0) {
        return OSP_OK; /* removing a filter that isn't there is fine */
    }
    return OSP_ERR_OS;
}

/* Bring fd's kernel filters in line with `interest` (adding/refreshing wanted
 * filters, deleting unwanted ones). */
static osp_status osp__kq_apply(int kq, int fd, int interest, void *token) {
    osp_status st;
    if (interest & OSP_READABLE) {
        st = osp__kq_change(kq, fd, EVFILT_READ, EV_ADD, token);
    } else {
        st = osp__kq_change(kq, fd, EVFILT_READ, EV_DELETE, NULL);
    }
    if (st != OSP_OK) {
        return st;
    }
    if (interest & OSP_WRITABLE) {
        st = osp__kq_change(kq, fd, EVFILT_WRITE, EV_ADD, token);
    } else {
        st = osp__kq_change(kq, fd, EVFILT_WRITE, EV_DELETE, NULL);
    }
    return st;
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
    r->kq = kqueue();
    if (r->kq < 0) {
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
    osp_status st;
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    /* Apply the kernel change first; only touch the array once it succeeds. */
    st = osp__kq_apply(r->kq, fd, interest, token);
    if (st != OSP_OK) {
        return st;
    }
    e = osp__find(r, fd);
    if (e != NULL) {
        e->interest = interest;
        e->token = token;
        return OSP_OK;
    }
    if (r->count == r->cap) {
        size_t ncap = (r->cap == 0) ? 8 : r->cap * 2;
        struct osp_entry *ne =
            (struct osp_entry *)realloc(r->entries, ncap * sizeof(*ne));
        if (ne == NULL) {
            /* Roll the kernel filters back so the record and kernel stay in sync
             * on this OOM path (parity with the epoll backend's rollback). */
            osp__kq_change(r->kq, fd, EVFILT_READ, EV_DELETE, NULL);
            osp__kq_change(r->kq, fd, EVFILT_WRITE, EV_DELETE, NULL);
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
        /* Remove both filters (EV_DELETE of an absent one is harmless). */
        osp__kq_change(r->kq, fd, EVFILT_READ, EV_DELETE, NULL);
        osp__kq_change(r->kq, fd, EVFILT_WRITE, EV_DELETE, NULL);
        *e = r->entries[r->count - 1]; /* swap-remove */
        r->count--;
    }
    return OSP_OK; /* deleting an absent fd is not an error */
}

osp_status osp_reactor_wait(osp_reactor *r, osp_event *out_events, int max_events,
                            int timeout_ms, int *out_count) {
    struct kevent *evs;
    int *seen; /* idents already placed in out_events, for coalescing */
    struct timespec ts;
    struct timespec *tsp;
    size_t nevents;
    int n;
    int produced;
    int i;

    if (r == NULL || out_events == NULL || out_count == NULL || max_events <= 0) {
        return OSP_ERR_INVAL;
    }
    *out_count = 0;
    if (r->count == 0) {
        return OSP_OK; /* nothing to watch */
    }
    /* A fd may fire under two filters at once, so ask for up to 2 per output
     * slot; we then coalesce down to at most max_events distinct descriptors. */
    nevents = (size_t)max_events * 2;
    evs = (struct kevent *)malloc(nevents * sizeof(*evs));
    if (evs == NULL) {
        return OSP_ERR_NOMEM;
    }
    seen = (int *)malloc((size_t)max_events * sizeof(*seen));
    if (seen == NULL) {
        free(evs);
        return OSP_ERR_NOMEM;
    }
    if (timeout_ms < 0) {
        tsp = NULL; /* block forever */
    } else {
        ts.tv_sec = timeout_ms / 1000;
        ts.tv_nsec = (long)(timeout_ms % 1000) * 1000000L;
        tsp = &ts;
    }
    do {
        n = kevent(r->kq, NULL, 0, evs, (int)nevents, tsp);
    } while (n < 0 && errno == EINTR);
    if (n < 0) {
        free(seen);
        free(evs);
        return OSP_ERR_OS;
    }
    produced = 0;
    for (i = 0; i < n; i++) {
        int fd = (int)evs[i].ident;
        int bits = 0;
        int j;
        int found = -1;
        if (evs[i].filter == EVFILT_READ) {
            bits |= OSP_READABLE;
        }
        if (evs[i].filter == EVFILT_WRITE) {
            bits |= OSP_WRITABLE;
        }
        /* Peer close / filter error surface as readable so the next read sees
         * it — parity with the poll backend's POLLHUP/POLLERR handling. */
        if ((evs[i].flags & (EV_EOF | EV_ERROR)) != 0) {
            bits |= OSP_READABLE;
        }
        for (j = 0; j < produced; j++) {
            if (seen[j] == fd) {
                found = j;
                break;
            }
        }
        if (found >= 0) {
            out_events[found].events |= bits;
        } else if (produced < max_events) {
            seen[produced] = fd;
            out_events[produced].token = evs[i].udata;
            out_events[produced].events = bits;
            produced++;
        }
    }
    free(seen);
    free(evs);
    *out_count = produced;
    return OSP_OK;
}

osp_status osp_reactor_destroy(osp_reactor *r) {
    if (r == NULL) {
        return OSP_ERR_INVAL;
    }
    close(r->kq);
    free(r->entries);
    free(r);
    return OSP_OK;
}
