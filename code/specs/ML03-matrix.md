# ML03: Matrix Mathematics Library (Object-Oriented Specification)

## 1. Overview
The `matrix` package is the foundational computational architecture replacing naive loops across the Machine Learning suite. 
Inspired heavily by MATLAB, it establishes a singular, robust `Matrix` Object (Class/Struct) that seamlessly handles mathematical operations across multiple dimensions. Every node in the network—from a single variable weight to an entire dataset of 10,000 houses—is strictly treated as a Matrix.

## 2. The Abstract Object Architecture
To provide a universally elegant API across all 6 host languages (Python, Go, Ruby, TypeScript, Rust, Elixir), the Matrix logic encapsulates a standard private 2D Array mapping while exposing a dynamic initializer.

### 2.1 Dynamic Instantiation
The library dynamically standardizes memory inputs.
- **Scalars**: `Matrix(5.0)` mathematically maps to a `1x1` matrix `[[5.0]]`.
- **1D Arrays**: `Matrix([1.0, 2.0])` mathematically maps to a `1x2` matrix `[[1.0, 2.0]]`.
- **2D Grids**: `Matrix([[1.0], [2.0]])` natively stays a `2x1` matrix.

This structural safety guarantee ensures developers never have to write nested bounds-checking loops inside their neural network layers.

## 3. Core Mathematical Methods
The `Matrix` structure explicitly exposes the following mathematical behaviors attached to the instantiated objects.

### 3.1 Initializers
- `.zeros(rows, cols)` ➔ Returns an $M \times N$ Matrix instance populated rigorously with `0.0`.

### 3.2 Additive Arithmetic (Operator Overloading)
Where the native language allows (Python `__add__`, Ruby `+`), matrix operations are overloaded to allow literal algebraic syntax (`A + B` or `A - 2.0`). Otherwise, standard `.add(B)` and `.subtract(B)` methods apply.
- **Matrix-to-Matrix**: Elements are grouped $A_{ij} \pm B_{ij}$. Throws strict dimension mismatches.
- **Matrix-to-Scalar**: Broadcasts the scalar addition across the entire grid mapping ($A_{ij} \pm \text{scalar}$).

### 3.3 Scaling
- `.scale(scalar)` / `A * scalar` ➔ Applies uniform scalar multiplication across all elements.

### 3.4 Topography Reshaping
- `.transpose()` ➔ Structurally inverts the $M \times N$ layout into a completely swapped $N \times M$ grid. Critical for resolving derivative alignment during Backpropagation!

### 3.5 The Dot Product
- `.dot(B)` ➔ The engine of neural networks. Executing true Matrix Multiplication algorithms ($C_{ij} = \sum A_{ik} B_{kj}$). Matrix `A`'s column count strictly must perfectly balance Matrix `B`'s row count. 

## 4. Cross-Language Parity & Safety
By binding everything to an Object format, we ensure universal crash-safety across all environments. All environments have a standardized unit test file strictly validating dimensional collision logic (matrix dot product bounds) before the training loops are artificially spawned.
