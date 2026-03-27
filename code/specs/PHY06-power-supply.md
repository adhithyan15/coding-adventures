# PHY06: Power Supply

## Overview

The `power-supply` package starts as a deliberately simple abstraction for
ideal sources.

At first, this is just:

- ideal DC supply
- ideal sinusoidal source

Later, it can grow into:

- source impedance
- current limiting
- ripple
- regulator behavior
- switching supply models

This gives the electronics track a place for "where energy comes from" that is
more explicit than hard-coding voltage numbers into every demo.
