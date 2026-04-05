/**
 * # Median
 *
 * The median is the "middle" value when data is sorted. Unlike the mean,
 * it is robust against outliers. A billionaire walking into a room doesn't
 * change the median income much, but it skews the mean enormously.
 *
 * ## Algorithm
 *
 * 1. Sort the values in ascending order.
 * 2. If the count is odd, return the middle element.
 * 3. If the count is even, return the average of the two middle elements.
 *
 * ## Examples
 *
 *     median([1, 3, 5])       -> 3     (odd count, middle value)
 *     median([1, 3, 5, 7])    -> 4     (even count, average of 3 and 5)
 *     median([2, 4, 4, 4, 5, 5, 7, 9]) -> 4.5  (average of 4 and 5)
 *
 * @param values - Array of numbers
 * @returns The median value
 * @throws Error if the array is empty
 */
export function median(values: number[]): number {
  if (values.length === 0) {
    throw new Error("Cannot compute median of an empty array");
  }

  // Sort a copy so we don't mutate the input (pure function principle).
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);

  // Odd length: the middle element is the median.
  // Even length: average the two elements straddling the center.
  if (sorted.length % 2 !== 0) {
    return sorted[mid];
  } else {
    return (sorted[mid - 1] + sorted[mid]) / 2;
  }
}
