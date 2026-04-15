# wave (Kotlin)

Simple harmonic wave model: y(t) = A sin(2*pi*f*t + phi). PHY01.

## Usage

```kotlin
import com.codingadventures.wave.Wave

val w = Wave(1.0, 440.0)
val value = w.evaluate(0.25)
val period = w.period
```
