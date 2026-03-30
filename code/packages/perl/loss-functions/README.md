# CodingAdventures::LossFunctions (Perl)

Pure-Perl implementation of the core machine-learning loss functions and their analytical derivatives.

## What it does

Eight exported functions across four loss families:

| Loss | Forward | Derivative |
|------|---------|-----------|
| Mean Squared Error | `mse($y_true, $y_pred)` | `mse_derivative($y_true, $y_pred)` |
| Mean Absolute Error | `mae($y_true, $y_pred)` | `mae_derivative($y_true, $y_pred)` |
| Binary Cross-Entropy | `bce($y_true, $y_pred)` | `bce_derivative($y_true, $y_pred)` |
| Categorical Cross-Entropy | `cce($y_true, $y_pred)` | `cce_derivative($y_true, $y_pred)` |

All functions accept array references and return `($value, undef)` on success or `(undef, "error message")` on failure.

## How it fits in the stack

This is the Perl mirror of `code/packages/lua/loss_functions` and `code/packages/elixir/loss_functions`. It is a pure-math leaf package — it depends on nothing outside Perl's built-in `abs` and `log`.

## Usage

```perl
use CodingAdventures::LossFunctions qw(
    mse mae bce cce
    mse_derivative mae_derivative bce_derivative cce_derivative
);

my @y_true = (0.0, 1.0, 0.0);
my @y_pred = (0.1, 0.9, 0.2);

# Forward pass.
my ($loss, $err) = bce(\@y_true, \@y_pred);
die $err if $err;
printf "BCE loss: %.4f\n", $loss;

# Backward pass.
my ($grad, $err2) = bce_derivative(\@y_true, \@y_pred);
die $err2 if $err2;
for my $i (0 .. $#$grad) {
    printf "  grad[%d] = %.6f\n", $i, $grad->[$i];
}
```

## Formulas

```
MSE  = (1/n) * sum_i (y_true[i] - y_pred[i])^2
MAE  = (1/n) * sum_i |y_true[i] - y_pred[i]|
BCE  = -(1/n) * sum_i [y * log(p) + (1-y) * log(1-p)]
CCE  = -(1/n) * sum_i [y * log(p)]

  where p = clamp(y_pred[i], epsilon, 1-epsilon),  epsilon = 1e-7

MSED[i]  = (2/n) * (y_pred[i] - y_true[i])
MAED[i]  = +1/n  if y_pred > y_true
           -1/n  if y_pred < y_true
            0    if equal
BCED[i]  = (1/n) * (p - y_true[i]) / (p * (1-p))
CCED[i]  = -(1/n) * (y_true[i] / p)
```

## Running the tests

```
cpanm --installdeps .
prove -l -v t/
```
