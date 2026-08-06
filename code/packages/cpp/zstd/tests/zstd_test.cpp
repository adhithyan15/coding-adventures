// Tests for the C++ zstd (CMP07), using the iso_test.h harness. Test cases
// TC-1..TC-10 mirror code/specs/CMP07-zstd.md exactly; additional cases probe
// internal FSE-codec helpers directly (mirroring the Rust crate's own
// low-level unit tests) and TC-9 in particular is the one that actually
// proves RFC 8878 conformance — see the "THE FSE BUG CLASS" banner in
// include/zstd.hpp for why a same-codebase round-trip test alone can never
// catch a systematic, symmetric wire-format deviation.
#include "iso_test.h"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <string>
#include <vector>

#include "zstd.hpp"

namespace zstd = ca::zstd;
using Bytes = std::vector<std::uint8_t>;

static Bytes bytes_of(const std::string& s) {
    return Bytes(s.begin(), s.end());
}

static Bytes rt(const Bytes& data) {
    return zstd::decompress(zstd::compress(data));
}

// ─── File helpers for TC-9 CLI interop ─────────────────────────────────────
//
// Pure ISO C++ has no process-spawning API, so we shell out via
// std::system() (declared in <cstdlib>, standard) with shell redirection —
// the same approach the harness's own build scripts use, just from C++.
// std::system() itself is entirely standard; what it invokes is platform
// shell syntax, which is why this is gated behind a CLI-availability probe
// and skips gracefully rather than failing when `zstd` isn't on PATH.

#ifdef _WIN32
static const char* NULL_DEVICE = "NUL";
#else
static const char* NULL_DEVICE = "/dev/null";
#endif

static void write_binary_file(const std::string& path, const Bytes& data) {
    std::ofstream f(path, std::ios::binary | std::ios::trunc);
    if (!data.empty()) {
        f.write(reinterpret_cast<const char*>(data.data()), static_cast<std::streamsize>(data.size()));
    }
}

static Bytes read_binary_file(const std::string& path) {
    std::ifstream f(path, std::ios::binary);
    Bytes out((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
    return out;
}

static bool is_zstd_cli_available() {
    const std::string cmd = std::string("zstd --version > ") + NULL_DEVICE + " 2>" + NULL_DEVICE;
    return std::system(cmd.c_str()) == 0;
}

// Runs `zstd <args>`, returning true iff it exited successfully.
static bool run_zstd(const std::string& args) {
    const std::string cmd = "zstd " + args;
    return std::system(cmd.c_str()) == 0;
}

int main() {
    // ── TC-1: empty input ───────────────────────────────────────────────
    ISO_CHECK(rt(Bytes{}) == Bytes{});

    // ── TC-2: single byte ───────────────────────────────────────────────
    ISO_CHECK(rt(Bytes{0x42}) == Bytes{0x42});

    // ── TC-3: all 256 byte values ───────────────────────────────────────
    {
        Bytes input;
        for (int i = 0; i < 256; ++i) {
            input.push_back(static_cast<std::uint8_t>(i));
        }
        ISO_CHECK(rt(input) == input);
    }

    // ── TC-4: highly repetitive data (RLE) ──────────────────────────────
    {
        Bytes input(1024, static_cast<std::uint8_t>('A'));
        Bytes compressed = zstd::compress(input);
        ISO_CHECK(zstd::decompress(compressed) == input);
        ISO_CHECK_MSG(compressed.size() < 30, "RLE of 1024 bytes should compress to < 30 bytes");
    }

    // ── TC-5: English prose ─────────────────────────────────────────────
    {
        std::string text;
        for (int i = 0; i < 25; ++i) {
            text += "the quick brown fox jumps over the lazy dog ";
        }
        Bytes input = bytes_of(text);
        Bytes compressed = zstd::compress(input);
        ISO_CHECK(zstd::decompress(compressed) == input);
        std::size_t threshold = input.size() * 80 / 100;
        ISO_CHECK_MSG(compressed.size() < threshold, "prose should compress to < 80% of input size");
    }

    // ── TC-6: pseudo-random binary blob (LCG seed=42) ───────────────────
    {
        std::uint32_t seed = 42;
        Bytes input;
        for (int i = 0; i < 512; ++i) {
            seed = static_cast<std::uint32_t>(seed * 1664525u + 1013904223u);
            input.push_back(static_cast<std::uint8_t>(seed & 0xFF));
        }
        ISO_CHECK(rt(input) == input);
    }

    // ── TC-7: multi-block frame (200 KB > 128 KB block cap) ─────────────
    {
        Bytes input(200 * 1024, static_cast<std::uint8_t>('x'));
        ISO_CHECK(rt(input) == input);
    }

    // ── TC-8: repeat-offset pattern ─────────────────────────────────────
    {
        Bytes pattern = bytes_of("ABCDEFGH");
        Bytes input = pattern;
        for (int i = 0; i < 10; ++i) {
            input.insert(input.end(), 128, static_cast<std::uint8_t>('X'));
            input.insert(input.end(), pattern.begin(), pattern.end());
        }
        Bytes compressed = zstd::compress(input);
        ISO_CHECK(zstd::decompress(compressed) == input);
        std::size_t threshold = input.size() * 70 / 100;
        ISO_CHECK_MSG(compressed.size() < threshold,
                      "repeat-offset pattern should compress to < 70% of input size");
    }

    // ── TC-9: cross-language / CLI interoperability ─────────────────────
    //
    // This is the test that actually proves the wire format is real RFC
    // 8878, not a self-consistent internal format — see the module banner
    // in include/zstd.hpp ("THE FSE BUG CLASS") for the three compounding
    // bugs this class of test catches that no internal round-trip test can.
    // Gracefully skipped (not failed) when `zstd` isn't on PATH.
    if (!is_zstd_cli_available()) {
        printf("  zstd CLI not found on PATH -- skipping TC-9 interop test\n");
    } else {
        std::string text;
        for (int i = 0; i < 25; ++i) {
            text += "the quick brown fox jumps over the lazy dog ";
        }
        Bytes original = bytes_of(text);

        // Direction 1: compress with ours, decompress with real `zstd -d`.
        {
            Bytes our_compressed = zstd::compress(original);
            write_binary_file("_build/zstd_tc9_ours.zst", our_compressed);
            bool ok = run_zstd("-d -q -f -c _build/zstd_tc9_ours.zst > _build/zstd_tc9_ours.out");
            ISO_CHECK_MSG(ok, "real `zstd -d` failed to run on our compressed output");
            if (ok) {
                Bytes decoded_by_cli = read_binary_file("_build/zstd_tc9_ours.out");
                ISO_CHECK_MSG(decoded_by_cli == original,
                              "real `zstd -d` decoded our compressed output incorrectly");
            }
            std::remove("_build/zstd_tc9_ours.zst");
            std::remove("_build/zstd_tc9_ours.out");
        }

        // Direction 2: compress with real `zstd`, decompress with ours.
        {
            write_binary_file("_build/zstd_tc9_input.txt", original);
            bool ok = run_zstd("-q -f -c _build/zstd_tc9_input.txt > _build/zstd_tc9_input.zst");
            ISO_CHECK_MSG(ok, "real `zstd` failed to compress the interop input");
            if (ok) {
                Bytes their_compressed = read_binary_file("_build/zstd_tc9_input.zst");
                Bytes decoded_by_us = zstd::decompress(their_compressed);
                ISO_CHECK_MSG(decoded_by_us == original,
                              "our decompress() failed to decode real zstd's compressed output");
            }
            std::remove("_build/zstd_tc9_input.txt");
            std::remove("_build/zstd_tc9_input.zst");
        }

        // Extra regression coverage: an input large enough to push a single
        // block's sequence count past 128 — the exact boundary where
        // Number_of_Sequences' wire encoding switches from its 1-byte form
        // to its 2-byte form (RFC 8878 §3.1.1.3.1). A marker-byte-order bug
        // in that form round-trips fine against ITSELF but silently
        // produces a non-conformant frame; only real CLI interop catches
        // it. See `detail::encode_seq_count`'s doc comment.
        {
            const char* cycle = "ABCDEF";
            Bytes input;
            input.reserve(9000);
            for (int i = 0; i < 9000; ++i) {
                input.push_back(static_cast<std::uint8_t>(cycle[i % 6]));
            }
            Bytes our_compressed = zstd::compress(input);
            write_binary_file("_build/zstd_tc9_highseq.zst", our_compressed);
            bool ok = run_zstd("-d -q -f -c _build/zstd_tc9_highseq.zst > _build/zstd_tc9_highseq.out");
            ISO_CHECK_MSG(ok, "real `zstd -d` failed to run on our high-sequence-count output");
            if (ok) {
                Bytes decoded = read_binary_file("_build/zstd_tc9_highseq.out");
                ISO_CHECK_MSG(decoded == input,
                              "real `zstd -d` mis-decoded our high-sequence-count output "
                              "(likely a sequence-count wire-format regression)");
            }
            std::remove("_build/zstd_tc9_highseq.zst");
            std::remove("_build/zstd_tc9_highseq.out");
        }
    }

    // ── TC-10: hand-built minimal wire-format frame ─────────────────────
    //
    // Manually constructed ZStd frame, independent of our own encoder, to
    // verify the decoder reads the wire format correctly.
    //   [0..3]  Magic = 0xFD2FB528 LE = 28 B5 2F FD
    //   [4]     FHD = 0x20: FCS_flag=00, Single_Segment=1, rest 0.
    //           With Single_Segment=1 and FCS_flag=00, FCS is 1 byte.
    //   [5]     FCS = 0x05 (content_size = 5)
    //   [6..8]  Block header: Last=1, Type=Raw, Size=5
    //             = (5 << 3) | (0 << 1) | 1 = 41 = 0x29 -> [0x29, 0x00, 0x00]
    //   [9..13] b"hello"
    {
        Bytes frame = {0x28, 0xB5, 0x2F, 0xFD,        // magic
                       0x20,                           // FHD
                       0x05,                           // FCS = 5
                       0x29, 0x00, 0x00,                // block header
                       'h',  'e',  'l',  'l',  'o'};
        ISO_CHECK(zstd::decompress(frame) == bytes_of("hello"));
    }

    // ── Additional round trips ───────────────────────────────────────────
    ISO_CHECK(rt(bytes_of("hello world")) == bytes_of("hello world"));
    {
        Bytes input(1000, 0x00);
        ISO_CHECK(rt(input) == input);
    }
    {
        Bytes input(1000, 0xFF);
        ISO_CHECK(rt(input) == input);
    }
    {
        const char* cycle = "ABCDEF";
        Bytes input;
        for (int i = 0; i < 3000; ++i) {
            input.push_back(static_cast<std::uint8_t>(cycle[i % 6]));
        }
        ISO_CHECK(rt(input) == input);
    }
    {
        Bytes input;
        for (int i = 0; i < 300; ++i) {
            input.push_back(static_cast<std::uint8_t>(i % 256));
        }
        ISO_CHECK(rt(input) == input);
    }

    // Determinism: compressing the same data twice must produce identical
    // bytes (reproducible builds).
    {
        std::string text;
        for (int i = 0; i < 50; ++i) {
            text += "hello, ZStd world! ";
        }
        Bytes data = bytes_of(text);
        ISO_CHECK(zstd::compress(data) == zstd::compress(data));
    }

    // Malformed decompress input must throw, not crash: bad magic.
    {
        bool threw = false;
        try {
            Bytes bad = {0, 0, 0, 0, 0};
            zstd::decompress(bad);
        } catch (const zstd::ZstdError&) {
            threw = true;
        }
        ISO_CHECK_MSG(threw, "bad magic number must throw ZstdError");
    }

    // Malformed decompress input must throw: truncated frame.
    {
        bool threw = false;
        try {
            Bytes bad = {0x28, 0xB5, 0x2F, 0xFD, 0x20};
            zstd::decompress(bad);
        } catch (const zstd::ZstdError&) {
            threw = true;
        }
        ISO_CHECK_MSG(threw, "truncated frame must throw ZstdError");
    }

    // Malformed decompress input must throw: trailing garbage after a
    // well-formed frame (Lesson 94 — must be rejected, not ignored).
    {
        bool threw = false;
        try {
            Bytes frame = {0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x05, 0x29, 0x00, 0x00,
                           'h',  'e',  'l',  'l',  'o',  0xDE, 0xAD, 0xBE, 0xEF};
            zstd::decompress(frame);
        } catch (const zstd::ZstdError&) {
            threw = true;
        }
        ISO_CHECK_MSG(threw, "trailing bytes after frame end must throw ZstdError");
    }

    // Malformed decompress input must throw, not crash: a Compressed
    // block whose literals-section header claims more bytes than the block
    // actually carries.
    {
        bool threw = false;
        try {
            Bytes frame = {
                0x28, 0xB5, 0x2F, 0xFD,             // magic
                0xE0,                                 // FHD (Single_Segment, 8B FCS, no checksum)
                0x0A, 0,    0,    0,    0, 0, 0, 0,  // FCS = 10 (untrusted hint; ignored)
                0x15, 0x00, 0x00,                     // block hdr: last=1 type=Compressed(10) size=2
                0x28, 0x00,                            // Raw_Literals header claiming n=5, but only
                                                        // 1 more byte follows in this 2-byte block
            };
            zstd::decompress(frame);
        } catch (const zstd::ZstdError&) {
            threw = true;
        }
        ISO_CHECK_MSG(threw, "truncated literals section in a compressed block must throw ZstdError");
    }

    // ── Internal helper unit tests (mirror the Rust crate's own) ────────
    {
        for (std::uint32_t i = 0; i < 16; ++i) {
            ISO_CHECK_EQ_UINT(zstd::detail::ll_to_code(i), i);
        }
        for (std::uint32_t i = 3; i < 35; ++i) {
            ISO_CHECK_EQ_UINT(zstd::detail::ml_to_code(i), i - 3);
        }
    }

    {
        Bytes lits;
        for (int i = 0; i < 20; ++i) {
            lits.push_back(static_cast<std::uint8_t>(i));
        }
        Bytes enc = zstd::detail::encode_literals_section(lits);
        auto [dec, consumed] = zstd::detail::decode_literals_section(enc.data(), enc.size());
        (void)consumed;
        ISO_CHECK(dec == lits);
    }
    {
        Bytes lits;
        for (int i = 0; i < 200; ++i) {
            lits.push_back(static_cast<std::uint8_t>(i % 256));
        }
        Bytes enc = zstd::detail::encode_literals_section(lits);
        auto [dec, consumed] = zstd::detail::decode_literals_section(enc.data(), enc.size());
        (void)consumed;
        ISO_CHECK(dec == lits);
    }
    {
        Bytes lits;
        for (int i = 0; i < 5000; ++i) {
            lits.push_back(static_cast<std::uint8_t>(i % 256));
        }
        Bytes enc = zstd::detail::encode_literals_section(lits);
        auto [dec, consumed] = zstd::detail::decode_literals_section(enc.data(), enc.size());
        (void)consumed;
        ISO_CHECK(dec == lits);
    }

    // Reverse bit-writer/reader round trip: writes are read back in REVERSE
    // order (last-written bits first), mirroring how the sequences codec
    // writes the initial FSE states last so a forward decoder reads them
    // first.
    {
        zstd::detail::RevBitWriter bw;
        bw.add_bits(0b101, 3);       // A: written first -> read last
        bw.add_bits(0b11001100, 8);  // B
        bw.add_bits(0b1, 1);         // C: written last -> read first
        bw.flush();
        Bytes buf = std::move(bw).finish();

        zstd::detail::RevBitReader br(buf.data(), buf.size());
        ISO_CHECK_EQ_UINT(br.read_bits(1), 0b1u);
        ISO_CHECK_EQ_UINT(br.read_bits(8), 0b11001100u);
        ISO_CHECK_EQ_UINT(br.read_bits(3), 0b101u);
    }

    // FSE decode table coverage: every slot must hold a valid symbol index.
    {
        auto dt = zstd::detail::build_decode_table(zstd::detail::LL_NORM.data(),
                                                     zstd::detail::LL_NORM.size(), zstd::detail::LL_ACC_LOG);
        ISO_CHECK_EQ_UINT(dt.size(), std::size_t(1) << zstd::detail::LL_ACC_LOG);
        bool all_valid = true;
        for (const auto& cell : dt) {
            if (cell.sym >= zstd::detail::LL_NORM.size()) {
                all_valid = false;
            }
        }
        ISO_CHECK(all_valid);
    }

    // Sequence-count wire encoding round trip across the 1/2/3-byte
    // boundaries (RFC 8878 §3.1.1.3.1).
    {
        std::size_t values[] = {0, 1, 50, 127, 128, 1000, 0x7FFEu};
        for (std::size_t n : values) {
            Bytes enc = zstd::detail::encode_seq_count(n);
            auto [dec, consumed] = zstd::detail::decode_seq_count(enc.data(), enc.size());
            (void)consumed;
            ISO_CHECK_EQ_UINT(dec, n);
        }
    }

    // Low-level FSE sequences-codec round trip: encode a handful of
    // sequences, decode them back using the SAME peek/extras/update order
    // `detail::decompress_block` uses (see the "THE FSE BUG CLASS" banner —
    // an isolated test that agrees with itself proves nothing about wire
    // conformance on its own; TC-9 above is what actually proves that; this
    // test exists to pin the internal contract precisely).
    {
        auto decode_seqs_for_test = [](const Bytes& bitstream, std::size_t n_seqs) {
            auto dt_ll = zstd::detail::build_decode_table(zstd::detail::LL_NORM.data(),
                                                            zstd::detail::LL_NORM.size(),
                                                            zstd::detail::LL_ACC_LOG);
            auto dt_ml = zstd::detail::build_decode_table(zstd::detail::ML_NORM.data(),
                                                            zstd::detail::ML_NORM.size(),
                                                            zstd::detail::ML_ACC_LOG);
            auto dt_of = zstd::detail::build_decode_table(zstd::detail::OF_NORM.data(),
                                                            zstd::detail::OF_NORM.size(),
                                                            zstd::detail::OF_ACC_LOG);
            zstd::detail::RevBitReader br(bitstream.data(), bitstream.size());
            std::uint16_t state_ll = static_cast<std::uint16_t>(br.read_bits(zstd::detail::LL_ACC_LOG));
            std::uint16_t state_of = static_cast<std::uint16_t>(br.read_bits(zstd::detail::OF_ACC_LOG));
            std::uint16_t state_ml = static_cast<std::uint16_t>(br.read_bits(zstd::detail::ML_ACC_LOG));

            std::vector<zstd::detail::Seq> out;
            out.reserve(n_seqs);
            for (std::size_t i = 0; i < n_seqs; ++i) {
                auto ll_entry = zstd::detail::fse_peek(state_ll, dt_ll);
                auto ml_entry = zstd::detail::fse_peek(state_ml, dt_ml);
                auto of_entry = zstd::detail::fse_peek(state_of, dt_of);

                const auto& ll_info = zstd::detail::LL_CODES[ll_entry.sym];
                const auto& ml_info = zstd::detail::ML_CODES[ml_entry.sym];

                std::uint32_t of_raw = (std::uint32_t(1) << of_entry.sym) |
                                        static_cast<std::uint32_t>(br.read_bits(of_entry.sym));
                std::uint32_t ml = ml_info.baseline + static_cast<std::uint32_t>(br.read_bits(ml_info.extra_bits));
                std::uint32_t ll = ll_info.baseline + static_cast<std::uint32_t>(br.read_bits(ll_info.extra_bits));
                std::uint32_t off = of_raw - 3;

                if (i + 1 != n_seqs) {
                    state_ll = zstd::detail::fse_update_state(ll_entry, br);
                    state_ml = zstd::detail::fse_update_state(ml_entry, br);
                    state_of = zstd::detail::fse_update_state(of_entry, br);
                }

                out.push_back(zstd::detail::Seq{ll, ml, off});
            }
            return out;
        };

        // Single sequence: exercises the fse_init_state direct-formula path
        // (there is no "previous" iteration, so this is the ONLY sequence
        // and must be initialised, not transitioned into).
        {
            std::vector<zstd::detail::Seq> seqs = {{3, 5, 2}};
            Bytes bitstream = zstd::detail::encode_sequences_section(seqs);
            auto decoded = decode_seqs_for_test(bitstream, seqs.size());
            ISO_CHECK_EQ_UINT(decoded[0].ll, 3u);
            ISO_CHECK_EQ_UINT(decoded[0].ml, 5u);
            ISO_CHECK_EQ_UINT(decoded[0].off, 2u);
        }

        // Two sequences: exercises one non-last transition plus the
        // last-sequence update-skip.
        {
            std::vector<zstd::detail::Seq> seqs = {{2, 4, 1}, {0, 3, 2}};
            Bytes bitstream = zstd::detail::encode_sequences_section(seqs);
            auto decoded = decode_seqs_for_test(bitstream, seqs.size());
            for (std::size_t i = 0; i < seqs.size(); ++i) {
                ISO_CHECK_EQ_UINT(decoded[i].ll, seqs[i].ll);
                ISO_CHECK_EQ_UINT(decoded[i].ml, seqs[i].ml);
                ISO_CHECK_EQ_UINT(decoded[i].off, seqs[i].off);
            }
        }

        // Five sequences: exercises multiple non-last transitions.
        {
            std::vector<zstd::detail::Seq> seqs = {
                {1, 3, 1}, {5, 10, 4}, {0, 6, 2}, {12, 40, 100}, {3, 3, 1},
            };
            Bytes bitstream = zstd::detail::encode_sequences_section(seqs);
            auto decoded = decode_seqs_for_test(bitstream, seqs.size());
            for (std::size_t i = 0; i < seqs.size(); ++i) {
                ISO_CHECK_EQ_UINT(decoded[i].ll, seqs[i].ll);
                ISO_CHECK_EQ_UINT(decoded[i].ml, seqs[i].ml);
                ISO_CHECK_EQ_UINT(decoded[i].off, seqs[i].off);
            }
        }
    }

    return ISO_TEST_RESULT();
}
