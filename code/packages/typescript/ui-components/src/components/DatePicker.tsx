/**
 * DatePicker — accessible date input with clear button.
 *
 * A thin wrapper around `<input type="date">` that integrates with the
 * shared dark theme and provides a consistent API across the app.
 *
 * The value is always a YYYY-MM-DD string (the native format of the HTML
 * date input) or an empty string when no date is selected. This avoids
 * Date object parsing and timezone issues — a date string means the same
 * thing everywhere.
 *
 * === Why wrap <input type="date">? ===
 *
 * The native date input varies wildly across browsers (Chrome shows a
 * calendar popup, Firefox shows spinners, Safari barely styles it). This
 * wrapper provides:
 *
 *   1. Consistent dark-theme styling via CSS custom properties
 *   2. A clear button (✕) to reset the date to "no date"
 *   3. Accessible labelling via aria-label
 *   4. A clean value/onChange API that matches React conventions
 *
 * @example
 * ```tsx
 * <DatePicker
 *   value={dueDate}
 *   onChange={setDueDate}
 *   label="Due date"
 *   id="todo-due"
 * />
 * ```
 */

export interface DatePickerProps {
  /** Current value as YYYY-MM-DD string, or empty string for no date. */
  value: string;
  /** Called with the new YYYY-MM-DD string on change. Empty string = cleared. */
  onChange: (value: string) => void;
  /** Accessible label for the input. */
  label: string;
  /** HTML id for the input element. */
  id?: string;
  /** Additional CSS class on the outer wrapper. */
  className?: string;
}

export function DatePicker({
  value,
  onChange,
  label,
  id,
  className,
}: DatePickerProps) {
  return (
    <div className={`date-picker${className ? ` ${className}` : ""}`}>
      <input
        type="date"
        id={id}
        className="date-picker__input"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        aria-label={label}
      />
      {value && (
        <button
          className="date-picker__clear"
          onClick={() => onChange("")}
          type="button"
          aria-label={`Clear ${label}`}
          title="Clear date"
        >
          ✕
        </button>
      )}
    </div>
  );
}
