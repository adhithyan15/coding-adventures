/*
 * thread_test.c — integration tests for os_platform/thread, run on each OS.
 * ===========================================================================
 *
 * Concurrency can't be checked by a golden vector, so we assert on observable
 * outcomes that only hold when the primitives actually work:
 *
 *   1. MUTUAL EXCLUSION — four threads each do 100 000 locked increments of a
 *      shared counter. If the mutex truly serialises them the total is exactly
 *      400 000; a broken lock loses updates and the count comes up short. This
 *      is a deterministic pass/fail (not timing-dependent, so not flaky).
 *
 *   2. CONDITION-VARIABLE HANDOFF + RETURN VALUE — the main thread waits on a
 *      condition variable until a worker sets a flag and signals it, proving
 *      wait/signal wake-up and the mutex/cond pairing. The worker returns a
 *      pointer that join() must deliver back unchanged.
 *
 *   3. ARGUMENT VALIDATION — NULL arguments are rejected with OSP_ERR_INVAL.
 *
 * Only the main thread calls the ISO_CHECK macros (their counters are not
 * thread-safe); workers touch only the os-platform primitives.
 */
#include "iso_test.h"

#include "os_platform/thread.h"

#include <stddef.h> /* NULL */

#define NUM_THREADS 4
#define ITERS 100000

/* ── Test 1: mutual exclusion ───────────────────────────────────────────── */

typedef struct {
    osp_mutex *mu;
    long counter;
} counter_ctx;

static void *bump_counter(void *arg) {
    counter_ctx *ctx = (counter_ctx *)arg;
    int i;
    for (i = 0; i < ITERS; i++) {
        osp_mutex_lock(ctx->mu);
        ctx->counter++;
        osp_mutex_unlock(ctx->mu);
    }
    return NULL;
}

/* ── Test 2: condition-variable handoff ─────────────────────────────────── */

typedef struct {
    osp_mutex *mu;
    osp_cond *cv;
    int ready;
} handoff_ctx;

/* A distinct object whose address is the worker's return value — comparing
 * pointers avoids any integer-to-pointer casting in the test. */
static int producer_token = 7;

static void *producer(void *arg) {
    handoff_ctx *h = (handoff_ctx *)arg;
    osp_mutex_lock(h->mu);
    h->ready = 1;
    osp_cond_signal(h->cv);
    osp_mutex_unlock(h->mu);
    return &producer_token;
}

int main(void) {
    counter_ctx cctx;
    osp_thread *threads[NUM_THREADS];
    handoff_ctx h;
    osp_thread *pt = NULL;
    osp_thread *dummy = NULL;
    void *ret = NULL;
    int i;

    /* ── Test 1: mutual exclusion ───────────────────────────────────────── */
    ISO_CHECK(osp_mutex_init(&cctx.mu) == OSP_OK);
    cctx.counter = 0;
    for (i = 0; i < NUM_THREADS; i++) {
        ISO_CHECK(osp_thread_spawn(&threads[i], bump_counter, &cctx) == OSP_OK);
    }
    for (i = 0; i < NUM_THREADS; i++) {
        ISO_CHECK(osp_thread_join(threads[i], NULL) == OSP_OK);
    }
    ISO_CHECK_MSG(cctx.counter == (long)NUM_THREADS * ITERS,
                  "mutex must serialise increments (no lost updates)");
    ISO_CHECK(osp_mutex_destroy(cctx.mu) == OSP_OK);

    /* ── Test 2: condition-variable handoff + return value ──────────────── */
    ISO_CHECK(osp_mutex_init(&h.mu) == OSP_OK);
    ISO_CHECK(osp_cond_init(&h.cv) == OSP_OK);
    h.ready = 0;
    ISO_CHECK(osp_thread_spawn(&pt, producer, &h) == OSP_OK);

    osp_mutex_lock(h.mu);
    while (!h.ready) {                 /* loop guards against spurious wake-ups */
        ISO_CHECK(osp_cond_wait(h.cv, h.mu) == OSP_OK);
    }
    osp_mutex_unlock(h.mu);

    ISO_CHECK(osp_thread_join(pt, &ret) == OSP_OK);
    ISO_CHECK_MSG(h.ready == 1, "worker must have set the ready flag");
    ISO_CHECK_MSG(ret == &producer_token, "join must return the worker's result");
    ISO_CHECK(osp_cond_destroy(h.cv) == OSP_OK);
    ISO_CHECK(osp_mutex_destroy(h.mu) == OSP_OK);

    /* ── Test 3: argument validation ────────────────────────────────────── */
    ISO_CHECK(osp_thread_spawn(NULL, producer, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_thread_spawn(&dummy, NULL, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_thread_join(NULL, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_mutex_lock(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_mutex_unlock(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_cond_signal(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_cond_broadcast(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_cond_wait(NULL, NULL) == OSP_ERR_INVAL);

    return ISO_TEST_RESULT();
}
