// test_fuzz_corpus.cpp -- replay the committed fuzz-v1 corpus THROUGH THE C++ BINDING and prove
// the run CAN FAIL (VER06k sections 6 and 7).
//
// A conformance run that cannot fail proves nothing.  After the positive run (25/25 through
// ca::fuzz, never through fz_corpus_evaluate) this file:
//
//   - corrupts EVERY case, per op, in each way the op defines, and asserts the run fails, a
//     problem NAMES that case, and no OTHER case is named;
//   - makes every field of every case the wrong type;
//   - drops and truncates the case list, adds unknown ops, extra and missing fields, duplicate
//     ids and a wrong schema version;
//   - SABOTAGES THE C++ LAYER ITSELF through its seams and requires the run to notice.
// Only this translation unit compiles the seams in (CA_FUZZ_TEST_SEAMS); every other test
// exercises the production configuration with no seam global.
#define CA_FUZZ_TEST_SEAMS 1
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "fuzz_corpus_runner.hpp"
#include "fuzz_support.hpp"
#include "iso_test.h"

namespace {

namespace fz = ca::fuzz;
using fuzzsup::case_at;
using fuzzsup::case_id;
using fuzzsup::cases_of;
using fuzzsup::Doc;
using fuzzsup::op_of;
using fuzzsup::set_str;

fuzzcorpus::Report evaluate(pbt_json* doc) {
    fuzzcorpus::Runner r;
    return r.evaluate(doc);
}

bool names(const fuzzcorpus::Report& r, const std::string& id) {
    for (const std::string& p : r.problems)
        if (p.compare(0, id.size() + 1, id + ":") == 0) return true;
    return false;
}
bool names_other(const fuzzcorpus::Report& r, pbt_json* doc, const std::string& id) {
    for (std::size_t k = 0; k < cases_of(doc)->n; ++k) {
        std::string other = case_id(doc, k);
        if (other != id && names(r, other)) return true;
    }
    return false;
}

void test_positive(pbt_json* doc) {
    fuzzcorpus::Report rep = evaluate(doc);
    for (const std::string& p : rep.problems) std::printf("  problem: %s\n", p.c_str());
    ISO_CHECK_EQ_UINT(rep.problems.size(), 0);
    ISO_CHECK_EQ_UINT(rep.evaluated, 25);
    ISO_CHECK_EQ_UINT(cases_of(doc)->n, 25);
    std::printf("  fuzz-v1: %lu/25 cases evaluated through the C++ API, %lu problems\n",
                static_cast<unsigned long>(rep.evaluated), static_cast<unsigned long>(rep.problems.size()));
}

// ---- per-op corruptions ------------------------------------------------------------------------
void other_hex(pbt_json* bytes_obj, bool extend) {
    pbt_json* hex = pbt_json_get(bytes_obj, "hex");
    std::string h(hex->s, hex->slen);
    if (extend) h += "00";
    else if (h.empty()) h = "00";
    else h[0] = (h[0] == '0') ? '1' : '0';
    set_str(hex, h.c_str());
}

void corrupt(pbt_json* cs, int v) {
    std::string op = op_of(cs);
    if (op.compare(0, 7, "mutate_") == 0) {
        other_hex(pbt_json_get(cs, "output"), v == 1);
    } else if (op == "corpus_admit") {
        if (v == 0) {
            pbt_json* a = pbt_json_get(cs, "admitted");
            a->b = !a->b;
        } else {
            pbt_json* n = pbt_json_get(pbt_json_get(cs, "new_edges"), "value");
            set_str(n, std::strcmp(n->s, "7") == 0 ? "8" : "7");
        }
    } else if (op == "crash_dedup") {
        pbt_json* d = pbt_json_get(cs, "is_duplicate");
        d->b = !d->b;
    } else if (op == "failure_id_valid") {
        pbt_json* d = pbt_json_get(cs, "valid");
        d->b = !d->b;
    } else if (op == "private_reporting") {
        if (v == 0) set_str(pbt_json_get(cs, "tap_line"), "not ok 1 - other [fuzz-crash:3a8f19b2]");
        else {
            pbt_json* d = pbt_json_get(cs, "contains_raw_payload");
            d->b = !d->b;
        }
    } else if (op == "corpus_reject") {
        set_str(pbt_json_get(cs, "text"), "tfv 1\nrec R1 Kop S706274\n");  // a VALID tfv where a reject text belongs
    }
}
int variants(pbt_json* cs) {
    std::string op = op_of(cs);
    if (op.compare(0, 7, "mutate_") == 0 || op == "corpus_admit" || op == "private_reporting") return 2;
    return 1;
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
            fuzzcorpus::Report rep = evaluate(copy.root);
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

void test_structural(const Doc& doc) {
    std::size_t n = cases_of(doc.root)->n;
    {  // a dropped case
        Doc copy = doc.clone();
        cases_of(copy.root)->n = n - 1;
        fuzzcorpus::Report rep = evaluate(copy.root);
        ISO_CHECK(!rep.problems.empty());
        ISO_CHECK_EQ_UINT(rep.evaluated, 24);
        cases_of(copy.root)->n = n;
    }
    {  // a truncated and an empty corpus
        Doc copy = doc.clone();
        cases_of(copy.root)->n = n / 2;
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        cases_of(copy.root)->n = 0;
        fuzzcorpus::Report rep = evaluate(copy.root);
        ISO_CHECK(!rep.problems.empty());
        ISO_CHECK_EQ_UINT(rep.evaluated, 0);
        cases_of(copy.root)->n = n;
    }
    for (std::size_t drop = 0; drop < n; ++drop) {  // dropping each single case in turn
        Doc copy = doc.clone();
        pbt_json tmp = cases_of(copy.root)->items[drop];
        cases_of(copy.root)->items[drop] = cases_of(copy.root)->items[n - 1];
        cases_of(copy.root)->items[n - 1] = tmp;
        cases_of(copy.root)->n = n - 1;
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        cases_of(copy.root)->n = n;
    }
    {  // an unknown op names the case
        Doc copy = doc.clone();
        set_str(pbt_json_get(case_at(copy.root, 3), "op"), "no_such_op");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, 3)));
    }
    {  // a non-string id and a non-string op
        Doc copy = doc.clone();
        pbt_json_get(case_at(copy.root, 0), "id")->type = PBT_J_BOOL;
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        pbt_json_get(case_at(copy.root, 0), "id")->type = PBT_J_STR;
        pbt_json_get(case_at(copy.root, 1), "op")->type = PBT_J_NUM;
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        pbt_json_get(case_at(copy.root, 1), "op")->type = PBT_J_STR;
    }
    {  // a duplicate id
        Doc copy = doc.clone();
        set_str(pbt_json_get(case_at(copy.root, 1), "id"), case_id(copy.root, 0));
        ISO_CHECK(!evaluate(copy.root).problems.empty());
    }
    {  // an extra field plus a missing one (the last key renamed)
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
    {  // a wrong schema version; a bare array is not a document
        Doc copy = doc.clone();
        set_str(pbt_json_get(copy.root, "schema_version"), "2");
        ISO_CHECK(!evaluate(copy.root).problems.empty());
        ISO_CHECK(!evaluate(cases_of(doc.root)).problems.empty());
    }
}

std::size_t first_of(const Doc& doc, const char* op) {
    for (std::size_t i = 0; i < cases_of(doc.root)->n; ++i)
        if (op_of(case_at(doc.root, i)) == op) return i;
    return static_cast<std::size_t>(-1);
}
void set_int_field(pbt_json* cs, const char* field, const char* dec) {
    set_str(pbt_json_get(pbt_json_get(cs, field), "value"), dec);
}

void test_hostile(const Doc& doc) {
    std::size_t i = first_of(doc, "mutate_bit_flip");
    static const char* const bad[] = {"007", "-0", "", "-", "12a", "1e3"};
    for (const char* b : bad) {
        Doc copy = doc.clone();
        set_int_field(case_at(copy.root, i), "bit_offset", b);
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    {  // wider than the bignum
        Doc copy = doc.clone();
        set_int_field(case_at(copy.root, i), "count", std::string(PBT_BIG_MAX_DIGITS + 7, '9').c_str());
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    {  // uppercase hex is not canonical
        Doc copy = doc.clone();
        set_str(pbt_json_get(pbt_json_get(case_at(copy.root, i), "input"), "hex"), "0A");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    i = first_of(doc, "mutate_sequence");
    {  // steps above the cap are refused
        Doc copy = doc.clone();
        set_int_field(case_at(copy.root, i), "steps", "10001");
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    {  // the aggregate work budget: a huge input exhausts it, and the NEXT sequence case fails too
        Doc copy = doc.clone();
        pbt_json* cs = case_at(copy.root, i);
        std::string big(2 * 9000, 'a');
        set_str(pbt_json_get(pbt_json_get(cs, "input"), "hex"), big.c_str());
        set_int_field(cs, "steps", "10000");
        fuzzcorpus::Report rep = evaluate(copy.root);
        ISO_CHECK(names(rep, case_id(copy.root, i)));
    }
    i = first_of(doc, "crash_dedup");
    {  // an endian-like bad string is not a frame list
        Doc copy = doc.clone();
        pbt_json_get(case_at(copy.root, i), "crash_a_frames")->type = PBT_J_STR;
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
    i = first_of(doc, "mutate_arithmetic");
    {  // "little\0x" is not "little"
        Doc copy = doc.clone();
        pbt_json* e = pbt_json_get(case_at(copy.root, i), "endian");
        pbt_free(e->s);
        e->s = static_cast<char*>(pbt_alloc(9, 1));
        std::memcpy(e->s, "little\0x", 9);
        e->slen = 8;
        ISO_CHECK(names(evaluate(copy.root), case_id(copy.root, i)));
    }
}

void replace_member(pbt_json* obj, std::size_t member, pbt_jtype type) {
    pbt_json* holder = static_cast<pbt_json*>(pbt_alloc(1, sizeof *holder));
    *holder = obj->items[member];
    pbt_json_free(holder);
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
                fuzzcorpus::Report rep = evaluate(copy.root);
                if (!names(rep, id))
                    std::printf("  case %s field %s variant %d: no problem named the case\n", id.c_str(), key.c_str(),
                                variant);
                ISO_CHECK(names(rep, id));
                ISO_CHECK(!names_other(rep, copy.root, id));
                ++controls;
            }
        }
    }
    std::printf("  %d wrong-type field controls\n", controls);
}

// ---- sabotage of the C++ layer -----------------------------------------------------------------
void test_cpp_layer_sabotage(const Doc& doc) {
    auto& seams = fz::detail::seams;
    seams.reset();
    ISO_CHECK(evaluate(doc.root).problems.empty());

    seams.offset_delta = 1;
    {
        fuzzcorpus::Report rep = evaluate(doc.root);
        ISO_CHECK(!rep.problems.empty());
        std::printf("  offset_delta=1: %lu problems\n", static_cast<unsigned long>(rep.problems.size()));
    }
    seams.reset();
    ISO_CHECK(evaluate(doc.root).problems.empty());

    seams.swap_endian = true;
    {
        fuzzcorpus::Report rep = evaluate(doc.root);
        ISO_CHECK(names(rep, "fuzz/mutate_arithmetic/u16_le_add35"));
        ISO_CHECK(names(rep, "fuzz/mutate_boundary/u32_signed_min_le"));
        std::printf("  swap_endian: %lu problems\n", static_cast<unsigned long>(rep.problems.size()));
    }
    seams.reset();

    seams.seed_offset = 1;
    {
        fuzzcorpus::Report rep = evaluate(doc.root);
        ISO_CHECK(names(rep, "fuzz/mutate_sequence/seed42_steps4"));
    }
    seams.reset();

    seams.invert_dedup = true;
    {
        fuzzcorpus::Report rep = evaluate(doc.root);
        ISO_CHECK(names(rep, "fuzz/crash_dedup/same_signature"));
        ISO_CHECK(names(rep, "fuzz/crash_dedup/divergent_signature"));
    }
    seams.reset();

    seams.leak_tap_target = true;
    ISO_CHECK(names(evaluate(doc.root), "fuzz/private_reporting/opaque_token"));
    seams.reset();

    seams.admit_everything = true;
    ISO_CHECK(names(evaluate(doc.root), "fuzz/corpus_admit/redundant_edge"));
    seams.reset();

    seams.accept_any_failure_id = true;
    ISO_CHECK(names(evaluate(doc.root), "fuzz/failure_id/unregistered"));
    seams.reset();

    ISO_CHECK(evaluate(doc.root).problems.empty());  // production behaviour is back
}

}  // namespace

int main() {
    std::string text = fuzzsup::repo_file("code/specs/fixtures/fuzz-v1/cases.json");
    ISO_CHECK_MSG(!text.empty(), "the committed fuzz-v1 corpus must be readable");
    Doc doc = fuzzsup::parse(text);
    ISO_CHECK_MSG(doc.root != nullptr, "the committed fuzz-v1 corpus must parse");
    if (doc.root == nullptr) return ISO_TEST_RESULT();

    std::printf("== positive replay\n");
    test_positive(doc.root);
    std::printf("== per-op corruptions\n");
    test_every_case_corrupted(doc);
    std::printf("== structural corruptions\n");
    test_structural(doc);
    std::printf("== hostile fixtures\n");
    test_hostile(doc);
    std::printf("== wrong-type fields\n");
    test_every_field_must_be_the_right_type(doc);
    std::printf("== C++-layer sabotage seams\n");
    test_cpp_layer_sabotage(doc);
    return ISO_TEST_RESULT();
}
