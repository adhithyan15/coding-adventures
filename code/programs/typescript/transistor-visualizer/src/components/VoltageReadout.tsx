/**
 * Voltage Readout — displays electrical measurements with live updates.
 *
 * Shows key electrical values (voltage, current, region) in a clean
 * readout panel. Uses aria-live="polite" so screen readers announce
 * changes without interrupting the user.
 *
 * === Why aria-live="polite"? ===
 *
 * When the user adjusts a slider, the readout values change continuously.
 * "polite" tells the screen reader to announce the new values at the next
 * natural pause, rather than interrupting whatever it's currently saying.
 * This prevents a flood of announcements while the slider is being dragged.
 */

interface ReadoutItem {
  /** Display label for this measurement. */
  label: string;
  /** The value to display (already formatted as a string). */
  value: string;
}

interface VoltageReadoutProps {
  /** List of measurements to display. */
  items: ReadoutItem[];
  /** Optional CSS class for the container. */
  className?: string;
}

export function VoltageReadout({
  items,
  className = "voltage-readout",
}: VoltageReadoutProps) {
  return (
    <div className={className} aria-live="polite" role="status">
      {items.map((item, i) => (
        <div key={i} className="voltage-readout__item">
          <span className="voltage-readout__label">{item.label}</span>
          <span className="voltage-readout__value">{item.value}</span>
        </div>
      ))}
    </div>
  );
}
