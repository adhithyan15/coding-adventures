# Native and Rust-core coverage, by hand

The word **coverage** can hide an important engineering choice. Two languages
may both produce the right answer while getting there in very different ways.
One may calculate the answer itself. Another may ask a shared Rust library to
calculate it.

NN36 keeps those paths separate.

## First, keep the mathematics fixed

Every lane receives the same tiny neuron:

```text
inputs  = [2, -1]
weights = [0.5, -0.25]
bias    = 0.1
```

Multiply matching positions:

```text
contribution 1 =  2 *  0.5  = 1.0
contribution 2 = -1 * -0.25 = 0.25
```

Add both contributions and the bias:

```text
prediction = 1.0 + 0.25 + 0.1 = 1.35
```

That answer does not reveal who performed the arithmetic. Coverage metadata
answers the separate ownership question.

## What does native mean?

A **native implementation** owns the calculation inside its own language
lane. The Go program parses the fixture and multiplies in Go. Ruby does the
same in Ruby. The Rust command-line program does it in Rust.

NN36 has three such lanes:

```text
Go native   = 1 lane
Ruby native = 1 lane
Rust native = 1 lane
---------------------
native total = 3 lanes
```

The word native describes ownership here, not operating-system machine code.
Ruby is interpreted, but its lane still owns the two multiplications and the
sum instead of delegating them to the shared Rust core.

## What does Rust-core binding mean?

A **binding** is a small adapter that lets one language call an interface
owned by another implementation. The Python lane allocates input and output
buffers with `ctypes`, calls the versioned NN35 C ABI, and reads the result.
Rust performs the weighted sum.

That gives one binding lane:

```text
Python caller --ctypes--> C ABI v1 --calls--> Rust arithmetic

Rust-core binding total = 1 lane
```

Python still owns its objects and buffers. Rust owns the neural arithmetic.
The label records exactly that split.

## Count the two categories

Now the coverage arithmetic is small enough to audit on paper:

```text
native implementations = 3
Rust-core bindings      = 1
total verified lanes    = 3 + 1 = 4

native share            = 3 / 4
binding share           = 1 / 4
```

These fractions describe the four current implementation paths. A larger
number is not automatically better. Four copied bugs would still be four
lanes. One carefully tested shared core may be the safer design for some
products.

## Registered is not the same as verified

The browser can read the catalog, check its closed shape, and recompute 1.35.
It cannot run Go, Ruby, Python, or a native shared library.

The executable coverage gate does the stronger work:

```bash
python code/scripts/validate_neural_learning_implementation_coverage.py
```

It runs the three native consumers, distrusts and checks their receipts, builds
the Rust shared library, crosses the C ABI through Python, and only then reports
`3 native + 1 binding = 4 verified lanes`.

## Explore the ownership map

Open **Implementation Coverage** in the ML Learning Visualizer. Select a lane
and follow four facts:

1. which language starts the call;
2. whether the lane is native or a Rust-core binding;
3. which language owns the arithmetic;
4. which executable validator earns the coverage claim.

The prediction stays at 1.35 while ownership changes. That is the central idea:
portable mathematics and implementation architecture are related, but they are
not the same fact.

## Growing the matrix honestly

A future language can add a native lane, a Rust-core binding, both, or neither.
Each new checkmark needs a runnable path against the same language-neutral
fixture. Planned support remains missing until that gate exists. This keeps the
coverage table useful as an engineering inventory instead of a wish list.
