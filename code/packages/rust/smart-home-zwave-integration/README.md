# smart-home-zwave-integration

Executable Z-Wave adapter between the repository Serial API and command-class
packages and the normalized D23 smart-home runtime.

The adapter:

- installs a Serial API controller and interviewed nodes as normalized bridges,
  devices, entities, capabilities, and protocol identifiers
- parses Application Command Handler reports into normalized state and runtime
  events
- dynamically adds report-specific sensor capabilities discovered at runtime
- authorizes outbound commands through `SmartHomeRuntime` before producing
  reliable Serial API SendData frames
- tracks SendData response, callback, failure, and timeout state

Serial port ownership, inclusion, and S2 security remain host concerns. The
package owns the complete typed boundary immediately above serial byte I/O.

```bash
bash BUILD
```
