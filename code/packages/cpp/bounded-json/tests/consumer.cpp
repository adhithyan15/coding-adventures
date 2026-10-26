// This translation unit intentionally does NOT include the test harness. It
// proves the public header supplies its own includes and has no ODR collision.
#include "bounded_json.hpp"
bool second_translation_unit() {
    auto result = ca::bounded_json::parse("[42]");
    return result && result.document->root().at(0).integer() == 42;
}
