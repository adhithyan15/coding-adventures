// ge225_simulator.hpp — a GE-225 CPU simulator, header-only ISO C++17.
// ====================================================================
//
// A faithful port of the Rust `ge225-simulator` crate, in namespace
// `ca::ge225_simulator`: a fetch-decode-execute simulator for the GE-225 (1959),
// the mainframe Dartmouth BASIC was designed on. It models the 20-bit machine —
// accumulator A, extension Q, index-register groups, bit-addressed memory, the
// console typewriter and card reader, and the full instruction set.
//
// Same two portability rules as the C port keep it UBSan-clean while matching
// Rust's wrapping-shift semantics: LEFT shifts and double-word bit-shuffling use
// unsigned types (defined wrap, not UB overflow); signed RIGHT shifts stay
// signed (implementation-defined arithmetic on every target, matching Rust `>>`).
//
// Errors are C++ exceptions (`ca::ge225_simulator::Error`).
//
// Pure ISO C++17 — no <cmath>, no compiler extensions, no 128-bit integers.
#ifndef GE225_SIMULATOR_HPP
#define GE225_SIMULATOR_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace ge225_simulator {

inline constexpr std::int32_t kMask20 = (1 << 20) - 1;
inline constexpr std::int32_t kDataMask = (1 << 19) - 1;
inline constexpr std::int32_t kSignBit = 1 << 19;
inline constexpr std::int32_t kAddrMask = 0x1fff;
inline constexpr std::int32_t kXMask = 0x7fff;
inline constexpr std::int32_t kNMask = 0x3f;
inline constexpr std::size_t kWordBytes = 3;
inline constexpr std::size_t kMaxXGroups = 32;

// ── Errors ───────────────────────────────────────────────────────────────────

enum class ErrorKind {
    AddressOutOfRange,
    Halted,
    Decode,
    Range,
    DivideByZero,
    NoCardRecord,
    InvalidTypewriterCode,
    OddByteLength,
    UnknownMnemonic
};

class Error : public std::runtime_error {
   public:
    Error(ErrorKind kind, const std::string &msg)
        : std::runtime_error(msg), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

   private:
    ErrorKind kind_;
};

// ── Mnemonics + tables ────────────────────────────────────────────────────────

enum class Mnemonic {
    None,
    Lda, Add, Sub, Sta, Bxl, Bxh, Ldx, Spb, Dld, Dad, Dsu, Dst, Inx, Mpy, Dvd,
    Stx, Ext, Cab, Dcb, Ory, Moy, Rcd, Bru, Sto,
    Off, Typ, Ton, Rcs, Hpt, Ldz, Ldo, Lmo, Cpl, Neg, Chs, Nop, Laq, Lqa, Xaq,
    Maq, Ado, Sbo, SetDecmode, SetBinmode, Sxg, SetPst, SetPbk, Bod, Bev, Bmi,
    Bpl, Bze, Bnz, Bov, Bno, Bpe, Bpc, Bnr, Bnn,
    Sra, Sna, Sca, San, Srd, Naq, Scd, Anq, Sla, Sld, Nor, Dno
};

namespace detail {

struct FixedEntry {
    Mnemonic mnem;
    const char *name;
    std::int32_t word;
};
inline const FixedEntry kFixed[] = {
    {Mnemonic::Off, "OFF", 02500005},   {Mnemonic::Typ, "TYP", 02500006},
    {Mnemonic::Ton, "TON", 02500007},   {Mnemonic::Rcs, "RCS", 02500011},
    {Mnemonic::Hpt, "HPT", 02500016},   {Mnemonic::Ldz, "LDZ", 02504002},
    {Mnemonic::Ldo, "LDO", 02504022},   {Mnemonic::Lmo, "LMO", 02504102},
    {Mnemonic::Cpl, "CPL", 02504502},   {Mnemonic::Neg, "NEG", 02504522},
    {Mnemonic::Chs, "CHS", 02504040},   {Mnemonic::Nop, "NOP", 02504012},
    {Mnemonic::Laq, "LAQ", 02504001},   {Mnemonic::Lqa, "LQA", 02504004},
    {Mnemonic::Xaq, "XAQ", 02504005},   {Mnemonic::Maq, "MAQ", 02504006},
    {Mnemonic::Ado, "ADO", 02504032},   {Mnemonic::Sbo, "SBO", 02504112},
    {Mnemonic::SetDecmode, "SET_DECMODE", 02506011},
    {Mnemonic::SetBinmode, "SET_BINMODE", 02506012},
    {Mnemonic::Sxg, "SXG", 02506013},   {Mnemonic::SetPst, "SET_PST", 02506015},
    {Mnemonic::SetPbk, "SET_PBK", 02506016},
    {Mnemonic::Bod, "BOD", 02514000},   {Mnemonic::Bev, "BEV", 02516000},
    {Mnemonic::Bmi, "BMI", 02514001},   {Mnemonic::Bpl, "BPL", 02516001},
    {Mnemonic::Bze, "BZE", 02514002},   {Mnemonic::Bnz, "BNZ", 02516002},
    {Mnemonic::Bov, "BOV", 02514003},   {Mnemonic::Bno, "BNO", 02516003},
    {Mnemonic::Bpe, "BPE", 02514004},   {Mnemonic::Bpc, "BPC", 02516004},
    {Mnemonic::Bnr, "BNR", 02514005},   {Mnemonic::Bnn, "BNN", 02516005}};

struct ShiftEntry {
    Mnemonic mnem;
    const char *name;
    std::int32_t base;
};
inline const ShiftEntry kShifts[] = {
    {Mnemonic::Sra, "SRA", 02510000}, {Mnemonic::Sna, "SNA", 02510100},
    {Mnemonic::Sca, "SCA", 02510040}, {Mnemonic::San, "SAN", 02510400},
    {Mnemonic::Srd, "SRD", 02511000}, {Mnemonic::Naq, "NAQ", 02511100},
    {Mnemonic::Scd, "SCD", 02511200}, {Mnemonic::Anq, "ANQ", 02511400},
    {Mnemonic::Sla, "SLA", 02512000}, {Mnemonic::Sld, "SLD", 02512200},
    {Mnemonic::Nor, "NOR", 02513000}, {Mnemonic::Dno, "DNO", 02513200}};

inline const Mnemonic kMemrefByOpcode[24] = {
    Mnemonic::Lda, Mnemonic::Add, Mnemonic::Sub, Mnemonic::Sta, Mnemonic::Bxl,
    Mnemonic::Bxh, Mnemonic::Ldx, Mnemonic::Spb, Mnemonic::Dld, Mnemonic::Dad,
    Mnemonic::Dsu, Mnemonic::Dst, Mnemonic::Inx, Mnemonic::Mpy, Mnemonic::Dvd,
    Mnemonic::Stx, Mnemonic::Ext, Mnemonic::Cab, Mnemonic::Dcb, Mnemonic::Ory,
    Mnemonic::Moy, Mnemonic::Rcd, Mnemonic::Bru, Mnemonic::Sto};

inline const char *mnemonic_name(Mnemonic m) {
    for (const auto &e : kFixed)
        if (e.mnem == m) return e.name;
    for (const auto &e : kShifts)
        if (e.mnem == m) return e.name;
    for (int i = 0; i < 24; ++i)
        if (kMemrefByOpcode[i] == m) {
            static const char *memref[24] = {
                "LDA", "ADD", "SUB", "STA", "BXL", "BXH", "LDX", "SPB",
                "DLD", "DAD", "DSU", "DST", "INX", "MPY", "DVD", "STX",
                "EXT", "CAB", "DCB", "ORY", "MOY", "RCD", "BRU", "STO"};
            return memref[i];
        }
    return "?";
}
inline bool is_shift(Mnemonic m) {
    for (const auto &e : kShifts)
        if (e.mnem == m) return true;
    return false;
}

// Arithmetic helpers (UB-safe sign extension).
inline std::int32_t to_signed20(std::int32_t value) {
    std::int32_t word = value & kMask20;
    return (word & kSignBit) ? word - (1 << 20) : word;
}
inline std::int32_t from_signed20(std::int32_t value) { return value & kMask20; }
inline std::int32_t sign_of(std::int32_t word) {
    return (word & kSignBit) ? 1 : 0;
}
inline std::int32_t with_sign(std::int32_t word, std::int32_t sign) {
    return ((sign & 1) << 19) | (word & kDataMask);
}
inline std::int64_t combine_words(std::int32_t high, std::int32_t low) {
    return (static_cast<std::int64_t>(high & kMask20) << 20) |
           static_cast<std::int64_t>(low & kMask20);
}
inline std::int64_t to_signed40(std::int64_t value) {
    std::uint64_t raw = static_cast<std::uint64_t>(value) &
                        ((UINT64_C(1) << 40) - 1);
    if (raw & (UINT64_C(1) << 39)) raw |= ~((UINT64_C(1) << 40) - 1);
    return static_cast<std::int64_t>(raw);
}
inline void split_signed40(std::int64_t value, std::int32_t &high,
                           std::int32_t &low) {
    std::uint64_t raw = static_cast<std::uint64_t>(value) &
                        ((UINT64_C(1) << 40) - 1);
    high = static_cast<std::int32_t>((raw >> 20) &
                                     static_cast<std::uint64_t>(
                                         static_cast<std::uint32_t>(kMask20)));
    low = static_cast<std::int32_t>(
        raw & static_cast<std::uint64_t>(static_cast<std::uint32_t>(kMask20)));
}
inline std::int32_t arith_compare(std::int32_t l, std::int32_t r) {
    std::int32_t a = to_signed20(l), b = to_signed20(r);
    return a < b ? -1 : (a > b ? 1 : 0);
}
inline std::int32_t arith_compare_double(std::int32_t lh, std::int32_t ll,
                                         std::int32_t rh, std::int32_t rl) {
    std::int64_t l = to_signed40(combine_words(lh, ll));
    std::int64_t r = to_signed40(combine_words(rh, rl));
    return l < r ? -1 : (l > r ? 1 : 0);
}
inline bool ov20(std::int32_t total) {
    return total < -(1 << 19) || total > (1 << 19) - 1;
}
inline bool ov40(std::int64_t total) {
    return total < -(INT64_C(1) << 39) || total > (INT64_C(1) << 39) - 1;
}

inline const char *typewriter_char(std::int32_t code) {
    switch (code) {
        case 000: return "0"; case 001: return "1"; case 002: return "2";
        case 003: return "3"; case 004: return "4"; case 005: return "5";
        case 006: return "6"; case 007: return "7"; case 010: return "8";
        case 011: return "9"; case 013: return "/"; case 021: return "A";
        case 022: return "B"; case 023: return "C"; case 024: return "D";
        case 025: return "E"; case 026: return "F"; case 027: return "G";
        case 030: return "H"; case 031: return "I"; case 033: return "-";
        case 040: return "."; case 041: return "J"; case 042: return "K";
        case 043: return "L"; case 044: return "M"; case 045: return "N";
        case 046: return "O"; case 047: return "P"; case 050: return "Q";
        case 051: return "R"; case 053: return "$"; case 060: return " ";
        case 062: return "S"; case 063: return "T"; case 064: return "U";
        case 065: return "V"; case 066: return "W"; case 067: return "X";
        case 070: return "Y"; case 071: return "Z";
        default: return nullptr;
    }
}

struct Decoded {
    Mnemonic mnem = Mnemonic::None;
    std::int32_t modifier = 0;
    std::int32_t address = 0;
    std::int32_t count = 0;
    bool fixed_word = false;
};

}  // namespace detail

// ── Free functions ────────────────────────────────────────────────────────────

inline std::int32_t encode_instruction(std::int32_t opcode,
                                       std::int32_t modifier,
                                       std::int32_t address) {
    if (opcode < 0 || opcode > 037)
        throw Error(ErrorKind::Range, "opcode out of range");
    if (modifier < 0 || modifier > 03)
        throw Error(ErrorKind::Range, "modifier out of range");
    if (address < 0 || address > kAddrMask)
        throw Error(ErrorKind::Range, "address out of range");
    return ((opcode & 0x1f) << 15) | ((modifier & 0x03) << 13) |
           (address & kAddrMask);
}
struct DecodedFields {
    std::int32_t opcode, modifier, address;
};
inline DecodedFields decode_instruction(std::int32_t word) {
    std::int32_t n = word & kMask20;
    return {(n >> 15) & 0x1f, (n >> 13) & 0x03, n & kAddrMask};
}
inline std::int32_t assemble_fixed(const std::string &mnemonic) {
    for (const auto &e : detail::kFixed)
        if (mnemonic == e.name) return e.word;
    throw Error(ErrorKind::UnknownMnemonic, "unknown fixed instruction");
}
inline std::int32_t assemble_shift(const std::string &mnemonic,
                                   std::int32_t count) {
    if (count < 0 || count > 037)
        throw Error(ErrorKind::Range, "shift count out of range");
    for (const auto &e : detail::kShifts)
        if (mnemonic == e.name) return e.base | count;
    throw Error(ErrorKind::UnknownMnemonic, "unknown shift instruction");
}
inline std::vector<std::uint8_t> pack_words(const std::vector<std::int32_t> &w) {
    std::vector<std::uint8_t> blob(w.size() * kWordBytes, 0);
    for (std::size_t i = 0; i < w.size(); ++i) {
        std::int32_t n = w[i] & kMask20;
        blob[i * kWordBytes] = static_cast<std::uint8_t>((n >> 16) & 0xff);
        blob[i * kWordBytes + 1] = static_cast<std::uint8_t>((n >> 8) & 0xff);
        blob[i * kWordBytes + 2] = static_cast<std::uint8_t>(n & 0xff);
    }
    return blob;
}
inline std::vector<std::int32_t> unpack_words(
    const std::vector<std::uint8_t> &program) {
    if (program.size() % kWordBytes != 0)
        throw Error(ErrorKind::OddByteLength,
                    "byte stream must be a multiple of 3 bytes");
    std::vector<std::int32_t> words(program.size() / kWordBytes);
    for (std::size_t i = 0; i < words.size(); ++i) {
        const std::uint8_t *c = program.data() + i * kWordBytes;
        words[i] = ((static_cast<std::int32_t>(c[0]) << 16) |
                    (static_cast<std::int32_t>(c[1]) << 8) |
                    static_cast<std::int32_t>(c[2])) &
                   kMask20;
    }
    return words;
}

// One decoded/executed instruction.
struct Trace {
    std::int32_t address = 0;
    std::int32_t instruction_word = 0;
    std::string mnemonic;
    std::int32_t a_before = 0;
    std::int32_t a_after = 0;
    std::int32_t q_before = 0;
    std::int32_t q_after = 0;
    std::optional<std::int32_t> effective_address;
};

// ── The simulator ─────────────────────────────────────────────────────────────

class Simulator {
   public:
    explicit Simulator(std::int32_t memory_words)
        : memory_size_(memory_words),
          memory_(static_cast<std::size_t>(memory_words), 0) {
        if (memory_words <= 0)
            throw Error(ErrorKind::Range, "memory_words must be positive");
    }

    void reset() {
        a_ = q_ = m_ = n_ = pc_ = ir_ = 0;
        overflow_ = parity_error_ = decimal_mode_ = false;
        automatic_interrupt_mode_ = false;
        selected_x_group_ = 0;
        n_ready_ = true;
        typewriter_power_ = false;
        typewriter_output_.clear();
        control_switches_ = 0;
        halted_ = false;
        for (auto &g : x_groups_) g = {};
    }

    void set_control_switches(std::int32_t value) {
        control_switches_ = value & kMask20;
    }
    void queue_card_reader_record(const std::vector<std::int32_t> &words) {
        std::vector<std::int32_t> copy;
        copy.reserve(words.size());
        for (std::int32_t w : words) copy.push_back(w & kMask20);
        card_queue_.push_back(std::move(copy));
    }
    const std::string &typewriter_output() const { return typewriter_output_; }

    void load_words(const std::vector<std::int32_t> &words,
                    std::int32_t start_address) {
        for (std::size_t i = 0; i < words.size(); ++i) {
            // int64 so a large start_address/count can't overflow int32 before
            // check_address sees it.
            std::int64_t addr = static_cast<std::int64_t>(start_address) +
                                static_cast<std::int64_t>(i);
            if (addr < 0 || addr >= memory_size_)
                throw Error(ErrorKind::AddressOutOfRange, "address out of range");
            write_word(static_cast<std::int32_t>(addr), words[i]);
        }
    }
    std::int32_t read_word(std::int32_t address) const {
        check_address(address);
        return memory_[static_cast<std::size_t>(address)];
    }
    void write_word(std::int32_t address, std::int32_t value) {
        check_address(address);
        memory_[static_cast<std::size_t>(address)] = value & kMask20;
    }

    std::string disassemble_word(std::int32_t word) const {
        detail::Decoded d = decode_word(word);
        if (d.fixed_word) {
            if (detail::is_shift(d.mnem))
                return std::string(detail::mnemonic_name(d.mnem)) + " " +
                       std::to_string(d.count);
            return detail::mnemonic_name(d.mnem);
        }
        static const char *hexd = "0123456789ABCDEF";
        std::string out = detail::mnemonic_name(d.mnem);
        out += " 0x";
        out += hexd[(d.address >> 8) & 0xf];
        out += hexd[(d.address >> 4) & 0xf];
        out += hexd[d.address & 0xf];
        out += ",X";
        out += static_cast<char>('0' + (d.modifier & 3));
        return out;
    }

    Trace step() {
        using namespace detail;
        if (halted_) throw Error(ErrorKind::Halted, "cannot step a halted CPU");
        std::int32_t pc_before = pc_;
        ir_ = read_word(pc_);
        pc_ = (pc_ + 1) % memory_size_;
        Decoded d = decode_word(ir_);
        std::int32_t a_before = a_, q_before = q_;
        std::optional<std::int32_t> eff;
        if (!d.fixed_word) {
            std::int32_t address = d.address;
            bool no_eff =
                d.mnem == Mnemonic::Bxl || d.mnem == Mnemonic::Bxh ||
                d.mnem == Mnemonic::Ldx || d.mnem == Mnemonic::Spb ||
                d.mnem == Mnemonic::Inx || d.mnem == Mnemonic::Stx ||
                d.mnem == Mnemonic::Moy;
            if (!no_eff) eff = resolve_effective_address(address, d.modifier);
            execute_memory_reference(d.mnem, d.modifier, eff.value_or(address),
                                     address, pc_before);
        } else {
            execute_fixed(d);
        }
        Trace t;
        t.address = pc_before;
        t.instruction_word = ir_;
        t.mnemonic = disassemble_word(ir_);
        t.a_before = a_before;
        t.a_after = a_;
        t.q_before = q_before;
        t.q_after = q_;
        t.effective_address = eff;
        return t;
    }
    std::vector<Trace> run(std::size_t max_steps) {
        std::vector<Trace> traces;
        for (std::size_t i = 0; i < max_steps; ++i) {
            if (halted_) break;
            traces.push_back(step());
        }
        return traces;
    }

    // Accessors.
    std::int32_t a() const { return a_; }
    std::int32_t q() const { return q_; }
    std::int32_t m() const { return m_; }
    std::int32_t n() const { return n_; }
    std::int32_t pc() const { return pc_; }
    std::int32_t ir() const { return ir_; }
    bool overflow() const { return overflow_; }
    bool parity_error() const { return parity_error_; }
    bool decimal_mode() const { return decimal_mode_; }
    bool automatic_interrupt_mode() const { return automatic_interrupt_mode_; }
    std::size_t selected_x_group() const { return selected_x_group_; }
    bool n_ready() const { return n_ready_; }
    bool typewriter_power() const { return typewriter_power_; }
    bool halted() const { return halted_; }
    std::int32_t x_word(std::size_t slot) const {
        if (slot >= 4) return 0;  // guard the public API against an OOB slot
        return get_x_word(slot);
    }

   private:
    void check_address(std::int32_t address) const {
        if (address < 0 || address >= memory_size_)
            throw Error(ErrorKind::AddressOutOfRange, "address out of range");
    }
    std::int32_t get_x_word(std::size_t slot) const {
        return x_groups_[selected_x_group_][slot] & kXMask;
    }
    void set_x_word(std::size_t slot, std::int32_t value) {
        x_groups_[selected_x_group_][slot] = value & kXMask;
    }
    std::int32_t resolve_effective_address(std::int32_t address,
                                           std::int32_t modifier) const {
        std::int32_t base = address % memory_size_;
        if (modifier == 0) return base;
        return (base + (get_x_word(static_cast<std::size_t>(modifier)) %
                        memory_size_)) %
               memory_size_;
    }

    detail::Decoded decode_word(std::int32_t word) const {
        using namespace detail;
        std::int32_t normalized = word & kMask20;
        Decoded out;
        for (const auto &e : kFixed)
            if (e.word == normalized) {
                out.mnem = e.mnem;
                out.fixed_word = true;
                return out;
            }
        for (const auto &e : kShifts)
            if ((normalized & ~static_cast<std::int32_t>(037)) == e.base) {
                out.mnem = e.mnem;
                out.fixed_word = true;
                out.count = normalized & 037;
                return out;
            }
        DecodedFields f = decode_instruction(normalized);
        if (f.opcode < 0 || f.opcode >= 24)
            throw Error(ErrorKind::Decode, "unknown opcode field");
        out.mnem = kMemrefByOpcode[f.opcode];
        out.modifier = f.modifier;
        out.address = f.address;
        return out;
    }

    void execute_memory_reference(Mnemonic mnem, std::int32_t modifier,
                                  std::int32_t effective_or_raw,
                                  std::int32_t raw_address,
                                  std::int32_t pc_before) {
        using namespace detail;
        std::int32_t eff = effective_or_raw % memory_size_;
        switch (mnem) {
            case Mnemonic::Lda: m_ = read_word(eff); a_ = m_; break;
            case Mnemonic::Add: {
                m_ = read_word(eff);
                std::int32_t total = to_signed20(a_) + to_signed20(m_);
                a_ = from_signed20(total);
                overflow_ = ov20(total);
                break;
            }
            case Mnemonic::Sub: {
                m_ = read_word(eff);
                std::int32_t total = to_signed20(a_) - to_signed20(m_);
                a_ = from_signed20(total);
                overflow_ = ov20(total);
                break;
            }
            case Mnemonic::Sta: write_word(eff, a_); break;
            case Mnemonic::Bxl:
                if ((get_x_word(static_cast<std::size_t>(modifier)) &
                     kAddrMask) >= raw_address)
                    pc_ = (pc_ + 1) % memory_size_;
                break;
            case Mnemonic::Bxh:
                if ((get_x_word(static_cast<std::size_t>(modifier)) &
                     kAddrMask) < raw_address)
                    pc_ = (pc_ + 1) % memory_size_;
                break;
            case Mnemonic::Ldx:
                set_x_word(static_cast<std::size_t>(modifier),
                           read_word(raw_address % memory_size_));
                break;
            case Mnemonic::Spb:
                set_x_word(static_cast<std::size_t>(modifier), pc_before);
                pc_ = raw_address % memory_size_;
                break;
            case Mnemonic::Dld: {
                std::int32_t first = read_word(eff);
                if (eff & 1) {
                    a_ = first;
                    q_ = first;
                } else {
                    a_ = first;
                    q_ = read_word((eff + 1) % memory_size_);
                }
                break;
            }
            case Mnemonic::Dad:
            case Mnemonic::Dsu: {
                std::int64_t left = to_signed40(combine_words(a_, q_));
                std::int32_t first = read_word(eff);
                std::int32_t second =
                    (eff & 1) ? first : read_word((eff + 1) % memory_size_);
                std::int64_t total =
                    mnem == Mnemonic::Dad
                        ? left + to_signed40(combine_words(first, second))
                        : left - to_signed40(combine_words(first, second));
                split_signed40(total, a_, q_);
                overflow_ = ov40(total);
                break;
            }
            case Mnemonic::Dst:
                if (eff & 1) {
                    write_word(eff, q_);
                } else {
                    write_word(eff, a_);
                    write_word((eff + 1) % memory_size_, q_);
                }
                break;
            case Mnemonic::Inx:
                set_x_word(static_cast<std::size_t>(modifier),
                           (get_x_word(static_cast<std::size_t>(modifier)) +
                            raw_address) &
                               kXMask);
                break;
            case Mnemonic::Mpy: {
                m_ = read_word(eff);
                std::int64_t product =
                    static_cast<std::int64_t>(to_signed20(q_)) *
                        static_cast<std::int64_t>(to_signed20(m_)) +
                    static_cast<std::int64_t>(to_signed20(a_));
                split_signed40(product, a_, q_);
                overflow_ = ov40(product);
                break;
            }
            case Mnemonic::Dvd: {
                m_ = read_word(eff);
                std::int64_t divisor = to_signed20(m_);
                if (divisor == 0)
                    throw Error(ErrorKind::DivideByZero, "divide by zero");
                std::int32_t sa = to_signed20(a_);
                std::int64_t sa_abs = sa < 0 ? -static_cast<std::int64_t>(sa)
                                             : static_cast<std::int64_t>(sa);
                std::int64_t div_abs = divisor < 0 ? -divisor : divisor;
                if (sa_abs >= div_abs) {
                    overflow_ = true;
                    return;
                }
                std::int64_t dividend = to_signed40(combine_words(a_, q_));
                std::int64_t dvd_abs = dividend < 0 ? -dividend : dividend;
                std::int64_t qmag = dvd_abs / div_abs;
                std::int64_t rmag = dvd_abs % div_abs;
                std::int64_t quotient =
                    ((dividend < 0) ^ (divisor < 0)) ? -qmag : qmag;
                std::int64_t remainder = quotient < 0 ? -rmag : rmag;
                a_ = from_signed20(static_cast<std::int32_t>(quotient));
                q_ = from_signed20(static_cast<std::int32_t>(remainder));
                overflow_ = quotient < -(INT64_C(1) << 19) ||
                            quotient > (INT64_C(1) << 19) - 1;
                break;
            }
            case Mnemonic::Stx:
                write_word(raw_address % memory_size_,
                           get_x_word(static_cast<std::size_t>(modifier)));
                break;
            case Mnemonic::Ext:
                m_ = read_word(eff);
                a_ &= (~m_) & kMask20;
                break;
            case Mnemonic::Cab: {
                m_ = read_word(eff);
                std::int32_t cmp = arith_compare(m_, a_);
                if (cmp == 0)
                    pc_ = (pc_ + 1) % memory_size_;
                else if (cmp < 0)
                    pc_ = (pc_ + 2) % memory_size_;
                break;
            }
            case Mnemonic::Dcb: {
                std::int32_t first = read_word(eff);
                std::int32_t second =
                    (eff & 1) ? first : read_word((eff + 1) % memory_size_);
                std::int32_t cmp = arith_compare_double(first, second, a_, q_);
                if (cmp == 0)
                    pc_ = (pc_ + 1) % memory_size_;
                else if (cmp < 0)
                    pc_ = (pc_ + 2) % memory_size_;
                break;
            }
            case Mnemonic::Ory: {
                std::int32_t word = read_word(eff);
                write_word(eff, word | a_);
                break;
            }
            case Mnemonic::Moy: {
                std::int32_t sq = to_signed20(q_);
                std::int32_t word_count = -sq > 0 ? -sq : 0;
                std::int32_t destination = a_ & kXMask;
                for (std::int32_t offset = 0; offset < word_count; ++offset) {
                    std::int32_t word =
                        read_word((raw_address + offset) % memory_size_);
                    write_word((destination + offset) % memory_size_, word);
                }
                set_x_word(0, pc_);
                a_ = 0;
                break;
            }
            case Mnemonic::Rcd: {
                if (card_queue_.empty())
                    throw Error(ErrorKind::NoCardRecord, "no queued record");
                std::vector<std::int32_t> record = std::move(card_queue_.front());
                card_queue_.erase(card_queue_.begin());
                for (std::size_t offset = 0; offset < record.size(); ++offset)
                    write_word((eff + static_cast<std::int32_t>(offset)) %
                                   memory_size_,
                               record[offset]);
                break;
            }
            case Mnemonic::Bru: pc_ = eff; break;
            case Mnemonic::Sto: {
                std::int32_t existing = read_word(eff);
                write_word(eff, (existing & ~kAddrMask) | (a_ & kAddrMask));
                break;
            }
            default:
                throw Error(ErrorKind::Decode, "unimplemented memory-ref");
        }
    }

    void execute_branch_test(Mnemonic mnem) {
        bool cond = false;
        switch (mnem) {
            case Mnemonic::Bod: cond = (a_ & 1) != 0; break;
            case Mnemonic::Bev: cond = (a_ & 1) == 0; break;
            case Mnemonic::Bmi: cond = (a_ & kSignBit) != 0; break;
            case Mnemonic::Bpl: cond = (a_ & kSignBit) == 0; break;
            case Mnemonic::Bze: cond = a_ == 0; break;
            case Mnemonic::Bnz: cond = a_ != 0; break;
            case Mnemonic::Bov: cond = overflow_; break;
            case Mnemonic::Bno: cond = !overflow_; break;
            case Mnemonic::Bpe: cond = parity_error_; break;
            case Mnemonic::Bpc: cond = !parity_error_; break;
            case Mnemonic::Bnr: cond = n_ready_; break;
            case Mnemonic::Bnn: cond = !n_ready_; break;
            default: break;
        }
        if (mnem == Mnemonic::Bov || mnem == Mnemonic::Bno) overflow_ = false;
        if (mnem == Mnemonic::Bpe || mnem == Mnemonic::Bpc)
            parity_error_ = false;
        if (!cond) pc_ = (pc_ + 1) % memory_size_;
    }

    void execute_shift(Mnemonic mnem, std::int32_t count) {
        using namespace detail;
        if (count == 0) {
            if (mnem == Mnemonic::Srd)
                q_ = with_sign(q_, sign_of(a_));
            else if (mnem == Mnemonic::Sld)
                a_ = with_sign(a_, sign_of(q_));
            return;
        }
        std::int32_t a_sign = sign_of(a_);
        std::uint32_t a_data = static_cast<std::uint32_t>(a_ & kDataMask);
        std::int32_t q_sign = sign_of(q_);
        std::uint32_t q_data = static_cast<std::uint32_t>(q_ & kDataMask);
        const std::uint32_t data_mask = static_cast<std::uint32_t>(kDataMask);
        const std::uint32_t n_mask = static_cast<std::uint32_t>(kNMask);
        switch (mnem) {
            case Mnemonic::Sra: {
                int sh = count < 19 ? static_cast<int>(count) : 19;
                a_ = from_signed20(to_signed20(a_) >> sh);
                break;
            }
            case Mnemonic::Sla: {
                int ov_sh =
                    (19 - count) > 0 ? static_cast<int>(19 - count) : 0;
                overflow_ = (a_data >> ov_sh) != 0;
                a_ = with_sign(static_cast<std::int32_t>((a_data << count) &
                                                         data_mask),
                               a_sign);
                break;
            }
            case Mnemonic::Sca: {
                std::int32_t rot = count % 19;
                if (rot != 0)
                    a_data = ((a_data >> rot) | (a_data << (19 - rot))) &
                             data_mask;
                a_ = with_sign(static_cast<std::int32_t>(a_data), a_sign);
                break;
            }
            case Mnemonic::San: {
                std::uint32_t fill =
                    a_sign ? ((static_cast<std::uint32_t>(1) << count) - 1) : 0;
                std::uint32_t combu = ((a_data & data_mask) << 6) |
                                      (static_cast<std::uint32_t>(n_) & n_mask);
                std::int32_t comb = static_cast<std::int32_t>((fill << 25) |
                                                              combu);
                comb = comb >> count;
                a_ = with_sign((comb >> 6) & kDataMask, a_sign);
                n_ = comb & kNMask;
                break;
            }
            case Mnemonic::Sna: {
                std::int32_t comb = static_cast<std::int32_t>(
                    ((static_cast<std::uint32_t>(n_) & n_mask) << 19) | a_data);
                comb = comb >> count;
                n_ = (comb >> 19) & kNMask;
                a_ = with_sign(comb & kDataMask, a_sign);
                break;
            }
            case Mnemonic::Srd: {
                std::int64_t value = combine_words(a_, q_) >> count;
                a_ = with_sign(
                    static_cast<std::int32_t>((value >> 20) & kDataMask),
                    a_sign);
                q_ = with_sign(static_cast<std::int32_t>(value & kDataMask),
                               a_sign);
                break;
            }
            case Mnemonic::Naq: {
                std::int64_t comb = static_cast<std::int64_t>(
                    ((static_cast<std::uint64_t>(
                          static_cast<std::uint32_t>(n_ & kNMask)))
                     << 38) |
                    ((static_cast<std::uint64_t>(a_data & data_mask)) << 19) |
                    static_cast<std::uint64_t>(q_data));
                comb = comb >> count;
                n_ = static_cast<std::int32_t>((comb >> 38) & kNMask);
                a_ = with_sign(
                    static_cast<std::int32_t>((comb >> 19) & kDataMask), a_sign);
                q_ = with_sign(static_cast<std::int32_t>(comb & kDataMask),
                               a_sign);
                break;
            }
            case Mnemonic::Scd: {
                std::int32_t rot = count % 38;
                std::uint64_t comb =
                    ((static_cast<std::uint64_t>(a_data & data_mask)) << 19) |
                    static_cast<std::uint64_t>(q_data);
                if (rot != 0)
                    comb = ((comb >> rot) | (comb << (38 - rot))) &
                           ((UINT64_C(1) << 38) - 1);
                std::uint64_t dm =
                    static_cast<std::uint64_t>(static_cast<std::uint32_t>(kDataMask));
                a_ = with_sign(static_cast<std::int32_t>((comb >> 19) & dm),
                               a_sign);
                q_ = with_sign(static_cast<std::int32_t>(comb & dm), a_sign);
                break;
            }
            case Mnemonic::Anq: {
                for (std::int32_t i = 0; i < count; ++i) {
                    std::int32_t bit = a_ & 1;
                    a_ = from_signed20(to_signed20(a_) >> 1);
                    q_data = static_cast<std::uint32_t>(
                        ((bit << 18) | ((q_ & kDataMask) >> 1)) & kDataMask);
                    q_ = with_sign(static_cast<std::int32_t>(q_data), a_sign);
                    n_ = ((bit << 5) | (n_ >> 1)) & kNMask;
                }
                break;
            }
            case Mnemonic::Sld: {
                std::uint64_t comb =
                    ((static_cast<std::uint64_t>(a_data & data_mask)) << 19) |
                    static_cast<std::uint64_t>(q_data);
                int ov_sh =
                    (38 - count) > 0 ? static_cast<int>(38 - count) : 0;
                overflow_ = (comb >> ov_sh) != 0;
                comb = (comb << count) & ((UINT64_C(1) << 38) - 1);
                std::uint64_t dm =
                    static_cast<std::uint64_t>(static_cast<std::uint32_t>(kDataMask));
                a_ = with_sign(static_cast<std::int32_t>((comb >> 19) & dm),
                               q_sign);
                q_ = with_sign(static_cast<std::int32_t>(comb & dm), q_sign);
                break;
            }
            case Mnemonic::Nor: {
                std::int32_t shifts = 0;
                std::int32_t target = a_sign == 0 ? 0 : 1;
                while (shifts < count) {
                    std::int32_t lead =
                        static_cast<std::int32_t>((a_data >> 18) & 1u);
                    if (lead != target) break;
                    if (lead == 1) overflow_ = true;
                    a_data = (a_data << 1) & data_mask;
                    ++shifts;
                }
                a_ = with_sign(static_cast<std::int32_t>(a_data), a_sign);
                set_x_word(0, count - shifts);
                break;
            }
            case Mnemonic::Dno: {
                std::int32_t shifts = 0;
                std::int32_t target = a_sign == 0 ? 0 : 1;
                std::uint64_t comb =
                    ((static_cast<std::uint64_t>(a_data & data_mask)) << 19) |
                    static_cast<std::uint64_t>(q_data);
                while (shifts < count) {
                    std::int32_t lead =
                        static_cast<std::int32_t>((comb >> 37) & 1u);
                    if (lead != target) break;
                    if (lead == 1) overflow_ = true;
                    comb = (comb << 1) & ((UINT64_C(1) << 38) - 1);
                    ++shifts;
                }
                std::uint64_t dm =
                    static_cast<std::uint64_t>(static_cast<std::uint32_t>(kDataMask));
                a_ = with_sign(static_cast<std::int32_t>((comb >> 19) & dm),
                               q_sign);
                q_ = with_sign(static_cast<std::int32_t>(comb & dm), q_sign);
                set_x_word(0, count - shifts);
                break;
            }
            default: break;
        }
    }

    void execute_fixed(const detail::Decoded &d) {
        using namespace detail;
        Mnemonic mnem = d.mnem;
        std::int32_t count = d.count;
        switch (mnem) {
            case Mnemonic::Off:
                typewriter_power_ = false;
                n_ready_ = true;
                break;
            case Mnemonic::Typ: {
                if (!typewriter_power_) {
                    n_ready_ = false;
                    return;
                }
                std::int32_t code = n_ & kNMask;
                if (code == 037)
                    typewriter_output_ += "\r";
                else if (code == 076)
                    typewriter_output_ += "\t";
                else if (code != 072 && code != 075) {
                    const char *ch = typewriter_char(code);
                    if (ch == nullptr)
                        throw Error(ErrorKind::InvalidTypewriterCode,
                                    "invalid typewriter code");
                    typewriter_output_ += ch;
                }
                n_ready_ = true;
                break;
            }
            case Mnemonic::Ton: typewriter_power_ = true; break;
            case Mnemonic::Rcs: a_ |= control_switches_; break;
            case Mnemonic::Hpt: n_ready_ = false; break;
            case Mnemonic::Ldz: a_ = 0; break;
            case Mnemonic::Ldo: a_ = 1; break;
            case Mnemonic::Lmo: a_ = kMask20; break;
            case Mnemonic::Cpl: a_ = (~a_) & kMask20; break;
            case Mnemonic::Neg: {
                std::int32_t before = to_signed20(a_);
                a_ = from_signed20(-before);
                overflow_ = before == -(1 << 19);
                break;
            }
            case Mnemonic::Chs: a_ ^= kSignBit; break;
            case Mnemonic::Nop: break;
            case Mnemonic::Laq: a_ = q_; break;
            case Mnemonic::Lqa: q_ = a_; break;
            case Mnemonic::Xaq: std::swap(a_, q_); break;
            case Mnemonic::Maq:
                q_ = a_;
                a_ = 0;
                break;
            case Mnemonic::Ado: {
                std::int32_t total = to_signed20(a_) + 1;
                a_ = from_signed20(total);
                overflow_ = ov20(total);
                break;
            }
            case Mnemonic::Sbo: {
                std::int32_t total = to_signed20(a_) - 1;
                a_ = from_signed20(total);
                overflow_ = ov20(total);
                break;
            }
            case Mnemonic::SetDecmode: decimal_mode_ = true; break;
            case Mnemonic::SetBinmode: decimal_mode_ = false; break;
            case Mnemonic::Sxg:
                selected_x_group_ = static_cast<std::size_t>(a_ & 0x1f);
                break;
            case Mnemonic::SetPst: automatic_interrupt_mode_ = true; break;
            case Mnemonic::SetPbk: automatic_interrupt_mode_ = false; break;
            case Mnemonic::Bod: case Mnemonic::Bev: case Mnemonic::Bmi:
            case Mnemonic::Bpl: case Mnemonic::Bze: case Mnemonic::Bnz:
            case Mnemonic::Bov: case Mnemonic::Bno: case Mnemonic::Bpe:
            case Mnemonic::Bpc: case Mnemonic::Bnr: case Mnemonic::Bnn:
                execute_branch_test(mnem);
                break;
            case Mnemonic::Sra: case Mnemonic::Sna: case Mnemonic::Sca:
            case Mnemonic::San: case Mnemonic::Srd: case Mnemonic::Naq:
            case Mnemonic::Scd: case Mnemonic::Anq: case Mnemonic::Sla:
            case Mnemonic::Sld: case Mnemonic::Nor: case Mnemonic::Dno:
                execute_shift(mnem, count);
                break;
            default:
                throw Error(ErrorKind::Decode, "unimplemented fixed instruction");
        }
    }

    std::int32_t memory_size_;
    std::vector<std::int32_t> memory_;
    std::vector<std::vector<std::int32_t>> card_queue_;
    std::int32_t a_ = 0, q_ = 0, m_ = 0, n_ = 0, pc_ = 0, ir_ = 0;
    bool overflow_ = false, parity_error_ = false, decimal_mode_ = false;
    bool automatic_interrupt_mode_ = false;
    std::size_t selected_x_group_ = 0;
    bool n_ready_ = true, typewriter_power_ = false;
    std::string typewriter_output_;
    std::int32_t control_switches_ = 0;
    bool halted_ = false;
    std::array<std::array<std::int32_t, 4>, kMaxXGroups> x_groups_{};
};

}  // namespace ge225_simulator
}  // namespace ca

#endif  // GE225_SIMULATOR_HPP
