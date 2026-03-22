/**
 * Era Header — displays the year badge, inventors, title and subtitle
 * for each transistor era.
 *
 * This component creates a consistent visual hierarchy across all four tabs:
 *   - A prominent year badge (1906, 1947, 1959, 1963)
 *   - The inventor(s) who made the breakthrough
 *   - The era title and a brief subtitle
 *
 * All text comes from i18n keys — no hardcoded strings.
 */

import { useTranslation } from "@coding-adventures/ui-components";

interface EraHeaderProps {
  /** i18n key prefix (e.g., "era1" for vacuum tube) */
  eraKey: string;
}

export function EraHeader({ eraKey }: EraHeaderProps) {
  const { t } = useTranslation();

  return (
    <div className="era-header">
      <span className="era-header__year">{t(`${eraKey}.year`)}</span>
      <div className="era-header__text">
        <h2 className="era-header__title">{t(`${eraKey}.title`)}</h2>
        <p className="era-header__subtitle">{t(`${eraKey}.subtitle`)}</p>
        <p className="era-header__inventors">{t(`${eraKey}.inventors`)}</p>
      </div>
    </div>
  );
}
