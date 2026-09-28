### Added — recoverable native-complete startup

Native-complete WinUI shells now render loading and failure surfaces before
loading the Rust application runtime. Failures expose selectable loader detail
and a generated retry action that closes partial runtime state and attempts a
fresh load without restarting the process (#16097).

