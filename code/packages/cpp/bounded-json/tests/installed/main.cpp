#include <bounded_json.hpp>
int main() {
    auto parsed = ca::bounded_json::parse("{\"native\":true,\"step\":3000}");
    if (!parsed || !parsed.document->root().member(0).second->boolean()) return 1;
    return parsed.document->root().member(1).second->integer() == 3000 ? 0 : 2;
}
