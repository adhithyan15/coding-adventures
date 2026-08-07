/*
 * os_platform/thread.h — threads, mutexes, and condition variables, portably.
 * ===========================================================================
 *
 * The second os-platform primitive. Concurrency is squarely *bucket B*: there is
 * no portable pure-ISO way to start a thread. C11's <threads.h> is an OPTIONAL
 * feature and MSVC does not ship it, so C code must call the OS directly —
 * POSIX threads (pthreads) on macOS/Linux, the Win32 thread API on Windows. This
 * header hides that split behind three small object types:
 *
 *      concept   macOS / Linux (pthreads)     Windows (Win32)
 *      ────────  ───────────────────────────  ─────────────────────────────────
 *      thread    pthread_create / _join        _beginthreadex / WaitForSingleObject
 *      mutex     pthread_mutex_t               CRITICAL_SECTION
 *      cond      pthread_cond_t                CONDITION_VARIABLE
 *
 * OPAQUE HANDLES. The three types below are forward-declared and always used
 * through a pointer; the real struct (which embeds the pthread_t / HANDLE /
 * CRITICAL_SECTION) lives in the per-OS .c file. That keeps OS types out of this
 * shared header — the whole point of the os-platform design — and means a caller
 * never needs <pthread.h> or <windows.h> to use threads. Each object is created
 * on the heap by its `_init`/`_spawn` and released by its `_destroy`/`_join`, so
 * no OS handle is ever leaked.
 *
 * THREADING MODEL. A worker is an `osp_thread_fn`: it takes a `void *` argument
 * and returns a `void *` result (the pthreads shape, mirrored on Windows by a
 * trampoline). `osp_thread_join` hands that result back and frees the handle.
 * Mutexes are non-recursive; a condition variable is always waited on while
 * holding its paired mutex, exactly as with pthreads.
 *
 * BUILD. Compiled by platform-harness. The POSIX backend links the OS thread
 * library (`-pthread`, via PLATFORM_LIBS) and needs _POSIX_C_SOURCE; the Windows
 * backend uses only the CRT + kernel32 (linked by default). Per-OS source
 * selection is done by the BUILD file, so there are no OS #ifdefs in the code.
 */
#ifndef OS_PLATFORM_THREAD_H
#define OS_PLATFORM_THREAD_H

#include "os_platform/status.h" /* osp_status: OSP_OK / OSP_ERR_* */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Threads ────────────────────────────────────────────────────────────── */

/* Opaque thread handle. Created by osp_thread_spawn, consumed by
 * osp_thread_join (which frees it). Do not copy or free it yourself. */
typedef struct osp_thread osp_thread;

/* A thread body: receives its argument, returns a result pointer. The result is
 * delivered to whoever joins the thread. */
typedef void *(*osp_thread_fn)(void *arg);

/*
 * osp_thread_spawn — start `fn(arg)` on a new OS thread.
 *
 * On success writes a handle through *out and returns OSP_OK. The new thread
 * runs concurrently until it returns from `fn`. Returns OSP_ERR_INVAL if `out`
 * or `fn` is NULL, OSP_ERR_NOMEM if the handle allocation fails, OSP_ERR_OS if
 * the OS refuses to create the thread.
 */
osp_status osp_thread_spawn(osp_thread **out, osp_thread_fn fn, void *arg);

/*
 * osp_thread_join — wait for `t` to finish, then free it.
 *
 * Blocks until the thread's `fn` returns. If `retval_out` is non-NULL, the
 * thread's result pointer is written there. The handle `t` is freed and must not
 * be used again. Returns OSP_ERR_INVAL if `t` is NULL, OSP_ERR_OS on a join
 * failure.
 */
osp_status osp_thread_join(osp_thread *t, void **retval_out);

/* ── Mutexes (non-recursive mutual exclusion) ───────────────────────────── */

typedef struct osp_mutex osp_mutex;

/* Create a mutex (initially unlocked). OSP_ERR_INVAL if out is NULL,
 * OSP_ERR_NOMEM / OSP_ERR_OS on failure. */
osp_status osp_mutex_init(osp_mutex **out);
/* Lock the mutex, blocking until it is available. */
osp_status osp_mutex_lock(osp_mutex *m);
/* Unlock a mutex the caller holds. */
osp_status osp_mutex_unlock(osp_mutex *m);
/* Destroy a mutex and free it. It must be unlocked and unused. */
osp_status osp_mutex_destroy(osp_mutex *m);

/* ── Condition variables (wait/notify, paired with a mutex) ─────────────── */

typedef struct osp_cond osp_cond;

/* Create a condition variable. OSP_ERR_INVAL if out is NULL,
 * OSP_ERR_NOMEM / OSP_ERR_OS on failure. */
osp_status osp_cond_init(osp_cond **out);
/*
 * osp_cond_wait — atomically release `m` and sleep until signalled, then
 * re-acquire `m` before returning. The caller MUST hold `m` on entry. Guard
 * against spurious wake-ups by re-checking your predicate in a loop:
 *
 *     osp_mutex_lock(m);
 *     while (!ready) osp_cond_wait(c, m);
 *     osp_mutex_unlock(m);
 */
osp_status osp_cond_wait(osp_cond *c, osp_mutex *m);
/* Wake at least one waiter (if any). */
osp_status osp_cond_signal(osp_cond *c);
/* Wake all current waiters. */
osp_status osp_cond_broadcast(osp_cond *c);
/* Destroy a condition variable and free it. It must have no waiters. */
osp_status osp_cond_destroy(osp_cond *c);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_THREAD_H */
