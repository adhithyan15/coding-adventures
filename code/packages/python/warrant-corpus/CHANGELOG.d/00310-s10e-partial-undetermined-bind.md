## Unreleased - CV2 S10e review 3: only a wholly unknown bind is unusable

A variable bound by a `match` pattern is refused in an arm body only when its **whole** type is unknown (the payload of
a bare `err`). A `Result<U8,?>` or `Result<?,E>` (one side undetermined) is usable wherever the R41/R51 typing accepts
it: `is_err`, a nested `match`, `eq` against a `Result<U8,E>`, a tuple component. The kit run is unchanged (1,452 run, 0
failed). Tests in `tests/test_v2_tuple.py`.
