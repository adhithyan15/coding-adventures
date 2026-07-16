/*
 * jit_test.c — emit real machine code, commit it, and call it, on every OS+arch.
 * ===========================================================================
 *
 * This is the execute-and-call test mmap.h's scope note deferred. It emits the
 * machine code for `int f(void){ return N; }` for the current CPU architecture,
 * runs it through the jit primitive (alloc → write → commit), casts the entry to
 * a function pointer, calls it, and asserts it returns N. Two different constants
 * prove the CPU really executes the bytes we wrote, not a fixed stub.
 *
 * The machine code is the only architecture-specific thing here — a small byte
 * table selected by #if, exactly the per-arch data mmap.h anticipated. Encodings
 * (verified: arm64 run live, x86_64 disassembled):
 *
 *   int f(void){return N;}
 *     x86_64 : B8 <imm32-LE> C3            mov eax, N ; ret
 *     arm64  : <movz w0,#N LE> C0 03 5F D6 movz w0, #N ; ret   (N must fit imm16)
 *
 * The entry is called via a memcpy'd function pointer — the portable idiom for
 * turning a void* into a callable (object and function pointers need not share a
 * representation in ISO C, though they do on every target here).
 */
#include "iso_test.h"

#include "os_platform/jit.h"

#include <stddef.h> /* NULL */
#include <string.h> /* memcpy */

#if defined(__x86_64__) || defined(_M_X64)
/* mov eax, imm32 (little-endian) ; ret */
static const unsigned char CODE_42[] = {0xB8, 0x2A, 0x00, 0x00, 0x00, 0xC3};
static const unsigned char CODE_1337[] = {0xB8, 0x39, 0x05, 0x00, 0x00, 0xC3};
#elif defined(__aarch64__) || defined(_M_ARM64)
/* movz w0, #imm16 (little-endian) ; ret */
static const unsigned char CODE_42[] = {0x40, 0x05, 0x80, 0x52, 0xC0, 0x03, 0x5F, 0xD6};
static const unsigned char CODE_1337[] = {0x20, 0xA7, 0x80, 0x52, 0xC0, 0x03, 0x5F, 0xD6};
#else
#error "jit_test: no machine code for this architecture (need x86_64 or arm64)"
#endif

typedef int (*osp_fn0)(void);

/* Build a JIT function from `code`, call it, and store its result in *out.
 * Returns 1 on success, 0 if any jit step failed. */
static int osp__build_and_call(const unsigned char *code, size_t n, int *out) {
    osp_jit *j = NULL;
    void *entry;
    osp_fn0 fn;

    if (osp_jit_alloc(&j, n) != OSP_OK) {
        return 0;
    }
    if (osp_jit_write(j, code, n) != OSP_OK) {
        osp_jit_free(j);
        return 0;
    }
    if (osp_jit_commit(j) != OSP_OK) {
        osp_jit_free(j);
        return 0;
    }
    entry = osp_jit_entry(j);
    if (entry == NULL) {
        osp_jit_free(j);
        return 0;
    }
    memcpy(&fn, &entry, sizeof(fn)); /* void* -> function pointer */
    *out = fn();
    osp_jit_free(j);
    return 1;
}

int main(void) {
    int r = 0;
    osp_jit *j = NULL;
    osp_jit *tmp = NULL;
    unsigned char filler[8] = {0};

    /* ── emit and call: the CPU returns exactly what we wrote ────────────── */
    ISO_CHECK(osp__build_and_call(CODE_42, sizeof(CODE_42), &r));
    ISO_CHECK_EQ_INT(r, 42);
    ISO_CHECK(osp__build_and_call(CODE_1337, sizeof(CODE_1337), &r));
    ISO_CHECK_EQ_INT(r, 1337);

    /* ── lifecycle: entry is NULL until commit; no writes after commit ───── */
    ISO_CHECK(osp_jit_alloc(&j, 16) == OSP_OK);
    ISO_CHECK(osp_jit_entry(j) == NULL); /* not committed yet */
    ISO_CHECK(osp_jit_write(j, CODE_42, sizeof(CODE_42)) == OSP_OK);
    ISO_CHECK(osp_jit_entry(j) == NULL); /* still not committed */
    ISO_CHECK(osp_jit_commit(j) == OSP_OK);
    ISO_CHECK(osp_jit_entry(j) != NULL);                                   /* now callable */
    ISO_CHECK(osp_jit_write(j, CODE_42, sizeof(CODE_42)) == OSP_ERR_INVAL); /* no writes after commit */
    ISO_CHECK(osp_jit_commit(j) == OSP_ERR_INVAL);                          /* no double commit */
    ISO_CHECK(osp_jit_free(j) == OSP_OK);

    /* ── capacity is enforced against the requested size ─────────────────── */
    ISO_CHECK(osp_jit_alloc(&tmp, 6) == OSP_OK);
    ISO_CHECK(osp_jit_write(tmp, filler, 6) == OSP_OK);         /* fills capacity */
    ISO_CHECK(osp_jit_write(tmp, filler, 1) == OSP_ERR_INVAL);  /* would exceed capacity */
    ISO_CHECK(osp_jit_free(tmp) == OSP_OK);

    /* ── NULL-argument validation ────────────────────────────────────────── */
    tmp = NULL;
    ISO_CHECK(osp_jit_alloc(NULL, 16) == OSP_ERR_INVAL);
    ISO_CHECK(osp_jit_alloc(&tmp, 0) == OSP_ERR_INVAL);
    ISO_CHECK(osp_jit_write(NULL, CODE_42, 1) == OSP_ERR_INVAL);
    ISO_CHECK(osp_jit_commit(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_jit_entry(NULL) == NULL);
    ISO_CHECK(osp_jit_free(NULL) == OSP_ERR_INVAL);

    return ISO_TEST_RESULT();
}
