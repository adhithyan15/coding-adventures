#include <heap.hpp>

int main() {
    ca::min_heap<int> legacy;
    legacy.push(7);
    legacy.push(2);
    if (legacy.peek() != 2) {
        return 1;
    }

    ca::bounded_min_heap<int> bounded(2);
    if (bounded.try_push(5) != ca::push_result::inserted ||
        bounded.try_push(-3) != ca::push_result::inserted ||
        bounded.try_push(9) != ca::push_result::full) {
        return 2;
    }
    const auto borrowed = bounded.peek();
    if (!borrowed || borrowed->get() != -3) {
        return 3;
    }
    const auto removed = bounded.pop();
    if (!removed || *removed != -3 || bounded.size() != 1) {
        return 4;
    }
    return 0;
}
