/* Tests for the C intel-4004-assembler, using the header-only iso_test.h harness
 * (pure ISO). The reference vector is from the Rust crate; the rest are hand-
 * computed from the documented encoding table. */
#include "iso_test.h"

#include <stdlib.h> /* free */

#include "intel_4004_assembler.h"

/* Assemble `text` and assert the output equals `want` (length `n`). */
static void check_asm(const char *text, const uint8_t *want, size_t n) {
    uint8_t *out = NULL;
    size_t out_len = 0;
    char err[128];
    I4004Status st = i4004_assemble(text, &out, &out_len, err, sizeof err);
    ISO_CHECK_MSG(st == I4004_OK, err);
    ISO_CHECK_EQ_UINT(out_len, n);
    if (st == I4004_OK && out_len == n) {
        ISO_CHECK_MEM_EQ(out, want, n);
    }
    free(out);
}

/* Assert that assembling `text` fails with I4004_ERROR. */
static void check_err(const char *text) {
    uint8_t *out = NULL;
    size_t out_len = 0;
    char err[128];
    I4004Status st = i4004_assemble(text, &out, &out_len, err, sizeof err);
    ISO_CHECK_EQ_INT((int)st, (int)I4004_ERROR);
    free(out);
}

int main(void) {
    /* ── Rust reference vector ──────────────────────────────────────────── */
    {
        uint8_t want[] = {0xD5, 0xB2, 0x01};
        check_asm("ORG 0x000\nLDM 5\nXCH R2\nHLT\n", want, 3);
    }

    /* ── one-byte instructions ──────────────────────────────────────────── */
    {
        uint8_t want[] = {0x00, 0x01, 0xE0, 0x63, 0x81, 0x94,
                          0xA5, 0xB2, 0xC0, 0xD5};
        check_asm("NOP\nHLT\nWRM\nINC R3\nADD R1\nSUB R4\nLD R5\nXCH R2\n"
                  "BBL 0\nLDM 5\n",
                  want, 10);
    }

    /* ── register pairs: SRC / FIN / JIN ────────────────────────────────── */
    {
        uint8_t want[] = {0x23, 0x34, 0x31}; /* SRC P1, FIN P2, JIN P0 */
        check_asm("SRC P1\nFIN P2\nJIN P0\n", want, 3);
    }

    /* ── two-byte instructions ──────────────────────────────────────────── */
    {
        uint8_t want[] = {0x20, 0xAB}; /* FIM P0, 0xAB */
        check_asm("FIM P0, 0xAB\n", want, 2);
    }
    {
        uint8_t want[] = {0x12, 0x10}; /* JCN 0x2, 0x10 */
        check_asm("JCN 0x2, 0x10\n", want, 2);
    }
    {
        uint8_t want[] = {0x41, 0x23}; /* JUN 0x123 */
        check_asm("JUN 0x123\n", want, 2);
    }
    {
        uint8_t want[] = {0x52, 0x34}; /* JMS 0x234 */
        check_asm("JMS 0x234\n", want, 2);
    }
    {
        uint8_t want[] = {0x73, 0x40}; /* ISZ R3, 0x40 */
        check_asm("ISZ R3, 0x40\n", want, 2);
    }
    {
        uint8_t want[] = {0xD5, 0x82}; /* ADD_IMM (op0 ignored), R2, 5 */
        check_asm("ADD_IMM acc, R2, 5\n", want, 2);
    }

    /* ── labels resolve to the program counter ──────────────────────────── */
    {
        uint8_t want[] = {0xD5, 0x40, 0x00}; /* LDM 5; JUN start(=0) */
        check_asm("ORG 0x000\nstart: LDM 5\nJUN start\n", want, 3);
    }

    /* ── forward ORG pads with zeros ────────────────────────────────────── */
    {
        uint8_t want[] = {0x00, 0x00, 0x00, 0x01};
        check_asm("ORG 3\nHLT\n", want, 4);
    }

    /* ── comments and blank lines are ignored ───────────────────────────── */
    {
        uint8_t want[] = {0x01};
        check_asm("  ; a comment\n\nHLT   ; inline comment\n", want, 1);
    }

    /* ── error cases ────────────────────────────────────────────────────── */
    check_err("FOO\n");           /* unknown mnemonic */
    check_err("LDM\n");           /* wrong operand count */
    check_err("LDM missing\n");   /* unknown symbol */
    check_err("INC Rx\n");        /* invalid register */
    check_err("JUN 0x10000\n");   /* number out of range */
    check_err("ORG\n");           /* ORG without operand */

    return ISO_TEST_RESULT();
}
