# Changelog

## 0.1.0

- Add the exact 528,531-DFF Intel 8085A architectural topology.
- Compose the independent Intel 8080 gate datapath with gated RIM/SIM,
  interrupt arbitration, stack/vector entry, serial I/O, and delayed EI.
- Add exhaustive opcode, interrupt, lifecycle, trace, and full-state
  differential coverage against the checked functional model.
- Make both loaded-run entry points fully transactional and add differential
  boundary witnesses for wrapped memory, stack, call, return, and restart
  cycles at the 16-bit bus edge.
