// Tests for the C++ trie, using the header-only iso_test.h harness. Covers
// insert/search/contains, erase with pruning, prefix queries, sorted
// enumeration, and longest-prefix match.
#include "iso_test.h"

#include <string>
#include <utility>
#include <vector>

#include "trie.hpp"

int main() {
    ca::trie<int> t;
    ISO_CHECK(t.empty());
    ISO_CHECK(!t.contains_key("cat"));

    t.insert("cat", 1);
    t.insert("car", 2);
    t.insert("card", 3);
    t.insert("dog", 4);
    ISO_CHECK_EQ_UINT(t.size(), 4);
    ISO_CHECK(t.search("car").has_value());
    ISO_CHECK_EQ_INT(t.search("car").value(), 2);
    ISO_CHECK(t.contains_key("card"));
    ISO_CHECK(!t.contains_key("ca"));
    ISO_CHECK(!t.search("ca").has_value());

    // Overwrite keeps size.
    t.insert("cat", 9);
    ISO_CHECK_EQ_UINT(t.size(), 4);
    ISO_CHECK_EQ_INT(t.search("cat").value(), 9);

    // starts_with.
    ISO_CHECK(t.starts_with("ca"));
    ISO_CHECK(!t.starts_with("z"));
    ISO_CHECK(t.starts_with(""));

    // Sorted enumeration.
    std::vector<std::pair<std::string, int>> expected = {
        {"car", 2}, {"card", 3}, {"cat", 9}, {"dog", 4}};
    ISO_CHECK(t.all_words() == expected);

    // Prefix enumeration.
    std::vector<std::pair<std::string, int>> car = {{"car", 2}, {"card", 3}};
    ISO_CHECK(t.words_with_prefix("car") == car);

    // keys().
    std::vector<std::string> keys = {"car", "card", "cat", "dog"};
    ISO_CHECK(t.keys() == keys);

    // longest_prefix_match.
    auto m = t.longest_prefix_match("cards");
    ISO_CHECK(m.has_value());
    ISO_CHECK_STR_EQ(m->first.c_str(), "card");
    ISO_CHECK_EQ_INT(m->second, 3);
    ISO_CHECK(!t.longest_prefix_match("ca").has_value());

    // erase with pruning.
    ISO_CHECK(t.erase("card"));
    ISO_CHECK_EQ_UINT(t.size(), 3);
    ISO_CHECK(!t.contains_key("card"));
    ISO_CHECK(t.contains_key("car"));
    ISO_CHECK(!t.erase("card"));
    std::vector<std::pair<std::string, int>> after = {
        {"car", 2}, {"cat", 9}, {"dog", 4}};
    ISO_CHECK(t.all_words() == after);

    return ISO_TEST_RESULT();
}
