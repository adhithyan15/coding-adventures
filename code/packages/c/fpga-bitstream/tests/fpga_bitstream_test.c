/*
 * Tests for the C fpga-bitstream emitter, using the header-only iso_test.h
 * harness (pure ISO). The expected byte streams are the AUTHORITATIVE output of
 * the real Rust crate (captured via a temporary oracle test):
 *   empty Hx1k  → ff00020703050004800000ffff  (13 bytes)
 *   one CLB @ (1,2) → 149 bytes (offset payload 00 01 00 02, 128-byte zero CRAM)
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>

#include "fpga_bitstream.h"

int main(void) {
    /* ── part specs ─────────────────────────────────────────────────────── */
    {
        uint32_t r, c, bits;
        fpga_part_specs(ICE40_HX1K, &r, &c, &bits);
        ISO_CHECK(r == 33 && c == 17 && bits == 1024);
        fpga_part_specs(ICE40_HX8K, &r, &c, &bits);
        ISO_CHECK(r == 33 && c == 33 && bits == 1024);
        fpga_part_specs(ICE40_UP5K, &r, &c, &bits);
        ISO_CHECK(r == 33 && c == 33 && bits == 1024);
        fpga_part_specs(ICE40_LP1K, &r, &c, &bits);
        ISO_CHECK(r == 33 && c == 17 && bits == 1024);
    }

    /* ── the empty Hx1k stream (exact oracle bytes) ─────────────────────── */
    {
        FpgaConfig *cfg = fpga_config_new(ICE40_HX1K);
        ISO_CHECK(cfg != NULL);
        size_t len = 0;
        FpgaBitstreamReport rep;
        uint8_t *bytes = fpga_emit_bitstream(cfg, &len, &rep);
        ISO_CHECK(bytes != NULL);
        ISO_CHECK_EQ_UINT(len, 13u);
        static const uint8_t expected[13] = {0xFF, 0x00, 0x02, 0x07, 0x03,
                                             0x05, 0x00, 0x04, 0x80, 0x00,
                                             0x00, 0xFF, 0xFF};
        if (len == 13) ISO_CHECK_MEM_EQ(bytes, expected, 13);
        ISO_CHECK(rep.clb_count == 0u);
        ISO_CHECK(rep.cram_size == 128u);
        ISO_CHECK(rep.bytes_written == 13u);
        ISO_CHECK(rep.part == ICE40_HX1K);
        free(bytes);
        fpga_config_free(cfg);
    }

    /* ── one CLB at (1, 2): 149 bytes with the right framing ────────────── */
    {
        FpgaConfig *cfg = fpga_config_new(ICE40_HX1K);
        FpgaClbConfig clb = fpga_clb_config_default();
        ISO_CHECK(fpga_config_insert_clb(cfg, 1, 2, &clb) == 0);
        size_t len = 0;
        FpgaBitstreamReport rep;
        uint8_t *b = fpga_emit_bitstream(cfg, &len, &rep);
        ISO_CHECK(b != NULL);
        ISO_CHECK_EQ_UINT(len, 149u);
        ISO_CHECK(rep.clb_count == 1u);

        /* Preamble + reset + bank + the CLB offset record. */
        static const uint8_t head[13] = {0xFF, 0x00, 0x02, 0x07, 0x03, 0x05, 0x00,
                                         0x06, 0x06, 0x00, 0x01, 0x00, 0x02};
        if (len == 149) {
            ISO_CHECK_MEM_EQ(b, head, 13);
            ISO_CHECK(b[13] == 0x82); /* BRAM_DATA record length = 128 + 2 */
            ISO_CHECK(b[14] == 0x08); /* CMD_BRAM_DATA */
            ISO_CHECK(b[15] == 0x00 && b[142] == 0x00); /* the 128-byte zero CRAM */
            /* CRC record, then end marker. */
            static const uint8_t tail[6] = {0x04, 0x80, 0x00, 0x00, 0xFF, 0xFF};
            ISO_CHECK_MEM_EQ(b + 143, tail, 6);
        }
        free(b);
        fpga_config_free(cfg);
    }

    /* ── insertion order does not change the stream (deterministic sort) ── */
    {
        FpgaConfig *a = fpga_config_new(ICE40_HX8K);
        FpgaConfig *d = fpga_config_new(ICE40_HX8K);
        FpgaClbConfig clb = fpga_clb_config_default();
        /* Insert three CLBs in opposite orders. */
        fpga_config_insert_clb(a, 0, 0, &clb);
        fpga_config_insert_clb(a, 2, 5, &clb);
        fpga_config_insert_clb(a, 1, 3, &clb);
        fpga_config_insert_clb(d, 1, 3, &clb);
        fpga_config_insert_clb(d, 2, 5, &clb);
        fpga_config_insert_clb(d, 0, 0, &clb);

        size_t la = 0, ld = 0;
        FpgaBitstreamReport ra, rd;
        uint8_t *ba = fpga_emit_bitstream(a, &la, &ra);
        uint8_t *bd = fpga_emit_bitstream(d, &ld, &rd);
        ISO_CHECK(ba && bd);
        ISO_CHECK_EQ_UINT(la, ld);
        if (ba && bd && la == ld) ISO_CHECK_MEM_EQ(ba, bd, la);
        free(ba);
        free(bd);
        fpga_config_free(a);
        fpga_config_free(d);
    }

    /* ── inserting the same key overwrites (HashMap semantics) ──────────── */
    {
        FpgaConfig *cfg = fpga_config_new(ICE40_HX1K);
        FpgaClbConfig clb = fpga_clb_config_default();
        fpga_config_insert_clb(cfg, 4, 4, &clb);
        fpga_config_insert_clb(cfg, 4, 4, &clb);
        ISO_CHECK_EQ_UINT(fpga_config_clb_count(cfg), 1u);
        fpga_config_free(cfg);
    }

    /* ── the cmd helper builds a record; overlong payloads are rejected ──── */
    {
        uint8_t payload[4] = {0x00, 0x01, 0x00, 0x02};
        size_t n = 0;
        uint8_t *rec = fpga_cmd(0x06, payload, 4, &n);
        ISO_CHECK(rec != NULL);
        ISO_CHECK_EQ_UINT(n, 6u);
        static const uint8_t want[6] = {0x06, 0x06, 0x00, 0x01, 0x00, 0x02};
        if (rec && n == 6) ISO_CHECK_MEM_EQ(rec, want, 6);
        free(rec);

        uint8_t *empty = fpga_cmd(0x07, NULL, 0, &n);
        ISO_CHECK(empty != NULL);
        ISO_CHECK_EQ_UINT(n, 2u);
        if (empty) {
            ISO_CHECK(empty[0] == 0x02 && empty[1] == 0x07);
        }
        free(empty);

        /* A 254-byte payload exceeds the 253 limit → NULL (the Rust panic). */
        static uint8_t big[254];
        ISO_CHECK(fpga_cmd(0x08, big, 254, &n) == NULL);
    }

    return ISO_TEST_RESULT();
}
