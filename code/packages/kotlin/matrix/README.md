# matrix (Kotlin)

Immutable matrix mathematics library for machine learning with operator overloading. ML03.

## Usage

```kotlin
import com.codingadventures.matrix.Matrix

val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
val b = Matrix.of(arrayOf(doubleArrayOf(5.0, 6.0), doubleArrayOf(7.0, 8.0)))
val c = a.dot(b)     // [[19, 22], [43, 50]]
val d = a + b        // operator overloading
val e = a * 2.0      // scale
```

## Running Tests

```bash
gradle test
```
