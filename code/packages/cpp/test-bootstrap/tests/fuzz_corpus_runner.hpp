// fuzz_corpus_runner.hpp -- the C++ replay of the shared fuzz-v1 corpus (VER06k section 6).
// ============================================================================
//
// WHY A SECOND RUNNER
// -------------------
// The C package already evaluates this corpus (fz_corpus_evaluate).  If the C++
// binding were judged by THAT function, a broken C++ layer (an offset shifted, an
// endianness swapped, a dedup inverted) would hide behind a green C result.  So
// this runner shares only the DECODING of the strict JSON text with C
// (pbt_json_parse) and answers every operation through ca::fuzz.  It calls no
// fz_* function and never fz_corpus_evaluate.
//
// It is paranoid about its input because the input is a file: a case has exactly
// the fields of its op, integers are canonical decimal strings, bytes are
// lowercase even hex, an unknown op or malformed value is a PROBLEM (never a skip),
// sizes are capped, and every problem starts with the case id so a negative control
// can prove WHICH case failed.
#ifndef CA_FUZZ_TESTS_CORPUS_RUNNER_HPP
#define CA_FUZZ_TESTS_CORPUS_RUNNER_HPP

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <set>
#include <string>
#include <string_view>
#include <vector>

#include "fuzz_bootstrap.hpp"

extern "C" {
#include "pbt_json.h"
}

namespace fuzzcorpus {

namespace fz = ca::fuzz;

constexpr std::size_t kMaxSize = 10000;
constexpr std::size_t kCaseCount = 25;
constexpr std::size_t kClipFragment = 200;
constexpr std::size_t kClipProblem = 400;
constexpr std::uint64_t kSequenceWork = std::uint64_t(1) << 24;

struct Report {
    std::size_t evaluated = 0;
    std::vector<std::string> problems;
};

struct Bad {
    std::string why;
};

// ---- strict decoding ---------------------------------------------------------------

inline bool is_str(const pbt_json* n) { return n != nullptr && n->type == PBT_J_STR; }
inline bool str_is(const pbt_json* n, const char* lit) {
    return is_str(n) && std::string_view(n->s, n->slen) == std::string_view(lit);
}
inline bool has_exact_fields(const pbt_json* obj, const char* const* names, std::size_t n) {
    if (obj->type != PBT_J_OBJ || obj->n != n) return false;
    for (std::size_t i = 0; i < n; ++i)
        if (pbt_json_get(obj, names[i]) == nullptr) return false;
    return true;
}
inline bool canonical_int(std::string_view s) {
    std::size_t i = (!s.empty() && s[0] == '-') ? 1 : 0;
    std::size_t digits = s.size() - i;
    if (digits == 0) return false;
    for (std::size_t k = i; k < s.size(); ++k)
        if (s[k] < '0' || s[k] > '9') return false;
    if (s[i] == '0' && (digits > 1 || i == 1)) return false;
    return digits <= 300;
}
inline int hexval(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

inline fz::BigInt dec_int(const pbt_json* n, const char* what) {
    static const char* const names[] = {"kind", "value"};
    if (n == nullptr || !has_exact_fields(n, names, 2) || !str_is(pbt_json_get(n, "kind"), "int"))
        throw Bad{std::string(what) + " must be {kind: int, value}"};
    const pbt_json* val = pbt_json_get(n, "value");
    if (!is_str(val) || !canonical_int(std::string_view(val->s, val->slen)))
        throw Bad{std::string(what) + " must be a canonical decimal string within the digit cap"};
    try {
        return fz::BigInt::parse(std::string_view(val->s, val->slen));
    } catch (const fz::Error&) {
        throw Bad{std::string(what) + " is too wide for this lane"};
    }
}
inline std::size_t dec_size(const pbt_json* n, const char* what) {
    fz::BigInt b = dec_int(n, what);
    if (!b.fits_u64() || b.low_u64() > kMaxSize) throw Bad{std::string(what) + " is outside [0, 10000]"};
    return static_cast<std::size_t>(b.low_u64());
}
// A width outside [0, 64] becomes -1 ("not one of 1, 2, 4, 8": input unchanged).
inline int dec_width(const pbt_json* n) {
    fz::BigInt b = dec_int(n, "width");
    return (b.fits_i64() && b.to_i64() >= 0 && b.to_i64() <= 64) ? static_cast<int>(b.to_i64()) : -1;
}
inline fz::Bytes dec_bytes(const pbt_json* n, const char* what) {
    static const char* const names[] = {"kind", "hex"};
    if (n == nullptr || !has_exact_fields(n, names, 2) || !str_is(pbt_json_get(n, "kind"), "bytes") ||
        !is_str(pbt_json_get(n, "hex")))
        throw Bad{std::string(what) + " must be {kind: bytes, hex}"};
    const pbt_json* hex = pbt_json_get(n, "hex");
    if (hex->slen % 2 != 0 || hex->slen / 2 > kMaxSize)
        throw Bad{std::string(what) + " hex must be even length and within the cap"};
    fz::Bytes out(hex->slen / 2);
    for (std::size_t i = 0; i < hex->slen; i += 2) {
        int h = hexval(hex->s[i]);
        int l = hexval(hex->s[i + 1]);
        if (h < 0 || l < 0) throw Bad{std::string(what) + " hex must be lowercase"};
        out[i / 2] = static_cast<std::uint8_t>(h * 16 + l);
    }
    return out;
}
inline std::string dec_str(const pbt_json* n, const char* what) {
    if (!is_str(n)) throw Bad{std::string(what) + " must be a string"};
    return std::string(n->s, n->slen);
}
inline bool dec_bool(const pbt_json* n, const char* what) {
    if (n == nullptr || n->type != PBT_J_BOOL) throw Bad{std::string(what) + " must be a bool"};
    return n->b != 0;
}
inline fz::Endian dec_endian(const pbt_json* n) {
    if (str_is(n, "little")) return fz::Endian::Little;
    if (str_is(n, "big")) return fz::Endian::Big;
    throw Bad{"endian must be 'little' or 'big'"};
}
inline fz::Frames dec_strs(const pbt_json* n, const char* what) {
    if (n == nullptr || n->type != PBT_J_ARR || n->n > kMaxSize)
        throw Bad{std::string(what) + " must be a list of strings within the cap"};
    fz::Frames v;
    for (std::size_t i = 0; i < n->n; ++i) {
        if (!is_str(&n->items[i])) throw Bad{std::string(what) + " must be a list of strings"};
        v.emplace_back(n->items[i].s, n->items[i].slen);
    }
    return v;
}
inline std::vector<std::uint64_t> dec_edges(const pbt_json* n, const char* what) {
    if (n == nullptr || n->type != PBT_J_ARR || n->n > kMaxSize)
        throw Bad{std::string(what) + " must be a list of ints within the cap"};
    std::vector<std::uint64_t> v;
    for (std::size_t i = 0; i < n->n; ++i) {
        fz::BigInt b = dec_int(&n->items[i], what);
        if (!b.fits_u64()) throw Bad{std::string(what) + " element is outside [0, 2^64)"};
        v.push_back(b.low_u64());
    }
    return v;
}

// ---- problems ----------------------------------------------------------------------

inline std::string clip(std::string_view s, std::size_t max = kClipFragment) {
    std::string out;
    std::size_t m = std::min(s.size(), max);
    for (std::size_t i = 0; i < m; ++i) {
        unsigned char ch = static_cast<unsigned char>(s[i]);
        out.push_back((ch >= 0x20 && ch < 0x7f) ? static_cast<char>(ch) : '?');
    }
    if (s.size() > m && m >= 3) out.replace(m - 3, 3, "...");
    return out;
}
inline std::string hex_clip(const fz::Bytes& b) {
    static const char digits[] = "0123456789abcdef";
    std::string out;
    std::size_t m = std::min(b.size(), kClipFragment / 2);
    for (std::size_t i = 0; i < m; ++i) {
        out.push_back(digits[b[i] >> 4]);
        out.push_back(digits[b[i] & 15]);
    }
    if (b.size() > m && out.size() >= 2) out.replace(out.size() - 2, 2, "..");
    return out;
}

// ---- the runner --------------------------------------------------------------------

class Runner {
public:
    Report evaluate(const pbt_json* document) {
        rep_ = Report();
        work_ = kSequenceWork;
        static const char* const doc_fields[] = {"schema_version", "cases"};
        if (document == nullptr || !has_exact_fields(document, doc_fields, 2)) {
            doc_problem("fuzz-v1: the document must be exactly {schema_version, cases}");
            return rep_;
        }
        const pbt_json* ver = pbt_json_get(document, "schema_version");
        const pbt_json* cases = pbt_json_get(document, "cases");
        if (ver->type != PBT_J_NUM || ver->slen != 1 || ver->s[0] != '1') {
            doc_problem("fuzz-v1: schema_version must be 1");
            return rep_;
        }
        if (cases->type != PBT_J_ARR) {
            doc_problem("fuzz-v1: cases must be a list");
            return rep_;
        }
        if (cases->n > kMaxSize) {
            doc_problem("fuzz-v1: too many cases");
            return rep_;
        }
        std::vector<bool> ops_seen(shapes().size(), false);
        bool accepted_seen = false;
        bool rejected_seen = false;
        for (std::size_t i = 0; i < cases->n; ++i) {
            const pbt_json* cs = &cases->items[i];
            const pbt_json* id = pbt_json_get(cs, "id");
            const pbt_json* op = pbt_json_get(cs, "op");
            if (!is_str(id) || id->slen == 0) {
                doc_problem("fuzz-v1: case #" + std::to_string(i) + " is missing a valid string id");
                continue;
            }
            id_ = std::string(id->s, id->slen);
            for (std::size_t k = 0; k < i; ++k) {
                const pbt_json* other = pbt_json_get(&cases->items[k], "id");
                if (is_str(other) && std::string_view(other->s, other->slen) == std::string_view(id->s, id->slen)) {
                    flag("duplicate case id");
                    break;
                }
            }
            if (!is_str(op)) {
                flag("the op must be a string");
                continue;
            }
            std::size_t shape_index = shapes().size();
            for (std::size_t k = 0; k < shapes().size(); ++k)
                if (std::string_view(shapes()[k].op) == std::string_view(op->s, op->slen)) shape_index = k;
            if (shape_index == shapes().size()) {
                flag("unknown op \"" + clip(std::string_view(op->s, op->slen)) + "\"");
                continue;
            }
            const Shape& sh = shapes()[shape_index];
            if (!fields_match(cs, sh)) {
                flag(std::string("fields do not match the ") + sh.op + " shape exactly");
                continue;
            }
            ok_ = true;
            accepted_ = false;
            try {
                eval(sh.op, cs);
            } catch (const Bad& b) {
                ok_ = false;
                flag("could not be evaluated: " + b.why);
            } catch (const fz::Error& e) {
                ok_ = false;
                flag("could not be evaluated: the engine refused the case (status " + std::to_string(e.status()) + ")");
            }
            if (ok_) {
                ++rep_.evaluated;
                ops_seen[shape_index] = true;
                if (std::string_view(sh.op) == "failure_id_valid") {
                    accepted_seen = accepted_seen || accepted_;
                    rejected_seen = rejected_seen || !accepted_;
                }
            }
        }
        if (cases->n != kCaseCount)
            doc_problem("fuzz-v1: expected 25 cases, got " + std::to_string(cases->n));
        if (rep_.evaluated != kCaseCount)
            doc_problem("fuzz-v1: evaluated " + std::to_string(rep_.evaluated) + " cases, the corpus has 25");
        for (std::size_t k = 0; k < shapes().size(); ++k)
            if (!ops_seen[k]) doc_problem(std::string("fuzz-v1: operation ") + shapes()[k].op + " never exercised");
        if (!accepted_seen) doc_problem("fuzz-v1: failure_id_valid needs a registered-id acceptance");
        if (!rejected_seen) doc_problem("fuzz-v1: failure_id_valid needs an unregistered-id rejection");
        for (const char* fid : {FZ_FID_CRASH, FZ_FID_TIMEOUT, FZ_FID_OOM, FZ_FID_LEAK, FZ_FID_PANIC})
            if (!fz::is_failure_id(fid)) doc_problem(std::string("fuzz-v1: the engine rejects the registered id ") + fid);
        return rep_;
    }

private:
    struct Shape {
        const char* op;
        std::vector<const char*> fields;
    };
    static const std::vector<Shape>& shapes() {
        static const std::vector<Shape> s = {
            {"mutate_bit_flip", {"input", "bit_offset", "count", "output"}},
            {"mutate_byte_flip", {"input", "byte_offset", "count", "output"}},
            {"mutate_arithmetic", {"input", "byte_offset", "delta", "width", "endian", "output"}},
            {"mutate_boundary", {"input", "byte_offset", "width", "val", "endian", "output"}},
            {"mutate_block_delete", {"input", "offset", "length", "output"}},
            {"mutate_block_insert", {"input", "offset", "insert_bytes", "output"}},
            {"mutate_block_duplicate", {"input", "src_offset", "length", "dst_offset", "output"}},
            {"mutate_splice", {"target", "donor", "target_split", "donor_split", "output"}},
            {"mutate_sequence", {"input", "seed", "steps", "output"}},
            {"corpus_admit", {"existing_edges", "trace_edges", "admitted", "new_edges"}},
            {"crash_dedup", {"crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate"}},
            {"failure_id_valid", {"input", "valid"}},
            {"private_reporting", {"target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload"}},
            {"corpus_reject", {"text"}},
        };
        return s;
    }
    static bool fields_match(const pbt_json* cs, const Shape& sh) {
        if (cs->type != PBT_J_OBJ || cs->n != sh.fields.size() + 2) return false;
        if (pbt_json_get(cs, "id") == nullptr || pbt_json_get(cs, "op") == nullptr) return false;
        for (const char* f : sh.fields)
            if (pbt_json_get(cs, f) == nullptr) return false;
        return true;
    }

    void record(const std::string& text) { rep_.problems.push_back(clip(text, kClipProblem)); }
    void doc_problem(const std::string& text) { record(text); }
    void flag(const std::string& text) { record(clip(id_) + ": " + text); }
    const pbt_json* F(const pbt_json* cs, const char* name) const { return pbt_json_get(cs, name); }

    void compare_output(const char* op, const fz::Bytes& got, const pbt_json* want_node) {
        fz::Bytes want = dec_bytes(want_node, "output");
        if (want != got) flag(std::string(op) + " expected " + hex_clip(want) + ", got " + hex_clip(got));
    }
    static std::size_t clamp_big(const fz::BigInt& v, std::size_t n) {
        if (v.is_negative() || v.raw().n == 0) return 0;
        if (!v.fits_u64() || v.low_u64() > n) return n;
        return static_cast<std::size_t>(v.low_u64());
    }

    void eval(const char* opc, const pbt_json* cs) {
        std::string op = opc;
        if (op == "corpus_admit") return eval_admit(cs);
        if (op == "crash_dedup") return eval_dedup(cs);
        if (op == "failure_id_valid") return eval_failure_id(cs);
        if (op == "private_reporting") return eval_private(cs);
        if (op == "corpus_reject") {
            std::string text = dec_str(F(cs, "text"), "text");
            if (fz::tfv_valid(text)) flag("corpus_reject text was accepted by the strict parser");
            return;
        }
        eval_mutator(op, cs);
    }

    void eval_mutator(const std::string& op, const pbt_json* cs) {
        fz::Bytes got;
        bool check_len = false;
        fz::Bytes in;
        if (op == "mutate_splice") {
            fz::Bytes tgt = dec_bytes(F(cs, "target"), "target");
            fz::Bytes donor = dec_bytes(F(cs, "donor"), "donor");
            fz::BigInt a = dec_int(F(cs, "target_split"), "target_split");
            fz::BigInt b = dec_int(F(cs, "donor_split"), "donor_split");
            got = fz::splice(tgt, donor, a, b);
            std::size_t want_len = clamp_big(a, tgt.size()) + donor.size() - clamp_big(b, donor.size());
            compare_output(op.c_str(), got, F(cs, "output"));
            if (got.size() != want_len) flag("splice length is not prefix + suffix");
            return;
        }
        in = dec_bytes(F(cs, "input"), "input");
        if (op == "mutate_bit_flip" || op == "mutate_byte_flip") {
            bool bit = op == "mutate_bit_flip";
            fz::BigInt a = dec_int(F(cs, bit ? "bit_offset" : "byte_offset"), "offset");
            fz::BigInt b = dec_int(F(cs, "count"), "count");
            got = bit ? fz::bit_flip(in, a, b) : fz::byte_flip(in, a, b);
            check_len = true;
        } else if (op == "mutate_arithmetic") {
            fz::BigInt a = dec_int(F(cs, "byte_offset"), "byte_offset");
            fz::BigInt d = dec_int(F(cs, "delta"), "delta");
            int w = dec_width(F(cs, "width"));
            fz::Endian e = dec_endian(F(cs, "endian"));
            got = fz::byte_arithmetic(in, a, d, w, e);
            check_len = true;
        } else if (op == "mutate_boundary") {
            fz::BigInt a = dec_int(F(cs, "byte_offset"), "byte_offset");
            int w = dec_width(F(cs, "width"));
            fz::BigInt v = dec_int(F(cs, "val"), "val");
            fz::Endian e = dec_endian(F(cs, "endian"));
            got = fz::boundary_insert(in, a, w, v, e);
            check_len = true;
        } else if (op == "mutate_block_delete") {
            got = fz::block_delete(in, dec_int(F(cs, "offset"), "offset"), dec_int(F(cs, "length"), "length"));
        } else if (op == "mutate_block_insert") {
            fz::BigInt a = dec_int(F(cs, "offset"), "offset");
            fz::Bytes ins = dec_bytes(F(cs, "insert_bytes"), "insert_bytes");
            got = fz::block_insert(in, a, ins);
        } else if (op == "mutate_block_duplicate") {
            fz::BigInt a = dec_int(F(cs, "src_offset"), "src_offset");
            fz::BigInt l = dec_int(F(cs, "length"), "length");
            fz::BigInt d = dec_int(F(cs, "dst_offset"), "dst_offset");
            got = fz::block_duplicate(in, a, l, d);
        } else {  // mutate_sequence
            fz::BigInt seed = dec_int(F(cs, "seed"), "seed");
            std::size_t steps = dec_size(F(cs, "steps"), "steps");
            if (work_ == 0) throw Bad{"mutate_sequence work budget of 2^24 bytes exhausted"};
            fz::SequenceResult r = fz::mutate_sequence_budget(in, seed.mod_u64(), steps, FZ_MAX_INPUT_BYTES, work_);
            work_ = r.work_done >= work_ ? 0 : work_ - r.work_done;  // charge real work, a capped call included
            if (r.status == FZ_E_WORK) throw Bad{"mutate_sequence work budget of 2^24 bytes exhausted"};
            if (r.status != FZ_OK) throw fz::Error(r.status, "mutate_sequence");
            got = r.bytes;
        }
        compare_output(op.c_str(), got, F(cs, "output"));
        if (check_len && got.size() != in.size()) flag(op + " changed the length of its input");
    }

    void eval_admit(const pbt_json* cs) {
        std::vector<std::uint64_t> existing = dec_edges(F(cs, "existing_edges"), "existing_edges");
        std::vector<std::uint64_t> trace = dec_edges(F(cs, "trace_edges"), "trace_edges");
        bool admitted = dec_bool(F(cs, "admitted"), "admitted");
        fz::BigInt want_new = dec_int(F(cs, "new_edges"), "new_edges");
        fz::Novelty n = fz::corpus_admit(existing, trace);
        if (n.admitted != admitted) flag("corpus_admit admitted " + std::to_string(n.admitted) + ", corpus " + std::to_string(admitted));
        if (!want_new.fits_u64() || want_new.low_u64() != n.new_edges)
            flag("corpus_admit new_edges " + std::to_string(n.new_edges) + ", corpus differs");
        std::set<std::uint64_t> have(existing.begin(), existing.end());
        bool novel = false;
        for (std::uint64_t t : trace) novel = novel || have.count(t) == 0;
        if (n.admitted != novel) flag("corpus_admit disagrees with set difference");
    }

    static bool is_hex8(const std::string& s) {
        if (s.size() != 8) return false;
        for (char c : s)
            if (hexval(c) < 0) return false;
        return true;
    }
    void eval_dedup(const pbt_json* cs) {
        std::string ka = dec_str(F(cs, "crash_a_kind"), "crash_a_kind");
        fz::Frames fa = dec_strs(F(cs, "crash_a_frames"), "crash_a_frames");
        std::string kb = dec_str(F(cs, "crash_b_kind"), "crash_b_kind");
        fz::Frames fb = dec_strs(F(cs, "crash_b_frames"), "crash_b_frames");
        bool want = dec_bool(F(cs, "is_duplicate"), "is_duplicate");
        bool got = fz::is_duplicate_crash(ka, fa, kb, fb);
        if (got != want) flag("crash_dedup is_duplicate disagrees with the corpus (" + std::to_string(want) + ")");
        std::string sa = fz::crash_signature(ka, fa);
        std::string sb = fz::crash_signature(kb, fb);
        if (!is_hex8(sa) || !is_hex8(sb)) flag("a crash signature is not 8 lowercase hex digits");
        if ((sa == sb) != want) flag("crash signatures disagree with is_duplicate");
    }
    void eval_failure_id(const pbt_json* cs) {
        std::string in = dec_str(F(cs, "input"), "input");
        bool valid = dec_bool(F(cs, "valid"), "valid");
        bool got = fz::is_failure_id(in);
        accepted_ = got;
        if (got != valid) flag("failure id membership disagrees for \"" + clip(in) + "\"");
    }
    void eval_private(const pbt_json* cs) {
        std::string target = dec_str(F(cs, "target"), "target");
        std::string fid = dec_str(F(cs, "failure_id"), "failure_id");
        fz::Bytes raw = dec_bytes(F(cs, "raw_payload"), "raw_payload");
        std::string sig = dec_str(F(cs, "sig_hash"), "sig_hash");
        std::string tap = dec_str(F(cs, "tap_line"), "tap_line");
        bool want_leak = dec_bool(F(cs, "contains_raw_payload"), "contains_raw_payload");
        std::string line;
        try {
            line = fz::format_tap_failure(1, target, fid, sig);
        } catch (const fz::Error& e) {
            throw Bad{"format_tap_failure refused (status " + std::to_string(e.status()) + ")"};
        }
        if (line != tap) flag("tap_line expected " + clip(tap) + ", got " + clip(line));
        bool leaked = false;
        if (!raw.empty()) {
            static const char digits[] = "0123456789abcdef";
            std::string hx;
            for (std::uint8_t b : raw) {
                hx.push_back(digits[b >> 4]);
                hx.push_back(digits[b & 15]);
            }
            leaked = line.find(hx) != std::string::npos;
        }
        if (leaked != want_leak) flag("privacy verdict wrong, payload leaked: " + std::to_string(leaked));
        if (line.find_first_of("\r\n") != std::string::npos) flag("failure line spans more than one line");
    }

    Report rep_;
    std::string id_;
    bool ok_ = true;
    bool accepted_ = false;
    std::uint64_t work_ = kSequenceWork;
};

}  // namespace fuzzcorpus

#endif  // CA_FUZZ_TESTS_CORPUS_RUNNER_HPP
