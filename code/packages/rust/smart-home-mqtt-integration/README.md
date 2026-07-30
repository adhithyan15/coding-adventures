# smart-home-mqtt-integration

Executable MQTT integration for the normalized D23 smart-home runtime.

The package:

- owns a real MQTT 3.1.1 broker connection through `rumqttc`
- subscribes to Home Assistant discovery and dynamically installs lights,
  switches, binary sensors, sensors, and climate entities
- subscribes to discovered state and availability topics
- converts broker publications into normalized state, health, and event audit
- authorizes commands through `SmartHomeRuntime` before publishing MQTT payloads
- records broker publication failures as terminal command results
- stores only an opaque `VaultRef` in the durable bridge model; username and
  password values stay in the live host
- proves the host boundary against a scripted TCP broker that exchanges real
  MQTT CONNECT, SUBSCRIBE, SUBACK, retained discovery, and state packets

Run a bounded broker session:

```bash
SMART_HOME_MQTT_USERNAME=home \
SMART_HOME_MQTT_PASSWORD=secret \
cargo run -p smart-home-mqtt-integration -- \
  mqtt.local 1883 chief-smart-home --events 10
```

Validate:

```bash
bash BUILD
```
