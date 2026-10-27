// Keep the replacement allocation functions in their own translation unit.
// Their contract is the standard matching malloc/free implementation; putting
// arbitrary vector clients beside them can confuse GCC's inlining-based
// mismatched-new-delete diagnostic. No warning flags are disabled.
#include <cstddef>
#include <cstdlib>
#include <new>
static std::size_t request = 0, fail_at = 0;
void allocation_fault(std::size_t position) { request = 0; fail_at = position; }
void* operator new(std::size_t n) {
    if (fail_at != 0 && ++request == fail_at) throw std::bad_alloc();
    if (void* p = std::malloc(n == 0 ? 1 : n)) return p;
    throw std::bad_alloc();
}
void operator delete(void* p) noexcept { std::free(p); }
void operator delete(void* p, std::size_t) noexcept { std::free(p); }
void* operator new[](std::size_t n) { return ::operator new(n); }
void operator delete[](void* p) noexcept { ::operator delete(p); }
void operator delete[](void* p, std::size_t) noexcept { ::operator delete(p); }
