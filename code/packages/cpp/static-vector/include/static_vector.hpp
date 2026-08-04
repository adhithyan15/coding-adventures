// static_vector.hpp — a fixed-capacity vector, in pure ISO C++17 (header-only).
// ===========================================================================
//
// static_vector<T, N> behaves like a small std::vector whose capacity is fixed
// at N and whose storage lives INSIDE the object — there is no heap allocation
// at all. That makes it useful where allocation is undesirable (hot loops,
// embedded, allocator-free code) while keeping a familiar vector-like API.
//
//   ca::static_vector<int, 3> v;   // capacity 3, size 0, no allocation
//   v.push_back(10);               // size 1
//   v.push_back(20);               // size 2
//   int first = v[0];              // 10
//
// Design notes
// ------------
//   • Storage is a plain C array member `T data_[N]`, so the elements are
//     default-constructed up front. This keeps the type trivially usable for
//     the common case of simple element types (int, small structs) without
//     dragging in aligned-storage / placement-new machinery — which keeps the
//     header comfortably inside portable ISO C++17.
//   • size_ tracks how many leading slots are "live".
//   • push_back returns false instead of throwing when full, so callers in
//     no-exceptions builds can still use it; at() DOES throw std::out_of_range,
//     mirroring std::vector, for callers who want checked access.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef STATIC_VECTOR_HPP
#define STATIC_VECTOR_HPP

#include <cstddef>   // std::size_t
#include <stdexcept> // std::out_of_range

namespace ca {

template <typename T, std::size_t N>
class static_vector {
public:
    using value_type = T;
    using size_type = std::size_t;
    using iterator = T *;
    using const_iterator = const T *;

    // A fresh vector is empty; the N element slots exist but none are "live".
    static_vector() : size_(0) {}

    // Capacity is fixed at N; size is how many elements are currently stored.
    size_type capacity() const { return N; }
    size_type size() const { return size_; }
    bool empty() const { return size_ == 0; }
    bool full() const { return size_ == N; }

    // push_back — append a copy of `value`. Returns false (and stores nothing)
    // when the vector is already full, so it is safe in no-exceptions builds.
    bool push_back(const T &value) {
        if (size_ == N) {
            return false;
        }
        data_[size_] = value;
        ++size_;
        return true;
    }

    // pop_back — drop the last element. No-op on an empty vector.
    void pop_back() {
        if (size_ > 0) {
            --size_;
        }
    }

    void clear() { size_ = 0; }

    // Unchecked element access, like std::vector::operator[].
    T &operator[](size_type i) { return data_[i]; }
    const T &operator[](size_type i) const { return data_[i]; }

    // Checked element access — throws std::out_of_range for i >= size().
    T &at(size_type i) {
        if (i >= size_) {
            throw std::out_of_range("static_vector::at");
        }
        return data_[i];
    }
    const T &at(size_type i) const {
        if (i >= size_) {
            throw std::out_of_range("static_vector::at");
        }
        return data_[i];
    }

    // Iteration covers the live [0, size_) range, so range-for works as expected.
    iterator begin() { return data_; }
    iterator end() { return data_ + size_; }
    const_iterator begin() const { return data_; }
    const_iterator end() const { return data_ + size_; }

private:
    T data_[N];
    size_type size_;
};

} // namespace ca

#endif // STATIC_VECTOR_HPP
