/*
 * thread_windows.c — the Win32 backend of os_platform/thread.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS/Linux use thread_posix.c
 * via the shared `BUILD`). No OS #ifdefs — the build already chose this file.
 *
 * The three opaque handles map onto Win32 primitives:
 *   thread  →  _beginthreadex + HANDLE   (WaitForSingleObject to join)
 *   mutex   →  CRITICAL_SECTION
 *   cond    →  CONDITION_VARIABLE        (paired with the CRITICAL_SECTION)
 *
 * We use _beginthreadex rather than raw CreateThread because the worker runs
 * arbitrary caller code that may touch the C runtime; _beginthreadex sets up (and
 * tears down) the per-thread CRT state, avoiding a small resource leak. All of
 * these live in the CRT + kernel32, linked by default, so no PLATFORM_LIBS entry
 * is needed. Note: CONDITION_VARIABLE has no OS destroy call — you simply stop
 * using it — so osp_cond_destroy only frees the wrapper.
 */
#include "os_platform/thread.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <process.h> /* _beginthreadex */
#include <stdlib.h>

/* ── Threads ────────────────────────────────────────────────────────────── */

struct osp_thread {
    HANDLE handle;
    osp_thread_fn fn;
    void *arg;
    void *result;
};

/* Win32/CRT thread procs return `unsigned`, not `void *`, so the trampoline
 * bridges to our osp_thread_fn shape and stashes the result for join(). */
static unsigned __stdcall osp__thread_trampoline(void *p) {
    struct osp_thread *t = (struct osp_thread *)p;
    t->result = t->fn(t->arg);
    return 0;
}

osp_status osp_thread_spawn(osp_thread **out, osp_thread_fn fn, void *arg) {
    struct osp_thread *t;
    uintptr_t h;
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
    h = _beginthreadex(NULL, 0, osp__thread_trampoline, t, 0, NULL);
    if (h == 0) {
        free(t);
        return OSP_ERR_OS;
    }
    t->handle = (HANDLE)h;
    *out = t;
    return OSP_OK;
}

osp_status osp_thread_join(osp_thread *t, void **retval_out) {
    if (t == NULL) {
        return OSP_ERR_INVAL;
    }
    /* On failure keep the handle: the thread may still be running. */
    if (WaitForSingleObject(t->handle, INFINITE) != WAIT_OBJECT_0) {
        return OSP_ERR_OS;
    }
    CloseHandle(t->handle);
    if (retval_out != NULL) {
        *retval_out = t->result;
    }
    free(t);
    return OSP_OK;
}

/* ── Mutexes ────────────────────────────────────────────────────────────── */

struct osp_mutex {
    CRITICAL_SECTION cs;
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
    /* InitializeCriticalSection returns void and does not fail on supported
     * Windows versions. */
    InitializeCriticalSection(&mu->cs);
    *out = mu;
    return OSP_OK;
}

osp_status osp_mutex_lock(osp_mutex *m) {
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    EnterCriticalSection(&m->cs);
    return OSP_OK;
}

osp_status osp_mutex_unlock(osp_mutex *m) {
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    LeaveCriticalSection(&m->cs);
    return OSP_OK;
}

osp_status osp_mutex_destroy(osp_mutex *m) {
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    DeleteCriticalSection(&m->cs);
    free(m);
    return OSP_OK;
}

/* ── Condition variables ────────────────────────────────────────────────── */

struct osp_cond {
    CONDITION_VARIABLE cv;
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
    InitializeConditionVariable(&cv->cv);
    *out = cv;
    return OSP_OK;
}

osp_status osp_cond_wait(osp_cond *c, osp_mutex *m) {
    if (c == NULL || m == NULL) {
        return OSP_ERR_INVAL;
    }
    /* Atomically releases the CRITICAL_SECTION, sleeps, and re-acquires it on
     * wake. With INFINITE, a FALSE return signals a real error, not a timeout. */
    if (!SleepConditionVariableCS(&c->cv, &m->cs, INFINITE)) {
        return OSP_ERR_OS;
    }
    return OSP_OK;
}

osp_status osp_cond_signal(osp_cond *c) {
    if (c == NULL) {
        return OSP_ERR_INVAL;
    }
    WakeConditionVariable(&c->cv);
    return OSP_OK;
}

osp_status osp_cond_broadcast(osp_cond *c) {
    if (c == NULL) {
        return OSP_ERR_INVAL;
    }
    WakeAllConditionVariable(&c->cv);
    return OSP_OK;
}

osp_status osp_cond_destroy(osp_cond *c) {
    if (c == NULL) {
        return OSP_ERR_INVAL;
    }
    /* CONDITION_VARIABLE needs no OS teardown; just free the wrapper. */
    free(c);
    return OSP_OK;
}
