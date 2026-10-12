/**
 * Estimate the time a lesson actually directs a learner to spend.
 *
 * A single clock and separately named subtasks can describe the same attempt,
 * so take their maximum rather than double-counting them. The wrap-up is
 * additional work: without an explicit cap, the lesson has no proven bound.
 * This small test helper is shared by A1 corpus cases as languages move their
 * authentic long papers into separate assessment resources.
 */
const minutes = "(?:\\d+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|twenty)";
const minuteValues: Record<string, number> = {
  one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7,
  eight: 8, nine: 9, ten: 10, eleven: 11, twelve: 12, thirteen: 13,
  twenty: 20,
};

function minuteCount(value: string): number {
  return minuteValues[value.toLowerCase()] ?? Number(value);
}

export function directedLessonSeconds(markdown: string): number {
  const plain = markdown.replaceAll("**", "");
  const beforeWrapUp = plain.split(/^## Wrap-up/mi)[0];
  const wrapUp = plain.split(/^## Wrap-up/mi)[1] ?? "";
  const clock = [
    ...beforeWrapUp.matchAll(new RegExp(`\\bset (?:one|a) timer for (${minutes}) minutes?`, "gi")),
    ...beforeWrapUp.matchAll(new RegExp(`\\bset a (${minutes})-minute timer`, "gi")),
  ].map((match) => minuteCount(match[1]));
  const subtasks = [
    ...beforeWrapUp.matchAll(new RegExp(`\\bTask \\d+[^\\n]*?(${minutes}) minutes?`, "gi")),
  ].map((match) => minuteCount(match[1]));
  const review = wrapUp.match(new RegExp(`\\b(?:at most|no more than) (${minutes}) minutes?`, "i"));
  if (!review) return Number.POSITIVE_INFINITY;
  return (Math.max(0, ...clock, subtasks.reduce((sum, value) => sum + value, 0))
    + minuteCount(review[1])) * 60;
}
