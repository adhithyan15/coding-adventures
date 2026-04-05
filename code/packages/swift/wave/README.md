# wave (Swift)

Simple harmonic wave model: y(t) = A sin(2*pi*f*t + phi). PHY01.

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
