// fuzz_support.hpp -- fixture loading and JSON helpers shared by the fuzz test programs.
// ============================================================================
// Header-only and `inline` so a program that does not use a helper never trips
// -Wunused-function under -Werror.  Only the strict JSON READER and the allocator
// of the C package are used here; nothing of the fuzz engine.
#ifndef CA_FUZZ_TESTS_SUPPORT_HPP
#define CA_FUZZ_TESTS_SUPPORT_HPP

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

extern "C" {
#include "pbt_internal.h"
#include "pbt_json.h"
}

namespace fuzzsup {

inline std::string read_file(const std::string& path, bool* ok) {
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

// A repo-relative path (e.g. "code/specs/fixtures/fuzz-v1/cases.json"), found by walking up from the cwd.
inline std::string repo_file(const std::string& relpath) {
    static const char* const prefixes[] = {"", "../", "../../", "../../../", "../../../../", "../../../../../",
                                           "../../../../../../", "../../../../../../../"};
    for (const char* p : prefixes) {
        bool ok = false;
        std::string text = read_file(std::string(p) + relpath, &ok);
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

inline Doc parse(const std::string& text) {
    pbt_json* root = nullptr;
    char err[200];
    if (pbt_json_parse(text.data(), text.size(), &root, err, sizeof err) != 0) return Doc();
    return Doc(root);
}

inline void set_str(pbt_json* node, const char* text) {
    std::size_t n = std::strlen(text);
    pbt_free(node->s);
    node->s = static_cast<char*>(pbt_alloc(n + 1, 1));
    std::memcpy(node->s, text, n + 1);
    node->slen = n;
}
inline pbt_json* cases_of(pbt_json* doc) { return pbt_json_get(doc, "cases"); }
inline pbt_json* case_at(pbt_json* doc, std::size_t i) { return &cases_of(doc)->items[i]; }
inline const char* case_id(pbt_json* doc, std::size_t i) { return pbt_json_get(case_at(doc, i), "id")->s; }
inline std::string op_of(pbt_json* cs) { return pbt_json_get(cs, "op")->s; }

inline bool hexval_ok(char c) { return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'); }

}  // namespace fuzzsup

#endif  // CA_FUZZ_TESTS_SUPPORT_HPP
