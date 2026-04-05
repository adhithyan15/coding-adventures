package CodingAdventures::ReedSolomon;

# ============================================================================
# CodingAdventures::ReedSolomon — Reed-Solomon Error-Correcting Codes
# ============================================================================
#
# # What Is Reed-Solomon?
#
# Reed-Solomon (RS) is a block error-correcting code invented by Irving Reed
# and Gustave Solomon in 1960.  Given a message of k bytes, we produce a
# codeword of n = k + n_check bytes, where n_check is even.  The decoder can
# recover the original k bytes even when up to t = n_check/2 bytes have been
# corrupted in transit.
#
# Where RS codes appear in the wild:
#   * QR codes — up to 30% of the symbol can be obscured and still decoded
#   * CDs / DVDs — CIRC (two-level RS) corrects burst scratches
#   * Hard drives — sector-level error correction in firmware
#   * Voyager 1 probes — images sent across 20+ billion kilometres
#   * RAID-6 — the two parity drives ARE an (n, n-2) RS code over GF(256)
#
# # Building Blocks
#
#   MA00 Polynomial   — coefficient-array polynomial arithmetic
#   MA01 GF256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
#   MA02 ReedSolomon  — RS encoding and decoding (THIS MODULE)
#
# # Polynomial Conventions (critical — matches all other language ports)
#
# Codeword bytes use a **big-endian** polynomial:
#
#   codeword[0]·x^{n-1} + codeword[1]·x^{n-2} + … + codeword[n-1]·x^0
#
# Systematic layout:
#   [ message bytes (k) | check bytes (n_check) ]
#     degree n-1 … n_check   degree n_check-1 … 0
#
# Internal polynomials (generator, Λ, Ω) use **little-endian** arrays where
# index = degree.  Only codeword bytes use big-endian ordering.
#
# For error position p in a big-endian codeword of length n:
#   X_p     = α^{n-1-p}           (error locator value)
#   X_p⁻¹  = α^{(p+256-n) mod 255}  (its inverse, used in Chien/Forney)
#
# # The Five-Step Decode Pipeline
#
#   1. Syndromes:        S_j = received(α^j)  for j=1..n_check
#   2. Berlekamp-Massey: find Λ(x), the error locator polynomial
#   3. Chien search:     evaluate Λ at all possible X_p⁻¹; zeros → error positions
#   4. Forney algorithm: compute error magnitudes from Ω(x) and Λ'(x)
#   5. Apply:            XOR each magnitude into the received byte at each position
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

# Make the GF256 module findable.  In normal installation it is in @INC;
# for development we also accept the relative path used in the BUILD step.
use lib '../gf256/lib';
use CodingAdventures::GF256 qw(add subtract multiply divide power inverse);

our $VERSION = '0.01';

our @EXPORT_OK = qw(encode decode syndromes build_generator error_locator);

# ============================================================================
# build_generator($n_check)
# ============================================================================
#
# Build the RS generator polynomial for a given number of check bytes.
#
# The generator is the product of n_check linear factors:
#
#   g(x) = (x + α¹)(x + α²) … (x + α^{n_check})
#
# where α = 2 is the primitive element of GF(256).
#
# Returns a little-endian array ref (index = degree), length n_check+1.
# The last element is always 1 (monic polynomial — leading coefficient = 1).
#
# Algorithm (multiply iteratively):
#
#   Start with g = [1].
#   For i = 1 .. n_check:
#     alpha_i = 2^i   (in GF(256))
#     new_g   = zero array of length g.length + 1
#     For each coefficient g[j]:
#       new_g[j]   ^= g[j] * alpha_i     (constant term of factor: + alpha_i)
#       new_g[j+1] ^= g[j]               (x term of factor)
#
# Example for n_check = 2:
#   Start: g = [1]
#   i=1 (α¹=2): g = [1*2, 1] = [2, 1]
#   i=2 (α²=4): new_g[0] = 2*4 = 8; new_g[1] = 1*4 ^ 2 = 4^2 = 6; new_g[2] = 1
#               g = [8, 6, 1]
#
# Cross-language test vector: build_generator(2) must return [8, 6, 1].
#
# Verification that α¹=2 is a root of g(x) = 8 + 6x + x²:
#   g(2) = 8 ⊕ mul(6,2) ⊕ mul(1,4) = 8 ⊕ 12 ⊕ 4 = 0  ✓
#
# @param $n_check  Number of check bytes (positive even integer)
# @return          Array ref of LE generator coefficients, length n_check+1
# @die             "InvalidInput: ..." if n_check is 0, odd, or > 254
# ============================================================================
sub build_generator {
    my ($n_check) = @_;

    # Validate: n_check must be a positive even integer.
    # n_check == 0 → no redundancy, useless
    # n_check <= 0 → nonsensical (negative or zero)
    # n_check odd  → correction capacity t = n_check/2 would be a fraction
    if (!defined($n_check) || $n_check <= 0 || ($n_check % 2 != 0)) {
        die "InvalidInput: n_check must be a positive even number, got " . (defined($n_check) ? $n_check : 'undef');
    }

    my @g = (1);   # start with the polynomial "1" (constant)

    for my $i (1 .. $n_check) {
        # alpha_i = 2^i in GF(256); this is the root we are building into the factor
        my $alpha_i = power(2, $i);

        # new_g has one more term than g
        my @new_g = (0) x (@g + 1);

        for my $j (0 .. $#g) {
            # Multiply g[j] by the factor (alpha_i + x):
            #   g[j] * alpha_i  → contributes to position j (constant part)
            #   g[j] * x        → contributes to position j+1 (x part)
            $new_g[$j]     ^= multiply($g[$j], $alpha_i);
            $new_g[$j + 1] ^= $g[$j];
        }

        @g = @new_g;
    }

    return \@g;
}

# ============================================================================
# Internal helper: poly_eval_be(\@p, $x)
# ============================================================================
#
# Evaluate a big-endian GF(256) polynomial at x using Horner's method.
#
# p[0] is the highest-degree coefficient.  Iterate left to right:
#
#   acc = 0
#   for each coefficient b in p (highest to lowest degree):
#     acc = gf_add(gf_mul(acc, x), b)
#
# Why Horner's method?  Instead of computing x^k for each term (expensive),
# we factor:
#   a_n x^n + a_{n-1} x^{n-1} + … + a_0
#   = (…((a_n · x + a_{n-1}) · x + a_{n-2}) · x … + a_0)
#
# Each step: one multiply + one add → O(n) total.
#
# Used for syndrome evaluation: S_j = poly_eval_be(received, α^j).
#
# @param $p   Array ref of big-endian polynomial coefficients
# @param $x   Evaluation point in GF(256)
# @return     p(x) in GF(256) (integer 0..255)
# ============================================================================
sub _poly_eval_be {
    my ($p, $x) = @_;
    my $acc = 0;
    for my $b (@$p) {
        $acc = add(multiply($acc, $x), $b);
    }
    return $acc;
}

# ============================================================================
# Internal helper: poly_eval_le(\@p, $x)
# ============================================================================
#
# Evaluate a little-endian GF(256) polynomial at x using Horner's method.
#
# p[i] is the coefficient of x^i (index = degree).  To apply Horner, we
# iterate from the highest degree down to the constant term:
#
#   acc = 0
#   for each coefficient c from p[-1] down to p[0]:
#     acc = gf_add(gf_mul(acc, x), c)
#
# This is used for evaluating Λ(x), Ω(x), and Λ'(x) in Chien/Forney.
#
# @param $p   Array ref of little-endian polynomial coefficients
# @param $x   Evaluation point in GF(256)
# @return     p(x) in GF(256) (integer 0..255)
# ============================================================================
sub _poly_eval_le {
    my ($p, $x) = @_;
    my $acc = 0;
    for my $c (reverse @$p) {
        $acc = add(multiply($acc, $x), $c);
    }
    return $acc;
}

# ============================================================================
# Internal helper: poly_mul_le(\@a, \@b)
# ============================================================================
#
# Multiply two little-endian GF(256) polynomials (convolution).
#
# result[i+j] ^= a[i] · b[j]   for all i, j
#
# In GF(256), addition is XOR, so ^= is correct and no carries occur.
# Used in Forney to compute Ω(x) = S(x)·Λ(x) mod x^{2t}.
#
# Degree of result = degree(a) + degree(b), so length = len(a) + len(b) - 1.
#
# @param $a   Array ref of little-endian polynomial coefficients
# @param $b   Array ref of little-endian polynomial coefficients
# @return     Array ref of product polynomial (little-endian)
# ============================================================================
sub _poly_mul_le {
    my ($a, $b) = @_;
    return [] unless @$a && @$b;

    my @result = (0) x (@$a + @$b - 1);
    for my $i (0 .. $#$a) {
        for my $j (0 .. $#$b) {
            $result[$i + $j] ^= multiply($a->[$i], $b->[$j]);
        }
    }
    return \@result;
}

# ============================================================================
# Internal helper: poly_mod_be(\@dividend, \@divisor)
# ============================================================================
#
# Remainder of big-endian GF(256) polynomial long division.
#
# Both arrays are big-endian (index 0 = highest degree coefficient).
# The divisor must be monic (leading coefficient = 1).  The generator
# polynomial built by build_generator is always monic, so this holds.
#
# Algorithm (schoolbook long division):
#
#   rem = copy of dividend
#   for i = 0 .. len(rem) - len(divisor):
#     coeff = rem[i]        (current leading term)
#     if coeff == 0: skip   (already zero, nothing to eliminate)
#     for j = 0 .. len(divisor)-1:
#       rem[i+j] ^= gf_mul(coeff, divisor[j])
#   return rem[-(divisor.length-1)..]    (last deg(divisor) elements)
#
# Returns an array of length divisor.length - 1 (= n_check).
#
# @param $dividend  Array ref of big-endian dividend coefficients
# @param $divisor   Array ref of big-endian monic divisor coefficients
# @return           Array ref of remainder coefficients (big-endian)
# ============================================================================
sub _poly_mod_be {
    my ($dividend, $divisor) = @_;
    my @rem     = @$dividend;
    my $div_len = scalar @$divisor;
    return \@rem if @rem < $div_len;

    my $steps = @rem - $div_len + 1;
    for my $i (0 .. $steps - 1) {
        my $coeff = $rem[$i];
        next unless $coeff;

        for my $j (0 .. $div_len - 1) {
            $rem[$i + $j] ^= multiply($coeff, $divisor->[$j]);
        }
    }

    my @tail = @rem[-($div_len - 1) .. -1];
    return \@tail;
}

# ============================================================================
# encode(\@message, $n_check)
# ============================================================================
#
# Encode a message with Reed-Solomon systematic encoding.
#
# "Systematic" means the message bytes appear unchanged at the front of the
# output, followed by n_check check bytes:
#
#   output = [ message bytes (k) | check bytes (n_check) ]
#              position 0..k-1    position k..n-1
#
# The check bytes are chosen so that the entire codeword is divisible by
# the generator polynomial g(x).  This means evaluating the codeword at
# any root α^i of g gives 0 — the decoder exploits this property.
#
# Algorithm:
#   1. Build g(x) in LE; reverse to get g_be (big-endian, monic, g_be[0]=1).
#   2. Form shifted = message ++ [0]*n_check  (represents M(x)·x^{n_check}).
#   3. Check bytes = poly_mod_be(shifted, g_be)  (length n_check).
#   4. Return message ++ check_bytes.
#
# Why does this work?
#   If R(x) = M(x)·x^{n_check} mod g(x), then
#   C(x) = M(x)·x^{n_check} XOR R(x) = Q(x)·g(x) for some quotient Q.
#   So C(α^i) = Q(α^i)·g(α^i) = Q(α^i)·0 = 0 for i=1…n_check.
#
# @param $message   Array ref of data bytes (each 0..255)
# @param $n_check   Number of check bytes (positive even integer)
# @return           Array ref of n = k+n_check codeword bytes
# @die              "InvalidInput: ..." on bad n_check or oversized total
# ============================================================================
sub encode {
    my ($message, $n_check) = @_;

    # Validate n_check
    if (!defined($n_check) || $n_check <= 0 || ($n_check % 2 != 0)) {
        die "InvalidInput: n_check must be a positive even number, got " . (defined($n_check) ? $n_check : 'undef');
    }

    my $n = scalar(@$message) + $n_check;
    if ($n > 255) {
        die "InvalidInput: total codeword length $n exceeds GF(256) block size limit of 255";
    }

    # Step 1: Build generator poly in LE, reverse to BE for poly division.
    my $g_le = build_generator($n_check);
    my @g_be = reverse @$g_le;   # g_be[0] = 1 (monic)

    # Step 2: Append n_check zero bytes (represents multiplication by x^{n_check}).
    my @shifted = (@$message, (0) x $n_check);

    # Step 3: Compute remainder (the check bytes).
    my $remainder = _poly_mod_be(\@shifted, \@g_be);

    # Left-pad remainder to exactly n_check bytes if needed.
    while (scalar(@$remainder) < $n_check) {
        unshift @$remainder, 0;
    }

    return [@$message, @$remainder];
}

# ============================================================================
# syndromes(\@received, $n_check)
# ============================================================================
#
# Compute the n_check syndrome values of a received codeword.
#
#   S_j = received(α^j)   for j = 1, 2, …, n_check
#
# A valid (uncorrupted) codeword C(x) is divisible by the generator g(x),
# and α^j are roots of g.  Therefore C(α^j) = 0 for all j = 1…n_check.
#
# All-zero syndromes → no errors detected.
# Any non-zero syndrome → at least one byte was corrupted.
#
# Implementation: evaluate the big-endian received array at each α^j
# using Horner's method (see poly_eval_be).
#
# @param $received  Array ref of received codeword bytes (possibly corrupted)
# @param $n_check   Number of check bytes
# @return           Array ref of n_check syndrome values (each 0..255)
# ============================================================================
sub syndromes {
    my ($received, $n_check) = @_;
    my @synds;
    for my $j (1 .. $n_check) {
        push @synds, _poly_eval_be($received, power(2, $j));
    }
    return \@synds;
}

# ============================================================================
# _berlekamp_massey(\@synds)
# ============================================================================
#
# Find the shortest LFSR (Linear Feedback Shift Register) that generates
# the syndrome sequence — i.e., find the error locator polynomial Λ(x).
#
# # Background: What Is the Error Locator Polynomial?
#
# If errors occurred at positions p₁, p₂, …, pᵥ in the codeword, define
# the error locators X_k = α^{n-1-p_k}.  Then:
#
#   Λ(x) = ∏_{k=1}^{v} (1 - X_k·x)    with Λ(0) = 1
#
# The roots of Λ are {X_k⁻¹}, and Chien search finds them.
#
# # Berlekamp-Massey Algorithm
#
# We iteratively update a candidate Λ using the syndromes.  At step n,
# the "discrepancy" d measures how well the current Λ predicts S[n]:
#
#   d = S[n] ⊕ Σ_{j=1}^{L} Λ[j]·S[n-j]
#
# If d == 0: Λ is still consistent; extend the shift register by one.
# If d != 0 and 2L ≤ n: we discovered more errors; grow Λ.
# If d != 0 and 2L > n: adjust Λ without changing degree.
#
# State variables:
#   c       = current Λ (LE), starts [1]
#   b       = previous Λ (LE), starts [1]
#   big_l   = number of errors found so far
#   x_shift = iterations since last update of b (position shift)
#   b_scale = discrepancy at last update (for scaling)
#
# Returns (\@lambda, $num_errors) where @lambda is LE (lambda[0] = 1).
#
# @param $synds   Array ref of syndrome values (length 2t)
# @return         (lambda_poly_arrayref, num_errors)
# ============================================================================
sub _berlekamp_massey {
    my ($synds) = @_;
    my $two_t = scalar @$synds;

    my @c       = (1);   # current Λ (LE)
    my @b       = (1);   # previous Λ (LE)
    my $big_l   = 0;     # errors found so far
    my $x_shift = 1;     # iterations since last update
    my $b_scale = 1;     # discrepancy at last update

    for my $n (0 .. $two_t - 1) {
        # ------------------------------------------------------------------
        # Compute discrepancy: d = S[n] ⊕ Σ_{j=1}^{L} Λ[j]·S[n-j]
        # ------------------------------------------------------------------
        my $d = $synds->[$n];
        for my $j (1 .. $big_l) {
            $d ^= multiply($c[$j] // 0, $synds->[$n - $j]) if $n >= $j;
        }

        # ------------------------------------------------------------------
        # Update rule
        # ------------------------------------------------------------------
        if ($d == 0) {
            # Λ is still consistent with the syndromes seen so far.
            $x_shift++;

        } elsif (2 * $big_l <= $n) {
            # We have discovered more errors than Λ currently models.
            # Save c before modification, then grow Λ.
            my @t_save = @c;
            my $scale  = divide($d, $b_scale);

            # Extend c to accommodate the shifted b terms.
            my $target_len = $x_shift + scalar(@b);
            push @c, (0) x ($target_len - scalar(@c)) if scalar(@c) < $target_len;

            # c[x_shift + k] ^= scale * b[k]  for each k
            for my $k (0 .. $#b) {
                $c[$x_shift + $k] ^= multiply($scale, $b[$k]);
            }

            $big_l   = $n + 1 - $big_l;
            @b       = @t_save;
            $b_scale = $d;
            $x_shift = 1;

        } else {
            # Consistent: adjust Λ without growing degree.
            my $scale      = divide($d, $b_scale);
            my $target_len = $x_shift + scalar(@b);
            push @c, (0) x ($target_len - scalar(@c)) if scalar(@c) < $target_len;

            for my $k (0 .. $#b) {
                $c[$x_shift + $k] ^= multiply($scale, $b[$k]);
            }
            $x_shift++;
        }
    }

    return (\@c, $big_l);
}

# ============================================================================
# error_locator(\@syndromes)
# ============================================================================
#
# Public wrapper around Berlekamp-Massey.
#
# Returns the error locator polynomial Λ(x) in little-endian form.
# Λ[0] = 1 always.  The roots of Λ are the inverses of the error locators.
#
# @param $synds   Array ref of syndrome values
# @return         Array ref of Λ(x) coefficients (LE, Λ[0]=1)
# ============================================================================
sub error_locator {
    my ($synds) = @_;
    my ($lam) = _berlekamp_massey($synds);
    return $lam;
}

# ============================================================================
# _chien_search(\@lam, $n)
# ============================================================================
#
# Find which byte positions are error locations by trying all n positions.
#
# Position p is an error location iff Λ(X_p⁻¹) = 0, where:
#   X_p⁻¹ = α^{(p + 256 - n) mod 255}
#
# (This formula computes the inverse of α^{n-1-p}, the locator at position p
# in a big-endian codeword.  Since 256 ≡ 1 (mod 255) in the exponent group,
# (n-1-p) and (p+256-n) are additive inverses mod 255.)
#
# @param $lam   Array ref of error locator polynomial (LE)
# @param $n     Codeword length
# @return       Array ref of sorted error positions (0-indexed)
# ============================================================================
sub _chien_search {
    my ($lam, $n) = @_;
    my @positions;
    for my $p (0 .. $n - 1) {
        my $x_inv = power(2, ($p + 256 - $n) % 255);
        push @positions, $p if _poly_eval_le($lam, $x_inv) == 0;
    }
    return \@positions;
}

# ============================================================================
# _forney(\@lam, \@synds, \@positions, $n)
# ============================================================================
#
# Compute error magnitudes at known error positions using the Forney algorithm.
#
# For each error position p:
#   e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
#
# where:
#   Ω(x) = (S(x)·Λ(x)) mod x^{2t}      — error evaluator polynomial
#   S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}  — syndrome poly (LE, S[i] = S_{i+1})
#   Λ'(x) — formal derivative of Λ in characteristic 2
#
# # Formal Derivative in GF(2^8)
#
# In ordinary calculus, (x^k)' = k·x^{k-1}.  In GF(2), even k means the
# coefficient 2 ≡ 0, so even-degree terms vanish in the derivative:
#
#   Λ(x) = Λ₀ + Λ₁x + Λ₂x² + Λ₃x³ + Λ₄x⁴ + …
#   Λ'(x) =       Λ₁ + 0·x  + Λ₃x²  + 0·x³  + Λ₅x⁴ + …
#
# In LE array representation with Λ = [Λ₀, Λ₁, Λ₂, Λ₃, …] (0-indexed):
#   lambda_prime[k] = lambda[k+1]  if (k+1) is odd  (i.e., k is even)
#   lambda_prime[k] = 0            if (k+1) is even (i.e., k is odd)
# So:
#   lambda_prime[0] = lambda[1]    (from Λ₁x¹, coefficient of x⁰ in Λ')
#   lambda_prime[1] = 0            (from Λ₂x², even degree → vanishes)
#   lambda_prime[2] = lambda[3]    (from Λ₃x³, coefficient of x² in Λ')
#   lambda_prime[3] = 0
#   ...
#
# # Error Evaluator Polynomial Ω
#
# Ω(x) = S(x)·Λ(x) truncated to the first n_check = 2t terms.
# S(x) = S₁ + S₂x + … (syndrome poly, LE, with S[0] = S₁).
#
# # Forney Formula (b=1 convention)
#
# magnitude_p = poly_eval_le(omega, X_p⁻¹) / poly_eval_le(lambda_prime, X_p⁻¹)
#
# (No extra X_p multiplier because we use b=1, i.e. roots at α^1..α^{2t}.)
#
# @param $lam        Array ref of Λ(x) (LE)
# @param $synds      Array ref of syndromes (length 2t)
# @param $positions  Array ref of error positions from Chien search
# @param $n          Codeword length
# @return            Array ref of magnitudes (one per position)
# @die               "TooManyErrors: ..." if Λ'(X_p⁻¹) = 0
# ============================================================================
sub _forney {
    my ($lam, $synds, $positions, $n) = @_;
    my $two_t = scalar @$synds;

    # --- Ω(x) = S(x)·Λ(x) mod x^{2t} ------------------------------------
    # Truncate the product to the first 2t terms (indices 0..2t-1).
    my $full_omega = _poly_mul_le($synds, $lam);
    my @omega = @{$full_omega}[0 .. ($two_t - 1 < $#$full_omega ? $two_t - 1 : $#$full_omega)];

    # --- Formal derivative Λ'(x) in characteristic 2 ---------------------
    # Λ'[k] = Λ[k+1] if (k+1) is odd, else 0.
    my $lam_len = scalar @$lam;
    my @lambda_prime = (0) x ($lam_len > 1 ? $lam_len - 1 : 1);
    for my $k (0 .. $#lambda_prime) {
        # k+1 odd ↔ k even
        if (($k + 1) % 2 == 1) {
            $lambda_prime[$k] = $lam->[$k + 1] // 0;
        }
        # k+1 even: coefficient is 0 in char-2 derivative
    }

    my @magnitudes;
    for my $pos (@$positions) {
        my $x_inv = power(2, ($pos + 256 - $n) % 255);

        my $omega_val = _poly_eval_le(\@omega, $x_inv);
        my $lp_val    = _poly_eval_le(\@lambda_prime, $x_inv);

        die "TooManyErrors: formal derivative vanished at position $pos" if $lp_val == 0;

        push @magnitudes, divide($omega_val, $lp_val);
    }

    return \@magnitudes;
}

# ============================================================================
# decode(\@received, $n_check)
# ============================================================================
#
# Decode a received codeword, correcting up to t = n_check/2 byte errors.
#
# The five-step pipeline:
#
#   Step 1 — Syndromes
#     S_j = received(α^j) for j=1..n_check.
#     All zero → no errors → return message portion immediately.
#
#   Step 2 — Berlekamp-Massey
#     Find Λ(x) and the error count L.
#     If L > t → raise TooManyErrors.
#
#   Step 3 — Chien Search
#     Evaluate Λ at all X_p⁻¹; positions where Λ(X_p⁻¹)=0 are error sites.
#     If |positions| != L → something is inconsistent → raise TooManyErrors.
#
#   Step 4 — Forney
#     Compute magnitude e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹) for each position.
#
#   Step 5 — Apply Corrections
#     received[p] ^= e_p for each error position.
#
# Returns the recovered message (first k = n - n_check bytes of corrected).
#
# @param $received   Array ref of received codeword bytes (possibly corrupted)
# @param $n_check    Number of check bytes (positive even integer)
# @return            Array ref of recovered message bytes (length n - n_check)
# @die               "InvalidInput: ..." if n_check bad or received too short
# @die               "TooManyErrors: ..." if more than t errors are present
# ============================================================================
sub decode {
    my ($received, $n_check) = @_;

    # Validate n_check
    if (!defined($n_check) || $n_check <= 0 || ($n_check % 2 != 0)) {
        die "InvalidInput: n_check must be a positive even number, got " . (defined($n_check) ? $n_check : 'undef');
    }
    if (scalar(@$received) < $n_check) {
        die "InvalidInput: received length " . scalar(@$received) . " < n_check $n_check";
    }

    my $t = int($n_check / 2);    # correction capacity
    my $n = scalar @$received;
    my $k = $n - $n_check;        # message length

    # Step 1: Syndromes
    my $synds = syndromes($received, $n_check);

    # If all syndromes are zero, the codeword is valid — return message portion.
    my $all_zero = 1;
    for my $s (@$synds) { $all_zero = 0 if $s != 0 }
    if ($all_zero) {
        return [@{$received}[0 .. $k - 1]];
    }

    # Step 2: Berlekamp-Massey
    my ($lam, $num_errors) = _berlekamp_massey($synds);
    if ($num_errors > $t) {
        die "TooManyErrors: detected $num_errors errors but capacity is $t";
    }

    # Step 3: Chien search
    my $positions = _chien_search($lam, $n);
    if (scalar(@$positions) != $num_errors) {
        die "TooManyErrors: Chien found " . scalar(@$positions) . " roots but BM says $num_errors errors";
    }

    # Step 4: Forney magnitudes
    my $magnitudes = _forney($lam, $synds, $positions, $n);

    # Step 5: Apply corrections (XOR magnitude into corrupted byte)
    my @corrected = @$received;
    for my $i (0 .. $#$positions) {
        $corrected[$positions->[$i]] ^= $magnitudes->[$i];
    }

    return [@corrected[0 .. $k - 1]];
}

1;

__END__

=head1 NAME

CodingAdventures::ReedSolomon - Reed-Solomon error-correcting codes over GF(2^8)

=head1 SYNOPSIS

    use CodingAdventures::ReedSolomon qw(encode decode syndromes build_generator error_locator);

    # Encode 4 bytes with 4 check bytes (can correct 2 errors)
    my $codeword = encode([1, 2, 3, 4], 4);   # returns 8-byte arrayref

    # Introduce 1 error
    my @corrupted = @$codeword;
    $corrupted[0] ^= 0xFF;

    # Decode: recover original message
    my $recovered = decode(\@corrupted, 4);   # returns [1, 2, 3, 4]

    # Syndromes: all zero for a valid codeword
    my $synds = syndromes($codeword, 4);

    # Generator polynomial for 2 check bytes
    my $g = build_generator(2);   # [8, 6, 1] (little-endian)

    # Error locator polynomial from syndromes
    my $lam = error_locator($synds);

=head1 DESCRIPTION

Implements Reed-Solomon error-correcting codes over GF(2^8), part of the
coding-adventures math library (MA02).

Reed-Solomon codes can detect and correct byte-level errors in transmitted
data.  With n_check check bytes appended, the decoder can correct up to
t = n_check/2 corrupted bytes.

Polynomial conventions:
  * Codewords are big-endian (index 0 = highest-degree coefficient).
  * Internal polynomials (generator, lambda, omega) are little-endian
    (index = degree).

=head1 FUNCTIONS

=over 4

=item C<encode(\@message, $n_check)>

Systematically encode a message.  Returns arrayref of length
C<@message + n_check>.  Dies with C<"InvalidInput: ..."> if n_check is 0, odd,
or the total length exceeds 255.

=item C<decode(\@received, $n_check)>

Decode and error-correct a received codeword.  Returns arrayref of the
recovered message.  Dies with C<"InvalidInput: ..."> or C<"TooManyErrors: ...">
on failure.

=item C<syndromes(\@received, $n_check)>

Return an arrayref of n_check syndrome values S_j = received(α^j).
All zero iff the codeword has no errors.

=item C<build_generator($n_check)>

Return a little-endian arrayref of the monic generator polynomial g(x).
Cross-language test vector: C<build_generator(2)> returns C<[8, 6, 1]>.

=item C<error_locator(\@syndromes)>

Run Berlekamp-Massey on a syndrome array and return Λ(x) in LE form
(Λ[0]=1).

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
