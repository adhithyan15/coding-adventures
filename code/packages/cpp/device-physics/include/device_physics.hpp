// device_physics.hpp — semiconductor device-physics primitives, header-only in
// pure ISO C++17 (namespace ca::device_physics). A faithful port of the Rust
// `device-physics` crate.
// ===========================================================================
//
// Closed-form textbook models (Sedra/Smith, Pierret, Streetman) in SI units:
// thermal voltage, intrinsic carrier concentration, Fermi potential, a PN
// junction (built-in voltage, depletion width, Shockley current), and a MOSFET
// (oxide capacitance, flat-band / threshold voltage with body effect).
//
// NO libm / <cmath>: the transcendentals (exp, ln, sqrt) are computed from
// scratch; results match the Rust f64 models to ~1e-9 relative. `powf(x, 1.5)`
// is written as x * sqrt(x).
//
// DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port throws
// std::invalid_argument / std::domain_error with the same intent.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions.
#ifndef CA_DEVICE_PHYSICS_HPP
#define CA_DEVICE_PHYSICS_HPP

#include <stdexcept>
#include <string>

namespace ca {
namespace device_physics {

// ── Physical constants (SI) ──────────────────────────────────────────────────
inline constexpr double K_BOLTZMANN = 1.380649e-23;    // J/K
inline constexpr double Q_ELECTRON = 1.602176634e-19;  // C
inline constexpr double EPS0 = 8.8541878128e-12;       // F/m
inline constexpr double EPS_SI = 11.7 * EPS0;          // F/m
inline constexpr double EPS_OX = 3.9 * EPS0;           // F/m
inline constexpr double N_I_300K = 1.0e16;             // /m^3
inline constexpr double N_C = 2.8e25;                  // /m^3
inline constexpr double N_V = 1.04e25;                 // /m^3
inline constexpr double EG_SI_300K = 1.12;             // eV
inline constexpr double MU_N_300K = 1350e-4;           // m^2/V.s
inline constexpr double MU_P_300K = 480e-4;            // m^2/V.s

namespace detail {

inline double d_abs(double x) { return x < 0.0 ? -x : x; }

inline double d_sqrt(double x) {
    if (x <= 0.0) return 0.0;
    double guess = x >= 1.0 ? x : 1.0;
    for (int i = 0; i < 80; i++) {
        double next = (guess + x / guess) / 2.0;
        if (d_abs(next - guess) < 1e-15 * guess + 1e-300) return next;
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

inline double d_ln(double x) {
    // Guard non-finite / non-positive x BEFORE the reduction loops: for +inf
    // the `m >= 2` loop, and for +0 the `m < 1` loop, would otherwise never
    // terminate (a DoS reachable when e.g. na*nd overflows to +inf). Return a
    // large sentinel (Rust's ln yields +/-inf here) and never spin.
    if (x != x) return x;                            // NaN propagates
    if (x <= 0.0) return -1.7976931348623157e308;    // ln(<= 0): ~ -inf
    if (x > 1.7976931348623157e308) return 1.7976931348623157e308;  // +inf
    int e = 0;
    double m = x;
    while (m < 1.0) {
        m *= 2.0;
        e--;
    }
    while (m >= 2.0) {
        m *= 0.5;
        e++;
    }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u, sum = u;
    for (int n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / static_cast<double>(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) break;
    }
    constexpr double LN2 = 0.6931471805599453;
    return static_cast<double>(e) * LN2 + 2.0 * sum;
}

}  // namespace detail

// ── Bulk models ──────────────────────────────────────────────────────────────

inline double thermal_voltage(double t_kelvin) {
    return K_BOLTZMANN * t_kelvin / Q_ELECTRON;
}

// Intrinsic carrier concentration n_i(T) [/m^3]. Throws std::domain_error for
// T < 100 K (below model validity).
inline double intrinsic_concentration(double t_kelvin) {
    if (detail::d_abs(t_kelvin - 300.0) < 1e-9) return N_I_300K;
    if (t_kelvin < 100.0)
        throw std::domain_error("T below model validity (>= 100 K)");
    double ratio = t_kelvin / 300.0;
    double factor = ratio * detail::d_sqrt(ratio);  // ratio^1.5
    double vt = thermal_voltage(t_kelvin);
    double bandgap_term =
        detail::d_exp(-(EG_SI_300K / (2.0 * vt)) * (1.0 - t_kelvin / 300.0));
    return N_I_300K * factor * bandgap_term;
}

// Fermi potential [V]: +V_T ln(N/n_i) for "p", negated for "n".
inline double fermi_potential(double n_doping, const std::string& kind,
                              double t_kelvin) {
    if (n_doping <= 0.0)
        throw std::invalid_argument("doping N must be > 0");
    double n_i = intrinsic_concentration(t_kelvin);
    double magnitude = thermal_voltage(t_kelvin) * detail::d_ln(n_doping / n_i);
    if (kind == "p") return magnitude;
    if (kind == "n") return -magnitude;
    throw std::invalid_argument("kind must be 'p' or 'n'");
}

// ── PN junction ──────────────────────────────────────────────────────────────

class PNJunction {
public:
    double na, nd, a, t, tau_n, tau_p;

    PNJunction(double na_, double nd_, double a_, double t_, double tau_n_,
               double tau_p_)
        : na(na_), nd(nd_), a(a_), t(t_), tau_n(tau_n_), tau_p(tau_p_) {
        if (na <= 0.0 || nd <= 0.0)
            throw std::invalid_argument("doping must be > 0");
        if (a <= 0.0) throw std::invalid_argument("area A must be > 0");
    }

    double built_in_voltage() const {
        double n_i = ni_or_default(t);
        return thermal_voltage(t) * detail::d_ln((na * nd) / (n_i * n_i));
    }

    double depletion_width(double v_applied) const {
        double phi_bi = built_in_voltage();
        if (v_applied >= phi_bi) return 0.0;
        double num = 2.0 * EPS_SI * (na + nd) * (phi_bi - v_applied);
        double den = Q_ELECTRON * na * nd;
        return detail::d_sqrt(num / den);
    }

    double saturation_current() const {
        double n_i = ni_or_default(t);
        double vt = thermal_voltage(t);
        double d_n = MU_N_300K * vt;  // Einstein relation
        double d_p = MU_P_300K * vt;
        double l_n = detail::d_sqrt(d_n * tau_n);
        double l_p = detail::d_sqrt(d_p * tau_p);
        return Q_ELECTRON * a * n_i * n_i *
               (d_n / (l_n * na) + d_p / (l_p * nd));
    }

    double current(double v) const {
        double vt = thermal_voltage(t);
        return saturation_current() * (detail::d_exp(v / vt) - 1.0);
    }

private:
    // n_i(T), falling back to the 300 K value below model validity (matches the
    // Rust `.unwrap_or(N_I_300K)`).
    static double ni_or_default(double temp) {
        if (detail::d_abs(temp - 300.0) < 1e-9) return N_I_300K;
        if (temp < 100.0) return N_I_300K;
        return intrinsic_concentration(temp);
    }
};

// ── MOSFET ───────────────────────────────────────────────────────────────────

enum class MosType { NMOS, PMOS };

class MOSFETParams {
public:
    MosType device_type;
    double l, w, t_ox, n_body, phi_ms, q_ox, t;

    MOSFETParams(MosType type, double l_, double w_, double t_ox_,
                 double n_body_, double phi_ms_, double q_ox_, double t_)
        : device_type(type),
          l(l_),
          w(w_),
          t_ox(t_ox_),
          n_body(n_body_),
          phi_ms(phi_ms_),
          q_ox(q_ox_),
          t(t_) {
        if (l <= 0.0 || w <= 0.0)
            throw std::invalid_argument("L and W must be > 0");
        if (t_ox <= 0.0) throw std::invalid_argument("T_ox must be > 0");
        if (n_body <= 0.0) throw std::invalid_argument("N_body must be > 0");
    }

    double c_ox() const { return EPS_OX / t_ox; }
    double v_fb() const { return phi_ms - q_ox / c_ox(); }

    double phi_f() const {
        const char* kind = device_type == MosType::NMOS ? "p" : "n";
        try {
            return detail::d_abs(fermi_potential(n_body, kind, t));
        } catch (const std::exception&) {
            return 0.0;
        }
    }

    double gamma() const {
        return detail::d_sqrt(2.0 * EPS_SI * Q_ELECTRON * n_body) / c_ox();
    }

    // Threshold voltage with body effect. Throws std::domain_error when
    // V_SB < -2*phi_F (body-source forward biased).
    double threshold_voltage(double v_sb) const {
        double pf = phi_f();
        double two_phi_f = 2.0 * pf;
        if (-two_phi_f > v_sb)
            throw std::domain_error("V_SB below 2*phi_F; body-source forward");
        double g = gamma();
        double v_t0 = v_fb() + two_phi_f + g * detail::d_sqrt(two_phi_f);
        return v_t0 +
               g * (detail::d_sqrt(two_phi_f + v_sb) - detail::d_sqrt(two_phi_f));
    }
};

}  // namespace device_physics
}  // namespace ca

#endif  // CA_DEVICE_PHYSICS_HPP
