/*
 * thread_posix.c — the POSIX (pthreads) backend of os_platform/thread.
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * clock_windows.c's sibling thread_windows.c via `BUILD_windows`). No OS #ifdefs
 * here — the build already chose this file.
 *
 * Each opaque handle from thread.h is a small heap struct wrapping the real
 * pthreads object. The `_init`/`_spawn` allocate it; the `_destroy`/`_join` free
 * it — so no pthread object and no allocation is ever leaked.
 *
 * Links the OS thread library: the BUILD passes `-pthread` via PLATFORM_LIBS,
 * and _POSIX_C_SOURCE (also from the BUILD) exposes the pthreads declarations.
 */
#include "os_platform/thread.h"

#include <pthread.h>
#include <stdlib.h>

/* ── Threads ────────────────────────────────────────────────────────────── */

/*
 * The handle carries the OS thread id plus the user's function, argument, and
 * (once the thread finishes) its result. pthreads already speaks our
 * `void *(*)(void *)` shape, so the trampoline exists only to stash the result
 * where join() can find it — pthread_join's own out-parameter would also work,
 * but routing it through the struct keeps the Windows backend (whose thread
 * proc returns `unsigned`, not `void *`) structurally identical.
 */
struct osp_thread {
    pthread_t tid;
    osp_thread_fn fn;
    void *arg;
    void *result;
};

static void *osp__thread_trampoline(void *p) {
    struct osp_thread *t = (struct osp_thread *)p;
    t->result = t->fn(t->arg);
    return NULL;
}

osp_status osp_thread_spawn(osp_thread **out, osp_thread_fn fn, void *arg) {
    struct osp_thread *t;
    if (out == NULL || fn == NULL) {
        return OSP_ERR_INVAL;
    }
    t = (struct osp_thread *)malloc(sizeof(*t));
    if (t == NULL) {
        return OSP_ERR_NOMEM;
    }
    t->fn = fn;
    t->arg = arg;
    t->result = NULL;
    if (pthread_create(&t->tid, NULL, osp__thread_trampoline, t) != 0) {
        free(t);
        return OSP_ERR_OS;
    }
    *out = t;
    return OSP_OK;
}

osp_status osp_thread_join(osp_thread *t, void **retval_out) {
    if (t == NULL) {
        return OSP_ERR_INVAL;
    }
    /* On failure we deliberately keep the handle: the thread may still be
     * running, so freeing it (and its pthread_t) would be unsafe. */
    if (pthread_join(t->tid, NULL) != 0) {
        return OSP_ERR_OS;
    }
    if (retval_out != NULL) {
        *retval_out = t->result;
    }
    free(t);
    return OSP_OK;
}

/* ── Mutexes ────────────────────────────────────────────────────────────── */

struct osp_mutex {
    pthread_mutex_t m;
};

osp_status osp_mutex_init(osp_mutex **out) {
    struct osp_mutex *mu;
    if (out == NULL) {
        return OSP_ERR_INVAL;
    }
    mu = (struct osp_mutex *)malloc(sizeof(*mu));
    if (mu == NULL) {
        return OSP_ERR_NOMEM;
    }
    if (pthread_mutex_init(&mu->m, NULL) != 0) {
        free(mu);
        return OSP_ERR_OS;
    }
    *out = mu;
    return OSP_OK;
}

osp_status osp_mutex_lock(osp_mutex *m) {
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    return (pthread_mutex_lock(&m->m) == 0) ? OSP_OK : OSP_ERR_OS;
}

osp_status osp_mutex_unlock(osp_mutex *m) {
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    return (pthread_mutex_unlock(&m->m) == 0) ? OSP_OK : OSP_ERR_OS;
}

osp_status osp_mutex_destroy(osp_mutex *m) {
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    if (pthread_mutex_destroy(&m->m) != 0) {
        return OSP_ERR_OS;
    }
    free(m);
    return OSP_OK;
}

/* ── Condition variables ────────────────────────────────────────────────── */

struct osp_cond {
    pthread_cond_t c;
};

osp_status osp_cond_init(osp_cond **out) {
    struct osp_cond *cv;
    if (out == NULL) {
        return OSP_ERR_INVAL;
    }
    cv = (struct osp_cond *)malloc(sizeof(*cv));
    if (cv == NULL) {
        return OSP_ERR_NOMEM;
    }
    if (pthread_cond_init(&cv->c, NULL) != 0) {
        free(cv);
        return OSP_ERR_OS;
    }
    *out = cv;
    return OSP_OK;
}

osp_status osp_cond_wait(osp_cond *c, osp_mutex *m) {
    if (c == NULL || m == NULL) {
        return OSP_ERR_INVAL;
    }
    /* pthread_cond_wait atomically unlocks m, sleeps, and re-locks m on wake. */
    return (pthread_cond_wait(&c->c, &m->m) == 0) ? OSP_OK : OSP_ERR_OS;
}

osp_status osp_cond_signal(osp_cond *c) {
    if (c == NULL) {
        return OSP_ERR_INVAL;
    }
    return (pthread_cond_signal(&c->c) == 0) ? OSP_OK : OSP_ERR_OS;
}

osp_status osp_cond_broadcast(osp_cond *c) {
    if (c == NULL) {
        return OSP_ERR_INVAL;
    }
    return (pthread_cond_broadcast(&c->c) == 0) ? OSP_OK : OSP_ERR_OS;
}

osp_status osp_cond_destroy(osp_cond *c) {
    if (c == NULL) {
        return OSP_ERR_INVAL;
    }
    if (pthread_cond_destroy(&c->c) != 0) {
        return OSP_ERR_OS;
    }
    free(c);
    return OSP_OK;
}
