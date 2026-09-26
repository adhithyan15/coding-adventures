# Scaffold generator requires TypeScript dependency closure before execution

Installing only `code/programs/typescript/scaffold-generator` is insufficient in a clean
worktree because its local `cli-builder` dependency imports other unpublished sibling
packages such as `state-machine`. Before invoking the generator, install the complete
TypeScript dependency closure in leaf-to-root order (or run the generated BUILD setup
that does so). If recording a failure, invoke the lesson tool from the repository root
as `python code/scripts/lessons.py new ...`; there is no `code/lessons.py` entry point.
