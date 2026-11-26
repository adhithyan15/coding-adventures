/// Square-wave clock generator with edges, dividers, and phase clocks.
///
/// The Dart lane of `clock`: [Clock] toggles 0 → 1 → 0, emitting a
/// [ClockEdge] per half-cycle; [ClockDivider] slows a source down by an
/// integer factor; [MultiPhaseClock] rotates one active phase per source
/// rising edge.
library coding_adventures_clock;

export 'src/clock.dart';
