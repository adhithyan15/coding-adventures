// fpga_bitstream.hpp — emit iCE40 FPGA bitstreams in the Project IceStorm
// record-stream format, in pure ISO C++17, header-only, in namespace ca::fpga.
// A faithful port of the Rust `fpga-bitstream` crate.
// ===========================================================================
//
// A bitstream programs an FPGA's configuration RAM at power-on. The iCE40 stream
// is a sequence of variable-length records — [total_len][command][payload…] —
// framed by the preamble 0xFF 0x00 and the end marker 0xFFFF.
//
// SCOPE (matching the Rust crate). This emits a STRUCTURALLY correct stream with
// a stub CRAM image (all zeros); real-hardware bit placement needs the IceStorm
// chip database, which is out of scope.
//
// DIVERGENCE FROM RUST. `cmd` throws std::length_error on a payload > 253 bytes
// (the Rust panic). The `clbs` HashMap becomes a std::map keyed by (row, col),
// so it iterates in (row, col) order — the output is byte-identical to the Rust
// crate (which sorts before emitting) and deterministic. `write_bin` throws
// std::runtime_error on a file error rather than returning a Result.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_FPGA_BITSTREAM_HPP
#define CA_FPGA_BITSTREAM_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <fstream>
#include <map>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace fpga {

// Supported iCE40 part codes.
enum class Ice40Part { Hx1k, Hx8k, Up5k, Lp1k };

// Part dimensions.
struct PartSpec {
    std::uint32_t rows;
    std::uint32_t cols;
    std::uint32_t cram_bits;
};

inline PartSpec part_specs(Ice40Part part) {
    switch (part) {
        case Ice40Part::Hx1k: return {33, 17, 1024};
        case Ice40Part::Hx8k: return {33, 33, 1024};
        case Ice40Part::Up5k: return {33, 33, 1024};
        case Ice40Part::Lp1k: return {33, 17, 1024};
    }
    return {0, 0, 0};
}

// Per-tile CLB configuration (16-entry 4-input LUT truth tables). In this stub
// emitter the fields do not affect the zeroed CRAM image.
struct ClbConfig {
    std::array<std::uint8_t, 16> lut_a_truth_table{};
    std::array<std::uint8_t, 16> lut_b_truth_table{};
    bool ff_a_enabled = false;
    bool ff_b_enabled = false;
};

// The complete configuration for one FPGA image.
struct FpgaConfig {
    Ice40Part part;
    std::map<std::pair<std::uint32_t, std::uint32_t>, ClbConfig> clbs;

    explicit FpgaConfig(Ice40Part p) : part(p) {}
};

// Summary of what emit_bitstream produced.
struct BitstreamReport {
    Ice40Part part;
    std::size_t bytes_written;
    std::size_t clb_count;
    std::size_t cram_size;
};

namespace detail {

constexpr std::uint8_t CMD_CRAM_BANK = 0x05;
constexpr std::uint8_t CMD_CRAM_OFFSET = 0x06;
constexpr std::uint8_t CMD_CRAM_RESET = 0x07;
constexpr std::uint8_t CMD_BRAM_DATA = 0x08;
constexpr std::uint8_t CMD_CRC = 0x80;

// Append one command record [len, command, payload…]. Throws if payload > 253.
inline void append_cmd(std::vector<std::uint8_t>& out, std::uint8_t command,
                       const std::vector<std::uint8_t>& payload) {
    if (payload.size() > 253) {
        throw std::length_error("command payload too long (max 253 bytes)");
    }
    out.push_back(static_cast<std::uint8_t>(payload.size() + 2));
    out.push_back(command);
    out.insert(out.end(), payload.begin(), payload.end());
}

}  // namespace detail

// Build one command record. Throws std::length_error if payload > 253 bytes.
inline std::vector<std::uint8_t> cmd(std::uint8_t command,
                                     const std::vector<std::uint8_t>& payload) {
    std::vector<std::uint8_t> rec;
    detail::append_cmd(rec, command, payload);
    return rec;
}

// Emit the record stream.
inline std::pair<std::vector<std::uint8_t>, BitstreamReport> emit_bitstream(
    const FpgaConfig& config) {
    PartSpec spec = part_specs(config.part);
    std::size_t cram_bytes = (static_cast<std::size_t>(spec.cram_bits) + 7) / 8;

    std::vector<std::uint8_t> out;
    // Preamble.
    out.push_back(0xFF);
    out.push_back(0x00);
    // CRAM reset + bank 0 select.
    detail::append_cmd(out, detail::CMD_CRAM_RESET, {});
    detail::append_cmd(out, detail::CMD_CRAM_BANK, {0x00});

    // Per-CLB records — std::map iterates in (row, col) order.
    for (const auto& [key, clb] : config.clbs) {
        (void)clb;
        std::uint32_t row = key.first, col = key.second;
        detail::append_cmd(out, detail::CMD_CRAM_OFFSET,
                           {static_cast<std::uint8_t>(row >> 8),
                            static_cast<std::uint8_t>(row),
                            static_cast<std::uint8_t>(col >> 8),
                            static_cast<std::uint8_t>(col)});
        detail::append_cmd(out, detail::CMD_BRAM_DATA,
                           std::vector<std::uint8_t>(cram_bytes, 0));
    }

    // CRC placeholder + end marker.
    detail::append_cmd(out, detail::CMD_CRC, {0x00, 0x00});
    out.push_back(0xFF);
    out.push_back(0xFF);

    BitstreamReport report{config.part, out.size(), config.clbs.size(),
                           cram_bytes};
    return {std::move(out), report};
}

// Emit and write to `path`. Throws std::runtime_error on a file error.
inline BitstreamReport write_bin(const std::string& path,
                                 const FpgaConfig& config) {
    auto [data, report] = emit_bitstream(config);
    std::ofstream f(path, std::ios::binary);
    if (!f) throw std::runtime_error("cannot open bitstream file: " + path);
    f.write(reinterpret_cast<const char*>(data.data()),
            static_cast<std::streamsize>(data.size()));
    if (!f) throw std::runtime_error("cannot write bitstream file: " + path);
    return report;
}

}  // namespace fpga
}  // namespace ca

#endif  // CA_FPGA_BITSTREAM_HPP
