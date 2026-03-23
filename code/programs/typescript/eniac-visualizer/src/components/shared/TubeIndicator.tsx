/**
 * TubeIndicator — visual representation of one vacuum tube in a ring.
 *
 * Shows a circle/bulb shape that glows amber when conducting (on)
 * and is dim/gray when off. ENIAC's tubes had a warm orange glow
 * from the heated cathode filament.
 */

export interface TubeIndicatorProps {
  /** Position label (0-9). */
  label: number;
  /** Whether this tube is conducting. */
  conducting: boolean;
  /** Optional: highlight as recently changed. */
  highlight?: boolean;
}

export function TubeIndicator({ label, conducting, highlight }: TubeIndicatorProps) {
  const className = [
    "tube",
    conducting ? "tube--on" : "tube--off",
    highlight ? "tube--highlight" : "",
  ].join(" ");

  return (
    <div
      className={className}
      aria-label={`Tube ${label}: ${conducting ? "conducting" : "off"}`}
    >
      <span className="tube__filament" />
      <span className="tube__label">{label}</span>
    </div>
  );
}
