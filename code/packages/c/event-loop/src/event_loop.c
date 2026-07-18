/*
 * event_loop.c — the pluggable generic event loop, on os-platform.
 * ===========================================================================
 *
 * OS-agnostic: the only OS-touching pieces are `osp_mutex` (guards the stop flag
 * so a worker thread and the loop thread never race) and `osp_sleep_ns` (the idle
 * nap). Everything else is plain ISO C — growable arrays of sources and handlers,
 * and a reused event queue.
 *
 * WHY A MUTEX AND NOT A BARE FLAG. The stop flag is written by one thread
 * (whoever calls stop / signal) and read by the loop thread each iteration. A
 * bare `int` shared across threads is a data race in C's memory model; the mutex
 * makes the write visible and the read well-defined. It is uncontended in the
 * common case, so it costs essentially nothing.
 */
#include "event_loop/event_loop.h"

#include "os_platform/clock.h"  /* osp_sleep_ns — the idle nap */
#include "os_platform/thread.h" /* osp_mutex — guards the stop flag */

#include <stdlib.h>

/* Max events one source may yield in a single poll (it returns more on its next
 * poll). Bounds the per-source scratch buffer on the stack. */
#define EVL_SRC_BATCH 64u
/* Idle nap when an iteration collected nothing: ~0.2 ms. Mirrors the Rust
 * yield_now() — long enough to not spin a core at 100%, short enough that event
 * latency stays sub-millisecond. */
#define EVL_IDLE_SLEEP_NS 200000u

/* A registered source: its poll function and its own state. */
struct evl_source {
    evl_source_poll poll;
    void *ctx;
};

/* A registered handler: its function and its captured state. */
struct evl_hdlr {
    evl_handler fn;
    void *user;
};

/*
 * The stop flag: a mutex-guarded int, shared between the loop and every handle.
 * It is embedded in the loop (its lifetime is the loop's), so a handle is just a
 * pointer to it — no separate allocation, matching the Rust Arc<AtomicBool> where
 * all clones share one flag.
 */
struct evl_stop {
    osp_mutex *lock;
    int stopped;
};

struct event_loop {
    struct evl_source *sources;
    size_t nsrc, csrc;
    struct evl_hdlr *handlers;
    size_t nhdl, chdl;
    const void **queue; /* reused per-iteration event queue */
    size_t qcap;
    struct evl_stop stop;
};

/* Read the stop flag under the lock. */
static int evl__is_stopped(struct evl_stop *s) {
    int v;
    osp_mutex_lock(s->lock);
    v = s->stopped;
    osp_mutex_unlock(s->lock);
    return v;
}

/* Set the stop flag under the lock. */
static void evl__set_stopped(struct evl_stop *s, int v) {
    osp_mutex_lock(s->lock);
    s->stopped = v;
    osp_mutex_unlock(s->lock);
}

osp_status event_loop_create(event_loop **out) {
    event_loop *lp;
    osp_status st;

    if (out == NULL) {
        return OSP_ERR_INVAL;
    }
    lp = (event_loop *)calloc(1, sizeof(*lp));
    if (lp == NULL) {
        return OSP_ERR_NOMEM;
    }
    st = osp_mutex_init(&lp->stop.lock);
    if (st != OSP_OK) {
        free(lp);
        return st;
    }
    *out = lp;
    return OSP_OK;
}

osp_status event_loop_add_source(event_loop *lp, evl_source_poll poll,
                                 void *ctx) {
    if (lp == NULL || poll == NULL) {
        return OSP_ERR_INVAL;
    }
    if (lp->nsrc == lp->csrc) {
        size_t ncap = lp->csrc ? lp->csrc * 2 : 4;
        struct evl_source *na =
            (struct evl_source *)realloc(lp->sources, ncap * sizeof(*na));
        if (na == NULL) {
            return OSP_ERR_NOMEM;
        }
        lp->sources = na;
        lp->csrc = ncap;
    }
    lp->sources[lp->nsrc].poll = poll;
    lp->sources[lp->nsrc].ctx = ctx;
    lp->nsrc++;
    return OSP_OK;
}

osp_status event_loop_on_event(event_loop *lp, evl_handler fn, void *user) {
    if (lp == NULL || fn == NULL) {
        return OSP_ERR_INVAL;
    }
    if (lp->nhdl == lp->chdl) {
        size_t ncap = lp->chdl ? lp->chdl * 2 : 4;
        struct evl_hdlr *na =
            (struct evl_hdlr *)realloc(lp->handlers, ncap * sizeof(*na));
        if (na == NULL) {
            return OSP_ERR_NOMEM;
        }
        lp->handlers = na;
        lp->chdl = ncap;
    }
    lp->handlers[lp->nhdl].fn = fn;
    lp->handlers[lp->nhdl].user = user;
    lp->nhdl++;
    return OSP_OK;
}

evl_stop *event_loop_stop_handle(event_loop *lp) {
    if (lp == NULL) {
        return NULL;
    }
    return &lp->stop;
}

osp_status event_loop_stop(event_loop *lp) {
    if (lp == NULL) {
        return OSP_ERR_INVAL;
    }
    evl__set_stopped(&lp->stop, 1);
    return OSP_OK;
}

osp_status evl_stop_signal(evl_stop *h) {
    if (h == NULL) {
        return OSP_ERR_INVAL;
    }
    evl__set_stopped(h, 1);
    return OSP_OK;
}

osp_status event_loop_run(event_loop *lp) {
    const void *batch[EVL_SRC_BATCH];

    if (lp == NULL) {
        return OSP_ERR_INVAL;
    }
    /* Clear the flag so a loop can be re-run (mirrors Rust run()'s reset). */
    evl__set_stopped(&lp->stop, 0);

    for (;;) {
        size_t qn = 0; /* events collected this iteration */
        size_t si, qi, hi;

        /* Check stop at the top so stop() takes effect even with no events. */
        if (evl__is_stopped(&lp->stop)) {
            return OSP_OK;
        }

        /* ── Phase 1: collect from every source into the reused queue ──────── */
        for (si = 0; si < lp->nsrc; si++) {
            size_t got =
                lp->sources[si].poll(lp->sources[si].ctx, batch, EVL_SRC_BATCH);
            size_t k;
            if (got > EVL_SRC_BATCH) {
                got = EVL_SRC_BATCH; /* defend against a misbehaving source */
            }
            for (k = 0; k < got; k++) {
                if (qn == lp->qcap) {
                    size_t ncap = lp->qcap ? lp->qcap * 2 : 64;
                    const void **nq =
                        (const void **)realloc(lp->queue, ncap * sizeof(*nq));
                    if (nq == NULL) {
                        /* Out of memory: dispatch what we have, drop the rest. */
                        break;
                    }
                    lp->queue = nq;
                    lp->qcap = ncap;
                }
                lp->queue[qn++] = batch[k];
            }
        }

        /* ── Phase 2: dispatch the whole queue; EVL_EXIT stops immediately ─── */
        for (qi = 0; qi < qn; qi++) {
            for (hi = 0; hi < lp->nhdl; hi++) {
                if (lp->handlers[hi].fn(lp->queue[qi], lp->handlers[hi].user) ==
                    EVL_EXIT) {
                    return OSP_OK;
                }
            }
        }

        /* ── Phase 3: nap if the iteration was idle, so we don't spin a core ─ */
        if (qn == 0) {
            osp_sleep_ns(EVL_IDLE_SLEEP_NS);
        }
    }
}

osp_status event_loop_destroy(event_loop *lp) {
    if (lp == NULL) {
        return OSP_ERR_INVAL;
    }
    if (lp->stop.lock != NULL) {
        osp_mutex_destroy(lp->stop.lock);
    }
    free(lp->sources);
    free(lp->handlers);
    free(lp->queue);
    free(lp);
    return OSP_OK;
}
