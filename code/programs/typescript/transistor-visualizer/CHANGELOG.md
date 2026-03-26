# Changelog

## v0.1.0 — Initial Release

### Features

- **Vacuum Tube era**: Interactive triode cross-section with Child-Langmuir simulation, grid voltage slider, particle system for electron flow, plate current readout
- **BJT era**: NPN transistor cross-section with N-P-N layer visualization, depletion regions, base-emitter voltage slider, region/current/beta readouts
- **MOSFET era**: NMOS transistor cross-section with P-type substrate, N-type wells, SiO2 oxide layer, gate metal, dopant atoms, inversion channel formation, gate voltage slider
- **CMOS era**: Complementary inverter diagram with PMOS/NMOS pair, digital toggle, voltage transfer characteristic chart, Moore's Law scaling timeline
- **Particle system**: Pure TypeScript engine with Brownian jitter, fade-in/out, configurable spawn rate, respects prefers-reduced-motion
- **Accessibility**: All SVGs have dynamic aria-labels, readouts use aria-live="polite", full keyboard navigation via TabList, focus indicators, reduced motion support
- **i18n**: All visible text externalized to en.json, ready for additional languages
- **Shared components**: EraHeader, EducationalNarrative (collapsible), ParticleLayer, VoltageReadout
- **Responsive design**: Two-column layout collapses to single column on mobile
- **Tests**: Unit tests for particle system engine and vacuum tube model
