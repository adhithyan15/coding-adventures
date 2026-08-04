/*
 * mmap_test.c — anonymous-memory + protection tests for os_platform/mmap.
 * ===========================================================================
 *
 * Fully portable (no architecture-specific or JIT code): it exercises the parts
 * of the primitive that behave identically on every OS.
 *
 *   - map anonymous READ|WRITE memory; confirm base != NULL and size == request;
 *   - the fresh mapping is zero-filled (mmap / VirtualAlloc guarantee this);
 *   - write a byte pattern across a full page and read it back via a checksum
 *     (proves the pages are real, committed, writable memory);
 *   - re-protect to READ-only; reading still works (we do NOT test that writing
 *     now faults — that would crash the process by design);
 *   - unmap; then NULL / zero-length argument validation.
 *
 * The executable (EXEC) protection is plumbed through the API but a JIT
 * execute-and-call test is a separate follow-up (it needs per-architecture
 * machine code and, on Apple Silicon, the MAP_JIT write-protect protocol).
 */
#include "iso_test.h"

#include "os_platform/mmap.h"

#include <stddef.h> /* NULL */

#define PAGE_LEN 4096

int main(void) {
    osp_mapping *m = NULL;
    unsigned char *p;
    size_t i;
    unsigned long sum;

    /* ── anonymous READ|WRITE mapping ───────────────────────────────────── */
    ISO_CHECK(osp_map_anon(&m, PAGE_LEN, OSP_PROT_READ | OSP_PROT_WRITE) == OSP_OK);
    ISO_CHECK_MSG(osp_map_base(m) != NULL, "mapping base must be non-NULL");
    ISO_CHECK_EQ_UINT(osp_map_size(m), (unsigned long)PAGE_LEN);

    p = (unsigned char *)osp_map_base(m);

    /* fresh anonymous memory is zero-filled */
    ISO_CHECK_MSG(p[0] == 0 && p[PAGE_LEN - 1] == 0, "new mapping must be zeroed");

    /* write a pattern, then read it back through a checksum. Sum of (i & 0xFF)
     * over one page = 16 full 0..255 runs = 16 * 32640 = 522240. */
    for (i = 0; i < PAGE_LEN; i++) {
        p[i] = (unsigned char)(i & 0xFF);
    }
    sum = 0;
    for (i = 0; i < PAGE_LEN; i++) {
        sum += p[i];
    }
    ISO_CHECK_EQ_UINT(sum, 522240UL);

    /* ── re-protect to READ-only; reads still succeed ───────────────────── */
    ISO_CHECK(osp_map_protect(m, OSP_PROT_READ) == OSP_OK);
    ISO_CHECK_EQ_INT(p[0], 0);
    ISO_CHECK_EQ_INT(p[255], 255);

    /* ── unmap ──────────────────────────────────────────────────────────── */
    ISO_CHECK(osp_map_unmap(m) == OSP_OK);

    /* ── argument validation ────────────────────────────────────────────── */
    ISO_CHECK(osp_map_anon(NULL, PAGE_LEN, OSP_PROT_READ) == OSP_ERR_INVAL);
    ISO_CHECK(osp_map_anon(&m, 0, OSP_PROT_READ) == OSP_ERR_INVAL);
    ISO_CHECK(osp_map_protect(NULL, OSP_PROT_READ) == OSP_ERR_INVAL);
    ISO_CHECK(osp_map_base(NULL) == NULL);
    ISO_CHECK_EQ_UINT(osp_map_size(NULL), 0UL);
    ISO_CHECK(osp_map_unmap(NULL) == OSP_ERR_INVAL);

    return ISO_TEST_RESULT();
}
