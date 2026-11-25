// corpus_runner.hpp -- the C++ replay of the shared pbt-v1 corpus (VER05j section 5).
// ============================================================================
//
// WHY A SECOND RUNNER
// -------------------
// The C package already evaluates this corpus (pbt_corpus_evaluate).  If the C++
// binding were evaluated by THAT function, a broken C++ layer (a shifted seed, a
// dropped step, a swapped bound, a leaked closure) would hide behind a green C
// result.  So this runner shares only the DECODING of the strict JSON text with
// C (pbt_json_parse) and then goes through the C++ API for everything else:
// values are built with Value's factories, generators with ca::pbt::gen, shrinks
// with ca::pbt::shrink_*, the filter case with check_property.  It never calls
// pbt_corpus_evaluate.
//
// It is deliberately paranoid about its input, because the input is a file: a
// case has exactly the fields of its op, integers are canonical decimal strings,
// bytes are lowercase even hex, an unknown op / property / malformed value is a
// PROBLEM (never a skip), sizes are capped, and every problem starts with the
// case id so a negative control can prove WHICH case failed.

#ifndef CA_PBT_TESTS_CORPUS_RUNNER_HPP
#define CA_PBT_TESTS_CORPUS_RUNNER_HPP

#include <cstdio>
#include <cstring>
#include <functional>
#include <map>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "test_bootstrap.hpp"

extern "C" {
#include "pbt_json.h"
}

namespace corpus {

namespace pbt = ca::pbt;

constexpr std::size_t kMaxSize = 10000;
constexpr std::uint64_t kMaxTotalGen = std::uint64_t(1) << 20;
constexpr std::size_t kCaseCount = 25;

struct Report {
    std::size_t evaluated = 0;
    std::vector<std::string> problems;
};

// A fixture fragment that cannot be decoded.
struct Bad {
    std::string why;
};

// ---- strict decoding of the JSON tree -----------------------------------------------

inline const pbt_json* field(const pbt_json* c, const char* name) { return pbt_json_get(c, name); }
inline bool is_str(const pbt_json* n) { return n != nullptr && n->type == PBT_J_STR; }
// The node is a string equal to the literal, embedded NULs included.
inline bool str_is(const pbt_json* n, const char* lit) {
    return is_str(n) && std::string_view(n->s, n->slen) == std::string_view(lit);
}
inline std::string text_of(const pbt_json* n) { return std::string(n->s, n->slen); }

inline bool has_exact_fields(const pbt_json* obj, const char* const* names, std::size_t n) {
    if (obj->type != PBT_J_OBJ || obj->n != n) return false;
    for (std::size_t i = 0; i < n; ++i)
        if (pbt_json_get(obj, names[i]) == nullptr) return false;
    return true;
}

// -?(0|[1-9][0-9]*), never "-0", at most 300 digits.
inline bool canonical_int(std::string_view s) {
    std::size_t i = (!s.empty() && s[0] == '-') ? 1 : 0;
    std::size_t digits = s.size() - i;
    if (digits == 0) return false;
    for (std::size_t k = i; k < s.size(); ++k)
        if (s[k] < '0' || s[k] > '9') return false;
    if (s[i] == '0' && (digits > 1 || i == 1)) return false;
    return digits <= PBT_BIG_MAX_DIGITS;
}

// {"kind":"int","value":"<canonical decimal>"}
inline pbt::Value dec_int(const pbt_json* n) {
    static const char* const names[] = {"kind", "value"};
    if (n == nullptr || !has_exact_fields(n, names, 2)) throw Bad{"an int must be {kind, value}"};
    const pbt_json* kind = pbt_json_get(n, "kind");
    const pbt_json* val = pbt_json_get(n, "value");
    if (!str_is(kind, "int")) throw Bad{"an int needs kind \"int\""};
    if (!is_str(val) || !canonical_int(std::string_view(val->s, val->slen)))
        throw Bad{"an int value must be a canonical decimal string within the digit cap"};
    try {
        return pbt::Value::integer_dec(std::string_view(val->s, val->slen));
    } catch (const pbt::Error&) {
        throw Bad{"an int value is too wide for this lane"};
    }
}

// A small non-negative int in [lo, hi].
inline std::uint64_t dec_small(const pbt_json* n, std::uint64_t lo, std::uint64_t hi) {
    pbt::Value v = dec_int(n);
    std::uint64_t out = 0;
    try {
        if (v.cmp_i64(0) < 0) throw Bad{"a size must be a small non-negative int"};
        out = v.as_u64();
    } catch (const pbt::Error&) {
        throw Bad{"a size must be a small non-negative int"};
    }
    if (out < lo || out > hi) throw Bad{"a size is outside its cap"};
    return out;
}

inline int hexval(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

// {"kind":"bytes","hex":"<even-length lowercase hex>"}
inline pbt::Value dec_bytes(const pbt_json* n) {
    static const char* const names[] = {"kind", "hex"};
    if (n == nullptr || !has_exact_fields(n, names, 2)) throw Bad{"bytes must be {kind, hex}"};
    const pbt_json* kind = pbt_json_get(n, "kind");
    const pbt_json* hex = pbt_json_get(n, "hex");
    if (!str_is(kind, "bytes") || !is_str(hex)) throw Bad{"bytes need kind \"bytes\" and a hex string"};
    if (hex->slen % 2 != 0 || hex->slen / 2 > kMaxSize) throw Bad{"bytes hex must be even length and within the cap"};
    std::vector<std::uint8_t> data(hex->slen / 2);
    for (std::size_t i = 0; i < hex->slen; i += 2) {
        int h = hexval(hex->s[i]);
        int l = hexval(hex->s[i + 1]);
        if (h < 0 || l < 0) throw Bad{"bytes hex must be lowercase"};
        data[i / 2] = static_cast<std::uint8_t>(h * 16 + l);
    }
    return pbt::Value::bytes(data);
}

inline pbt::Value dec_value(const pbt_json* n, int depth);

inline pbt::Value dec_list(const pbt_json* n, int depth) {
    if (n == nullptr || n->type != PBT_J_ARR) throw Bad{"expected a list"};
    if (n->n > kMaxSize) throw Bad{"a list is longer than the cap"};
    std::vector<pbt::Value> items;
    items.reserve(n->n);
    for (std::size_t i = 0; i < n->n; ++i) items.push_back(dec_value(&n->items[i], depth + 1));
    return pbt::Value::list(std::move(items));
}

// bool | int | bytes | list, bounded to 64 levels.
inline pbt::Value dec_value(const pbt_json* n, int depth) {
    if (depth > PBT_MAX_VAL_DEPTH) throw Bad{"values nest too deeply"};
    if (n->type == PBT_J_BOOL) return pbt::Value::boolean(n->b != 0);
    if (n->type == PBT_J_ARR) return dec_list(n, depth);
    if (n->type == PBT_J_OBJ) {
        if (str_is(pbt_json_get(n, "kind"), "bytes")) return dec_bytes(n);
        return dec_int(n);
    }
    throw Bad{"unrecognised value"};
}

// ---- the named properties: the TEST's definition of each (true = holds) -------------------

using Holds = std::function<bool(pbt::ValueView)>;
struct NamedProp {
    pbt::Kind kind;
    Holds holds;
};

inline const std::map<std::string, NamedProp>& props() {
    static const std::map<std::string, NamedProp> table = {
        {"int_le_100", {pbt::Kind::Int, [](pbt::ValueView v) { return v.cmp_i64(100) <= 0; }}},
        {"int_in_range_neg10_50",
         {pbt::Kind::Int, [](pbt::ValueView v) { return v.cmp_i64(-10) >= 0 && v.cmp_i64(50) <= 0; }}},
        {"int_is_even",
         {pbt::Kind::Int,
          [](pbt::ValueView v) {
              std::string d = v.decimal();
              return ((d.back() - '0') % 2) == 0;
          }}},
        {"int_abs_le_10", {pbt::Kind::Int, [](pbt::ValueView v) { return v.cmp_i64(-10) >= 0 && v.cmp_i64(10) <= 0; }}},
        {"refinement_x_le_50", {pbt::Kind::Int, [](pbt::ValueView v) { return v.cmp_i64(50) <= 0; }}},
        {"list_no_consecutive_duplicates",
         {pbt::Kind::List,
          [](pbt::ValueView v) {
              for (std::size_t i = 0; i + 1 < v.size(); ++i)
                  if (v.at(i).as_i64() == v.at(i + 1).as_i64()) return false;  // a non-int throws: panic
              return true;
          }}},
        {"list_sum_le_20",
         {pbt::Kind::List,
          [](pbt::ValueView v) {
              std::int64_t sum = 0;
              for (std::size_t i = 0; i < v.size(); ++i) sum += v.at(i).as_i64();
              return sum <= 20;
          }}},
        {"bytes_len_lt_5", {pbt::Kind::Bytes, [](pbt::ValueView v) { return v.size() < 5; }}},
        {"bytes_no_zero_byte",
         {pbt::Kind::Bytes,
          [](pbt::ValueView v) {
              for (std::uint8_t b : v.as_bytes())
                  if (b == 0) return false;
              return true;
          }}},
    };
    return table;
}

// The property named by `n`, which must exist and expect `kind`; else Bad.
inline const NamedProp& find_prop(const pbt_json* n, pbt::Kind kind) {
    if (!is_str(n)) throw Bad{"unknown or mistyped property"};
    auto it = props().find(text_of(n));
    if (it == props().end() || it->second.kind != kind) throw Bad{"unknown or mistyped property"};
    return it->second;
}

// ---- the evaluator ---------------------------------------------------------------------------

class Runner {
public:
    Report evaluate(const pbt_json* document) {
        Report rep;
        rep_ = &rep;
        run(document);
        rep_ = nullptr;
        return rep;
    }

private:
    Report* rep_ = nullptr;
    std::string id_;

    static std::string clip(std::string s, std::size_t max) {
        for (char& c : s)
            if (static_cast<unsigned char>(c) < 0x20 || static_cast<unsigned char>(c) >= 0x7f) c = '?';
        if (s.size() > max) {
            s.resize(max);
            s.replace(max - 3, 3, "...");
        }
        return s;
    }
    // Record "<id>: <text>" (id empty: a document-level problem).
    void problem(const std::string& id, const std::string& text) {
        std::string line = id.empty() ? text : clip(id, 200) + ": " + text;
        rep_->problems.push_back(clip(line, 400));
    }
    void problem(const std::string& text) { problem(id_, text); }

    // ---- gen ops -------------------------------------------------------------------------
    void compare_gen(const pbt::Gen& gen, std::uint64_t seed, std::uint64_t count, const pbt_json* outputs,
                     const char* what) {
        if (outputs == nullptr || outputs->type != PBT_J_ARR || outputs->n != count) {
            problem(std::string(what) + ": outputs must be a list of exactly count (" + std::to_string(count) + ") values");
            return;
        }
        pbt::Value expected = dec_list(outputs, 0);
        std::vector<pbt::Value> want = expected.items();
        pbt::Rng rng(seed);
        for (std::uint64_t i = 0; i < count; ++i) {
            pbt::Value got;
            try {
                got = gen.sample(rng);
            } catch (const pbt::Error& e) {
                problem(std::string(what) + ": generation failed at index " + std::to_string(i) + " (status " +
                        std::to_string(e.status()) + ")");
                return;
            }
            if (!(got == want[i])) {
                problem(std::string(what) + " outputs mismatch at index " + std::to_string(i));
                return;
            }
        }
    }

    void eval_gen_primitive(const pbt_json* cs) {
        const pbt_json* type = field(cs, "type");
        if (!is_str(type)) {
            problem("gen_primitive: type must be a string");
            return;
        }
        std::uint64_t seed = dec_int(field(cs, "seed")).to_u64_wrapping();
        std::uint64_t count = dec_small(field(cs, "count"), 0, kMaxSize);
        if (str_is(type, "bool")) {
            compare_gen(pbt::gen::boolean(), seed, count, field(cs, "outputs"), "gen_primitive");
        } else if (str_is(type, "u8")) {
            compare_gen(pbt::gen::u8(), seed, count, field(cs, "outputs"), "gen_primitive");
        } else if (str_is(type, "u64")) {
            compare_gen(pbt::gen::u64(), seed, count, field(cs, "outputs"), "gen_primitive");
        } else if (str_is(type, "i128")) {
            compare_gen(pbt::gen::i128(), seed, count, field(cs, "outputs"), "gen_primitive");
        } else {
            problem("gen_primitive: unknown primitive type \"" + clip(text_of(type), 200) + "\"");
        }
    }

    void eval_gen_bounded(const pbt_json* cs) {
        pbt::Value lo = dec_int(field(cs, "min"));
        pbt::Value hi = dec_int(field(cs, "max"));
        std::uint64_t seed = dec_int(field(cs, "seed")).to_u64_wrapping();
        std::uint64_t count = dec_small(field(cs, "count"), 0, kMaxSize);
        if (pbt_big_cmp(&lo.raw()->u.i, &hi.raw()->u.i) > 0) {
            problem("gen_bounded: min exceeds max");
            return;
        }
        pbt_big span;
        (void)pbt_big_sub(&span, &hi.raw()->u.i, &lo.raw()->u.i);
        std::uint64_t width = (pbt_big_bitlen(&span) + 7u) / 8u;
        if (width < 1) width = 1;
        if (count * width > kMaxTotalGen) {
            problem("gen_bounded: count x width exceeds the total generation budget");
            return;
        }
        compare_gen(pbt::gen::integer(lo.decimal(), hi.decimal()), seed, count, field(cs, "outputs"), "gen_bounded");
    }

    void eval_gen_bytes(const pbt_json* cs) {
        std::uint64_t lo = dec_small(field(cs, "min_len"), 0, kMaxSize);
        std::uint64_t hi = dec_small(field(cs, "max_len"), 0, kMaxSize);
        std::uint64_t seed = dec_int(field(cs, "seed")).to_u64_wrapping();
        std::uint64_t count = dec_small(field(cs, "count"), 0, kMaxSize);
        if (lo > hi) {
            problem("gen_bytes: min_len exceeds max_len");
            return;
        }
        if (hi > 0 && count * hi > kMaxTotalGen) {
            problem("gen_bytes: count x max_len exceeds the total generation budget");
            return;
        }
        compare_gen(pbt::gen::bytes(static_cast<std::size_t>(lo), static_cast<std::size_t>(hi)), seed, count,
                    field(cs, "outputs"), "gen_bytes");
    }

    // ---- shrink ops ------------------------------------------------------------------------
    void check_steps(const char* op, int got, std::uint64_t want) {
        if (static_cast<std::uint64_t>(got) != want)
            problem(std::string(op) + " step count mismatch: got " + std::to_string(got) + ", want " +
                    std::to_string(want));
    }

    void eval_shrink_int(const pbt_json* cs, const char* op) {
        const NamedProp& p = find_prop(field(cs, "property"), pbt::Kind::Int);
        pbt::Value initial = dec_int(field(cs, "initial"));
        pbt::Value target = dec_int(field(cs, "target"));
        pbt::Value lo = dec_int(field(cs, "min"));
        pbt::Value hi = dec_int(field(cs, "max"));
        pbt::Value expected = dec_int(field(cs, "expected_minimal"));
        std::uint64_t exp_steps = dec_small(field(cs, "expected_steps"), 0, PBT_MAX_SHRINK_STEPS);
        try {
            pbt::Shrunk s = pbt::shrink_int(initial, target, lo, hi, p.holds, 1000);
            if (!(s.minimal == expected)) problem(std::string(op) + " minimal mismatch");
            check_steps(op, s.steps, exp_steps);
        } catch (const pbt::Error& e) {
            problem(std::string(op) + ": shrinking failed (status " + std::to_string(e.status()) + ")");
        }
    }

    void eval_shrink_list(const pbt_json* cs) {
        const NamedProp& p = find_prop(field(cs, "property"), pbt::Kind::List);
        pbt::Value list = dec_list(field(cs, "initial"), 0);
        pbt::Value expected = dec_list(field(cs, "expected_minimal"), 0);
        for (std::size_t i = 0; i < list.size(); ++i)
            if (list.at(i).kind() != pbt::Kind::Int) {
                problem("shrink_list: initial must be a list of ints");
                return;
            }
        pbt::Value target = dec_int(field(cs, "elem_target"));
        std::uint64_t exp_steps = dec_small(field(cs, "expected_steps"), 0, PBT_MAX_SHRINK_STEPS);
        try {
            pbt::Shrunk s = pbt::shrink_list_ints(list, target, p.holds, 1000);
            if (!(s.minimal == expected)) problem("shrink_list minimal mismatch");
            check_steps("shrink_list", s.steps, exp_steps);
        } catch (const pbt::Error& e) {
            problem("shrink_list: shrinking failed (status " + std::to_string(e.status()) + ")");
        }
    }

    void eval_shrink_bytes(const pbt_json* cs) {
        const NamedProp& p = find_prop(field(cs, "property"), pbt::Kind::Bytes);
        pbt::Value bytes = dec_bytes(field(cs, "initial"));
        pbt::Value expected = dec_bytes(field(cs, "expected_minimal"));
        std::uint64_t exp_steps = dec_small(field(cs, "expected_steps"), 0, PBT_MAX_SHRINK_STEPS);
        try {
            pbt::Shrunk s = pbt::shrink_bytes(bytes, p.holds, 1000);
            if (!(s.minimal == expected)) problem("shrink_bytes minimal mismatch");
            check_steps("shrink_bytes", s.steps, exp_steps);
        } catch (const pbt::Error& e) {
            problem("shrink_bytes: shrinking failed (status " + std::to_string(e.status()) + ")");
        }
    }

    void eval_replay(const pbt_json* cs) {
        const NamedProp& p = find_prop(field(cs, "property"), pbt::Kind::Int);
        std::uint64_t seed = dec_int(field(cs, "seed")).to_u64_wrapping();
        pbt::Value want_initial = dec_int(field(cs, "initial_fail"));
        pbt::Value want_min = dec_int(field(cs, "expected_minimal"));
        try {
            pbt::Replay r = pbt::simulate_replay(seed, p.holds);
            if (!(r.initial_fail == want_initial)) problem("replay initial_fail mismatch");
            if (!(r.minimal == want_min)) problem("replay minimal mismatch");
        } catch (const pbt::Error& e) {
            problem("replay_consistency: replay failed (status " + std::to_string(e.status()) + ")");
        }
    }

    // ---- the verdict op: a filter nothing satisfies, run end to end ---------------------------
    void eval_filter_exhausted(const pbt_json* cs) {
        std::uint64_t retries = dec_small(field(cs, "max_retries"), 1, kMaxSize);
        const pbt_json* verdict = field(cs, "verdict");
        const pbt_json* fid = field(cs, "failure_id");
        if (!is_str(verdict) || !is_str(fid)) {
            problem("filter_exhausted: verdict and failure_id must be strings");
            return;
        }
        unsigned long calls = 0;
        pbt::Gen filtered = pbt::gen::u8().filter(
            [&calls](pbt::ValueView) {
                ++calls;
                return false;
            },
            static_cast<int>(retries));
        pbt::Options o;
        o.num_tests = 1;
        o.max_shrink_steps = 1000;
        o.ci = pbt::CiMode::Off;
        pbt::Verdict v = pbt::check_property(1, filtered, [](pbt::ValueView) { return true; }, o);
        if (v.passed() || v.failure_id() != PBT_FAIL_EXHAUSTED)
            problem("filter_exhausted: the run did not report property-exhausted");
        if (calls != static_cast<unsigned long>(retries))
            problem("filter_exhausted: predicate called " + std::to_string(calls) + " times, want " +
                    std::to_string(retries));
        if (!str_is(verdict, "fail") || v.failure_id().empty() || text_of(fid) != v.failure_id())
            problem("filter_exhausted: expected verdict/failure_id disagree with the engine");
    }

    struct Seen {
        bool ids[4] = {false, false, false, false};
        bool rejected = false;
    };
    static const char* failure_id_name(int i) {
        static const char* const names[4] = {PBT_FAIL_COUNTEREXAMPLE, PBT_FAIL_EXHAUSTED, PBT_FAIL_TIMEOUT,
                                             PBT_FAIL_PANIC};
        return names[i];
    }

    void eval_failure_id(const pbt_json* cs, Seen& seen) {
        const pbt_json* in = field(cs, "input");
        const pbt_json* valid = field(cs, "valid");
        if (!is_str(in) || valid == nullptr || valid->type != PBT_J_BOOL) {
            problem("failure_id_valid: input must be a string and valid a boolean");
            return;
        }
        bool got = pbt::is_failure_id(std::string_view(in->s, in->slen));
        if (got != (valid->b != 0)) problem("failure_id_valid membership disagrees");
        if (got) {
            for (int i = 0; i < 4; ++i)
                if (text_of(in) == failure_id_name(i)) seen.ids[i] = true;
        } else {
            seen.rejected = true;
        }
    }

    void eval_corpus_reject(const pbt_json* cs) {
        const pbt_json* text = field(cs, "text");
        if (!is_str(text)) {
            problem("corpus_reject: text must be a string");
            return;
        }
        if (pbt::tfv_valid(std::string_view(text->s, text->slen)))
            problem("corpus_reject: text was accepted by the strict TFV validator");
    }

    // ---- shapes and the document ---------------------------------------------------------------
    struct Shape {
        const char* op;
        std::vector<const char*> fields;
    };
    static const std::vector<Shape>& shapes() {
        static const std::vector<const char*> shrink_int = {"initial", "target", "min", "max", "property",
                                                            "expected_minimal", "expected_steps"};
        static const std::vector<Shape> s = {
            {"gen_primitive", {"type", "seed", "count", "outputs"}},
            {"gen_bounded", {"min", "max", "seed", "count", "outputs"}},
            {"gen_bytes", {"min_len", "max_len", "seed", "count", "outputs"}},
            {"shrink_int", shrink_int},
            {"shrink_refinement", shrink_int},
            {"shrink_list", {"initial", "elem_target", "property", "expected_minimal", "expected_steps"}},
            {"shrink_bytes", {"initial", "property", "expected_minimal", "expected_steps"}},
            {"replay_consistency", {"seed", "property", "initial_fail", "expected_minimal"}},
            {"filter_exhausted", {"max_retries", "verdict", "failure_id"}},
            {"failure_id_valid", {"input", "valid"}},
            {"corpus_reject", {"text"}},
        };
        return s;
    }
    static bool shape_matches(const pbt_json* cs, const Shape& sh) {
        if (cs->type != PBT_J_OBJ || cs->n != sh.fields.size() + 2) return false;
        if (pbt_json_get(cs, "id") == nullptr || pbt_json_get(cs, "op") == nullptr) return false;
        for (const char* f : sh.fields)
            if (pbt_json_get(cs, f) == nullptr) return false;
        return true;
    }
    static bool id_seen_before(const pbt_json* cases, std::size_t upto, const pbt_json* id) {
        for (std::size_t i = 0; i < upto; ++i) {
            const pbt_json* other = pbt_json_get(&cases->items[i], "id");
            if (is_str(other) && other->slen == id->slen && std::memcmp(other->s, id->s, id->slen) == 0) return true;
        }
        return false;
    }

    void dispatch(const std::string& op, const pbt_json* cs, Seen& seen) {
        if (op == "gen_primitive") eval_gen_primitive(cs);
        else if (op == "gen_bounded") eval_gen_bounded(cs);
        else if (op == "gen_bytes") eval_gen_bytes(cs);
        else if (op == "shrink_int" || op == "shrink_refinement") eval_shrink_int(cs, op.c_str());
        else if (op == "shrink_list") eval_shrink_list(cs);
        else if (op == "shrink_bytes") eval_shrink_bytes(cs);
        else if (op == "replay_consistency") eval_replay(cs);
        else if (op == "filter_exhausted") eval_filter_exhausted(cs);
        else if (op == "failure_id_valid") eval_failure_id(cs, seen);
        else eval_corpus_reject(cs);
    }

    void run(const pbt_json* document) {
        static const char* const doc_fields[] = {"schema_version", "cases"};
        const std::vector<Shape>& sh_all = shapes();
        std::vector<bool> ops_seen(sh_all.size(), false);
        Seen seen;
        if (document == nullptr || !has_exact_fields(document, doc_fields, 2)) {
            problem("", "pbt-v1: the document must be exactly {schema_version, cases}");
            return;
        }
        const pbt_json* ver = pbt_json_get(document, "schema_version");
        const pbt_json* cases = pbt_json_get(document, "cases");
        if (ver->type != PBT_J_NUM || ver->slen != 1 || ver->s[0] != '1') {
            problem("", "pbt-v1: schema_version must be 1");
            return;
        }
        if (cases->type != PBT_J_ARR) {
            problem("", "pbt-v1: cases must be a list");
            return;
        }
        if (cases->n > kMaxSize) {
            problem("", "pbt-v1: " + std::to_string(cases->n) + " cases exceed the cap of " + std::to_string(kMaxSize));
            return;
        }
        for (std::size_t i = 0; i < cases->n; ++i) {
            const pbt_json* cs = &cases->items[i];
            const pbt_json* id = pbt_json_get(cs, "id");
            const pbt_json* op = pbt_json_get(cs, "op");
            if (!is_str(id) || id->slen == 0) {
                problem("", "pbt-v1: case #" + std::to_string(i) + " is missing a valid string id");
                continue;
            }
            id_ = text_of(id);
            if (id_seen_before(cases, i, id)) problem("duplicate case id");
            if (!is_str(op)) {
                problem("the op must be a string");
                continue;
            }
            const Shape* sh = nullptr;
            for (std::size_t k = 0; k < sh_all.size(); ++k) {
                if (text_of(op) == sh_all[k].op) {
                    sh = &sh_all[k];
                    ops_seen[k] = true;
                    break;
                }
            }
            if (sh == nullptr) {
                problem("unknown op \"" + clip(text_of(op), 200) + "\"");
                continue;
            }
            if (!shape_matches(cs, *sh)) {
                problem(std::string("fields do not match the ") + sh->op + " shape exactly");
                continue;
            }
            ++rep_->evaluated;
            try {
                dispatch(sh->op, cs, seen);
            } catch (const Bad& b) {
                problem(std::string(sh->op) + ": " + b.why);
            } catch (const pbt::Error& e) {
                problem(std::string(sh->op) + ": binding error: " + e.what());
            }
        }
        id_.clear();
        // Coverage gates: a corpus that silently stopped exercising something is a failure.
        if (rep_->evaluated != kCaseCount)
            problem("", "pbt-v1: evaluated " + std::to_string(rep_->evaluated) + " cases, the corpus has " +
                            std::to_string(kCaseCount));
        for (std::size_t k = 0; k < sh_all.size(); ++k)
            if (!ops_seen[k]) problem("", std::string("pbt-v1: operation ") + sh_all[k].op + " never exercised in the corpus");
        for (int k = 0; k < 4; ++k)
            if (!seen.ids[k]) problem("", std::string("pbt-v1: registered failure id ") + failure_id_name(k) + " missing");
        if (!seen.rejected) problem("", "pbt-v1: failure_id_valid needs at least one unregistered-id rejection");
    }
};

}  // namespace corpus

#endif  // CA_PBT_TESTS_CORPUS_RUNNER_HPP
