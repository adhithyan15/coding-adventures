//! Count actual production edge examinations, independently bounding visits.
use super::wardley_description_bounds;

#[test]
fn description_trimming_visits_each_edge_once_without_shifting_a_prefix() {
    for count in [0, 1, 16, 4096, 40_000] {
        for (leading, trailing) in [(count, 0), (0, count), (count, count)] {
            let mut lines = vec![""; leading];
            lines.push("body");
            lines.extend(std::iter::repeat_n("", trailing));
            let (start, end, visits) = wardley_description_bounds(&lines);
            assert_eq!((start, end), (leading, leading + 1));
            assert_eq!(&lines[start..end], ["body"]);
            assert!(visits <= lines.len() + 2, "{leading}/{trailing}: {visits}");
        }
    }
    for lines in [vec![], vec![""], vec![""; 40_000]] {
        let (start, end, visits) = wardley_description_bounds(&lines);
        assert_eq!((start, end), (lines.len(), lines.len()));
        assert!(visits <= lines.len() + 2);
    }
}
