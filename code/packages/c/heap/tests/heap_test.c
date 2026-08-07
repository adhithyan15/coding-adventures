/* Tests for the C binary heap, using the header-only iso_test.h harness.
 * Covers min/max ordering, push/pop/peek, empty behavior, and heap_sort. */
#include "iso_test.h"

#include "heap.h"

int main(void) {
    heap h;
    int out;
    int i;

    /* MIN heap: smallest pops first. */
    heap_init(&h, HEAP_MIN);
    ISO_CHECK(heap_is_empty(&h));
    ISO_CHECK(!heap_pop(&h, &out)); /* empty pop fails */
    ISO_CHECK(!heap_peek(&h, &out));

    {
        int vals[7] = {5, 3, 8, 1, 9, 2, 7};
        for (i = 0; i < 7; i++) {
            ISO_CHECK(heap_push(&h, vals[i]));
        }
    }
    ISO_CHECK_EQ_UINT(heap_len(&h), 7);
    ISO_CHECK(heap_peek(&h, &out));
    ISO_CHECK_EQ_INT(out, 1); /* min at the root */

    /* Draining yields ascending order. */
    {
        int expected[7] = {1, 2, 3, 5, 7, 8, 9};
        for (i = 0; i < 7; i++) {
            ISO_CHECK(heap_pop(&h, &out));
            ISO_CHECK_EQ_INT(out, expected[i]);
        }
    }
    ISO_CHECK(heap_is_empty(&h));
    heap_free(&h);

    /* MAX heap: largest pops first → descending drain. */
    heap_init(&h, HEAP_MAX);
    {
        int vals[5] = {4, 10, 6, 2, 8};
        for (i = 0; i < 5; i++) {
            ISO_CHECK(heap_push(&h, vals[i]));
        }
    }
    ISO_CHECK(heap_peek(&h, &out));
    ISO_CHECK_EQ_INT(out, 10);
    {
        int expected[5] = {10, 8, 6, 4, 2};
        for (i = 0; i < 5; i++) {
            ISO_CHECK(heap_pop(&h, &out));
            ISO_CHECK_EQ_INT(out, expected[i]);
        }
    }
    heap_free(&h);

    /* heap_sort sorts ascending in place; duplicates and negatives included. */
    {
        int arr[8] = {5, -3, 5, 0, 9, -3, 1, 2};
        int expected[8] = {-3, -3, 0, 1, 2, 5, 5, 9};
        ISO_CHECK(heap_sort(arr, 8));
        for (i = 0; i < 8; i++) {
            ISO_CHECK_EQ_INT(arr[i], expected[i]);
        }
        /* Sorting an empty array is a no-op success. */
        ISO_CHECK(heap_sort(arr, 0));
    }

    return ISO_TEST_RESULT();
}
