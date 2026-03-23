/**
 * Educational Narrative — collapsible text section for each era.
 *
 * Shows the first paragraph by default and hides the rest behind a
 * "Learn more" button. This progressive disclosure pattern keeps the
 * initial view clean while making deeper content accessible.
 *
 * === Accessibility ===
 *
 * - Uses aria-expanded to communicate toggle state to screen readers
 * - Uses aria-controls to link the button to the collapsible content
 * - The hidden content uses a unique id for the aria relationship
 */

import { useState } from "react";
import { useTranslation } from "@coding-adventures/ui-components";

interface EducationalNarrativeProps {
  /** i18n key prefix (e.g., "era1" for vacuum tube) */
  eraKey: string;
  /** How many paragraphs this era has (e.g., 3 for p1, p2, p3) */
  paragraphCount: number;
}

export function EducationalNarrative({
  eraKey,
  paragraphCount,
}: EducationalNarrativeProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  // Generate a stable id for the collapsible region
  const regionId = `${eraKey}-narrative-details`;

  // Build the list of paragraph keys
  const paragraphs = Array.from({ length: paragraphCount }, (_, i) =>
    t(`${eraKey}.narrative.p${i + 1}`),
  );

  return (
    <div className="narrative">
      {/* Always show the first paragraph */}
      <p className="narrative__lead">{paragraphs[0]}</p>

      {/* Additional paragraphs, hidden by default */}
      {paragraphCount > 1 && (
        <>
          <button
            className="narrative__toggle"
            onClick={() => setExpanded(!expanded)}
            aria-expanded={expanded}
            aria-controls={regionId}
          >
            {expanded
              ? t("shared.narrative.showLess")
              : t("shared.narrative.showMore")}
          </button>

          {expanded && (
            <div id={regionId} className="narrative__details">
              {paragraphs.slice(1).map((text, i) => (
                <p key={i} className="narrative__paragraph">
                  {text}
                </p>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
