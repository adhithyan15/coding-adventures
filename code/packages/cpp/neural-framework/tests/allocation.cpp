// Deterministic failure of REAL allocation requests, not a model-side fake.
// Only the bounded operation under test enables the fault. Assertions and JSON
// comparisons run with faults disabled so they cannot obscure the failed step.
#include "neural_framework.hpp"
#include <new>
#ifndef NN51_NO_ALLOCATION_FAULTS
void allocation_fault(std::size_t position);

std::size_t allocation_atomicity() {
    namespace nf = ca::neural_framework;
    auto original = nf::Network::seeded({2, 3, 1}, {nf::Activation::tanh, nf::Activation::linear}, 7);
    nf::Matrix x({{.2, .3}, {.4, .5}}), y({{.1}, {.8}});
    const auto before = original.to_json();
    std::size_t tested = 0;
    for (std::size_t f = 1; f < 10000; ++f) {
        auto n = original; bool failed = false;
        allocation_fault(f);
        try { n.train_step(x, y, .01); } catch (const std::bad_alloc&) { failed = true; }
        catch (...) { allocation_fault(0); throw; }
        allocation_fault(0);
        if (!failed) return tested;
        if (n.to_json() != before || n.step() != 0) throw std::runtime_error("allocation failure published a partial model");
        ++tested;
    }
    throw std::runtime_error("allocation sweep did not reach success");
}
#endif

bool second_translation_unit() {
    // Include the public header in another TU and use actual model operations.
    auto n = ca::neural_framework::Network::seeded({1, 1}, {ca::neural_framework::Activation::linear}, 7);
    auto cp = n.checkpoint();
    return ca::neural_framework::Network::from_checkpoint(cp).to_json() == n.to_json();
}
