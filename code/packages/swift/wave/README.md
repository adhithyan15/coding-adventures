# wave (Swift)

Simple harmonic wave model: y(t) = A sin(2*pi*f*t + phi). PHY01.

`init(validatingAmplitude:frequency:phase:)` and `evaluateChecked(at:)` expose
lane-native throwing validation for non-finite inputs and angular-frequency
overflow. The original initializer and `evaluate(at:)` remain source-compatible
precondition wrappers. Evaluation reduces time and phase through the local
first-principles `Trig` dependency and remains amplitude-bounded at binary64
extremes.

## Usage

```swift
import Wave

let w = Wave(amplitude: 1.0, frequency: 440.0)
let value = w.evaluate(at: 0.25)  // displacement at t=0.25s
let period = w.period              // 1/440 seconds
```

## Running Tests

```bash
swift test
```
