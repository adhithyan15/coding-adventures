---
category: Testing & coverage
---

# An optional vector assertion must construct the empty vector explicitly

An expected value written as `std::optional<std::vector<T>>({})` can select an
empty optional rather than an engaged optional containing an empty vector. A
test that compares a valid empty invocation array must spell out both layers:
`std::optional<std::vector<T>>(std::vector<T>{})`. Verify the value category of
the expected optional before treating a legitimate empty collection as a
validator failure.
