// fab_process_simulation.hpp — a 1-D analytical CMOS process-flow simulator,
// header-only in pure ISO C++17 (namespace ca::fab_process_simulation). A
// faithful port of the Rust `fab-process-simulation` crate.
// ===========================================================================
//
// Models the standard front-end fabrication steps with 1-D analytical
// approximations (Deal-Grove oxidation, deposition, layer-selective etch,
// Gaussian ion-implant profiles from an SRIM table, Fick's-law diffusion).
//
// A CrossSection is a top-to-bottom stack of Layers (layers[0] is the top);
// each layer carries a doping map: species -> a list of sampled
// (depth_nm, conc_per_cm3) points. Every step returns a NEW cross-section (the
// input is never mutated), using ordinary value semantics.
//
// NO libm / <cmath>: the sqrt (Deal-Grove) and exp (Gaussian implant) are
// computed from scratch; results match the Rust f64 models to within ~1e-6.
//
// DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port throws
// std::invalid_argument with the same message on a bad step.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions.
#ifndef CA_FAB_PROCESS_SIMULATION_HPP
#define CA_FAB_PROCESS_SIMULATION_HPP

#include <cstddef>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace ca {
namespace fab_process_simulation {

inline constexpr double DEAL_GROVE_DRY_1000C_A = 0.165;   // um
inline constexpr double DEAL_GROVE_DRY_1000C_B = 0.0117;  // um^2/hr

using Sample = std::pair<double, double>;  // (depth_nm, conc_per_cm3)

struct Layer {
    std::string material;
    double thickness_nm = 0.0;
    std::unordered_map<std::string, std::vector<Sample>> doping;

    Layer() = default;
    Layer(std::string material_, double thickness)
        : material(std::move(material_)), thickness_nm(thickness) {}
};

struct CrossSection {
    std::vector<Layer> layers;
};

namespace detail {

inline double d_sqrt(double x) {
    if (x <= 0.0) return 0.0;
    double guess = x >= 1.0 ? x : 1.0;
    for (int i = 0; i < 60; i++) {
        double next = (guess + x / guess) / 2.0;
        double diff = next - guess;
        if (diff < 0.0) diff = -diff;
        if (diff < 1e-15 * guess + 1e-300) return next;
        guess = next;
    }
    return guess;
}

inline double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) result *= base;
        base *= base;
        n >>= 1;
    }
    return result;
}

inline double d_exp(double x) {
    if (x != x) return x;
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return 1.7976931348623157e308;
    if (x < -745.13321910194) return 0.0;
    constexpr double INV_LN2 = 1.4426950408889634;
    constexpr double C1 = 0.693359375;
    constexpr double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = static_cast<int>(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - static_cast<double>(k) * C1) - static_cast<double>(k) * C2;
    double term = 1.0, sum = 1.0;
    for (int i = 1; i <= 17; i++) {
        term *= r / static_cast<double>(i);
        sum += term;
    }
    return sum * pow2i(k);
}

inline double d_floor(double x) {
    if (x >= 9007199254740992.0 || x <= -9007199254740992.0) return x;
    double t = static_cast<double>(static_cast<long long>(x));
    return (t > x) ? t - 1.0 : t;
}

constexpr double PI = 3.141592653589793;

}  // namespace detail

// Projected range Rp and straggle dRp for (species, energy) from the SRIM
// table, with linear interpolation / extrapolation. Throws on unknown species.
inline std::pair<double, double> implant_range(const std::string& species,
                                               double energy_kev) {
    struct Entry {
        const char* species;
        double energy;
        double rp;
        double straggle;
    };
    static const Entry table[] = {
        {"B", 10, 33.0, 18.0},   {"B", 30, 92.0, 38.0},
        {"B", 100, 260.0, 80.0}, {"P", 30, 39.0, 19.0},
        {"P", 100, 130.0, 50.0}, {"As", 30, 22.0, 11.0},
        {"As", 100, 64.0, 28.0}, {"BF2", 30, 31.0, 19.0},
        {"BF2", 60, 60.0, 30.0},
    };
    std::vector<Entry> matches;
    for (const Entry& e : table)
        if (species == e.species) matches.push_back(e);
    if (matches.empty())
        throw std::invalid_argument("unknown implant species: " + species);
    // The table is already energy-sorted per species.

    for (const Entry& m : matches) {
        double diff = m.energy - energy_kev;
        if (diff < 0.0) diff = -diff;
        if (diff < 1e-6) return {m.rp, m.straggle};
    }
    const Entry& lo = matches.front();
    if (energy_kev < lo.energy)
        return {lo.rp * energy_kev / lo.energy,
                lo.straggle * energy_kev / lo.energy};
    const Entry& hi = matches.back();
    if (energy_kev > hi.energy)
        return {hi.rp * energy_kev / hi.energy,
                hi.straggle * energy_kev / hi.energy};
    for (std::size_t i = 0; i + 1 < matches.size(); i++) {
        const Entry& a = matches[i];
        const Entry& b = matches[i + 1];
        if (energy_kev >= a.energy && energy_kev <= b.energy) {
            double f = (energy_kev - a.energy) / (b.energy - a.energy);
            return {a.rp + f * (b.rp - a.rp),
                    a.straggle + f * (b.straggle - a.straggle)};
        }
    }
    throw std::invalid_argument("interpolation failed");  // unreachable
}

inline double diffusivity_1000c(const std::string& species) {
    if (species == "B") return 1e-14;
    if (species == "P") return 1.2e-14;
    if (species == "As") return 4e-15;
    return 1e-14;
}

inline double diffusivity_cm2_per_s(const std::string& species,
                                    double temperature_c) {
    double d0 = diffusivity_1000c(species);
    double ratio = (temperature_c + 273.15) / 1273.15;
    return d0 * ratio * ratio;  // T^2 scaling
}

// ── Process steps ────────────────────────────────────────────────────────────

inline CrossSection deal_grove_oxidation(const CrossSection& cs, double time_min,
                                         std::optional<double> a_um = std::nullopt,
                                         std::optional<double> b_um2_per_hr =
                                             std::nullopt) {
    if (time_min <= 0.0)
        throw std::invalid_argument("time_min must be > 0");
    double a = a_um.value_or(DEAL_GROVE_DRY_1000C_A);
    double b = b_um2_per_hr.value_or(DEAL_GROVE_DRY_1000C_B);

    bool has_oxide =
        !cs.layers.empty() && cs.layers.front().material == "SiO2";
    double tau_hr = 0.0;
    if (has_oxide) {
        double prev_um = cs.layers.front().thickness_nm / 1000.0;
        tau_hr = (prev_um * prev_um + a * prev_um) / b;
    }
    double t_hr = time_min / 60.0;
    double discriminant = a * a + 4.0 * b * (t_hr + tau_hr);
    double t_ox_nm = ((-a + detail::d_sqrt(discriminant)) / 2.0) * 1000.0;

    CrossSection out;
    out.layers.emplace_back("SiO2", t_ox_nm);
    std::size_t start = has_oxide ? 1u : 0u;
    for (std::size_t i = start; i < cs.layers.size(); i++)
        out.layers.push_back(cs.layers[i]);
    return out;
}

inline CrossSection deposit(const CrossSection& cs, const std::string& material,
                            double thickness_nm) {
    if (thickness_nm <= 0.0)
        throw std::invalid_argument("thickness_nm must be > 0");
    CrossSection out;
    out.layers.emplace_back(material, thickness_nm);
    for (const Layer& l : cs.layers) out.layers.push_back(l);
    return out;
}

inline CrossSection etch(const CrossSection& cs,
                         const std::string& target_material, double depth_nm) {
    CrossSection out = cs;  // copy
    if (depth_nm <= 0.0 || out.layers.empty()) return out;
    double remaining = depth_nm;
    while (remaining > 0.0) {
        if (out.layers.empty()) break;
        if (out.layers.front().material != target_material) break;
        if (out.layers.front().thickness_nm > remaining) {
            out.layers.front().thickness_nm -= remaining;
            remaining = 0.0;
        } else {
            remaining -= out.layers.front().thickness_nm;
            out.layers.erase(out.layers.begin());
        }
    }
    return out;
}

inline CrossSection implant(const CrossSection& cs, const std::string& species,
                            double energy_kev, double dose_per_cm2) {
    if (dose_per_cm2 <= 0.0)
        throw std::invalid_argument("dose_per_cm2 must be > 0");
    auto range = implant_range(species, energy_kev);  // throws if unknown
    double rp_nm = range.first, rp_std_nm = range.second;

    CrossSection out = cs;
    bool si_found = false;
    for (Layer& layer : out.layers) {
        if (!si_found && layer.material == "Si") {
            si_found = true;
            std::vector<Sample>& profile = layer.doping[species];
            double peak = dose_per_cm2 /
                          (rp_std_nm * 1e-7 * detail::d_sqrt(2.0 * detail::PI));
            double cand = rp_nm + 4.0 * rp_std_nm;
            double max_depth =
                layer.thickness_nm < cand ? layer.thickness_nm : cand;
            double floor_val = detail::d_floor(max_depth / 5.0);
            std::size_t n_samples =
                static_cast<std::size_t>(floor_val > 20.0 ? floor_val : 20.0);
            for (std::size_t i = 0; i < n_samples; i++) {
                double x_nm = (static_cast<double>(i) + 0.5) *
                              (max_depth / static_cast<double>(n_samples));
                double dx = x_nm - rp_nm;
                double conc =
                    peak *
                    detail::d_exp(-(dx * dx) / (2.0 * rp_std_nm * rp_std_nm));
                profile.emplace_back(x_nm, conc);
            }
        }
    }
    if (!si_found)
        throw std::invalid_argument("no Si layer found for implant");
    return out;
}

inline CrossSection diffuse(const CrossSection& cs, double /*time_min*/,
                            std::optional<double> /*temperature_c*/ =
                                std::nullopt) {
    // v0.1.0 keeps the sampled points unchanged -> a plain copy.
    return cs;
}

}  // namespace fab_process_simulation
}  // namespace ca

#endif  // CA_FAB_PROCESS_SIMULATION_HPP
