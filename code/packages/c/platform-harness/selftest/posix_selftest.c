/*
 * posix_selftest.c — proves platform-harness can build and RUN OS-dependent C.
 *
 * It spawns a POSIX thread, so it exercises three things the pure-ISO iso-harness
 * cannot: it compiles <pthread.h> under -Wall -Wextra -Werror WITHOUT
 * -pedantic-errors, it links an OS-provided library (-pthread via PLATFORM_LIBS),
 * and it runs the resulting binary. If this fails, the platform harness or its
 * flags/link line are broken.
 *
 * (iso_test.h is reused from the sibling iso-harness via PLATFORM_INCLUDE.)
 */
#include "iso_test.h"

#include <pthread.h>

static void *worker(void *arg) {
    int *counter = (int *)arg;
    *counter += 1;
    return NULL;
}

int main(void) {
    pthread_t t;
    int counter = 0;

    /* The main thread does not touch `counter` until after the join, so the
     * join's happens-before edge means there is no data race. */
    ISO_CHECK(pthread_create(&t, NULL, worker, &counter) == 0);
    ISO_CHECK(pthread_join(t, NULL) == 0);
    ISO_CHECK_EQ_INT(counter, 1);

    return ISO_TEST_RESULT();
}
