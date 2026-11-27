#include "heap.hpp"

struct ThrowingDestructorOrder {
    ~ThrowingDestructorOrder() noexcept(false) {}
    bool operator()(const int &left, const int &right) const noexcept {
        return left < right;
    }
};

int main() {
    ca::bounded_priority_queue<int, ThrowingDestructorOrder> rejected(
        4, ThrowingDestructorOrder{});
    return static_cast<int>(rejected.size());
}
