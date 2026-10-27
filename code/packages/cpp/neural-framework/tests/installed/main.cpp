// No repository-relative includes: this consumer sees only the install prefix.
#include <neural_framework.hpp>
int main() {
    namespace nf = ca::neural_framework;
    auto n = nf::Network::seeded({1, 1}, {nf::Activation::linear}, 7);
    const nf::Matrix x({{1.0}}), y({{0.5}});
    const double before = n.loss(x, y);
    n.train_step(x, y, .01);
    auto restored = nf::Network::from_json(n.to_json());
    return restored.step() == 1 && restored.loss(x, y) < before &&
        restored.predict(x).equals(n.predict(x)) ? 0 : 1;
}
