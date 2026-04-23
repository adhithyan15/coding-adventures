/**
 * # FrequencyChart -- Letter Frequency Comparison
 *
 * This component renders a pure-CSS bar chart comparing the actual letter
 * frequencies in the ciphertext against the expected frequencies of English
 * text. This visualization is central to understanding why the Caesar cipher
 * is weak: the frequency distribution of the ciphertext is just a shifted
 * version of the plaintext distribution.
 *
 * ## How Frequency Analysis Breaks Caesar
 *
 * In English, the letter 'E' appears about 12.7% of the time. If you encrypt
 * with shift 3, then 'H' (which is E shifted by 3) will appear about 12.7%
 * of the time in the ciphertext. By comparing the ciphertext's frequency
 * distribution against the known English distribution, you can identify the
 * shift.
 *
 * ## Visual Layout
 *
 * Each letter gets a column with two bars side by side:
 *   - Left (gray): the expected English frequency
 *   - Right (accent): the actual frequency in the ciphertext
 *
 * When the bars roughly align (after mentally shifting the whole chart), you
 * have likely found the correct shift.
 *
 * @module FrequencyChart
 */

interface FrequencyChartProps {
  /** Actual letter frequencies in the ciphertext (lowercase letter -> proportion). */
  frequencies: Record<string, number>;
  /** Expected English letter frequencies (lowercase letter -> proportion). */
  expectedFrequencies: Record<string, number>;
}

/**
 * Renders a pure-CSS bar chart comparing observed vs expected letter frequencies.
 */
export function FrequencyChart({ frequencies, expectedFrequencies }: FrequencyChartProps) {
  const alphabet = "abcdefghijklmnopqrstuvwxyz".split("");

  // Find the maximum frequency across both distributions so we can scale
  // the bars to fill the available height. Using the same scale for both
  // makes visual comparison meaningful.
  const maxFreq = Math.max(
    ...alphabet.map((letter) => frequencies[letter] ?? 0),
    ...alphabet.map((letter) => expectedFrequencies[letter] ?? 0),
    0.001, // Prevent division by zero when both distributions are empty.
  );

  return (
    <div>
      <div className="freq-chart" role="img" aria-label="letter frequency chart">
        {alphabet.map((letter) => {
          const expected = expectedFrequencies[letter] ?? 0;
          const actual = frequencies[letter] ?? 0;

          // Scale heights as a percentage of the chart height (156px usable).
          const expectedHeight = (expected / maxFreq) * 156;
          const actualHeight = (actual / maxFreq) * 156;

          return (
            <div key={letter} className="freq-col">
              <div className="freq-bars">
                <div
                  className="freq-bar expected"
                  style={{ height: `${expectedHeight}px` }}
                  title={`${letter.toUpperCase()} expected: ${(expected * 100).toFixed(1)}%`}
                />
                <div
                  className="freq-bar actual"
                  style={{ height: `${actualHeight}px` }}
                  title={`${letter.toUpperCase()} actual: ${(actual * 100).toFixed(1)}%`}
                />
              </div>
              <span className="freq-label">{letter}</span>
            </div>
          );
        })}
      </div>
      <div className="freq-legend">
        <div className="freq-legend-item">
          <div className="freq-legend-swatch expected" />
          <span>Expected (English)</span>
        </div>
        <div className="freq-legend-item">
          <div className="freq-legend-swatch actual" />
          <span>Actual (ciphertext)</span>
        </div>
      </div>
    </div>
  );
}
