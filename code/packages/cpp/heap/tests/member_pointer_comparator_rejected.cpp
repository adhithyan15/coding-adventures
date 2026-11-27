#include "heap.hpp"

struct MemberComparator {
    bool higher(const int &left, const int &right) const { return left < right; }
};

int main() {
    using Comparator = bool (MemberComparator::*)(const int &, const int &) const;
    ca::bounded_priority_queue<int, Comparator> rejected(4, &MemberComparator::higher);
    return static_cast<int>(rejected.size());
}
