# Discrete Waveform

Provides a native implementation bridging explicit continuous-time variables to numerical computing domains.

## Overview

A discrete waveform represents a time series layout gathered explicitly from evaluating an `AnalogWaveform` periodically along a fixed sampling array.

- Configures sampling rates and derives respective frequency lengths automatically.
- Facilitates `zeroOrderHold` implementations ensuring intermediate gaps reflect the last sampled digital plateau, aligning directly with core ADC/DAC physical mappings.

## Dependencies
- Extends the `analog-waveform` library protocol definitions.
