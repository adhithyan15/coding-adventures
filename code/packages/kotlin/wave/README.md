# wave (Kotlin)

Simple harmonic wave model: y(t) = A sin(2*pi*f*t + phi). PHY01.

Construction rejects non-finite parameters and angular-frequency overflow.
Evaluation rejects non-finite time, preserves exact zero amplitude, reduces
time and phase, and uses the local first-principles `trig` package for a finite,
amplitude-bounded result across the accepted binary64 range.

## Usage

```kotlin
import com.codingadventures.wave.Wave

val w = Wave(1.0, 440.0)
val value = w.evaluate(0.25)
val period = w.period
```
