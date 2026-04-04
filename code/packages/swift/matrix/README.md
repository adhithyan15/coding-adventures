# matrix (Swift)

Immutable matrix mathematics library for machine learning. ML03.

## Features

- Dynamic instantiation: scalar, 1D array, 2D array
- Element-wise add/subtract (matrix and scalar)
- Scalar multiplication, transpose, dot product

## Usage

```swift
import Matrix

let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
let b = Matrix([[5.0, 6.0], [7.0, 8.0]])
let c = a.dot(b)  // [[19, 22], [43, 50]]
```

## Running Tests

```bash
swift test
```
