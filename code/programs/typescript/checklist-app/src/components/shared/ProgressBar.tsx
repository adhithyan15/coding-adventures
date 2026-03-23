/**
 * ProgressBar — displays checked / total as a filled bar.
 *
 * The fill turns green when all items are complete (completionRate = 100).
 * A text label below shows "N of M complete" for screen readers and users
 * who prefer numbers over visual metaphors.
 */

import { useTranslation } from "@coding-adventures/ui-components";

interface ProgressBarProps {
  checked: number;
  total: number;
}

export function ProgressBar({ checked, total }: ProgressBarProps) {
  const { t } = useTranslation();
  const pct = total === 0 ? 0 : Math.min(100, (checked / total) * 100);
  const isComplete = total > 0 && checked >= total;

  return (
    <div>
      <div
        className="progress-bar"
        role="progressbar"
        aria-valuenow={checked}
        aria-valuemin={0}
        aria-valuemax={total}
        aria-label={t("runner.progress")
          .replace("{checked}", String(checked))
          .replace("{total}", String(total))}
      >
        <div
          className={`progress-bar__fill${isComplete ? " progress-bar__fill--complete" : ""}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <p className="progress-bar__label">
        {t("runner.progress")
          .replace("{checked}", String(checked))
          .replace("{total}", String(total))}
      </p>
    </div>
  );
}
