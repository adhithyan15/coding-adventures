//! R-Vector
//!
//! Minimal Layer 0 vector substrate for the Rust statistics stack. This
//! implements the NA-aware `Double` vector and the small generic `Vector`
//! contract that `statistics-core` needs for its Phase 1 functions.

use numeric_tower::Number;

pub const NA_REAL_BITS: u64 = 0x7ff0_0000_0000_07a2;

pub fn na_real() -> f64 {
    f64::from_bits(NA_REAL_BITS)
}

pub fn is_na_real(value: f64) -> bool {
    value.to_bits() == NA_REAL_BITS
}

pub fn is_nan_real(value: f64) -> bool {
    value.is_nan() && !is_na_real(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorError {
    NamesLengthMismatch { names: usize, values: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Names {
    values: Vec<String>,
}

impl Names {
    pub fn new(values: Vec<String>, expected_len: usize) -> Result<Self, VectorError> {
        if values.len() != expected_len {
            return Err(VectorError::NamesLengthMismatch {
                names: values.len(),
                values: expected_len,
            });
        }
        Ok(Self { values })
    }

    pub fn as_slice(&self) -> &[String] {
        &self.values
    }
}

pub trait Vector {
    type Element;

    fn len(&self) -> usize;
    fn is_na(&self, index: usize) -> bool;
    fn names(&self) -> Option<&Names>;
    fn get(&self, index: usize) -> Option<&Self::Element>;
    fn type_name(&self) -> &'static str;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Double {
    data: Vec<f64>,
    names: Option<Names>,
}

impl Double {
    pub fn from_values(data: Vec<f64>) -> Self {
        Self { data, names: None }
    }

    pub fn from_optional<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Option<f64>>,
    {
        Self {
            data: values
                .into_iter()
                .map(|value| value.unwrap_or_else(na_real))
                .collect(),
            names: None,
        }
    }

    pub fn singleton(value: f64) -> Self {
        Self::from_values(vec![value])
    }

    pub fn na(len: usize) -> Self {
        Self::from_values(vec![na_real(); len])
    }

    pub fn with_names(mut self, names: Vec<String>) -> Result<Self, VectorError> {
        self.names = Some(Names::new(names, self.data.len())?);
        Ok(self)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn data(&self) -> &[f64] {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.data.iter().copied()
    }

    pub fn get_value(&self, index: usize) -> Option<f64> {
        self.data.get(index).copied()
    }

    pub fn push_value(&mut self, value: f64) {
        self.data.push(value);
    }

    pub fn push_na(&mut self) {
        self.data.push(na_real());
    }

    pub fn to_number_at(&self, index: usize) -> Option<Number> {
        self.data.get(index).copied().map(Number::Float)
    }
}

impl Vector for Double {
    type Element = f64;

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_na(&self, index: usize) -> bool {
        self.data
            .get(index)
            .copied()
            .map(is_na_real)
            .unwrap_or(false)
    }

    fn names(&self) -> Option<&Names> {
        self.names.as_ref()
    }

    fn get(&self, index: usize) -> Option<&Self::Element> {
        self.data.get(index)
    }

    fn type_name(&self) -> &'static str {
        "double"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    data: Vec<Option<String>>,
    names: Option<Names>,
}

impl Character {
    pub fn from_options(data: Vec<Option<String>>) -> Self {
        Self { data, names: None }
    }

    pub fn from_strings<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            data: values.into_iter().map(|value| Some(value.into())).collect(),
            names: None,
        }
    }

    pub fn with_names(mut self, names: Vec<String>) -> Result<Self, VectorError> {
        self.names = Some(Names::new(names, self.data.len())?);
        Ok(self)
    }

    pub fn is_blank(&self, index: usize) -> bool {
        matches!(self.data.get(index), Some(Some(value)) if value.is_empty())
    }
}

impl Vector for Character {
    type Element = Option<String>;

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_na(&self, index: usize) -> bool {
        matches!(self.data.get(index), Some(None))
    }

    fn names(&self) -> Option<&Names> {
        self.names.as_ref()
    }

    fn get(&self, index: usize) -> Option<&Self::Element> {
        self.data.get(index)
    }

    fn type_name(&self) -> &'static str {
        "character"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_na_uses_r_bit_pattern() {
        let value = na_real();
        assert!(value.is_nan());
        assert!(is_na_real(value));
        assert!(!is_nan_real(value));
    }

    #[test]
    fn optional_constructor_maps_none_to_na() {
        let vector = Double::from_optional([Some(1.0), None, Some(f64::NAN)]);
        assert_eq!(vector.len(), 3);
        assert!(vector.is_na(1));
        assert!(is_nan_real(vector.get_value(2).unwrap()));
    }

    #[test]
    fn names_must_match_vector_length() {
        let err = Double::from_values(vec![1.0, 2.0])
            .with_names(vec!["x".into()])
            .unwrap_err();
        assert_eq!(
            err,
            VectorError::NamesLengthMismatch {
                names: 1,
                values: 2
            }
        );
    }
}
