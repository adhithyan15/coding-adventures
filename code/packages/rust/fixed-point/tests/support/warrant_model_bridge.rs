// This file is included inside the generated harness module so these small
// calls reach its actual private Warrant-model evaluators, not the op adapters.
pub fn q16_from_integer_model(value: V) -> V {
    model_q16_from_integer_model(value)
}
pub fn q16_checked_add_model(a: V, b: V) -> V {
    model_q16_checked_add_model(a, b)
}
pub fn q16_checked_sub_model(a: V, b: V) -> V {
    model_q16_checked_sub_model(a, b)
}
pub fn q16_saturating_add_model(a: V, b: V) -> V {
    model_q16_saturating_add_model(a, b)
}
pub fn q16_saturating_sub_model(a: V, b: V) -> V {
    model_q16_saturating_sub_model(a, b)
}
pub fn q16_checked_mul_model(a: V, b: V, mode: V) -> V {
    model_q16_checked_mul_model(a, b, mode)
}
pub fn q16_saturating_mul_model(a: V, b: V, mode: V) -> V {
    model_q16_saturating_mul_model(a, b, mode)
}
pub fn q16_checked_div_model(a: V, b: V, mode: V) -> V {
    model_q16_checked_div_model(a, b, mode)
}
pub fn q16_saturating_div_model(a: V, b: V, mode: V) -> V {
    model_q16_saturating_div_model(a, b, mode)
}
pub fn q16_to_integer_model(value: V, mode: V) -> V {
    model_q16_to_integer_model(value, mode)
}
