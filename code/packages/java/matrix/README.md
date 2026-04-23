# matrix (Java)

Immutable matrix mathematics library for machine learning. ML03.

## Usage

```java
import com.codingadventures.matrix.Matrix;

Matrix a = new Matrix(new double[][]{{1, 2}, {3, 4}});
Matrix b = new Matrix(new double[][]{{5, 6}, {7, 8}});
Matrix c = a.dot(b);  // [[19, 22], [43, 50]]
```

## Running Tests

```bash
gradle test
```
