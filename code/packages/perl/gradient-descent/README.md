# CodingAdventures::GradientDescent (Perl)

Gradient descent weight optimiser — the iterative algorithm that trains
virtually every machine-learning model.

## What Is Gradient Descent?

Given a loss function L(w) that measures prediction error, gradient descent
finds weights w that minimise L by repeatedly stepping downhill:

```
w_new[i] = w[i] - learning_rate * gradient[i]
```

## Usage

```perl
use CodingAdventures::GradientDescent;

my $gd = CodingAdventures::GradientDescent->new(
    learning_rate  => 0.1,
    max_iterations => 1000,
    tolerance      => 1e-6,
);

# Loss function: MSE for y = w*x
my $loss = sub {
    my ($w, $inp, $tgt) = @_;
    my $sum = 0;
    for my $i (0..$#$inp) {
        my $pred = $w->[0] * $inp->[$i][0];
        $sum += ($pred - $tgt->[$i])**2;
    }
    return $sum / @$inp;
};

my ($trained, $err) = $gd->train([0.0], \@inputs, \@targets, $loss);
die $err if $err;
# $trained->[0] ≈ 2.0 for y = 2x
```

## API

### `new(%args)` → $gd

| Argument        | Default | Description                      |
|-----------------|---------|----------------------------------|
| `learning_rate` | 0.01    | Step size for each update        |
| `max_iterations`| 1000    | Maximum gradient steps           |
| `tolerance`     | 1e-6    | Stop when loss change < this     |

### `step($weights, $gradient)` → ($new_weights, $err)

Apply one update: `w_new[i] = w[i] - lr * grad[i]`.

### `compute_loss($w, $inputs, $targets, $loss_fn)` → scalar

Evaluate the loss function.

### `numerical_gradient($w, $inputs, $targets, $loss_fn, $epsilon)` → arrayref

Central finite-difference approximation. Default ε = 1e-5.

### `train($w, $inputs, $targets, $loss_fn, $grad_fn)` → ($trained, $err)

Full training loop. If `$grad_fn` is undef, numerical gradients are used.

## License

MIT
