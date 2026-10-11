### BATCH XAML ROW PROJECTION NOTIFICATIONS

Generated components batch row projection notifications through ISupportInitialize during shared runtime prop assignment. Nested batches deduplicate projection names in order; direct changes still notify immediately. The runtime releases batches even when setters fail. Compiled C# probes cover batching and cleanup; native performance and keyboard acceptance remain required.

