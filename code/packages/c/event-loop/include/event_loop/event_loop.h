/*
 * event_loop/event_loop.h — a pluggable, generic event loop.
 * ===========================================================================
 *
 * The C port of the Rust `event-loop` crate, and the first consumer of the
 * CCPP02 **thread** slice: it proves a real deferred crate runs on os-platform's
 * concurrency primitives (mutex + thread) without a line of per-OS code.
 *
 * WHAT IS AN EVENT LOOP? The outermost structure of any interactive program. It
 * runs until told to stop, repeatedly asking "did anything happen?" and handing
 * whatever happened to registered handlers:
 *
 *      while running:
 *          collect events from every source     (Phase 1 — must not block)
 *          for each event, for each handler:     (Phase 2 — dispatch in order)
 *              if a handler says EVL_EXIT → stop
 *          if nothing arrived → nap briefly      (Phase 3 — don't spin at 100%)
 *
 * GENERIC OVER THE EVENT TYPE. The Rust original is `EventLoop<E>`; C has no
 * generics, so an event is an opaque `const void *` and the caller defines what
 * it points at — exactly the `void *` + function-pointer idiom the rest of this
 * repo uses (cf. tcp_runtime's handler). A *source* is a poll function plus its
 * state; a *handler* is a function plus its captured state (`user`).
 *
 * THE THREADING STORY (why this is a "thread"-bucket port). `run()` itself is
 * single-threaded: sources and handlers all run on the thread that calls it. The
 * concurrency is the STOP SIGNAL — a handle you can hand to another thread (or a
 * timer callback) to stop the loop from outside. That handle wraps a mutex-guarded
 * flag (os-platform's `osp_mutex`), so `evl_stop_signal` from a worker thread and
 * the loop's own stop-check never race. The idle nap uses `osp_sleep_ns`.
 *
 * BUILD. OS-agnostic — one source file, no #ifdef. Compiled with the os-platform
 * thread backend (`-pthread` on POSIX; the CRT on Windows) for the mutex, and the
 * clock backend for the idle nap.
 */
#ifndef EVENT_LOOP_EVENT_LOOP_H
#define EVENT_LOOP_EVENT_LOOP_H

#include <stddef.h> /* size_t */

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * What a handler decides after seeing an event. An enum (not a bool) so call
 * sites read for themselves — `return EVL_EXIT;` says what `return 1;` cannot.
 */
typedef enum {
    EVL_CONTINUE = 0, /* keep looping — there is more work to do */
    EVL_EXIT = 1      /* stop the loop immediately after this event */
} evl_control;

/*
 * A source's poll function. Writes up to `cap` event pointers into `out` and
 * returns how many it produced (0 = nothing ready right now). `ctx` is the
 * source's own mutable state.
 *
 * THE CONTRACT: poll MUST NOT BLOCK (blocking is the loop's job), and the events
 * it yields are BORROWED — each must stay valid until the loop finishes
 * dispatching this batch (in practice, until this source's next poll). The loop
 * never frees an event. A source with more than `cap` events ready returns `cap`
 * now and the rest on its next poll.
 */
typedef size_t (*evl_source_poll)(void *ctx, const void **out, size_t cap);

/*
 * A handler. Receives one event (the opaque pointer a source yielded) and its own
 * captured state `user`, and returns whether the loop should continue or exit.
 */
typedef evl_control (*evl_handler)(const void *event, void *user);

/* Opaque loop. Created by event_loop_create, freed by event_loop_destroy. */
typedef struct event_loop event_loop;

/*
 * A stop handle: a thread-safe way to stop the loop from outside a handler. It
 * points at the loop's internal stop flag, so it is valid only while the loop is
 * alive — quiesce every thread holding one before event_loop_destroy, as with any
 * shared object. `evl_stop_signal` is safe to call from any thread.
 */
typedef struct evl_stop evl_stop;

/* Create an empty loop. OSP_ERR_INVAL (out NULL), OSP_ERR_NOMEM / OSP_ERR_OS. */
osp_status event_loop_create(event_loop **out);

/*
 * Register an event source (`poll` + its state `ctx`). Sources are polled in
 * registration order, once per loop iteration. OSP_ERR_INVAL / OSP_ERR_NOMEM.
 */
osp_status event_loop_add_source(event_loop *lp, evl_source_poll poll, void *ctx);

/*
 * Register a handler (`fn` + its captured state `user`). Handlers see each event
 * in registration order; the first to return EVL_EXIT stops the loop immediately,
 * so later handlers (and later events) are not called. OSP_ERR_INVAL / OSP_ERR_NOMEM.
 */
osp_status event_loop_on_event(event_loop *lp, evl_handler fn, void *user);

/*
 * A stop handle for this loop (mirrors Rust `EventLoop::stop_handle`). Borrowed —
 * owned by the loop, do not free; valid while the loop is alive. NULL if lp is NULL.
 */
evl_stop *event_loop_stop_handle(event_loop *lp);

/* Ask the loop to exit on its next iteration. Thread-safe. OSP_ERR_INVAL if NULL. */
osp_status event_loop_stop(event_loop *lp);

/*
 * event_loop_stop via a handle (from another thread or a callback without the
 * loop). Thread-safe. OSP_ERR_INVAL if h is NULL.
 */
osp_status evl_stop_signal(evl_stop *h);

/*
 * Run the loop. Blocks until a handler returns EVL_EXIT or the loop is stopped
 * (via event_loop_stop / evl_stop_signal). Clears the stop flag on entry, so a
 * loop may be run again — which also means a stop signalled BEFORE run() starts
 * is discarded; stop from another thread only after the loop is running.
 * OSP_ERR_INVAL if lp is NULL, else OSP_OK on a clean stop.
 */
osp_status event_loop_run(event_loop *lp);

/* Free the loop (its sources, handlers, and stop flag). OSP_ERR_INVAL if NULL. */
osp_status event_loop_destroy(event_loop *lp);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* EVENT_LOOP_EVENT_LOOP_H */
