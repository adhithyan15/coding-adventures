# hdd-seek-timing

`Plan` maps a relative logical block through `hdd-geometry`, then computes
seek, rotation, and one transfer tick without reading or changing media. It
preserves locator errors before validating the head cylinder and portable
clock. Every refusal returns a zero result.

The modular arithmetic reduces tick and seek before combining phases and uses
subtraction around wrap boundaries. It checks seek distance against remaining
clock budget before multiplication, then reserves room for rotation and the
transfer tick before summing. The package allocates no memory and does not
change the existing fixed DB02 simulator API.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
