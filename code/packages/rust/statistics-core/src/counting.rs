use r_vector::{Character, Vector};

pub fn length<V: Vector>(x: &V) -> usize {
    x.len()
}

pub fn count_non_na<V: Vector>(x: &V) -> usize {
    (0..x.len()).filter(|&index| !x.is_na(index)).count()
}

pub fn count_blank(x: &Character) -> usize {
    (0..x.len()).filter(|&index| x.is_blank(index)).count()
}

pub fn count_if<V, P>(x: &V, pred: P) -> usize
where
    V: Vector,
    P: Fn(&V::Element) -> bool,
{
    (0..x.len())
        .filter(|&index| !x.is_na(index))
        .filter_map(|index| x.get(index))
        .filter(|value| pred(value))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::{na_real, Character, Double};

    #[test]
    fn count_non_na_excludes_na_slots() {
        let values = Double::from_values(vec![1.0, na_real(), f64::NAN, 4.0]);
        assert_eq!(length(&values), 4);
        assert_eq!(count_non_na(&values), 3);
    }

    #[test]
    fn blank_count_is_distinct_from_na_count() {
        let values = Character::from_options(vec![Some("".into()), None, Some("x".into())]);
        assert_eq!(count_blank(&values), 1);
        assert_eq!(count_non_na(&values), 2);
    }
}
