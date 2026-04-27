# wave (Java)

Simple harmonic wave model: y(t) = A sin(2*pi*f*t + phi). PHY01.

## Usage

```java
import com.codingadventures.wave.Wave;

Wave w = new Wave(1.0, 440.0);
double value = w.evaluate(0.25);
double period = w.period();
```
