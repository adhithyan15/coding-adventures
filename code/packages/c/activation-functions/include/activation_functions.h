/*
 * activation_functions.h — neural-network activation functions and their
 * derivatives, in pure ISO C17. A faithful port of the Rust
 * `activation-functions` crate.
 * ===========================================================================
 *
 * The activation function is the nonlinearity a neuron applies to its weighted
 * input; its derivative is what backpropagation multiplies through. This library
 * provides the classic set, each as a pair (function, derivative):
 *
 *   linear      f(x) = x                        f'(x) = 1
 *   sigmoid     f(x) = 1 / (1 + e^-x)           f'(x) = f(x)(1 - f(x))
 *   relu        f(x) = max(0, x)                f'(x) = x > 0 ? 1 : 0
 *   leaky_relu  f(x) = x>0 ? x : 0.01x          f'(x) = x > 0 ? 1 : 0.01
 *   tanh        f(x) = tanh(x)                  f'(x) = 1 - tanh(x)^2
 *   softplus    f(x) = ln(1 + e^x)              f'(x) = sigmoid(x)
 *
 * NO libm. The transcendental helpers (e^x, tanh, ln(1+x)) are computed from
 * scratch — range-reduced Taylor/Newton series — so the package links with no
 * math library. Results match the C standard library / the Rust std methods to
 * within about 1e-12 (the tolerance the Rust crate's own tests use).
 *
 * All functions are total (defined for every finite input): sigmoid saturates
 * to 0/1 outside +/-709 and softplus/tanh use numerically stable forms, so
 * there is nothing to fail — no status codes, exactly like the Rust crate.
 *
 * PORTABILITY. Pure ISO C17, no <math.h>, no compiler extensions. Builds clean
 * under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_ACTIVATION_FUNCTIONS_H
#define CA_ACTIVATION_FUNCTIONS_H

#ifdef __cplusplus
extern "C" {
#endif

/* The negative-side slope of leaky ReLU (matches the Rust constant). */
#define AF_LEAKY_RELU_SLOPE 0.01

double af_linear(double x);
double af_linear_derivative(double x);

double af_sigmoid(double x);
double af_sigmoid_derivative(double x);

double af_relu(double x);
double af_relu_derivative(double x);

double af_leaky_relu(double x);
double af_leaky_relu_derivative(double x);

double af_tanh(double x);
double af_tanh_derivative(double x);

double af_softplus(double x);
double af_softplus_derivative(double x);

#ifdef __cplusplus
}
#endif

#endif /* CA_ACTIVATION_FUNCTIONS_H */
