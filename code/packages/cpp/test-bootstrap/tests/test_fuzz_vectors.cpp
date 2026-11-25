// test_fuzz_vectors.cpp -- the 820 differential vectors of the Go lane, replayed THROUGH THE C++
// API (VER06k section 6).
//
// code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go is a generated table: seeded
// random inputs run through the independent Python oracle (scripts/fuzz_conformance.py).  The
// fuzz-v1 corpus cannot pin these (its splice has equal splits, its mutate_sequence does not pin
// every constant), so every row is checked through BOTH the int64 and the BigInt overloads of
// ca::fuzz, read straight from the committed path (there is no copy).  A row looks like
//
//     {"splice", []string{"efdd...", "0162...", "8", "39"}, "efdd..."},
//
// Every problem names its row as "vector N (op)".  The run is then proven able to FAIL:
// corrupted expected bytes, a short table, a missing table and a sabotaged C++ layer.
#define CA_FUZZ_TEST_SEAMS 1
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

#include "fuzz_bootstrap.hpp"
#include "fuzz_support.hpp"
#include "iso_test.h"

namespace {

namespace fz = ca::fuzz;

struct Row {
    std::string op;
    std::vector<std::string> args;
    std::string out;
};

// Copy the quoted string starting at s[pos] (the opening quote); advance past the closing quote.
bool take_quoted(const std::string& s, std::size_t& pos, std::string& dst) {
    if (pos >= s.size() || s[pos] != '"') return false;
    std::size_t e = s.find('"', pos + 1);
    if (e == std::string::npos) return false;
    dst = s.substr(pos + 1, e - pos - 1);
    pos = e + 1;
    return true;
}

bool parse_row(const std::string& line, Row& r) {
    std::size_t p = 0;
    if (line.compare(0, 2, "\t{") != 0) return false;
    p = 2;
    if (!take_quoted(line, p, r.op) || line.compare(p, 11, ", []string{") != 0) return false;
    p += 11;
    while (p < line.size() && line[p] == '"') {
        std::string a;
        if (r.args.size() >= 6 || !take_quoted(line, p, a)) return false;
        r.args.push_back(a);
        if (p < line.size() && line[p] == ',') p += 2;
    }
    if (line.compare(p, 3, "}, ") != 0) return false;
    p += 3;
    if (!take_quoted(line, p, r.out)) return false;
    return line.substr(p) == "},";
}

fz::Bytes unhex(const std::string& h) {
    fz::Bytes b(h.size() / 2);
    for (std::size_t i = 0; i < b.size(); ++i) {
        auto v = [](char c) { return c >= '0' && c <= '9' ? c - '0' : c - 'a' + 10; };
        b[i] = static_cast<std::uint8_t>(v(h[2 * i]) * 16 + v(h[2 * i + 1]));
    }
    return b;
}

fz::Endian endian_of(const std::string& s) { return s == "big" ? fz::Endian::Big : fz::Endian::Little; }

// Check one row through both paths; every problem starts with "vector N (op)".
void check_row(std::size_t index, const Row& r, std::vector<std::string>& problems) {
    std::string name = "vector " + std::to_string(index) + " (" + r.op + ")";
    fz::Bytes want = unhex(r.out);
    auto report = [&](const char* path, const fz::Bytes& got) {
        if (got != want) problems.push_back(name + ": differs on the " + path + " path");
    };
    try {
        fz::Bytes in;
        if (r.op != "splice") in = unhex(r.args[0]);
        auto big = [&](std::size_t k) { return fz::BigInt::parse(r.args[k]); };
        auto fits = [&](std::size_t k) { return big(k).fits_i64(); };
        auto i64 = [&](std::size_t k) { return big(k).to_i64(); };
        if (r.op == "bit_flip" || r.op == "byte_flip") {
            bool bit = r.op == "bit_flip";
            report("BigInt", bit ? fz::bit_flip(in, big(1), big(2)) : fz::byte_flip(in, big(1), big(2)));
            if (fits(1) && fits(2))
                report("int64", bit ? fz::bit_flip(in, i64(1), i64(2)) : fz::byte_flip(in, i64(1), i64(2)));
        } else if (r.op == "arithmetic") {
            int w = std::atoi(r.args[3].c_str());
            fz::Endian e = endian_of(r.args[4]);
            report("BigInt", fz::byte_arithmetic(in, big(1), big(2), w, e));
            if (fits(1) && fits(2)) report("int64", fz::byte_arithmetic(in, i64(1), i64(2), w, e));
        } else if (r.op == "boundary") {
            int w = std::atoi(r.args[2].c_str());
            fz::Endian e = endian_of(r.args[4]);
            fz::BigInt v = big(3);
            report("BigInt", fz::boundary_insert(in, big(1), w, v, e));
            if (fits(1) && v.fits_u64()) report("int64", fz::boundary_insert(in, i64(1), w, v.low_u64(), e));
        } else if (r.op == "block_delete") {
            report("BigInt", fz::block_delete(in, big(1), big(2)));
            if (fits(1) && fits(2)) report("int64", fz::block_delete(in, i64(1), i64(2)));
        } else if (r.op == "block_insert") {
            fz::Bytes ins = unhex(r.args[2]);
            report("BigInt", fz::block_insert(in, big(1), ins));
            if (fits(1)) report("int64", fz::block_insert(in, i64(1), ins));
        } else if (r.op == "block_duplicate") {
            report("BigInt", fz::block_duplicate(in, big(1), big(2), big(3)));
            if (fits(1) && fits(2) && fits(3)) report("int64", fz::block_duplicate(in, i64(1), i64(2), i64(3)));
        } else if (r.op == "splice") {
            fz::Bytes t = unhex(r.args[0]);
            fz::Bytes d = unhex(r.args[1]);
            report("BigInt", fz::splice(t, d, big(2), big(3)));
            if (fits(2) && fits(3)) report("int64", fz::splice(t, d, i64(2), i64(3)));
        } else if (r.op == "sequence") {
            report("BigInt", fz::mutate_sequence(in, big(1).mod_u64(), static_cast<std::size_t>(std::atoi(r.args[2].c_str())),
                                                 static_cast<std::size_t>(std::atol(r.args[3].c_str()))));
        } else {
            problems.push_back(name + ": unknown vector op");
        }
    } catch (const fz::Error& e) {
        problems.push_back(name + ": the engine refused the row (status " + std::to_string(e.status()) + ")");
    }
}

struct Result {
    std::size_t count = 0;
    std::size_t per_op[9] = {0};
    std::vector<std::string> problems;
};

const char* const kOps[9] = {"bit_flip", "byte_flip", "arithmetic", "boundary", "block_delete",
                             "block_insert", "block_duplicate", "splice", "sequence"};

// Evaluate a whole Go table text.
Result replay(const std::string& text) {
    Result res;
    bool in_table = false;
    std::size_t pos = 0;
    while (pos < text.size()) {
        std::size_t nl = text.find('\n', pos);
        std::string line = text.substr(pos, nl == std::string::npos ? std::string::npos : nl - pos);
        pos = nl == std::string::npos ? text.size() : nl + 1;
        if (!in_table) {
            in_table = line == "var referenceVectors = []refVector{";
            continue;
        }
        if (line == "}") break;
        Row r;
        if (!parse_row(line, r)) {
            res.problems.push_back("vector " + std::to_string(res.count + 1) + ": unparseable row");
            continue;
        }
        for (int k = 0; k < 9; ++k)
            if (r.op == kOps[k]) ++res.per_op[k];
        check_row(res.count, r, res.problems);
        ++res.count;
    }
    if (!in_table) res.problems.push_back("vector table: the table header was not found");
    if (res.count != 820) res.problems.push_back("vector table: expected 820 rows, got " + std::to_string(res.count));
    for (int k = 0; k < 9; ++k)
        if (res.per_op[k] < 50) res.problems.push_back(std::string("vector table: too few rows for ") + kOps[k]);
    return res;
}

bool names_row(const Result& r, std::size_t index) {
    std::string prefix = "vector " + std::to_string(index) + " (";
    for (const std::string& p : r.problems)
        if (p.compare(0, prefix.size(), prefix) == 0) return true;
    return false;
}

// Flip the expected output of the n-th data row in the text.
std::string corrupt_row(const std::string& text, std::size_t n) {
    std::string out = text;
    std::size_t pos = out.find("var referenceVectors = []refVector{\n");
    pos = out.find('\n', pos) + 1;
    for (std::size_t i = 0; i < n; ++i) pos = out.find('\n', pos) + 1;
    std::size_t eol = out.find('\n', pos);
    // the last quoted string ends at eol - 3 ("\"},"): its first char is after the quote before it
    std::size_t close = eol - 3;
    std::size_t open = out.rfind('"', close - 1);
    if (open + 1 == close) out.insert(close, "00");  // an empty expectation becomes non-empty
    else out[open + 1] = out[open + 1] == '0' ? '1' : '0';
    return out;
}

}  // namespace

int main() {
    std::string text = fuzzsup::repo_file("code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go");
    ISO_CHECK_MSG(!text.empty(), "the Go reference vector table must be found");
    if (text.empty()) return ISO_TEST_RESULT();

    std::printf("== positive replay\n");
    Result base = replay(text);
    for (const std::string& p : base.problems) std::printf("  problem: %s\n", p.c_str());
    ISO_CHECK_EQ_UINT(base.problems.size(), 0);
    ISO_CHECK_EQ_UINT(base.count, 820);
    for (int k = 0; k < 9; ++k) {
        std::printf("  %-16s %lu rows\n", kOps[k], static_cast<unsigned long>(base.per_op[k]));
        ISO_CHECK(base.per_op[k] >= 50);
    }
    std::printf("  %lu/820 vectors replayed through the C++ API (int64 and BigInt), %lu problems\n",
                static_cast<unsigned long>(base.count), static_cast<unsigned long>(base.problems.size()));

    std::printf("== per-row corruption: every row must fail and name itself\n");
    {
        std::size_t controls = 0;
        std::size_t pos = 0;
        bool in_table = false;
        std::size_t index = 0;
        while (pos < text.size()) {
            std::size_t nl = text.find('\n', pos);
            std::string line = text.substr(pos, nl - pos);
            pos = nl + 1;
            if (!in_table) {
                in_table = line == "var referenceVectors = []refVector{";
                continue;
            }
            if (line == "}") break;
            Row r;
            ISO_CHECK(parse_row(line, r));
            fz::Bytes was = unhex(r.out);
            r.out = r.out.empty() ? "00" : std::string(1, r.out[0] == '0' ? '1' : '0') + r.out.substr(1);
            std::vector<std::string> problems;
            check_row(index, r, problems);
            std::string prefix = "vector " + std::to_string(index) + " (" + r.op + "):";
            ISO_CHECK(!problems.empty());
            if (problems.empty() || problems[0].compare(0, prefix.size(), prefix) != 0)
                std::printf("  row %lu did not name itself\n", static_cast<unsigned long>(index));
            ISO_CHECK(!problems.empty() && problems[0].compare(0, prefix.size(), prefix) == 0);
            ++controls;
            ++index;
        }
        std::printf("  %lu row controls\n", static_cast<unsigned long>(controls));
        ISO_CHECK_EQ_UINT(controls, 820);
    }

    std::printf("== table-level corruption\n");
    for (int k = 0; k < 9; ++k) {  // the first row of every op, corrupted in the TEXT
        std::size_t first = 0;
        {
            Result tmp;
            std::size_t idx = 0;
            std::size_t pos = text.find("var referenceVectors = []refVector{\n");
            pos = text.find('\n', pos) + 1;
            while (true) {
                std::size_t nl = text.find('\n', pos);
                Row r;
                if (!parse_row(text.substr(pos, nl - pos), r)) break;
                if (r.op == kOps[k]) {
                    first = idx;
                    break;
                }
                pos = nl + 1;
                ++idx;
            }
        }
        Result r = replay(corrupt_row(text, first));
        ISO_CHECK(!r.problems.empty());
        ISO_CHECK(names_row(r, first));
    }
    {  // a dropped row: the count gate fires
        std::size_t pos = text.find("var referenceVectors = []refVector{\n");
        pos = text.find('\n', pos) + 1;
        std::size_t eol = text.find('\n', pos);
        std::string shorter = text.substr(0, pos) + text.substr(eol + 1);
        Result r = replay(shorter);
        ISO_CHECK(!r.problems.empty());
        ISO_CHECK_EQ_UINT(r.count, 819);
    }
    {  // a missing table, a truncated table and an unparseable row
        ISO_CHECK(!replay("").problems.empty());
        ISO_CHECK(!replay("var referenceVectors = []refVector{\n}\n").problems.empty());
        std::string garbled = text;
        std::size_t pos = garbled.find("\t{\"bit_flip\"");
        garbled.replace(pos, 12, "\t{\"bit_flip");
        ISO_CHECK(!replay(garbled).problems.empty());
    }

    std::printf("== a sabotaged C++ layer must fail the vectors\n");
    {
        auto& seams = fz::detail::seams;
        seams.offset_delta = 1;
        ISO_CHECK(!replay(text).problems.empty());
        seams.reset();
        seams.swap_endian = true;
        {
            Result r = replay(text);
            ISO_CHECK(!r.problems.empty());
        }
        seams.reset();
        seams.seed_offset = 3;
        ISO_CHECK(!replay(text).problems.empty());
        seams.reset();
        ISO_CHECK(replay(text).problems.empty());
    }
    return ISO_TEST_RESULT();
}
