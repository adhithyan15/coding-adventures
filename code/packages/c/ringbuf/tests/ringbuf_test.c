/*
 * ringbuf_test.c — behavioral tests for the ring buffer, using the header-only
 * iso_test.h harness (pure ISO). Exercises the interesting states: empty, full,
 * wraparound, peek-vs-pop, and clear.
 */
#include "iso_test.h"

#include "ringbuf.h"

int main(void) {
    int backing[4];
    ringbuf r;
    int out;

    ringbuf_init(&r, backing, 4);

    /* Fresh buffer: empty, not full, count 0, capacity 4. */
    ISO_CHECK(ringbuf_is_empty(&r));
    ISO_CHECK(!ringbuf_is_full(&r));
    ISO_CHECK_EQ_INT(ringbuf_count(&r), 0);
    ISO_CHECK_EQ_INT(ringbuf_capacity(&r), 4);

    /* pop/peek on empty must fail and not touch `out`. */
    out = -1;
    ISO_CHECK(!ringbuf_pop(&r, &out));
    ISO_CHECK(!ringbuf_peek(&r, &out));
    ISO_CHECK_EQ_INT(out, -1);

    /* Fill it. The fifth push must fail (full). */
    ISO_CHECK(ringbuf_push(&r, 10));
    ISO_CHECK(ringbuf_push(&r, 20));
    ISO_CHECK(ringbuf_push(&r, 30));
    ISO_CHECK(ringbuf_push(&r, 40));
    ISO_CHECK(ringbuf_is_full(&r));
    ISO_CHECK(!ringbuf_push(&r, 50));
    ISO_CHECK_EQ_INT(ringbuf_count(&r), 4);

    /* peek shows the oldest without removing it. */
    ISO_CHECK(ringbuf_peek(&r, &out));
    ISO_CHECK_EQ_INT(out, 10);
    ISO_CHECK_EQ_INT(ringbuf_count(&r), 4);

    /* FIFO order: pop returns 10, 20. */
    ISO_CHECK(ringbuf_pop(&r, &out));
    ISO_CHECK_EQ_INT(out, 10);
    ISO_CHECK(ringbuf_pop(&r, &out));
    ISO_CHECK_EQ_INT(out, 20);

    /* Two free slots now — pushing 50, 60 forces the write index to WRAP past
     * the end of the backing array, which is the behavior we most want to test. */
    ISO_CHECK(ringbuf_push(&r, 50));
    ISO_CHECK(ringbuf_push(&r, 60));
    ISO_CHECK(ringbuf_is_full(&r));

    /* Drain and confirm FIFO order survives the wraparound: 30, 40, 50, 60. */
    ISO_CHECK(ringbuf_pop(&r, &out));
    ISO_CHECK_EQ_INT(out, 30);
    ISO_CHECK(ringbuf_pop(&r, &out));
    ISO_CHECK_EQ_INT(out, 40);
    ISO_CHECK(ringbuf_pop(&r, &out));
    ISO_CHECK_EQ_INT(out, 50);
    ISO_CHECK(ringbuf_pop(&r, &out));
    ISO_CHECK_EQ_INT(out, 60);
    ISO_CHECK(ringbuf_is_empty(&r));

    /* clear() resets a partially-filled buffer to empty. */
    ISO_CHECK(ringbuf_push(&r, 99));
    ringbuf_clear(&r);
    ISO_CHECK(ringbuf_is_empty(&r));
    ISO_CHECK_EQ_INT(ringbuf_count(&r), 0);

    return ISO_TEST_RESULT();
}
