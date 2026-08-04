/* Tests for ge225-simulator, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests. */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>

#include "ge225_simulator.h"

/* Rust test helper: ins(opcode, address, modifier). */
static int32_t ins(int32_t opcode, int32_t address, int32_t modifier) {
    int32_t w = 0;
    (void)ge225_encode_instruction(opcode, modifier, address, &w);
    return w;
}
static int32_t fixed(const char *m) {
    int32_t w = 0;
    (void)ge225_assemble_fixed(m, &w);
    return w;
}
static int32_t rd(Ge225Simulator *s, int32_t addr) {
    int32_t v = 0;
    (void)ge225_read_word(s, addr, &v);
    return v;
}

int main(void) {
    /* ── encode / decode / pack round-trip ─────────────────────────────────*/
    {
        int32_t word = ins(001, 0x1234 & 0x1fff, 002);
        int32_t op, mod, addr;
        int32_t words[2];
        uint8_t *blob;
        size_t blob_len = 0;
        int32_t *unpacked;
        size_t un = 0;
        ge225_decode_instruction(word, &op, &mod, &addr);
        ISO_CHECK(op == 001 && mod == 002 && addr == (0x1234 & 0x1fff));

        words[0] = word;
        words[1] = fixed("NOP");
        blob = ge225_pack_words(words, 2, &blob_len);
        ISO_CHECK(blob != NULL && blob_len == 6);
        ISO_CHECK(ge225_unpack_words(blob, blob_len, &unpacked, &un) == GE_OK);
        ISO_CHECK(un == 2 && unpacked[0] == word && unpacked[1] == words[1]);
        free(blob);
        free(unpacked);

        /* range errors */
        ISO_CHECK(ge225_encode_instruction(0100, 0, 0, &op) == GE_ERR_RANGE);
        /* odd byte length */
        {
            static const uint8_t odd[4] = {0, 0, 0, 0};
            int32_t *w2;
            size_t n2;
            ISO_CHECK(ge225_unpack_words(odd, 4, &w2, &n2) ==
                      GE_ERR_ODD_BYTE_LENGTH);
        }
    }

    /* ── LDA / ADD / STA program ───────────────────────────────────────────*/
    {
        Ge225Simulator *s = ge225_new(4096);
        int32_t prog[13];
        ISO_CHECK(s != NULL);
        prog[0] = ins(000, 10, 0);
        prog[1] = ins(001, 11, 0);
        prog[2] = ins(003, 12, 0);
        prog[3] = fixed("NOP");
        prog[4] = 0; prog[5] = 0; prog[6] = 0; prog[7] = 0; prog[8] = 0;
        prog[9] = 0; prog[10] = 1; prog[11] = 2; prog[12] = 0;
        ISO_CHECK(ge225_load_words(s, prog, 13, 0) == GE_OK);
        ISO_CHECK(ge225_run(s, 4) == GE_OK);
        ISO_CHECK(ge225_get_a(s) == 3);
        ISO_CHECK(rd(s, 12) == 3);
        ge225_free(s);
    }

    /* ── SPB stores P ──────────────────────────────────────────────────────*/
    {
        Ge225Simulator *s = ge225_new(4096);
        int32_t prog[11];
        ISO_CHECK(s != NULL);
        prog[0] = ins(007, 4, 2);
        prog[1] = fixed("NOP");
        prog[2] = fixed("NOP");
        prog[3] = fixed("NOP");
        prog[4] = ins(000, 10, 0);
        prog[5] = fixed("NOP");
        prog[6] = 0; prog[7] = 0; prog[8] = 0; prog[9] = 0; prog[10] = 0x12345;
        ISO_CHECK(ge225_load_words(s, prog, 11, 0) == GE_OK);
        ISO_CHECK(ge225_run(s, 3) == GE_OK);
        ISO_CHECK(ge225_get_x_word(s, 2) == 0);
        ISO_CHECK(ge225_get_a(s) == 0x12345);
        ge225_free(s);
    }

    /* ── odd-address double ops (DLD / DST) ────────────────────────────────*/
    {
        Ge225Simulator *s = ge225_new(4096);
        int32_t prog[3];
        ISO_CHECK(s != NULL);
        ISO_CHECK(ge225_write_word(s, 11, 0x13579) == GE_OK);
        prog[0] = ins(010, 11, 0);
        prog[1] = ins(013, 13, 0);
        prog[2] = fixed("NOP");
        ISO_CHECK(ge225_load_words(s, prog, 3, 0) == GE_OK);
        ISO_CHECK(ge225_run(s, 3) == GE_OK);
        ISO_CHECK(ge225_get_a(s) == 0x13579);
        ISO_CHECK(ge225_get_q(s) == 0x13579);
        ISO_CHECK(rd(s, 13) == 0x13579);
        ge225_free(s);
    }

    /* ── MOY moves blocks ──────────────────────────────────────────────────*/
    {
        Ge225Simulator *s = ge225_new(4096);
        int32_t prog[6];
        ISO_CHECK(s != NULL);
        ge225_write_word(s, 20, 0x11111);
        ge225_write_word(s, 21, 0x22222);
        ge225_write_word(s, 30, 40);
        ge225_write_word(s, 31, (1 << 20) - 2);
        prog[0] = ins(000, 30, 0);
        prog[1] = fixed("LQA");
        prog[2] = ins(000, 31, 0);
        prog[3] = fixed("XAQ");
        prog[4] = ins(024, 20, 0);
        prog[5] = fixed("NOP");
        ISO_CHECK(ge225_load_words(s, prog, 6, 0) == GE_OK);
        ISO_CHECK(ge225_run(s, 6) == GE_OK);
        ISO_CHECK(ge225_get_a(s) == 0);
        ISO_CHECK(rd(s, 40) == 0x11111);
        ISO_CHECK(rd(s, 41) == 0x22222);
        ge225_free(s);
    }

    /* ── console typewriter path (RCS / TON / SAN 6 / TYP) ─────────────────*/
    {
        Ge225Simulator *s = ge225_new(4096);
        int32_t prog[5];
        char out[16];
        ISO_CHECK(s != NULL);
        ge225_set_control_switches(s, 01633);
        prog[0] = fixed("RCS");
        prog[1] = fixed("TON");
        (void)ge225_assemble_shift("SAN", 6, &prog[2]);
        prog[3] = fixed("TYP");
        prog[4] = fixed("NOP");
        ISO_CHECK(ge225_load_words(s, prog, 5, 0) == GE_OK);
        ISO_CHECK(ge225_run(s, 5) == GE_OK);
        ge225_typewriter_output(s, out, sizeof out);
        ISO_CHECK_STR_EQ(out, "-");
        ISO_CHECK(ge225_get_typewriter_power(s));
        ge225_free(s);
    }

    /* ── RCD loads a queued card record ────────────────────────────────────*/
    {
        Ge225Simulator *s = ge225_new(4096);
        int32_t rec[2] = {0x11111, 0x22222};
        int32_t prog[2];
        ISO_CHECK(s != NULL);
        ISO_CHECK(ge225_queue_card_reader_record(s, rec, 2) == GE_OK);
        prog[0] = ins(025, 10, 0);
        prog[1] = fixed("NOP");
        ISO_CHECK(ge225_load_words(s, prog, 2, 0) == GE_OK);
        ISO_CHECK(ge225_run(s, 2) == GE_OK);
        ISO_CHECK(rd(s, 10) == 0x11111);
        ISO_CHECK(rd(s, 11) == 0x22222);
        ge225_free(s);
    }

    /* ── disassembly + a divide-by-zero error path ─────────────────────────*/
    {
        Ge225Simulator *s = ge225_new(256);
        char buf[32];
        ISO_CHECK(s != NULL);
        ISO_CHECK(ge225_disassemble_word(s, fixed("NOP"), buf, sizeof buf) ==
                  GE_OK);
        ISO_CHECK_STR_EQ(buf, "NOP");
        ISO_CHECK(ge225_disassemble_word(s, ins(001, 0x123, 2), buf,
                                         sizeof buf) == GE_OK);
        ISO_CHECK_STR_EQ(buf, "ADD 0x123,X2");
        /* DVD by zero: mem[5]=0 (divisor); program DVD 5 */
        {
            int32_t prog[1];
            prog[0] = ins(016, 5, 0); /* DVD addr 5 (mem[5]==0) */
            (void)ge225_load_words(s, prog, 1, 0);
            ISO_CHECK(ge225_step(s, NULL) == GE_ERR_DIVIDE_BY_ZERO);
        }
        ge225_free(s);
    }

    return ISO_TEST_RESULT();
}
