// The engine's sabotage hooks (Hooks.*) are process-global, so tests that swap them
// must never overlap with any other test: run the whole assembly serially.
[assembly: CollectionBehavior(DisableTestParallelization = true)]
