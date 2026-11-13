/**
 * A one-shot `vitest run` needs the source bytes, but not a dev-server watcher.
 * The command must occupy Vitest's command slot: `-t run` is an interactive
 * name filter, not a run command. An explicit watch override keeps HMR on.
 */
export function watchCurriculumSourcesForVitest(argv: readonly string[]): boolean {
  if (argv[2] !== "run") return true;
  return argv.some((argument) =>
    argument === "-w" ||
    argument.startsWith("-w=") ||
    argument === "--watch" ||
    argument.startsWith("--watch="),
  );
}
