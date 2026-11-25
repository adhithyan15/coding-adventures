// test_corpus.cpp -- replay the committed pbt-v1 corpus THROUGH THE C++ BINDING and
// prove the run CAN FAIL (VER05j sections 5 and 6).
//
// A conformance run that cannot fail proves nothing (the LH-003 false pass this
// repo has been burned by).  After the positive run (25/25, exact outputs,
// minimals AND step counts) this file:
//
//   - corrupts EVERY real case, per op, in each way the op defines, and asserts
//     the run fails, that a problem NAMES that case, and that no OTHER case is
//     named (a single-case control proves only one path);
//   - makes every field of every case the wrong type;
//   - drops and truncates the case list, adds unknown ops, extra and missing
//     fields, duplicate ids, a wrong schema version;
//   - SABOTAGES THE C++ LAYER ITSELF (shift every seed, drop a step, swap
//     bounds) and requires the corpus run to notice, then restores it.
// Only this translation unit compiles the C++-layer test seams in (VER05j section 6).  Every
// other test, test_api included, exercises the production configuration with no seam global.
#define CA_PBT_TEST_SEAMS 1
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "corpus_runner.hpp"
#include "iso_test.h"

extern "C" {
#include "pbt_internal.h"
}

namespace {

namespace pbt = ca::pbt;

// ---- loading the committed fixture ---------------------------------------------------------
std::string read_file(const std::string& path, bool* ok) {
    std::FILE* f = std::fopen(path.c_str(), "rb");
    *ok = f != nullptr;
    std::string out;
    if (f == nullptr) return out;
    char buf[4096];
    std::size_t n;
    while ((n = std::fread(buf, 1, sizeof buf, f)) > 0) out.append(buf, n);
    std::fclose(f);
    return out;
}

// code/specs/fixtures/pbt-v1/cases.json, found by walking up from the working directory.
std::string corpus_text() {
    static const char* const prefixes[] = {"", "../", "../../", "../../../", "../../../../", "../../../../../",
                                           "../../../../../../", "../../../../../../../"};
    for (const char* p : prefixes) {
        bool ok = false;
        std::string text = read_file(std::string(p) + "code/specs/fixtures/pbt-v1/cases.json", &ok);
        if (ok) return text;
    }
    return std::string();
}

// A parsed document that frees itself.
struct Doc {
    pbt_json* root = nullptr;
    Doc() = default;
    explicit Doc(pbt_json* r) : root(r) {}
    Doc(const Doc&) = delete;
    Doc& operator=(const Doc&) = delete;
    Doc(Doc&& o) noexcept : root(o.root) { o.root = nullptr; }
    ~Doc() { pbt_json_free(root); }
    Doc clone() const { return Doc(pbt_json_clone(root)); }
};

Doc parse(const std::string& text) {
    pbt_json* root = nullptr;
    char err[200];
    if (pbt_json_parse(text.data(), text.size(), &root, err, sizeof err) != 0) return Doc();
    return Doc(root);
}

// ---- JSON tree helpers for building corruptions ---------------------------------------------
void set_str(pbt_json* node, const char* text) {
    std::size_t n = std::strlen(text);
    pbt_free(node->s);
    node->s = static_cast<char*>(pbt_alloc(n + 1, 1));
    std::memcpy(node->s, text, n + 1);
    node->slen = n;
}
pbt_json* cases_of(pbt_json* doc) { return pbt_json_get(doc, "cases"); }
pbt_json* case_at(pbt_json* doc, std::size_t i) { return &cases_of(doc)->items[i]; }
const char* case_id(pbt_json* doc, std::size_t i) { return pbt_json_get(case_at(doc, i), "id")->s; }
std::string op_of(pbt_json* cs) { return pbt_json_get(cs, "op")->s; }
void set_int_field(pbt_json* cs, const char* field, const char* dec) {
    set_str(pbt_json_get(pbt_json_get(cs, field), "value"), dec);
}

corpus::Report evaluate(pbt_json* doc) {
    corpus::Runner r;
    return r.evaluate(doc);
}

bool names(const corpus::Report& r, const std::string& id) {
    for (const std::string& p : r.problems)
        if (p.compare(0, id.size() + 1, id + ":") == 0) return true;
    return false;
}
bool names_other(const corpus::Report& r, pbt_json* doc, const std::string& id) {
    for (std::size_t k = 0; k < cases_of(doc)->n; ++k) {
        std::string other = case_id(doc, k);
        if (other != id && names(r, other)) return true;
    }
    return false;
}

// ---- the positive run ---------------------------------------------------------------------------
void test_positive(pbt_json* doc) {
    corpus::Report rep = evaluate(doc);
    for (const std::string& p : rep.problems) std::printf("  problem: %s\n", p.c_str());
    ISO_CHECK_EQ_UINT(rep.problems.size(), 0);
    ISO_CHECK_EQ_UINT(rep.evaluated, 25);
    ISO_CHECK_EQ_UINT(cases_of(doc)->n, 25);
    std::printf("  corpus: %lu/25 cases evaluated through the C++ API, %lu problems\n",
                static_cast<unsigned long>(rep.evaluated), static_cast<unsigned long>(rep.problems.size()));
}

// ---- per-op corruptions ---------------------------------------------------------------------------
void bump_int(pbt_json* intobj) {
    pbt_json* v = pbt_json_get(intobj, "value");
    set_str(v, std::strcmp(v->s, "424242") == 0 ? "424243" : "424242");
}
void steps_plus_one(pbt_json* cs) {
    pbt_json* v = pbt_json_get(pbt_json_get(cs, "expected_steps"), "value");
    set_str(v, std::to_string(std::atoi(v->s) + 1).c_str());
}

int variants(pbt_json* cs) {
    std::string op = op_of(cs);
    if (op.compare(0, 4, "gen_") == 0) return 2;
    if (op.compare(0, 7, "shrink_") == 0 || op == "replay_consistency" || op == "filter_exhausted") return 2;
    return 1;
}

void corrupt(pbt_json* cs, int v) {
    std::string op = op_of(cs);
    if (op.compare(0, 4, "gen_") == 0) {
        pbt_json* outs = pbt_json_get(cs, "outputs");
        if (v == 0) {
            pbt_json* first = &outs->items[0];
            if (first->type == PBT_J_BOOL) {
                first->b = !first->b;
            } else if (pbt_json_get(first, "hex") != nullptr) {
                pbt_json* hex = pbt_json_get(first, "hex");
                set_str(hex, hex->slen == 0 ? "00" : (hex->s[0] == '0' ? "11" : "00"));
            } else {
                bump_int(first);
            }
        } else {
            pbt_json* cv = pbt_json_get(pbt_json_get(cs, "count"), "value");
            set_str(cv, std::strcmp(cv->s, "9") == 0 ? "10" : "9");
        }
    } else if (op == "shrink_int" || op == "shrink_refinement") {
        if (v == 0) steps_plus_one(cs); else bump_int(pbt_json_get(cs, "expected_minimal"));
    } else if (op == "shrink_list") {
        if (v == 0) steps_plus_one(cs); else bump_int(&pbt_json_get(cs, "expected_minimal")->items[0]);
    } else if (op == "shrink_bytes") {
        if (v == 0) steps_plus_one(cs); else set_str(pbt_json_get(pbt_json_get(cs, "expected_minimal"), "hex"), "ff");
    } else if (op == "replay_consistency") {
        if (v == 0) bump_int(pbt_json_get(cs, "initial_fail")); else bump_int(pbt_json_get(cs, "expected_minimal"));
    } else if (op == "filter_exhausted") {
        if (v == 0) set_str(pbt_json_get(cs, "failure_id"), "property-panic");
        else set_str(pbt_json_get(cs, "verdict"), "pass");
    } else if (op == "failure_id_valid") {
        pbt_json* valid = pbt_json_get(cs, "valid");
        valid->b = !valid->b;
    } else if (op == "corpus_reject") {
        set_str(pbt_json_get(cs, "text"), "tfv 1\nrec R1 Kop S706274\n");  // a VALID tfv where a reject text belongs
    }
}

void test_every_case_corrupted(const Doc& doc) {
    std::size_t ncases = cases_of(doc.root)->n;
    int controls = 0;
    for (std::size_t i = 0; i < ncases; ++i) {
        int nv = variants(case_at(doc.root, i));
        for (int v = 0; v < nv; ++v) {
            Doc copy = doc.clone();
            ISO_CHECK(copy.root != nullptr);
            if (copy.root == nullptr) return;
            std::string id = case_id(copy.root, i);
            corrupt(case_at(copy.root, i), v);
            corpus::Report rep = evaluate(copy.root);
            bool named = names(rep, id);
            bool other = names_other(rep, copy.root, id);
            if (rep.problems.empty() || !named || other)
                std::printf("  case %s variant %d: problems=%lu names=%d other=%d\n", id.c_str(), v,
                            static_cast<unsigned long>(rep.problems.size()), named, other);
            ISO_CHECK(!rep.problems.empty());
            ISO_CHECK(named);
            ISO_CHECK(!other);
            ++controls;
        }
    }
    std::printf("  %d per-op corruption controls, each failed and named exactly its own case\n", controls);
    ISO_CHECK(controls >= 25);
}

// ---- structural corruptions ------------------------------------------------------------------------
void test_structural(const Doc& doc) {
    std::size_t n = cases_of(doc.root)->n;
    {  // A dropped case: the case-count gate fires.
        Doc copy = doc.clone();
        cases_of(copy.root)->n = n - 1;
        corpus::Report rep = evaluate(copy.root);
        ISO_CHECK(!rep.problems.empty());
        ISO_CHECK_EQ_UINT(rep.evaluated, 24);
        cases_of(copy.root)->n = n;  // restore so the free sees every item
    }
    {  // A truncated corpus (half the cases).
        Doc copy = doc.clone();
        cases_of(copy.root)->n = n / 2;
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        cases_of(copy.root)->n = n;
    }
    {  // An empty corpus.
        Doc copy = doc.clone();
        cases_of(copy.root)->n = 0;
        corpus::Report rep = evaluate(copy.root);
        ISO_CHECK(!rep.problems.empty());
        ISO_CHECK_EQ_UINT(rep.evaluated, 0);
        cases_of(copy.root)->n = n;
    }
    {  // Dropping each single case in turn is a problem (not just the last).
        for (std::size_t drop = 0; drop < n; ++drop) {
            Doc copy = doc.clone();
            pbt_json tmp = cases_of(copy.root)->items[drop];
            cases_of(copy.root)->items[drop] = cases_of(copy.root)->items[n - 1];
            cases_of(copy.root)->items[n - 1] = tmp;
            cases_of(copy.root)->n = n - 1;  // the dropped case now sits past the end
            ISO_CHECK(!evaluate(copy.root).problems.empty());
            cases_of(copy.root)->n = n;
        }
    }
    {  // An unknown op is a problem naming the case, never a skip.
        Doc copy = doc.clone();
        set_str(pbt_json_get(case_at(copy.root, 3), "op"), "no_such_op");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, 3)));
    }
    {  // A non-string id and a non-string op are problems, not crashes.
        Doc copy = doc.clone();
        pbt_json_get(case_at(copy.root, 0), "id")->type = PBT_J_BOOL;
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        pbt_json_get(case_at(copy.root, 0), "id")->type = PBT_J_STR;
        pbt_json_get(case_at(copy.root, 1), "op")->type = PBT_J_NUM;
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        pbt_json_get(case_at(copy.root, 1), "op")->type = PBT_J_STR;
    }
    {  // A duplicate id.
        Doc copy = doc.clone();
        set_str(pbt_json_get(case_at(copy.root, 1), "id"), case_id(copy.root, 0));
        ISO_CHECK(!evaluate(copy.root).problems.empty());
    }
    {  // An extra field and a missing field (the last key renamed: one missing, one extra).
        Doc copy = doc.clone();
        pbt_json* cs = case_at(copy.root, 5);
        char* saved = cs->keys[cs->n - 1];
        std::size_t saved_len = cs->keylens[cs->n - 1];
        cs->keys[cs->n - 1] = static_cast<char*>(pbt_alloc(6, 1));
        std::memcpy(cs->keys[cs->n - 1], "bogus", 6);
        cs->keylens[cs->n - 1] = 5;
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, 5)));
        pbt_free(cs->keys[cs->n - 1]);
        cs->keys[cs->n - 1] = saved;
        cs->keylens[cs->n - 1] = saved_len;
    }
    {  // A wrong schema version and a document with the wrong shape.
        Doc copy = doc.clone();
        set_str(pbt_json_get(copy.root, "schema_version"), "2");
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        ISO_CHECK(!evaluate(cases_of(doc.root)).problems.empty());  // the bare array is not a document
    }
}

// ---- hostile sizes: refused, not run ---------------------------------------------------------------
std::size_t first_of(const Doc& doc, const char* op) {
    for (std::size_t i = 0; i < cases_of(doc.root)->n; ++i)
        if (op_of(case_at(doc.root, i)) == op) return i;
    return static_cast<std::size_t>(-1);
}

void test_hostile(const Doc& doc) {
    std::size_t i = first_of(doc, "gen_bytes");
    ISO_CHECK(i != static_cast<std::size_t>(-1));
    {
        Doc copy = doc.clone();
        set_int_field(case_at(copy.root, i), "count", "99999999");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    {  // count x max_len above the total budget though each is under the cap
        Doc copy = doc.clone();
        set_int_field(case_at(copy.root, i), "count", "10000");
        set_int_field(case_at(copy.root, i), "max_len", "10000");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    i = first_of(doc, "shrink_int");
    static const char* const bad[] = {"007", "-0", "", "-", "12a", "1e3"};
    for (const char* b : bad) {
        Doc copy = doc.clone();
        set_int_field(case_at(copy.root, i), "initial", b);
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    {
        Doc copy = doc.clone();
        set_int_field(case_at(copy.root, i), "initial", std::string(PBT_BIG_MAX_DIGITS + 7, '9').c_str());
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    i = first_of(doc, "shrink_bytes");
    {
        Doc copy = doc.clone();
        set_str(pbt_json_get(pbt_json_get(case_at(copy.root, i), "initial"), "hex"), "0A");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    i = first_of(doc, "gen_primitive");
    {  // "bool\0x" is not "bool"
        Doc copy = doc.clone();
        pbt_json* type = pbt_json_get(case_at(copy.root, i), "type");
        pbt_free(type->s);
        type->s = static_cast<char*>(pbt_alloc(7, 1));
        std::memcpy(type->s, "bool\0x", 7);
        type->slen = 6;
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    i = first_of(doc, "shrink_list");
    {
        Doc copy = doc.clone();
        set_str(pbt_json_get(case_at(copy.root, i), "property"), "no_such_property");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
}

// Replace a member with a null / an empty object: every op must refuse a field of the wrong type, by case id.
void replace_member(pbt_json* obj, std::size_t member, pbt_jtype type) {
    pbt_json* holder = static_cast<pbt_json*>(pbt_alloc(1, sizeof *holder));
    *holder = obj->items[member];
    pbt_json_free(holder);  // frees the old subtree and the holder
    std::memset(&obj->items[member], 0, sizeof obj->items[member]);
    obj->items[member].type = type;
}

void test_every_field_must_be_the_right_type(const Doc& doc) {
    std::size_t ncases = cases_of(doc.root)->n;
    int controls = 0;
    for (std::size_t i = 0; i < ncases; ++i) {
        std::size_t nfields = case_at(doc.root, i)->n;
        for (std::size_t f = 0; f < nfields; ++f) {
            std::string key = case_at(doc.root, i)->keys[f];
            if (key == "id") continue;
            for (int variant = 0; variant < 2; ++variant) {
                Doc copy = doc.clone();
                std::string id = case_id(copy.root, i);
                replace_member(case_at(copy.root, i), f, variant == 0 ? PBT_J_NULL : PBT_J_OBJ);
                corpus::Report rep = evaluate(copy.root);
                if (!names(rep, id)) std::printf("  case %s field %s variant %d: no problem named the case\n", id.c_str(), key.c_str(), variant);
                ISO_CHECK(names(rep, id));
                ISO_CHECK(!names_other(rep, copy.root, id));
                ++controls;
            }
        }
    }
    std::printf("  %d wrong-type field controls\n", controls);
}

// The corpus names three properties it never uses; expected values come from the independent
// Python reference, as in the C package's own test.
void test_properties_the_corpus_does_not_use() {
    static const char* const text =
        "{\"schema_version\":1,\"cases\":["
        "{\"id\":\"syn/even\",\"op\":\"shrink_int\",\"initial\":{\"kind\":\"int\",\"value\":\"15\"},"
        "\"target\":{\"kind\":\"int\",\"value\":\"0\"},\"min\":{\"kind\":\"int\",\"value\":\"-100\"},"
        "\"max\":{\"kind\":\"int\",\"value\":\"100\"},\"property\":\"int_is_even\","
        "\"expected_minimal\":{\"kind\":\"int\",\"value\":\"15\"},\"expected_steps\":{\"kind\":\"int\",\"value\":\"0\"}},"
        "{\"id\":\"syn/abs\",\"op\":\"shrink_int\",\"initial\":{\"kind\":\"int\",\"value\":\"-77\"},"
        "\"target\":{\"kind\":\"int\",\"value\":\"0\"},\"min\":{\"kind\":\"int\",\"value\":\"-100\"},"
        "\"max\":{\"kind\":\"int\",\"value\":\"100\"},\"property\":\"int_abs_le_10\","
        "\"expected_minimal\":{\"kind\":\"int\",\"value\":\"-11\"},\"expected_steps\":{\"kind\":\"int\",\"value\":\"10\"}},"
        "{\"id\":\"syn/zero\",\"op\":\"shrink_bytes\",\"initial\":{\"kind\":\"bytes\",\"hex\":\"0500070009\"},"
        "\"property\":\"bytes_no_zero_byte\",\"expected_minimal\":{\"kind\":\"bytes\",\"hex\":\"00\"},"
        "\"expected_steps\":{\"kind\":\"int\",\"value\":\"2\"}},"
        "{\"id\":\"syn/dup\",\"op\":\"shrink_list\",\"initial\":[{\"kind\":\"int\",\"value\":\"1\"},{\"kind\":\"int\",\"value\":\"2\"},"
        "{\"kind\":\"int\",\"value\":\"3\"},{\"kind\":\"int\",\"value\":\"3\"}],\"elem_target\":{\"kind\":\"int\",\"value\":\"0\"},"
        "\"property\":\"list_no_consecutive_duplicates\",\"expected_minimal\":[{\"kind\":\"int\",\"value\":\"3\"},{\"kind\":\"int\",\"value\":\"3\"}],"
        "\"expected_steps\":{\"kind\":\"int\",\"value\":\"1\"}}"
        "]}";
    Doc doc = parse(text);
    ISO_CHECK(doc.root != nullptr);
    if (doc.root == nullptr) return;
    corpus::Report rep = evaluate(doc.root);
    ISO_CHECK_EQ_UINT(rep.evaluated, 4);
    ISO_CHECK(!names(rep, "syn/even"));
    ISO_CHECK(!names(rep, "syn/abs"));
    ISO_CHECK(!names(rep, "syn/zero"));
    ISO_CHECK(!names(rep, "syn/dup"));
    ISO_CHECK(!rep.problems.empty());  // only the document-level gates remain
    set_int_field(case_at(doc.root, 0), "expected_steps", "3");
    ISO_CHECK(names(evaluate(doc.root), "syn/even"));  // and a wrong expectation IS caught
}

// ---- sabotage of the C++ layer ---------------------------------------------------------------------
// Each seam makes the BINDING wrong.  The corpus run must notice every one, and the
// problems must name real cases; a reset must make the run green again.
void test_cpp_layer_sabotage(const Doc& doc) {
    auto& seams = pbt::detail::seams;

    seams.reset();
    ISO_CHECK(evaluate(doc.root).problems.empty());

    seams.seed_offset = 1;  // every seed the binding hands to the engine is shifted
    {
        corpus::Report rep = evaluate(doc.root);
        ISO_CHECK(!rep.problems.empty());
        ISO_CHECK(names(rep, "pbt/gen_primitive/u64_seed0"));
        ISO_CHECK(names(rep, "pbt/replay/seed42_le_100"));
        std::printf("  seed_offset=1: %lu problems\n", static_cast<unsigned long>(rep.problems.size()));
    }
    seams.reset();

    seams.shrink_step_delta = -1;  // the binding drops one step from every shrink it reports
    {
        corpus::Report rep = evaluate(doc.root);
        ISO_CHECK(!rep.problems.empty());
        ISO_CHECK(names(rep, "pbt/shrink_int/54321_le_100"));
        ISO_CHECK(names(rep, "pbt/shrink_list/sum_le_20"));
        ISO_CHECK(names(rep, "pbt/shrink_bytes/len_lt_5"));
        std::printf("  shrink_step_delta=-1: %lu problems\n", static_cast<unsigned long>(rep.problems.size()));
    }
    seams.shrink_step_delta = 1;
    ISO_CHECK(!evaluate(doc.root).problems.empty());
    seams.reset();

    seams.swap_bounds = true;  // gen::integer swaps min and max
    {
        corpus::Report rep = evaluate(doc.root);
        ISO_CHECK(!rep.problems.empty());
        ISO_CHECK(names(rep, "pbt/gen_bounded/small_seed7"));
        ISO_CHECK(names(rep, "pbt/gen_bounded/large_seed99"));
        std::printf("  swap_bounds: %lu problems\n", static_cast<unsigned long>(rep.problems.size()));
    }
    seams.reset();

    ISO_CHECK(evaluate(doc.root).problems.empty());  // production behaviour is back
}

}  // namespace

int main() {
    std::string text = corpus_text();
    ISO_CHECK_MSG(!text.empty(), "the committed pbt-v1 corpus must be readable");
    Doc doc = parse(text);
    ISO_CHECK_MSG(doc.root != nullptr, "the committed pbt-v1 corpus must parse");
    if (doc.root == nullptr) return ISO_TEST_RESULT();

    std::printf("== positive replay\n");
    test_positive(doc.root);
    std::printf("== per-op corruptions\n");
    test_every_case_corrupted(doc);
    std::printf("== structural corruptions\n");
    test_structural(doc);
    std::printf("== hostile sizes\n");
    test_hostile(doc);
    std::printf("== wrong-type fields\n");
    test_every_field_must_be_the_right_type(doc);
    std::printf("== properties the corpus does not use\n");
    test_properties_the_corpus_does_not_use();
    std::printf("== C++-layer sabotage seams\n");
    test_cpp_layer_sabotage(doc);
    return ISO_TEST_RESULT();
}
