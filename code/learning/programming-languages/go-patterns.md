# Go Patterns Used in This Project

This document explains the Go language features and patterns used
throughout the coding-adventures Go packages.

## Package Organization

Go uses packages as the fundamental unit of code organization.
Each directory is a package, and the package name matches the directory:

```
code/packages/go/arithmetic/
├── adder.go        package arithmetic
├── adder_test.go   package arithmetic  (test file)
├── alu.go          package arithmetic
└── alu_test.go     package arithmetic
```

All `.go` files in a directory must have the same `package` declaration.
Test files use the same package name and can access unexported functions.

**Where used:** Every Go package

## Interfaces — Implicit Implementation

Go interfaces are satisfied implicitly — you don't declare that a type
"implements" an interface. If it has the right methods, it satisfies it:

```go
type BranchPredictor interface {
    Predict(address uint32) bool
    Update(address uint32, taken bool)
}

// TwoBitPredictor satisfies BranchPredictor without declaring it
type TwoBitPredictor struct {
    counters map[uint32]uint8
}

func (p *TwoBitPredictor) Predict(address uint32) bool {
    return p.counters[address] >= 2
}

func (p *TwoBitPredictor) Update(address uint32, taken bool) {
    // ...
}
```

This is structural typing (like Python's Protocol) — the compiler checks
at the call site, not at the definition site.

**Where used:** `code/packages/go/branch-predictor/`, `code/packages/go/cache/`

## Goroutines and Channels — Lightweight Concurrency

Go's killer feature for the build tool. Goroutines are lightweight
threads (thousands can run concurrently):

```go
// Launch a goroutine for each package at this level
var wg sync.WaitGroup
semaphore := make(chan struct{}, maxJobs)

for _, pkg := range level {
    wg.Add(1)
    semaphore <- struct{}{}  // acquire slot (blocks if full)

    go func(p Package) {
        defer wg.Done()
        defer func() { <-semaphore }()  // release slot
        buildPackage(p)
    }(pkg)
}

wg.Wait()  // wait for all goroutines to finish
```

Key patterns:
- `go func()` launches a goroutine
- `sync.WaitGroup` waits for all goroutines to finish
- Buffered channel as a semaphore limits concurrency
- `defer` ensures cleanup runs even if the function panics

**Where used:** `code/programs/go/build-tool/internal/executor/`

## Error Handling — Explicit Returns

Go doesn't have exceptions. Functions return errors as values:

```go
func ReadFile(path string) ([]byte, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return nil, fmt.Errorf("reading %s: %w", path, err)
    }
    return data, nil
}
```

The `if err != nil` pattern is ubiquitous. `%w` wraps the error for
later unwrapping. The caller decides how to handle it:

```go
data, err := ReadFile("BUILD")
if err != nil {
    log.Fatal(err)  // or return err, or ignore
}
```

**Where used:** Every Go package

## Table-Driven Tests

Go's conventional test pattern — a slice of test cases:

```go
func TestHalfAdder(t *testing.T) {
    tests := []struct {
        a, b       int
        wantSum    int
        wantCarry  int
    }{
        {0, 0, 0, 0},
        {0, 1, 1, 0},
        {1, 0, 1, 0},
        {1, 1, 0, 1},
    }

    for _, tt := range tests {
        sum, carry := HalfAdder(tt.a, tt.b)
        if sum != tt.wantSum || carry != tt.wantCarry {
            t.Errorf("HalfAdder(%d, %d) = (%d, %d), want (%d, %d)",
                tt.a, tt.b, sum, carry, tt.wantSum, tt.wantCarry)
        }
    }
}
```

Each entry in the `tests` slice is a test case. The loop runs all of
them, and `t.Errorf` reports failures without stopping (unlike `t.Fatalf`).

**Where used:** Every Go test file

## Slices — Dynamic Arrays

Go's primary collection type. Slices are backed by arrays but
dynamically sized:

```go
// Create a slice of bits
bits := []int{1, 0, 1, 1}

// Append (may allocate new backing array)
bits = append(bits, 0)

// Slice a slice (shared backing array — no copy)
firstTwo := bits[:2]

// Length and capacity
len(bits)  // 5
cap(bits)  // 8 (Go over-allocates for growth)
```

**Where used:** Everywhere — bits are represented as `[]int` slices

## Maps — Hash Tables

Go's built-in hash map:

```go
// Package name → Package struct
packages := make(map[string]Package)

// Insert
packages["python/logic-gates"] = pkg

// Lookup (two-value form checks existence)
pkg, ok := packages["python/logic-gates"]
if !ok {
    // not found
}

// Iterate (order is NOT guaranteed)
for name, pkg := range packages {
    fmt.Println(name, pkg.Language)
}
```

**Where used:** `code/programs/go/build-tool/` — dependency graphs, caches

## Structs — No Classes, No Inheritance

Go has no classes. Structs hold data, methods are defined on them:

```go
type ALU struct {
    BitWidth int
}

func NewALU(width int) *ALU {
    return &ALU{BitWidth: width}
}

func (alu *ALU) Execute(op ALUOp, a, b []int) ALUResult {
    // ...
}
```

Instead of inheritance, Go uses composition (embedding):

```go
type CPU struct {
    ALU          // embedded — CPU "has an" ALU
    Registers [16]uint32
    PC        uint32
}

// CPU can directly call ALU methods
cpu.Execute(ADD, a, b)
```

**Where used:** `code/packages/go/cpu-simulator/`, `code/packages/go/arithmetic/`

## `go.mod` — Dependency Management

Every Go package has a `go.mod` file:

```
module github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic

go 1.26

require (
    github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0
)

replace (
    github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
)
```

The `replace` directive redirects to the local path during development.
For published modules, users would use the full module path.

**Where used:** Every Go package

## `init()` Functions — Package Initialization

Go runs `init()` functions automatically when a package is imported:

```go
func init() {
    // Register default prediction algorithms
    defaultPredictors["1-bit"] = NewOneBitPredictor()
    defaultPredictors["2-bit"] = NewTwoBitPredictor()
}
```

`init()` runs before `main()`. Multiple `init()` functions per file are
allowed. Use sparingly — they make testing harder.

## `defer` — Guaranteed Cleanup

`defer` schedules a function to run when the enclosing function returns:

```go
func processFile(path string) error {
    f, err := os.Open(path)
    if err != nil {
        return err
    }
    defer f.Close()  // runs when processFile returns, even on error

    // ... use f ...
    return nil
}
```

Multiple defers execute in LIFO order (last deferred, first executed).

**Where used:** `code/programs/go/build-tool/` — file handles, mutex unlocking
