// reed_solomon.hpp — Reed-Solomon error correction over GF(2^8), in pure ISO
// C++17 (header-only). A faithful port of the Rust `reed-solomon` crate, in
// namespace `ca::reed_solomon`.
// ===========================================================================
//
// Reed-Solomon adds `n_check` parity bytes so up to `t = n_check/2` corrupted
// bytes can be located AND corrected (QR codes, CDs, deep-space comms). Encoding
// is systematic (message first, then check bytes); decoding runs syndromes ->
// Berlekamp-Massey -> Chien search -> Forney.
//
//   ca::reed_solomon::encode / decode        — the code (decode -> std::optional)
//   ca::reed_solomon::build_generator        — the generator polynomial (LE)
//   ca::reed_solomon::syndromes / error_locator — decode internals
//
// Field arithmetic comes from the sibling header-only `gf256` (default
// Reed-Solomon polynomial 0x11D). Invalid arguments throw std::invalid_argument;
// `decode` returns std::nullopt when there are too many errors to correct.
//
// Constraints: `n_check` even and >= 2; total codeword length <= 255.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef REED_SOLOMON_HPP
#define REED_SOLOMON_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

#include "gf256.hpp"

namespace ca {
namespace reed_solomon {

namespace detail {

namespace gf = ca::gf256;

inline void require_even_n_check(std::size_t n_check) {
    // Even, >= 2, and small enough that a codeword still fits a GF(256) block
    // (with a >= 1-byte message, n_check <= 254). Matches the C sibling.
    if (n_check == 0 || n_check % 2 != 0 || n_check > 254) {
        throw std::invalid_argument("n_check must be an even number in 2..254");
    }
}

// Evaluate a big-endian polynomial (p[0] = highest degree) at x.
inline std::uint8_t poly_eval_be(const std::vector<std::uint8_t> &p,
                                 std::uint8_t x) {
    std::uint8_t acc = 0;
    for (std::uint8_t c : p) {
        acc = gf::add(gf::multiply(acc, x), c);
    }
    return acc;
}
// Evaluate a little-endian polynomial (p[i] = coeff of x^i) at x.
inline std::uint8_t poly_eval_le(const std::vector<std::uint8_t> &p,
                                 std::uint8_t x) {
    std::uint8_t acc = 0;
    for (std::size_t i = p.size(); i > 0; i--) {
        acc = gf::add(gf::multiply(acc, x), p[i - 1]);
    }
    return acc;
}
// Multiply two little-endian polynomials (convolution).
inline std::vector<std::uint8_t> poly_mul_le(const std::vector<std::uint8_t> &a,
                                             const std::vector<std::uint8_t> &b) {
    if (a.empty() || b.empty()) {
        return {};
    }
    std::vector<std::uint8_t> out(a.size() + b.size() - 1, 0);
    for (std::size_t i = 0; i < a.size(); i++) {
        for (std::size_t j = 0; j < b.size(); j++) {
            out[i + j] = gf::add(out[i + j], gf::multiply(a[i], b[j]));
        }
    }
    return out;
}
// Remainder of big-endian division by a monic divisor.
inline std::vector<std::uint8_t> poly_mod_be(std::vector<std::uint8_t> rem,
                                             const std::vector<std::uint8_t> &divisor) {
    std::size_t div_len = divisor.size();
    if (rem.size() < div_len) {
        return rem;
    }
    std::size_t steps = rem.size() - div_len + 1;
    for (std::size_t i = 0; i < steps; i++) {
        std::uint8_t coeff = rem[i];
        if (coeff == 0) {
            continue;
        }
        for (std::size_t j = 0; j < div_len; j++) {
            rem[i + j] = gf::add(rem[i + j], gf::multiply(coeff, divisor[j]));
        }
    }
    return std::vector<std::uint8_t>(rem.end() - static_cast<std::ptrdiff_t>(div_len - 1),
                                     rem.end());
}

// Berlekamp-Massey: returns (locator LE with Lambda[0]=1, error count L).
inline std::pair<std::vector<std::uint8_t>, std::size_t> berlekamp_massey(
    const std::vector<std::uint8_t> &synds) {
    std::vector<std::uint8_t> c = {1}, b = {1};
    std::size_t big_l = 0, x = 1;
    std::uint8_t b_scale = 1;
    for (std::size_t n = 0; n < synds.size(); n++) {
        std::uint8_t d = synds[n];
        for (std::size_t j = 1; j <= big_l; j++) {
            if (j < c.size() && n >= j) {
                d = gf::add(d, gf::multiply(c[j], synds[n - j]));
            }
        }
        if (d == 0) {
            x++;
        } else if (2 * big_l <= n) {
            std::vector<std::uint8_t> t_save = c;
            std::uint8_t scale = gf::divide(d, b_scale);
            std::size_t shifted_len = x + b.size();
            if (c.size() < shifted_len) {
                c.resize(shifted_len, 0);
            }
            for (std::size_t k = 0; k < b.size(); k++) {
                c[x + k] = gf::add(c[x + k], gf::multiply(scale, b[k]));
            }
            big_l = n + 1 - big_l;
            b = t_save;
            b_scale = d;
            x = 1;
        } else {
            std::uint8_t scale = gf::divide(d, b_scale);
            std::size_t shifted_len = x + b.size();
            if (c.size() < shifted_len) {
                c.resize(shifted_len, 0);
            }
            for (std::size_t k = 0; k < b.size(); k++) {
                c[x + k] = gf::add(c[x + k], gf::multiply(scale, b[k]));
            }
            x++;
        }
    }
    return {c, big_l};
}

inline std::uint8_t inv_locator(std::size_t p, std::size_t n) {
    std::size_t exp = (p + 256 - n) % 255;
    return gf::power(2, static_cast<std::uint32_t>(exp));
}

inline std::vector<std::size_t> chien_search(const std::vector<std::uint8_t> &lambda,
                                             std::size_t n) {
    std::vector<std::size_t> positions;
    for (std::size_t p = 0; p < n; p++) {
        if (poly_eval_le(lambda, inv_locator(p, n)) == 0) {
            positions.push_back(p);
        }
    }
    return positions;
}

// Forney: returns error magnitudes, or nullopt if unrecoverable.
inline std::optional<std::vector<std::uint8_t>> forney(
    const std::vector<std::uint8_t> &lambda,
    const std::vector<std::uint8_t> &synds,
    const std::vector<std::size_t> &positions, std::size_t n) {
    std::size_t two_t = synds.size();
    std::vector<std::uint8_t> omega = poly_mul_le(synds, lambda);
    if (omega.size() > two_t) {
        omega.resize(two_t);
    }
    std::vector<std::uint8_t> lambda_prime(
        lambda.empty() ? 0 : lambda.size() - 1, 0);
    for (std::size_t j = 0; j < lambda.size(); j++) {
        if (j % 2 == 1) {
            std::size_t out_idx = j - 1;
            if (out_idx < lambda_prime.size()) {
                lambda_prime[out_idx] = gf::add(lambda_prime[out_idx], lambda[j]);
            }
        }
    }
    std::vector<std::uint8_t> mags;
    mags.reserve(positions.size());
    for (std::size_t pos : positions) {
        std::uint8_t xi_inv = inv_locator(pos, n);
        std::uint8_t omega_val = poly_eval_le(omega, xi_inv);
        std::uint8_t lp_val = poly_eval_le(lambda_prime, xi_inv);
        if (lp_val == 0) {
            return std::nullopt;
        }
        mags.push_back(gf::divide(omega_val, lp_val));
    }
    return mags;
}

} // namespace detail

// The generator polynomial for `n_check` check bytes (little-endian, monic).
inline std::vector<std::uint8_t> build_generator(std::size_t n_check) {
    detail::require_even_n_check(n_check);
    std::vector<std::uint8_t> g = {1};
    for (std::size_t i = 1; i <= n_check; i++) {
        std::uint8_t alpha_i =
            detail::gf::power(2, static_cast<std::uint32_t>(i));
        std::vector<std::uint8_t> new_g(g.size() + 1, 0);
        for (std::size_t j = 0; j < g.size(); j++) {
            new_g[j] = detail::gf::add(new_g[j],
                                       detail::gf::multiply(g[j], alpha_i));
            new_g[j + 1] = detail::gf::add(new_g[j + 1], g[j]);
        }
        g = new_g;
    }
    return g;
}

// Systematic RS encoding: message bytes followed by n_check check bytes.
inline std::vector<std::uint8_t> encode(const std::vector<std::uint8_t> &message,
                                        std::size_t n_check) {
    detail::require_even_n_check(n_check);
    std::size_t n = message.size() + n_check;
    if (n > 255) {
        throw std::invalid_argument("total codeword length exceeds 255");
    }
    std::vector<std::uint8_t> g_le = build_generator(n_check);
    std::vector<std::uint8_t> g_be(g_le.rbegin(), g_le.rend());
    std::vector<std::uint8_t> shifted = message;
    shifted.resize(n, 0);
    std::vector<std::uint8_t> remainder = detail::poly_mod_be(shifted, g_be);
    std::vector<std::uint8_t> codeword = message;
    codeword.resize(message.size() + (n_check - remainder.size()), 0);
    codeword.insert(codeword.end(), remainder.begin(), remainder.end());
    return codeword;
}

// The n_check syndromes of a received codeword (all zero -> no errors).
inline std::vector<std::uint8_t> syndromes(const std::vector<std::uint8_t> &received,
                                           std::size_t n_check) {
    std::vector<std::uint8_t> out;
    out.reserve(n_check);
    for (std::size_t i = 1; i <= n_check; i++) {
        out.push_back(detail::poly_eval_be(
            received, detail::gf::power(2, static_cast<std::uint32_t>(i))));
    }
    return out;
}

// The error locator polynomial from syndromes (LE, Lambda[0] = 1).
inline std::vector<std::uint8_t> error_locator(
    const std::vector<std::uint8_t> &synds) {
    return detail::berlekamp_massey(synds).first;
}

// Decode, correcting up to t = n_check/2 errors. nullopt if too many errors.
inline std::optional<std::vector<std::uint8_t>> decode(
    const std::vector<std::uint8_t> &received, std::size_t n_check) {
    detail::require_even_n_check(n_check);
    if (received.size() < n_check || received.size() > 255) {
        throw std::invalid_argument("received length must be n_check..255");
    }
    std::size_t t = n_check / 2;
    std::size_t n = received.size();
    std::size_t k = n - n_check;

    std::vector<std::uint8_t> synds = syndromes(received, n_check);
    bool zero = true;
    for (std::uint8_t s : synds) {
        if (s != 0) {
            zero = false;
        }
    }
    if (zero) {
        return std::vector<std::uint8_t>(received.begin(),
                                         received.begin() + static_cast<std::ptrdiff_t>(k));
    }
    auto bm = detail::berlekamp_massey(synds);
    const std::vector<std::uint8_t> &lambda = bm.first;
    std::size_t num_errors = bm.second;
    if (num_errors > t) {
        return std::nullopt;
    }
    std::vector<std::size_t> positions = detail::chien_search(lambda, n);
    if (positions.size() != num_errors) {
        return std::nullopt;
    }
    auto mags = detail::forney(lambda, synds, positions, n);
    if (!mags.has_value()) {
        return std::nullopt;
    }
    std::vector<std::uint8_t> corrected = received;
    for (std::size_t i = 0; i < positions.size(); i++) {
        corrected[positions[i]] = detail::gf::add(corrected[positions[i]],
                                                  mags.value()[i]);
    }
    return std::vector<std::uint8_t>(corrected.begin(),
                                     corrected.begin() + static_cast<std::ptrdiff_t>(k));
}

} // namespace reed_solomon
} // namespace ca

#endif // REED_SOLOMON_HPP
