# CodingAdventures::Perceptron (Perl)

Single-layer perceptron neural network — the simplest model that learns to
classify linearly separable problems.

## Usage

```perl
use CodingAdventures::Perceptron;

my $p = CodingAdventures::Perceptron->new(
    n_inputs      => 2,
    learning_rate => 0.1,
);

# Train AND gate
my @inputs  = ([0,0],[0,1],[1,0],[1,1]);
my @targets = (0, 0, 0, 1);
$p->train(\@inputs, \@targets, 200);

my ($out) = $p->predict([1, 1]);  # 1
my ($out2) = $p->predict([0, 1]); # 0
```

## API

### `new(%args)` → $perceptron

| Arg             | Default | Description             |
|-----------------|---------|-------------------------|
| `n_inputs`      | required| Number of input features|
| `learning_rate` | 0.1     | Learning step size      |
| `activation_fn` | `\&step`| Activation function ref |
| `weights`       | zeros   | Initial weight vector   |
| `bias`          | 0.0     | Initial bias            |

### `predict($input)` → ($output, $z)

Forward pass. Returns output and pre-activation z.

### `train_step($input, $target)` → ($output, $error)

One Rosenblatt update. Mutates weights in place.

### `train($inputs, $targets, $epochs)` → $self

Full training loop. Returns self for chaining.

## Activation Functions

- `CodingAdventures::Perceptron::step($z)` — binary {0, 1}
- `CodingAdventures::Perceptron::sigmoid($z)` — smooth (0, 1)
- `CodingAdventures::Perceptron::sigmoid_derivative($z)`

## License

MIT
