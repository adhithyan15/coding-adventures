# bloom-filter — Kotlin

A space-efficient probabilistic set membership filter. Answers "Have I seen this element before?" with two possible responses:

- **"Definitely NO"** — zero false negatives. Trust this completely.
- **"Probably YES"** — bounded false positive rate, tunable at construction.

## Usage

```kotlin
import com.codingadventures.bloomfilter.BloomFilter

// Create a filter for 1,000 elements with 1% false positive rate
val bf = BloomFilter<String>(1000, 0.01)

bf.add("alice")
bf.add("bob")

bf.contains("alice")  // true  — definitely was added
bf.contains("carol")  // false — definitely not added
bf.contains("dave")   // false or true — if true, it's a false positive (~1% chance)
```

## Running Tests

```bash
gradle test
```

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
