//! Host-neutral Unicode text-flow analysis for inline layout and paint.

use std::ops::Range;

pub const VERSION: &str = "0.1.0";

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
        let base_direction = resolve_base_direction(text, requested);
        let logical_runs = directional_runs(text, base_direction);
        let visual_run_order = visual_order(&logical_runs, base_direction);
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
    let mut output = Vec::new();
    let mut start = 0;
    let mut previous = None;
    let mut regional_count = 0usize;

    for (index, ch) in text.char_indices() {
        let should_break =
            previous.is_some_and(|prev| !continues_grapheme(prev, ch, regional_count, text, index));
        if should_break {
            output.push(Grapheme {
                bytes: start..index,
            });
            start = index;
            regional_count = 0;
        }
        if is_regional_indicator(ch) {
            regional_count += 1;
        } else if !is_extend(ch) {
            regional_count = 0;
        }
        previous = Some(ch);
    }
    if !text.is_empty() {
        output.push(Grapheme {
            bytes: start..text.len(),
        });
    }
    output
}

fn continues_grapheme(
    prev: char,
    next: char,
    regional_count: usize,
    text: &str,
    index: usize,
) -> bool {
    if prev == '\r' && next == '\n' {
        return true;
    }
    if is_control(prev) || is_control(next) {
        return false;
    }
    if is_extend(next) || next == '\u{200d}' || is_spacing_mark(next) {
        return true;
    }
    if prev == '\u{200d}' && preceding_extended_pictographic(text, index) {
        return true;
    }
    if is_regional_indicator(prev) && is_regional_indicator(next) {
        return regional_count % 2 == 1;
    }
    hangul_continues(prev, next)
}

fn preceding_extended_pictographic(text: &str, index: usize) -> bool {
    text[..index]
        .chars()
        .rev()
        .find(|ch| !is_extend(*ch) && *ch != '\u{200d}')
        .is_some_and(is_extended_pictographic)
}

fn directional_runs(text: &str, base: Direction) -> Vec<DirectionalRun> {
    let mut runs = Vec::new();
    let mut start = 0;
    let mut current = base;
    let mut saw_strong = false;
    for (index, ch) in text.char_indices() {
        let Some(direction) = strong_direction(ch) else {
            continue;
        };
        if saw_strong && direction != current {
            runs.push(run(start, index, current, base));
            start = index;
        }
        current = direction;
        saw_strong = true;
    }
    if !text.is_empty() {
        runs.push(run(start, text.len(), current, base));
    }
    runs
}

fn run(start: usize, end: usize, direction: Direction, base: Direction) -> DirectionalRun {
    DirectionalRun {
        bytes: start..end,
        direction,
        level: match (base, direction) {
            (Direction::Ltr, Direction::Ltr) => 0,
            (Direction::Ltr, Direction::Rtl) | (Direction::Rtl, Direction::Rtl) => 1,
            (Direction::Rtl, Direction::Ltr) => 2,
        },
    }
}

fn visual_order(runs: &[DirectionalRun], base: Direction) -> Vec<usize> {
    let mut order: Vec<_> = (0..runs.len()).collect();
    if base == Direction::Rtl {
        order.reverse();
    }
    order
}

fn line_breaks(text: &str, clusters: &[Grapheme]) -> Vec<BreakOpportunity> {
    let mut output = Vec::new();
    for (index, cluster) in clusters.iter().enumerate() {
        let value = &text[cluster.bytes.clone()];
        if value == "\n" || value == "\r" || value == "\r\n" {
            output.push(BreakOpportunity {
                byte_index: cluster.bytes.end,
                kind: BreakKind::Mandatory,
            });
            continue;
        }
        let last = value.chars().last().unwrap();
        let next = clusters
            .get(index + 1)
            .and_then(|next| text[next.bytes.clone()].chars().next());
        let allowed = last == '\u{200b}'
            || last == '\u{00ad}'
            || (last.is_whitespace() && last != '\u{00a0}' && last != '\u{202f}')
            || (is_break_hyphen(last) && next.is_some_and(|ch| !is_closing_punctuation(ch)))
            || next.is_some_and(|ch| {
                is_ideographic(last)
                    && is_ideographic(ch)
                    && !is_closing_punctuation(ch)
                    && !is_opening_punctuation(last)
            });
        if allowed {
            output.push(BreakOpportunity {
                byte_index: cluster.bytes.end,
                kind: BreakKind::Allowed,
            });
        }
    }
    output
}

fn resolve_base_direction(text: &str, requested: BaseDirection) -> Direction {
    match requested {
        BaseDirection::Ltr => Direction::Ltr,
        BaseDirection::Rtl => Direction::Rtl,
        BaseDirection::Auto => text
            .chars()
            .find_map(strong_direction)
            .unwrap_or(Direction::Ltr),
    }
}

fn strong_direction(ch: char) -> Option<Direction> {
    if is_rtl(ch) {
        Some(Direction::Rtl)
    } else if ch.is_alphabetic() || ch.is_ascii_digit() || is_ideographic(ch) {
        Some(Direction::Ltr)
    } else {
        None
    }
}

fn is_rtl(ch: char) -> bool {
    matches!(ch as u32, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfeff | 0x10800..=0x10fff | 0x1e800..=0x1edff)
}

fn is_extend(ch: char) -> bool {
    matches!(ch as u32, 0x0300..=0x036f | 0x0483..=0x0489 | 0x0591..=0x05bd | 0x05bf | 0x05c1..=0x05c2 | 0x0610..=0x061a | 0x064b..=0x065f | 0x0670 | 0x06d6..=0x06ed | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x20d0..=0x20ff | 0xfe00..=0xfe0f | 0xfe20..=0xfe2f | 0x1f3fb..=0x1f3ff | 0xe0100..=0xe01ef)
}

fn is_spacing_mark(ch: char) -> bool {
    matches!(ch as u32, 0x0903 | 0x093b | 0x093e..=0x0940 | 0x0949..=0x094c | 0x0982..=0x0983 | 0x09be..=0x09c0 | 0x0bbe..=0x0bc2 | 0x0bc6..=0x0bc8 | 0x0bca..=0x0bcc)
}

fn is_control(ch: char) -> bool {
    matches!(ch, '\r' | '\n') || (ch.is_control() && ch != '\u{200d}')
}

fn is_regional_indicator(ch: char) -> bool {
    matches!(ch as u32, 0x1f1e6..=0x1f1ff)
}

fn is_extended_pictographic(ch: char) -> bool {
    matches!(ch as u32, 0x1f000..=0x1faff | 0x2300..=0x23ff | 0x2600..=0x27bf)
}

fn hangul_continues(prev: char, next: char) -> bool {
    let p = prev as u32;
    let n = next as u32;
    let l = (0x1100..=0x115f).contains(&p) || (0xa960..=0xa97c).contains(&p);
    let v = (0x1160..=0x11a7).contains(&n) || (0xd7b0..=0xd7c6).contains(&n);
    let t = (0x11a8..=0x11ff).contains(&n) || (0xd7cb..=0xd7fb).contains(&n);
    l && v || ((0xac00..=0xd7a3).contains(&p) || v) && t
}

fn is_ideographic(ch: char) -> bool {
    matches!(ch as u32, 0x2e80..=0x9fff | 0xac00..=0xd7af | 0xf900..=0xfaff | 0x20000..=0x323af)
}

fn is_break_hyphen(ch: char) -> bool {
    matches!(ch, '-' | '\u{058a}' | '\u{2010}' | '\u{2012}' | '\u{2013}')
}

fn is_opening_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '(' | '['
            | '{'
            | '\u{2018}'
            | '\u{201c}'
            | '\u{3008}'
            | '\u{300a}'
            | '\u{300c}'
            | '\u{300e}'
            | '\u{3010}'
    )
}

fn is_closing_punctuation(ch: char) -> bool {
    matches!(
        ch,
        ')' | ']'
            | '}'
            | ','
            | '.'
            | '!'
            | '?'
            | ':'
            | ';'
            | '\u{2019}'
            | '\u{201d}'
            | '\u{3001}'
            | '\u{3002}'
            | '\u{3009}'
            | '\u{300b}'
            | '\u{300d}'
            | '\u{300f}'
            | '\u{3011}'
    )
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
}
