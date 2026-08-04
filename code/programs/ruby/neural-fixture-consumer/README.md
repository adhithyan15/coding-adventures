# Ruby neural fixture consumer

This dependency-free Ruby program reads the NN03 weighted-neuron JSON fixture,
recomputes its native binary64 forward pass, and writes one strict JSON receipt.

```bash
ruby main.rb --fixture ../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json
ruby -Itest test/test_consumer.rb
```

It is the dynamic, interpreted lane in NN34. It rejects duplicate and unknown
JSON keys and does not call Python or copy a stored receipt.
