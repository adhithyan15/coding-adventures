/**
 * Accessible slider control for adjusting voltage, current, or other values.
 *
 * Renders a labeled range input with:
 *   - Visible label linked via htmlFor
 *   - Current value display with optional unit
 *   - Full ARIA support (aria-valuemin, aria-valuemax, aria-valuenow)
 *   - Keyboard accessible (native range input)
 *
 * @example
 * ```tsx
 * <SliderControl
 *   label="Gate Voltage"
 *   value={vgs}
 *   min={0}
 *   max={3.3}
 *   step={0.1}
 *   onChange={setVgs}
 *   unit="V"
 * />
 * ```
 */

import { useId } from "react";

export interface SliderControlProps {
  /** Label displayed above the slider. */
  label: string;
  /** Current value. */
  value: number;
  /** Minimum value. */
  min: number;
  /** Maximum value. */
  max: number;
  /** Step increment. */
  step: number;
  /** Called when the user changes the slider value. */
  onChange: (value: number) => void;
  /** Optional unit to display after the value (e.g., "V", "mA"). */
  unit?: string;
  /** Optional formatter for the displayed value. Defaults to toFixed(2). */
  formatValue?: (value: number) => string;
  /** Optional CSS class for the container. */
  className?: string;
}

export function SliderControl({
  label,
  value,
  min,
  max,
  step,
  onChange,
  unit = "",
  formatValue,
  className = "slider-control",
}: SliderControlProps) {
  const id = useId();
  const displayValue = formatValue ? formatValue(value) : value.toFixed(2);

  return (
    <div className={className}>
      <label htmlFor={id} className="slider-control__label">
        {label}
      </label>
      <div className="slider-control__row">
        <input
          id={id}
          type="range"
          className="slider-control__input"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          aria-valuemin={min}
          aria-valuemax={max}
          aria-valuenow={value}
        />
        <output className="slider-control__value" htmlFor={id}>
          {displayValue}
          {unit && <span className="slider-control__unit">{unit}</span>}
        </output>
      </div>
    </div>
  );
}
