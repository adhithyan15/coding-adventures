//! Host-neutral Unicode text-flow analysis for inline layout and paint.

use icu_properties::{props::BidiClass, CodePointMapData};
use icu_segmenter::{GraphemeClusterSegmenter, LineSegmenter};
use std::ops::Range;
use unicode_bidi::{BidiInfo, Level};

pub const VERSION: &str = "0.2.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConformanceProfile {
    pub unicode_version: &'static str,
    pub bidi_data: &'static str,
    pub grapheme_data: &'static str,
    pub line_break_data: &'static str,
    pub complex_line_break_scripts: &'static [&'static str],
}

/// The generated Unicode data profile shared by every text-flow consumer.
pub const CONFORMANCE_PROFILE: ConformanceProfile = ConformanceProfile {
    unicode_version: "17.0.0",
    bidi_data: "ICU4X generated Bidi_Class map + unicode-bidi UAX #9 resolver",
    grapheme_data: "ICU4X generated UAX #29 state machine",
    line_break_data: "ICU4X generated UAX #14 v17 full pair state machine",
    complex_line_break_scripts: &["Thai", "Lao", "Khmer", "Myanmar"],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseDirection {
    Auto,
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakKind {
    Allowed,
    Mandatory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grapheme {
    pub bytes: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectionalRun {
    pub bytes: Range<usize>,
    pub direction: Direction,
    pub level: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakOpportunity {
    pub byte_index: usize,
    pub kind: BreakKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextFlow {
    pub base_direction: Direction,
    pub graphemes: Vec<Grapheme>,
    pub logical_runs: Vec<DirectionalRun>,
    pub visual_run_order: Vec<usize>,
    pub breaks: Vec<BreakOpportunity>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionSpan {
    pub x: f64,
    pub width: f64,
    pub bytes: Range<usize>,
    pub direction: Direction,
}

impl TextFlow {
    pub fn analyze(text: &str, requested: BaseDirection) -> Self {
        let graphemes = graphemes(text);
        let (base_direction, logical_runs, visual_run_order) = bidi_analysis(text, requested);
        let breaks = line_breaks(text, &graphemes);
        Self {
            base_direction,
            graphemes,
            logical_runs,
            visual_run_order,
            breaks,
        }
    }

    /// Snap an arbitrary UTF-8 byte range outward to whole grapheme clusters.
    pub fn snap_selection(&self, range: Range<usize>) -> Range<usize> {
        let start = self
            .graphemes
            .iter()
            .find(|cluster| cluster.bytes.end > range.start)
            .map_or(range.start, |cluster| cluster.bytes.start);
        let end = self
            .graphemes
            .iter()
            .rev()
            .find(|cluster| cluster.bytes.start < range.end)
            .map_or(range.end, |cluster| cluster.bytes.end);
        start.min(end)..end.max(start)
    }

    /// Project a logical selection into visual one-line spans.
    ///
    /// The caller supplies device-independent cluster measurement so this
    /// package remains independent of fonts and paint backends. Selections are
    /// snapped to complete graphemes and split at bidi run boundaries.
    pub fn selection_spans<F>(
        &self,
        text: &str,
        selection: Range<usize>,
        mut measure: F,
    ) -> Vec<SelectionSpan>
    where
        F: FnMut(&str) -> f64,
    {
        let selection = self.snap_selection(selection);
        let mut spans = Vec::new();
        let mut x = 0.0;
        for run_index in &self.visual_run_order {
            let run = &self.logical_runs[*run_index];
            let mut clusters: Vec<_> = self
                .graphemes
                .iter()
                .filter(|cluster| {
                    cluster.bytes.start >= run.bytes.start && cluster.bytes.end <= run.bytes.end
                })
                .collect();
            if run.direction == Direction::Rtl {
                clusters.reverse();
            }
            let mut selected_start = None;
            let mut selected_width = 0.0;
            let mut selected_bytes: Option<Range<usize>> = None;
            for cluster in clusters {
                let width = measure(&text[cluster.bytes.clone()]).max(0.0);
                let selected =
                    cluster.bytes.start < selection.end && cluster.bytes.end > selection.start;
                if selected {
                    selected_start.get_or_insert(x);
                    selected_width += width;
                    selected_bytes = Some(match selected_bytes {
                        Some(bytes) => {
                            bytes.start.min(cluster.bytes.start)..bytes.end.max(cluster.bytes.end)
                        }
                        None => cluster.bytes.clone(),
                    });
                } else if let (Some(start), Some(bytes)) =
                    (selected_start.take(), selected_bytes.take())
                {
                    spans.push(SelectionSpan {
                        x: start,
                        width: selected_width,
                        bytes,
                        direction: run.direction,
                    });
                    selected_width = 0.0;
                }
                x += width;
            }
            if let (Some(start), Some(bytes)) = (selected_start, selected_bytes) {
                spans.push(SelectionSpan {
                    x: start,
                    width: selected_width,
                    bytes,
                    direction: run.direction,
                });
            }
        }
        spans
    }
}

pub fn graphemes(text: &str) -> Vec<Grapheme> {
    let boundaries: Vec<_> = GraphemeClusterSegmenter::new().segment_str(text).collect();
    boundaries
        .windows(2)
        .map(|boundary| Grapheme {
            bytes: boundary[0]..boundary[1],
        })
        .collect()
}

fn line_breaks(text: &str, clusters: &[Grapheme]) -> Vec<BreakOpportunity> {
    let mut segmenter = LineSegmenter::new_17_for_non_complex_scripts(Default::default());
    segmenter.load_dictionary();
    segmenter
        .segment_str(text)
        .filter(|index| *index > 0)
        .filter_map(|byte_index| {
            let mandatory = mandatory_break_before(text, byte_index);
            if byte_index == text.len() && !mandatory {
                return None;
            }
            debug_assert!(clusters
                .iter()
                .any(|cluster| cluster.bytes.end == byte_index));
            Some(BreakOpportunity {
                byte_index,
                kind: if mandatory {
                    BreakKind::Mandatory
                } else {
                    BreakKind::Allowed
                },
            })
        })
        .collect()
}

fn bidi_info(text: &str, requested: BaseDirection) -> BidiInfo<'_> {
    let default_level = match requested {
        BaseDirection::Auto => None,
        BaseDirection::Ltr => Some(Level::ltr()),
        BaseDirection::Rtl => Some(Level::rtl()),
    };
    let bidi_classes = CodePointMapData::<BidiClass>::new();
    BidiInfo::new_with_data_source(&bidi_classes, text, default_level)
}

fn bidi_analysis(
    text: &str,
    requested: BaseDirection,
) -> (Direction, Vec<DirectionalRun>, Vec<usize>) {
    let info = bidi_info(text, requested);
    let base_direction = info
        .paragraphs
        .first()
        .map_or(Direction::Ltr, |paragraph| direction(paragraph.level));
    let mut runs = Vec::new();
    let mut order = Vec::new();
    for paragraph in &info.paragraphs {
        let (levels, visual_ranges) = info.visual_runs(paragraph, paragraph.range.clone());
        let first_run = runs.len();
        let mut start = paragraph.range.start;
        let mut current = levels.get(start).copied().unwrap_or(paragraph.level);
        for (index, _) in text[paragraph.range.clone()].char_indices().skip(1) {
            let index = paragraph.range.start + index;
            let level = levels[index];
            if level != current {
                runs.push(directional_run(start, index, current));
                start = index;
                current = level;
            }
        }
        if start < paragraph.range.end {
            runs.push(directional_run(start, paragraph.range.end, current));
        }
        for visual_range in visual_ranges {
            if let Some(index) = runs[first_run..]
                .iter()
                .position(|run| run.bytes == visual_range)
            {
                order.push(first_run + index);
            }
        }
    }
    debug_assert_eq!(order.len(), runs.len());
    (base_direction, runs, order)
}

fn directional_run(start: usize, end: usize, level: Level) -> DirectionalRun {
    DirectionalRun {
        bytes: start..end,
        direction: direction(level),
        level: level.number(),
    }
}

fn direction(level: Level) -> Direction {
    if level.is_rtl() {
        Direction::Rtl
    } else {
        Direction::Ltr
    }
}

fn mandatory_break_before(text: &str, byte_index: usize) -> bool {
    text[..byte_index].chars().next_back().is_some_and(|ch| {
        // UAX #14 LB4/LB5 classes BK, CR, LF, and NL.
        matches!(
            ch,
            '\u{000b}' | '\u{000c}' | '\r' | '\n' | '\u{0085}' | '\u{2028}' | '\u{2029}'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values<'a>(text: &'a str, clusters: &[Grapheme]) -> Vec<&'a str> {
        clusters.iter().map(|c| &text[c.bytes.clone()]).collect()
    }

    #[test]
    fn keeps_combining_emoji_and_flags_as_graphemes() {
        let text = "e\u{301} 👩‍💻 🇺🇳";
        assert_eq!(
            values(text, &graphemes(text)),
            vec!["e\u{301}", " ", "👩‍💻", " ", "🇺🇳"]
        );
    }

    #[test]
    fn selection_snaps_outward_to_graphemes() {
        let text = "Ae\u{301}B";
        let flow = TextFlow::analyze(text, BaseDirection::Auto);
        assert_eq!(flow.snap_selection(2..3), 1..4);
    }

    #[test]
    fn resolves_mixed_direction_runs_and_visual_order() {
        let flow = TextFlow::analyze("שלום Venture", BaseDirection::Auto);
        assert_eq!(flow.base_direction, Direction::Rtl);
        assert_eq!(flow.logical_runs.len(), 2);
        assert_eq!(flow.logical_runs[0].direction, Direction::Rtl);
        assert_eq!(flow.logical_runs[1].direction, Direction::Ltr);
        assert_eq!(flow.visual_run_order, vec![1, 0]);
    }

    #[test]
    fn exposes_cjk_space_hyphen_and_mandatory_breaks() {
        let text = "日本語 test-case\nnext";
        let flow = TextFlow::analyze(text, BaseDirection::Auto);
        let breaks: Vec<_> = flow.breaks.iter().map(|b| (b.byte_index, b.kind)).collect();
        assert!(breaks.contains(&(3, BreakKind::Allowed)));
        assert!(breaks.contains(&(6, BreakKind::Allowed)));
        assert!(breaks.iter().any(|(_, kind)| *kind == BreakKind::Mandatory));
    }

    #[test]
    fn non_breaking_spaces_and_punctuation_stay_attached() {
        let text = "A\u{a0}B 日本。";
        let flow = TextFlow::analyze(text, BaseDirection::Ltr);
        assert!(!flow.breaks.iter().any(|b| b.byte_index == 2));
        let ideograph_end = text.find('本').unwrap() + '本'.len_utf8();
        assert!(!flow.breaks.iter().any(|b| b.byte_index == ideograph_end));
    }

    #[test]
    fn selection_geometry_uses_whole_graphemes_and_visual_bidi_runs() {
        let text = "A e\u{301} שלום";
        let flow = TextFlow::analyze(text, BaseDirection::Ltr);
        let start = text.find('e').unwrap() + 1;
        let spans = flow.selection_spans(text, start..text.len(), |_| 10.0);
        assert_eq!(spans.len(), 2);
        assert_eq!(&text[spans[0].bytes.clone()], "e\u{301} ");
        assert_eq!(spans[0].direction, Direction::Ltr);
        assert_eq!(&text[spans[1].bytes.clone()], "שלום");
        assert_eq!(spans[1].direction, Direction::Rtl);
        assert!(spans.iter().all(|span| span.width > 0.0));
    }

    #[test]
    fn reports_the_generated_unicode_conformance_profile() {
        assert_eq!(CONFORMANCE_PROFILE.unicode_version, "17.0.0");
        assert!(CONFORMANCE_PROFILE.line_break_data.contains("full pair"));
        assert_eq!(
            CONFORMANCE_PROFILE.complex_line_break_scripts,
            ["Thai", "Lao", "Khmer", "Myanmar"]
        );
    }

    #[test]
    fn resolves_isolates_embeddings_and_numbers_with_generated_bidi_classes() {
        let text = "LTR \u{2067}שלום 123\u{2069} \u{202b}مرحبا\u{202c} tail";
        let flow = TextFlow::analyze(text, BaseDirection::Ltr);
        assert_eq!(flow.base_direction, Direction::Ltr);
        assert!(flow.logical_runs.iter().any(|run| run.level >= 2));
        assert!(flow
            .logical_runs
            .iter()
            .any(|run| run.direction == Direction::Rtl));
        let mut visual = flow.visual_run_order.clone();
        visual.sort_unstable();
        assert_eq!(visual, (0..flow.logical_runs.len()).collect::<Vec<_>>());
    }

    #[test]
    fn keeps_visual_reordering_scoped_to_each_paragraph() {
        let flow = TextFlow::analyze("abc\nשלום 12", BaseDirection::Auto);
        assert_eq!(flow.base_direction, Direction::Ltr);
        let mut visual = flow.visual_run_order.clone();
        visual.sort_unstable();
        assert_eq!(visual, (0..flow.logical_runs.len()).collect::<Vec<_>>());
        assert!(flow
            .logical_runs
            .windows(2)
            .all(|runs| runs[0].bytes.end <= runs[1].bytes.start));
    }

    #[test]
    fn applies_the_full_line_break_table_around_cjk_punctuation() {
        let text = "（日本）語 A\u{202f}B\u{0085}next";
        let flow = TextFlow::analyze(text, BaseDirection::Ltr);
        let after_open = '（'.len_utf8();
        let before_close = text.find('）').unwrap();
        assert!(!flow.breaks.iter().any(|item| item.byte_index == after_open));
        assert!(!flow
            .breaks
            .iter()
            .any(|item| item.byte_index == before_close));
        assert!(flow
            .breaks
            .iter()
            .any(|item| item.kind == BreakKind::Mandatory));
    }

    #[test]
    fn dictionary_segments_thai_lao_and_khmer_without_spaces() {
        for (text, expected) in [
            ("ทุกสองสัปดาห์", vec![9, 18]),
            ("ພາສາລາວພາສາລາວ", vec![12, 21, 33]),
            ("ភាសាខ្មែរភាសាខ្មែរ", vec![27]),
        ] {
            let flow = TextFlow::analyze(text, BaseDirection::Auto);
            let actual: Vec<_> = flow.breaks.iter().map(|item| item.byte_index).collect();
            for boundary in expected {
                assert!(
                    actual.contains(&boundary),
                    "missing dictionary boundary {boundary} for {text:?}: {actual:?}"
                );
            }
        }
    }
}
