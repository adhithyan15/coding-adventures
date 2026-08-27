//! # diagram-layout-temporal
//!
//! Layout engine for temporal diagrams (DG04): Gantt and git-graph.
//!
//! ## Gantt
//! Maps each task to a horizontal bar on a time axis.  Task dates are
//! parsed as `YYYY-MM-DD`; `After(id)` dependencies are resolved in a
//! second pass after all absolute starts are known.
//!
//! ## Git-graph
//! Assigns each branch a horizontal lane and replays commit events to
//! place commit nodes and merge arcs.

use diagram_ir::{
    DiagramDirection, GanttDateFormat, GanttDateFormatPart, GitCommitSymbol, GitCommitType, GitDiagram, GitEvent, JourneyDiagram,
    LayoutedTemporalDiagram, LayoutedTemporalInteraction, LayoutedTemporalItem, TaskEnd, TaskStart,
    TemporalBody, TemporalDiagram,
};
use std::collections::{BTreeSet, HashMap};

pub const VERSION: &str = "0.34.0";

// ── Constants ─────────────────────────────────────────────────────────────

const AXIS_H:         f64 = 28.0;
const TASK_H:         f64 = 20.0;
const TASK_GAP:       f64 = 4.0;
const LABEL_W:        f64 = 120.0;
const TICK_DAYS:      f64 = 7.0;
const SECTION_H:      f64 = 24.0;
const LANE_H:         f64 = 60.0;
const COMMIT_SPACING: f64 = 80.0;

const BRANCH_COLORS: &[&str] = &[
    "#3b82f6", "#ef4444", "#22c55e", "#f59e0b", "#a855f7", "#14b8a6",
];
const JOURNEY_ACTOR_COLORS: &[&str] = &[
    "#8fbc8f", "#7cfc00", "#00ffff", "#20b2aa", "#b0e0e6", "#ffffe0",
];
const JOURNEY_SECTION_FILLS: &[&str] = &[
    "#191970", "#8b008b", "#4b0082", "#2f4f4f", "#800000", "#8b4513", "#00008b",
];
const JOURNEY_SECTION_COLORS: &[&str] = &["#ffffff"];

fn multiline_height(label: &str, base_height: f64, line_height: f64) -> f64 {
    base_height + label.lines().count().saturating_sub(1) as f64 * line_height
}

/// Lay out a `TemporalDiagram` on a canvas of `cw` pixels wide.
pub fn layout_temporal_diagram(d: &TemporalDiagram, cw: f64) -> LayoutedTemporalDiagram {
    match &d.body {
        TemporalBody::Gantt(g) => layout_gantt(&d.title, g, cw),
        TemporalBody::Git(g)   => layout_git(g, cw),
        TemporalBody::Journey(j) => layout_journey(&d.title, j, cw),
    }
}

fn layout_journey(title: &Option<String>, diagram: &JourneyDiagram, cw: f64) -> LayoutedTemporalDiagram {
    let mut items = Vec::new();
    let margin_x = diagram.config.diagram_margin_x.unwrap_or(16.0);
    let margin_y = diagram.config.diagram_margin_y.unwrap_or(0.0);
    let task_margin = diagram.config.task_margin.unwrap_or(6.0);
    let task_x = diagram.config.left_margin.unwrap_or(margin_x).max(margin_x);
    let task_width = diagram.config.task_width.unwrap_or(150.0).max(1.0);
    let task_height = diagram.config.task_height.unwrap_or(50.0).max(1.0);
    let actor_label_width = diagram.config.max_label_width.unwrap_or(360.0).max(1.0);
    let mut content_y = margin_y;
    if let Some(title) = title {
        let title_font_size = diagram.config.title_font_size.unwrap_or(18.0);
        let title_height = (12.0
            + title.lines().count().max(1) as f64 * title_font_size * 1.2)
            .max(32.0);
        items.push(LayoutedTemporalItem::JourneyTitle {
            x: 0.0,
            y: content_y,
            width: cw,
            height: title_height,
            label: title.clone(),
            font_size: diagram.config.title_font_size,
            font_family: diagram.config.title_font_family.clone(),
            color: diagram.config.title_color.clone(),
        });
        content_y += title_height + 4.0;
    }
    let actors = diagram
        .sections
        .iter()
        .flat_map(|section| &section.tasks)
        .flat_map(|task| &task.people)
        .filter(|person| !person.is_empty())
        .cloned()
        .collect::<BTreeSet<_>>();
    let configured_actor_colors = &diagram.config.actor_colors;
    let mut actor_y = content_y + 12.0;
    let actor_colors = actors
        .iter()
        .enumerate()
        .map(|(index, actor)| {
            let color = if configured_actor_colors.is_empty() {
                JOURNEY_ACTOR_COLORS[index % JOURNEY_ACTOR_COLORS.len()].to_string()
            } else {
                configured_actor_colors[index % configured_actor_colors.len()].clone()
            };
            let label = wrap_journey_label(actor, actor_label_width, 14.0);
            let height = (label.lines().count().max(1) as f64 * 18.0).max(20.0);
            items.push(LayoutedTemporalItem::JourneyActor {
                x: margin_x + 8.0,
                y: actor_y + height / 2.0,
                width: actor_label_width,
                height,
                color: color.clone(),
                label,
            });
            actor_y += height + 4.0;
            (actor.clone(), color)
        })
        .collect::<HashMap<_, _>>();
    let section_height = diagram
        .sections
        .iter()
        .map(|section| 12.0 + section.label.lines().count().max(1) as f64 * 16.0)
        .fold(28.0, f64::max);
    let task_y = content_y + section_height + 4.0;
    let activity_y = task_y + task_height + 14.0;
    let total_tasks = diagram.sections.iter().map(|section| section.tasks.len()).sum::<usize>();
    if total_tasks > 0 {
        let first_center = task_x + task_width / 2.0;
        let last_center = first_center + (total_tasks.saturating_sub(1) as f64) * (task_width + task_margin);
        items.push(LayoutedTemporalItem::JourneyActivityLine {
            x1: first_center,
            y: activity_y,
            x2: last_center,
        });
    }
    let mut task_index = 0usize;
    let mut max_score_y = activity_y;
    for (section_index, section) in diagram.sections.iter().enumerate() {
        let fill = if diagram.config.section_fills.is_empty() {
            JOURNEY_SECTION_FILLS[section_index % JOURNEY_SECTION_FILLS.len()].to_string()
        } else {
            diagram.config.section_fills[section_index % diagram.config.section_fills.len()].clone()
        };
        let text_color = if diagram.config.section_colors.is_empty() {
            JOURNEY_SECTION_COLORS[section_index % JOURNEY_SECTION_COLORS.len()].to_string()
        } else {
            diagram.config.section_colors[section_index % diagram.config.section_colors.len()].clone()
        };
        let section_x = task_x + task_index as f64 * (task_width + task_margin);
        let section_width = section.tasks.len() as f64 * task_width
            + section.tasks.len().saturating_sub(1) as f64 * task_margin;
        items.push(LayoutedTemporalItem::JourneySection {
            x: section_x, y: content_y, width: section_width, height: section_height, label: section.label.clone(),
            fill: fill.clone(), text_color: text_color.clone(),
        });
        for task in &section.tasks {
            let x = task_x + task_index as f64 * (task_width + task_margin);
            let score_y = activity_y + 18.0 + (5 - task.score) as f64 * 24.0;
            max_score_y = max_score_y.max(score_y);
            items.push(LayoutedTemporalItem::JourneyTaskLine {
                x: x + task_width / 2.0,
                y1: task_y + task_height,
                y2: score_y,
            });
            items.push(LayoutedTemporalItem::JourneyTask {
                x, y: task_y, width: task_width, height: task_height, score_y,
                score: task.score, label: task.label.clone(), people: task.people.clone(),
                person_colors: task.people.iter().filter_map(|person| actor_colors.get(person).cloned()).collect(),
                font_size: diagram.config.task_font_size,
                font_family: diagram.config.task_font_family.clone(),
                fill: fill.clone(),
                text_color: text_color.clone(),
            });
            task_index += 1;
        }
    }
    let resolved_width = if total_tasks == 0 {
        cw
    } else {
        (task_x + total_tasks as f64 * task_width
            + total_tasks.saturating_sub(1) as f64 * task_margin
            + margin_x)
            .max(cw)
    };
    LayoutedTemporalDiagram {
        width: resolved_width,
        height: (max_score_y + 20.0).max(actor_y + margin_y + 16.0),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        interactions: Vec::new(),
        items,
    }
}

fn wrap_journey_label(label: &str, max_width: f64, font_size: f64) -> String {
    let max_chars = (max_width / (font_size * 0.65)).floor().max(1.0) as usize;
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in label.split_whitespace() {
        if word.chars().count() > max_chars {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let chars = word.chars().collect::<Vec<_>>();
            for chunk in chars.chunks(max_chars) {
                lines.push(chunk.iter().collect());
            }
        } else if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.join("\n")
}

// ── Date helpers ──────────────────────────────────────────────────────────

fn date_to_days(source: &str, format: &GanttDateFormat) -> Option<f64> {
    if format.parts.as_slice() == [GanttDateFormatPart::UnixSeconds] {
        return source.parse::<f64>().ok().map(|seconds| seconds / 86_400.0);
    }
    if format.parts.as_slice() == [GanttDateFormatPart::UnixMilliseconds] {
        return source.parse::<f64>().ok().map(|milliseconds| milliseconds / 86_400_000.0);
    }
    let mut rest = source;
    let seconds_only = format.parts.as_slice() == [GanttDateFormatPart::Second];
    let (mut year, mut month, mut day) = (1970_i64, 1_i64, 1_i64);
    let (mut hour, mut minute, mut second, mut millisecond) = (0_i64, 0_i64, 0_i64, 0_i64);
    let mut hour_is_twelve_hour = false;
    let mut meridiem_is_pm = false;
    let mut timezone_offset_minutes = 0_i64;
    let mut named_weekday = None;
    let mut numeric_weekday = None;
    for part in &format.parts {
        match part {
            GanttDateFormatPart::Literal(literal) => rest = rest.strip_prefix(literal)?,
            GanttDateFormatPart::Year4 => (year, rest) = take_number(rest, 4, 4)?,
            GanttDateFormatPart::Year2 => {
                let (short_year, next) = take_number(rest, 2, 2)?;
                year = if short_year <= 68 { 2000 + short_year } else { 1900 + short_year };
                rest = next;
            }
            GanttDateFormatPart::Quarter => {
                let (quarter, next) = take_number(rest, 1, 1)?;
                if !(1..=4).contains(&quarter) { return None; }
                month = (quarter - 1) * 3 + 1;
                rest = next;
            }
            GanttDateFormatPart::Month => (month, rest) = take_number(rest, 1, 2)?,
            GanttDateFormatPart::Month2 => (month, rest) = take_number(rest, 2, 2)?,
            GanttDateFormatPart::MonthShort | GanttDateFormatPart::MonthLong => {
                let maximum = if matches!(part, GanttDateFormatPart::MonthShort) { 3 } else { 9 };
                let length = rest.bytes().take(maximum).take_while(u8::is_ascii_alphabetic).count();
                if length < 3 { return None; }
                month = month_number(&rest[..length])?;
                rest = &rest[length..];
            }
            GanttDateFormatPart::WeekdayShort => {
                let (weekday, next) = take_letters(rest, 3, 3)?;
                named_weekday = Some(weekday.to_ascii_lowercase());
                rest = next;
            }
            GanttDateFormatPart::WeekdayMin => {
                let (weekday, next) = take_letters(rest, 2, 2)?;
                named_weekday = Some(weekday.to_ascii_lowercase());
                rest = next;
            }
            GanttDateFormatPart::WeekdayNumber => {
                let (weekday, next) = take_number(rest, 1, 1)?;
                if !(0..=6).contains(&weekday) { return None; }
                numeric_weekday = Some(weekday);
                rest = next;
            }
            GanttDateFormatPart::WeekdayLong => {
                let (weekday, next) = take_letters(rest, 6, 9)?;
                named_weekday = Some(weekday.to_ascii_lowercase());
                rest = next;
            }
            GanttDateFormatPart::Day => (day, rest) = take_number(rest, 1, 2)?,
            GanttDateFormatPart::Day2 => (day, rest) = take_number(rest, 2, 2)?,
            GanttDateFormatPart::DayOrdinal => (day, rest) = take_ordinal_day(rest)?,
            GanttDateFormatPart::Hour24Unpadded => (hour, rest) = take_number(rest, 1, 2)?,
            GanttDateFormatPart::Hour24 => (hour, rest) = take_number(rest, 2, 2)?,
            GanttDateFormatPart::Hour12 => {
                (hour, rest) = take_number(rest, 1, 2)?;
                hour_is_twelve_hour = true;
            }
            GanttDateFormatPart::Hour12Padded => {
                (hour, rest) = take_number(rest, 2, 2)?;
                hour_is_twelve_hour = true;
            }
            GanttDateFormatPart::MinuteUnpadded => (minute, rest) = take_number(rest, 1, 2)?,
            GanttDateFormatPart::Minute => (minute, rest) = take_number(rest, 2, 2)?,
            GanttDateFormatPart::SecondUnpadded => (second, rest) = take_number(rest, 1, 2)?,
            GanttDateFormatPart::Second if seconds_only => {
                (second, rest) = take_number(rest, 1, 2)?;
            }
            GanttDateFormatPart::Second => (second, rest) = take_number(rest, 2, 2)?,
            GanttDateFormatPart::Millisecond1 => {
                (millisecond, rest) = take_number(rest, 1, 1)?;
                millisecond *= 100;
            }
            GanttDateFormatPart::Millisecond2 => {
                (millisecond, rest) = take_number(rest, 2, 2)?;
                millisecond *= 10;
            }
            GanttDateFormatPart::Millisecond => (millisecond, rest) = take_number(rest, 3, 3)?,
            GanttDateFormatPart::MeridiemUpper => {
                (meridiem_is_pm, rest) = take_meridiem(rest, true)?;
            }
            GanttDateFormatPart::MeridiemLower => {
                (meridiem_is_pm, rest) = take_meridiem(rest, false)?;
            }
            GanttDateFormatPart::TimezoneOffsetColon => {
                (timezone_offset_minutes, rest) = take_timezone_offset(rest, true)?;
            }
            GanttDateFormatPart::TimezoneOffsetCompact => {
                (timezone_offset_minutes, rest) = take_timezone_offset(rest, false)?;
            }
            GanttDateFormatPart::UnixSeconds | GanttDateFormatPart::UnixMilliseconds => return None,
        }
    }
    if !rest.is_empty() { return None; }
    if hour_is_twelve_hour {
        if !(1..=12).contains(&hour) { return None; }
        hour = hour % 12 + i64::from(meridiem_is_pm) * 12;
    }
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 59 || millisecond > 999 { return None; }
    let civil_day = days_from_civil(year, month, day);
    if let Some(named_weekday) = named_weekday {
        let actual = weekday_name(civil_day);
        if actual != named_weekday && !actual.starts_with(&named_weekday) { return None; }
    }
    if numeric_weekday.is_some_and(|weekday| weekday != (civil_day + 4).rem_euclid(7)) {
        return None;
    }
    let seconds = hour * 3600 + minute * 60 + second;
    Some(civil_day as f64
        + seconds as f64 / 86_400.0 + millisecond as f64 / 86_400_000.0
        - timezone_offset_minutes as f64 / 1_440.0)
}

fn take_letters(source: &str, minimum: usize, maximum: usize) -> Option<(&str, &str)> {
    let length = source.bytes().take(maximum).take_while(u8::is_ascii_alphabetic).count();
    if length < minimum { return None; }
    Some((&source[..length], &source[length..]))
}

fn take_ordinal_day(source: &str) -> Option<(i64, &str)> {
    let (day, rest) = take_number(source, 1, 2)?;
    if !(1..=31).contains(&day) { return None; }
    Some((day, rest.strip_prefix(ordinal_suffix(day))?))
}

fn ordinal_suffix(value: i64) -> &'static str {
    if (11..=13).contains(&(value % 100)) { return "th"; }
    match value % 10 { 1 => "st", 2 => "nd", 3 => "rd", _ => "th" }
}

fn take_meridiem(source: &str, uppercase: bool) -> Option<(bool, &str)> {
    let (am, pm) = if uppercase { ("AM", "PM") } else { ("am", "pm") };
    if let Some(rest) = source.strip_prefix(am) { return Some((false, rest)); }
    source.strip_prefix(pm).map(|rest| (true, rest))
}

fn take_timezone_offset(source: &str, colon: bool) -> Option<(i64, &str)> {
    if let Some(rest) = source.strip_prefix('Z') { return Some((0, rest)); }
    let sign = match source.as_bytes().first()? { b'+' => 1, b'-' => -1, _ => return None };
    let (hour, rest) = take_number(&source[1..], 2, 2)?;
    let rest = if colon { rest.strip_prefix(':')? } else { rest };
    let (minute, rest) = take_number(rest, 2, 2)?;
    if hour > 23 || minute > 59 { return None; }
    Some((sign * (hour * 60 + minute), rest))
}

fn take_number(source: &str, minimum: usize, maximum: usize) -> Option<(i64, &str)> {
    let length = source.bytes().take(maximum).take_while(u8::is_ascii_digit).count();
    if length < minimum { return None; }
    Some((source[..length].parse().ok()?, &source[length..]))
}

fn month_number(source: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = ["january", "february", "march", "april", "may", "june",
        "july", "august", "september", "october", "november", "december"];
    let lowercase = source.to_ascii_lowercase();
    MONTHS.iter().position(|month| *month == lowercase || month.starts_with(&lowercase))
        .map(|index| index as i64 + 1)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}

// Proleptic Gregorian conversion, anchored to the Unix epoch.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let zero_day = days + 719_468;
    let era = zero_day.div_euclid(146_097);
    let day_of_era = zero_day - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn weekday_name(days: i64) -> &'static str {
    const WEEKDAYS: [&str; 7] = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
    WEEKDAYS[(days + 3).rem_euclid(7) as usize]
}

fn gantt_date_is_excluded(day: i64, diagram: &diagram_ir::GanttDiagram) -> bool {
    let (year, month, date) = civil_from_days(day);
    let iso_date = format!("{year:04}-{month:02}-{date:02}");
    if diagram.config.includes.iter().any(|value| value == &iso_date) {
        return false;
    }
    let weekday = weekday_name(day);
    let weekend_start = diagram.config.weekend.as_deref().unwrap_or("saturday");
    let is_weekend = match weekend_start {
        "friday" => matches!(weekday, "friday" | "saturday"),
        _ => matches!(weekday, "saturday" | "sunday"),
    };
    diagram.config.excludes.iter().any(|value| {
        value == &iso_date || value == weekday || (value == "weekends" && is_weekend)
    })
}

fn gantt_elapsed_duration(start: f64, duration: f64, diagram: &diagram_ir::GanttDiagram) -> f64 {
    if duration < 1.0 || diagram.config.excludes.is_empty() {
        return duration;
    }
    let whole_days = duration.floor() as i64;
    let fraction = duration - whole_days as f64;
    let mut elapsed = 0_i64;
    let mut consumed = 0_i64;
    while consumed < whole_days && elapsed < 10_000 {
        elapsed += 1;
        if !gantt_date_is_excluded(start as i64 + elapsed, diagram) {
            consumed += 1;
        }
    }
    elapsed as f64 + fraction
}

fn gantt_task_elapsed_duration(
    start: f64,
    task: &diagram_ir::GanttTask,
    diagram: &diagram_ir::GanttDiagram,
    starts: &HashMap<String, f64>,
) -> f64 {
    match &task.end {
        Some(TaskEnd::Date(end_date)) => {
            if let Some(end_date) = date_to_days(end_date, &diagram.date_format) {
                return (end_date - start + f64::from(diagram.config.inclusive_end_dates)).max(0.0);
            }
        }
        Some(TaskEnd::Until(ids)) => {
            if let Some(end) = ids.iter().filter_map(|id| starts.get(id)).copied().reduce(f64::min) {
                return (end - start).max(0.0);
            }
        }
        None => {}
    }
    gantt_elapsed_duration(start, task.duration.days(), diagram)
}

fn gantt_task_end_if_resolved(
    start: f64,
    task: &diagram_ir::GanttTask,
    diagram: &diagram_ir::GanttDiagram,
    starts: &HashMap<String, f64>,
) -> Option<f64> {
    if let Some(TaskEnd::Until(ids)) = &task.end {
        let referenced_starts = ids
            .iter()
            .map(|id| starts.get(id).copied())
            .collect::<Option<Vec<_>>>()?;
        return referenced_starts.into_iter().reduce(f64::min);
    }
    Some(start + gantt_task_elapsed_duration(start, task, diagram, starts))
}

fn gantt_tick_days(interval: Option<&str>) -> f64 {
    let Some(interval) = interval else { return TICK_DAYS; };
    let compact = interval.trim().to_ascii_lowercase().replace(' ', "");
    let split = compact.find(|character: char| !character.is_ascii_digit()).unwrap_or(compact.len());
    let count = compact[..split].parse::<f64>().unwrap_or(1.0).max(1.0);
    match &compact[split..] {
        "day" | "days" => count,
        "week" | "weeks" => count * 7.0,
        "month" | "months" => count * 30.0,
        "year" | "years" => count * 365.0,
        _ => TICK_DAYS,
    }
}

fn gantt_axis_label(day: i64, format: Option<&str>, offset: f64) -> String {
    let Some(format) = format else { return format!("d{offset:.0}"); };
    let (year, month, date) = civil_from_days(day);
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format
        .replace("%Y", &format!("{year:04}"))
        .replace("%m", &format!("{month:02}"))
        .replace("%d", &format!("{date:02}"))
        .replace("%b", MONTHS[(month - 1) as usize])
}

fn append_gantt_axis(
    items: &mut Vec<LayoutedTemporalItem>,
    y: f64,
    t_min: f64,
    t_range: f64,
    x_scale: f64,
    diagram: &diagram_ir::GanttDiagram,
    label_above: bool,
) {
    items.push(LayoutedTemporalItem::TimeAxisSpine {
        x1: LABEL_W, y1: y, x2: LABEL_W + t_range * x_scale, y2: y,
    });
    let mut tick_day = 0.0;
    let tick_days = gantt_tick_days(diagram.config.tick_interval.as_deref());
    while tick_day <= t_range {
        items.push(LayoutedTemporalItem::TimeAxisTick {
            x: LABEL_W + tick_day * x_scale,
            y: if label_above { y } else { y + 4.0 },
            label: gantt_axis_label(
                (t_min + tick_day) as i64,
                diagram.config.axis_format.as_deref(),
                tick_day,
            ),
            label_above,
        });
        tick_day += tick_days;
    }
}

fn today_marker_style(source: Option<&str>) -> (String, f64, Option<Vec<f64>>) {
    let mut stroke = "#ef4444".to_string();
    let mut stroke_width = 2.0;
    let mut stroke_dash = Some(vec![6.0, 3.0]);
    if let Some(source) = source {
        for declaration in source.split([',', ';']) {
            let Some((name, value)) = declaration.split_once(':') else { continue };
            match name.trim().to_ascii_lowercase().as_str() {
                "stroke" => stroke = value.trim().to_string(),
                "stroke-width" => {
                    if let Ok(width) = value.trim().trim_end_matches("px").parse::<f64>() {
                        stroke_width = width.max(0.0);
                    }
                }
                "stroke-dasharray" if value.trim().eq_ignore_ascii_case("none") => stroke_dash = None,
                "stroke-dasharray" => {
                    let dash = value.split_whitespace().filter_map(|part| part.parse().ok()).collect::<Vec<_>>();
                    if !dash.is_empty() { stroke_dash = Some(dash); }
                }
                _ => {}
            }
        }
    }
    (stroke, stroke_width, stroke_dash)
}

fn current_epoch_day() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_secs() / 86_400) as i64)
        .unwrap_or(0)
}

// ── Gantt layout ──────────────────────────────────────────────────────────

fn layout_gantt(
    title: &Option<String>,
    diagram: &diagram_ir::GanttDiagram,
    cw: f64,
) -> LayoutedTemporalDiagram {
    // First pass: collect all task absolute starts (Date variant only).
    let mut starts: HashMap<String, f64> = HashMap::new();
    for section in &diagram.sections {
        for task in &section.tasks {
            if let TaskStart::Date(ref ds) = task.start {
                if let Some(d) = date_to_days(ds, &diagram.date_format) {
                    starts.insert(task.id.clone(), d);
                }
            }
        }
    }
    let tasks = diagram.sections.iter().flat_map(|section| section.tasks.iter()).collect::<Vec<_>>();
    // Resolve dependency chains to a fixed point. Multi-source `after` starts at
    // the latest dependency end, matching Mermaid's scheduling semantics.
    for _ in 0..tasks.len() {
        let mut changed = false;
        for task in &tasks {
            if starts.contains_key(&task.id) { continue; }
            let TaskStart::After(dependency_ids) = &task.start else { continue; };
            let dependency_ends = dependency_ids.iter().map(|dependency_id| {
                let dependency = tasks.iter().find(|candidate| candidate.id == *dependency_id)?;
                let dependency_start = *starts.get(dependency_id)?;
                gantt_task_end_if_resolved(dependency_start, dependency, diagram, &starts)
            }).collect::<Option<Vec<_>>>();
            if let Some(dependency_ends) = dependency_ends {
                if let Some(start) = dependency_ends.into_iter().reduce(f64::max) {
                    starts.insert(task.id.clone(), start);
                    changed = true;
                }
            }
        }
        if !changed { break; }
    }

    // Determine time range.
    let t_min = starts.values().cloned().fold(f64::INFINITY, f64::min);
    let t_max = diagram.sections.iter()
        .flat_map(|s| s.tasks.iter())
        .filter_map(|t| starts.get(&t.id).map(|&s| s + gantt_task_elapsed_duration(s, t, diagram, &starts)))
        .fold(f64::NEG_INFINITY, f64::max);
    let t_min = if t_min.is_infinite() { 0.0 } else { t_min };
    let t_max = if t_max.is_infinite() { t_min + 30.0 } else { t_max };
    // Keep sub-day schedules legible instead of forcing their scale into a
    // one-day minimum range. One millisecond is Mermaid's finest duration unit.
    let t_range = (t_max - t_min).max(1.0 / 86_400_000.0);

    let plot_w = (cw - LABEL_W - 32.0).max(100.0);
    let x_scale = plot_w / t_range;

    let mut items: Vec<LayoutedTemporalItem> = Vec::new();
    let mut interactions = Vec::new();
    let mut y = AXIS_H;

    // Title
    if let Some(ref t) = title {
        let title_height = multiline_height(t, AXIS_H, 20.0);
        items.push(LayoutedTemporalItem::SectionHeader {
            x: 0.0, y: 0.0, width: cw, height: title_height, label: t.clone(),
        });
        y += title_height;
    }

    if diagram.config.top_axis {
        append_gantt_axis(&mut items, y + AXIS_H - 4.0, t_min, t_range, x_scale, diagram, true);
        y += AXIS_H;
    }
    let marker_top = y;

    let mut vertical_markers = Vec::new();
    // Sections and tasks.
    for section in &diagram.sections {
        if let Some(ref lbl) = section.label {
            let section_height = multiline_height(lbl, SECTION_H, 20.0);
            items.push(LayoutedTemporalItem::SectionHeader {
                x: 0.0, y, width: cw, height: section_height, label: lbl.clone(),
            });
            y += section_height;
        }
        for task in &section.tasks {
            let task_height = multiline_height(&task.label, TASK_H, 14.0);
            let start_day = starts.get(&task.id).copied().unwrap_or(t_min) - t_min;
            let bx = LABEL_W + start_day * x_scale;
            let elapsed_duration = gantt_task_elapsed_duration(t_min + start_day, task, diagram, &starts);
            let bw = (elapsed_duration * x_scale).max(4.0);
            if task.tags.vertical {
                vertical_markers.push((
                    bx,
                    task.id.clone(),
                    task.label.clone(),
                    task.link.clone(),
                    task.callback.clone(),
                    task.callback_args.clone(),
                ));
                continue;
            }
            if task.tags.milestone {
                items.push(LayoutedTemporalItem::MilestoneMarker {
                    x: bx, y: y + task_height / 2.0, tags: task.tags.clone(), label: task.label.clone(),
                });
            } else {
                items.push(LayoutedTemporalItem::TaskBar {
                    x: bx, y, width: bw, height: task_height,
                    tags: task.tags.clone(),
                    label: task.label.clone(),
                });
            }
            if task.link.is_some() || task.callback.is_some() {
                interactions.push(LayoutedTemporalInteraction {
                    task_id: task.id.clone(),
                    bounds: (bx, y, bw.max(16.0), task_height),
                    link: task.link.clone(),
                    callback: task.callback.clone(),
                    callback_args: task.callback_args.clone(),
                });
            }
            y += task_height + TASK_GAP;
        }
    }

    let chart_bottom = y;
    let vertical_label_height = if vertical_markers.is_empty() { 0.0 } else { 20.0 };
    for (x, task_id, label, link, callback, callback_args) in vertical_markers {
        items.push(LayoutedTemporalItem::VerticalMarker {
            x,
            y1: marker_top,
            y2: chart_bottom,
            label,
        });
        if link.is_some() || callback.is_some() {
            interactions.push(LayoutedTemporalInteraction {
                task_id,
                bounds: (x - 8.0, marker_top, 16.0, chart_bottom - marker_top),
                link,
                callback,
                callback_args,
            });
        }
    }
    append_gantt_axis(
        &mut items,
        y + vertical_label_height + 4.0,
        t_min,
        t_range,
        x_scale,
        diagram,
        false,
    );
    y += AXIS_H + vertical_label_height;

    if diagram.config.today_marker.as_deref() != Some("off") {
        let today = current_epoch_day() as f64;
        if (t_min..=t_max).contains(&today) {
            let (stroke, stroke_width, stroke_dash) =
                today_marker_style(diagram.config.today_marker.as_deref());
            items.push(LayoutedTemporalItem::TodayMarker {
                x: LABEL_W + (today - t_min) * x_scale,
                y1: marker_top,
                y2: chart_bottom,
                stroke,
                stroke_width,
                stroke_dash,
            });
        }
    }

    LayoutedTemporalDiagram {
        width: cw, height: y + 16.0,
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        interactions,
        items,
    }
}

// ── Git layout ────────────────────────────────────────────────────────────

fn layout_git(diagram: &GitDiagram, cw: f64) -> LayoutedTemporalDiagram {
    let mut items: Vec<LayoutedTemporalItem> = Vec::new();
    let title_offset = if let Some(title) = &diagram.title {
        items.push(LayoutedTemporalItem::TemporalTitle {
            x: 0.0,
            y: 0.0,
            width: cw,
            height: 40.0,
            label: title.clone(),
        });
        40.0
    } else {
        0.0
    };
    let mut branch_lanes: HashMap<String, usize> = HashMap::new();
    let mut commit_positions: HashMap<String, (f64, f64)> = HashMap::new();
    let mut current_branch = "main".to_string();

    // Mermaid gives unordered branches stable fractional keys (0.0, 0.1, ...)
    // before sorting them together with explicit integer orders.
    let mut ordered_branches = diagram.branches.iter().enumerate().collect::<Vec<_>>();
    ordered_branches.sort_by(|(left_index, left), (right_index, right)| {
        mermaid_branch_order(left.order, *left_index)
            .total_cmp(&mermaid_branch_order(right.order, *right_index))
    });
    if ordered_branches.is_empty() {
        branch_lanes.insert("main".into(), 0);
    } else {
        for (lane, (_, branch)) in ordered_branches.iter().enumerate() {
            branch_lanes.insert(branch.name.clone(), lane);
        }
    }
    let mut next_lane = ordered_branches.len().max(1);

    let commit_count = diagram.events.iter().filter(|event| matches!(
        event,
        GitEvent::Commit { .. } | GitEvent::Merge { .. } | GitEvent::CherryPick { .. }
    )).count().max(1);
    let vertical = matches!(diagram.direction, DiagramDirection::Tb | DiagramDirection::Bt);
    let lane_position = |lane: usize| -> f64 {
        if vertical {
            60.0 + lane as f64 * LANE_H
        } else {
            title_offset + 30.0 + lane as f64 * LANE_H
        }
    };
    let progress_extent = 60.0 + (commit_count - 1) as f64 * COMMIT_SPACING + 60.0;
    let width = if vertical {
        cw.max(lane_position(next_lane - 1) + LANE_H)
    } else {
        cw.max(progress_extent)
    };
    let height = if vertical {
        title_offset + progress_extent
    } else {
        lane_position(next_lane - 1) + LANE_H
    };
    let point = |lane: usize, progress_index: usize| -> (f64, f64) {
        let progress = 60.0 + progress_index as f64 * COMMIT_SPACING;
        match diagram.direction {
            DiagramDirection::Tb => (lane_position(lane), title_offset + progress),
            DiagramDirection::Bt => (lane_position(lane), height - progress),
            DiagramDirection::Rl => (width - progress, lane_position(lane)),
            DiagramDirection::Lr => (progress, lane_position(lane)),
        }
    };

    // Emit labels in lane order so PaintScene instruction order is deterministic.
    let mut labeled_lanes = branch_lanes.iter().collect::<Vec<_>>();
    labeled_lanes.sort_by_key(|(_, lane)| **lane);
    for (name, &lane) in labeled_lanes {
        let color = BRANCH_COLORS[lane % BRANCH_COLORS.len()].to_string();
        let position = lane_position(lane);
        let (x1, y1, x2, y2, label_x, label_y) = if vertical {
            (position, title_offset, position, height, position - 28.0, title_offset + 4.0)
        } else {
            (0.0, position, width, position, 4.0, position - 7.0)
        };
        items.push(LayoutedTemporalItem::BranchLane {
            x1, y1, x2, y2,
            label_x, label_y,
            label_width: 56.0,
            label_height: 16.0,
            color,
            label: name.clone(),
        });
    }

    let mut progress_index = 0_usize;
    for event in &diagram.events {
        match event {
            GitEvent::Commit { id, resolved_id, message, tags, branch, type_, .. } => {
                let lane = *branch_lanes.entry(branch.clone()).or_insert_with(|| {
                    let l = next_lane; next_lane += 1; l
                });
                let (x, y) = point(lane, progress_index);
                let commit_id = id.clone().unwrap_or_else(|| resolved_id.clone());
                commit_positions.insert(resolved_id.clone(), (x, y));
                items.push(LayoutedTemporalItem::CommitNode {
                    x, y,
                    id: commit_id,
                    message: message.clone(),
                    tags: tags.clone(),
                    symbol: git_commit_symbol(type_),
                });
                progress_index += 1;
            }
            GitEvent::Checkout { branch } => {
                current_branch = branch.clone();
                // Ensure lane exists.
                branch_lanes.entry(branch.clone()).or_insert_with(|| {
                    let l = next_lane; next_lane += 1; l
                });
            }
            GitEvent::Merge { from, id, resolved_id, parents, tags, type_ } => {
                let from_lane = *branch_lanes.get(from).unwrap_or(&0);
                let to_lane   = *branch_lanes.get(&current_branch).unwrap_or(&0);
                let (from_x, from_y) = parents
                    .get(1)
                    .and_then(|parent| commit_positions.get(parent))
                    .copied()
                    .unwrap_or_else(|| point(from_lane, progress_index.saturating_sub(1)));
                let (to_x, to_y) = point(to_lane, progress_index);
                items.push(LayoutedTemporalItem::GitHistoryArc {
                    from_x, from_y, to_x, to_y,
                });
                // The merge itself is a commit on the target branch.
                let commit_id = id.clone().unwrap_or_else(|| resolved_id.clone());
                commit_positions.insert(resolved_id.clone(), (to_x, to_y));
                items.push(LayoutedTemporalItem::CommitNode {
                    x: to_x, y: to_y,
                    id: commit_id,
                    message: Some(format!("merge {from}")),
                    tags: tags.clone(),
                    symbol: if *type_ == GitCommitType::Normal {
                        GitCommitSymbol::Merge
                    } else {
                        git_commit_symbol(type_)
                    },
                });
                progress_index += 1;
            }
            GitEvent::CherryPick { id, resolved_id, parents, tags, parent, branch } => {
                let lane = *branch_lanes.entry(branch.clone()).or_insert_with(|| {
                    let l = next_lane; next_lane += 1; l
                });
                let (x, y) = point(lane, progress_index);
                if let Some((from_x, from_y)) = parents
                    .get(1)
                    .and_then(|source| commit_positions.get(source))
                    .copied()
                {
                    items.push(LayoutedTemporalItem::GitHistoryArc {
                        from_x, from_y, to_x: x, to_y: y,
                    });
                }
                commit_positions.insert(resolved_id.clone(), (x, y));
                items.push(LayoutedTemporalItem::CommitNode {
                    x, y,
                    id: resolved_id.clone(),
                    message: Some(match parent {
                        Some(parent) => format!("cherry-pick {id} from {parent}"),
                        None => format!("cherry-pick {id}"),
                    }),
                    tags: tags.clone(),
                    symbol: GitCommitSymbol::CherryPick,
                });
                progress_index += 1;
            }
        }
    }

    LayoutedTemporalDiagram {
        width, height,
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        interactions: Vec::new(),
        items,
    }
}

fn git_commit_symbol(type_: &GitCommitType) -> GitCommitSymbol {
    match type_ {
        GitCommitType::Normal => GitCommitSymbol::Normal,
        GitCommitType::Reverse => GitCommitSymbol::Reverse,
        GitCommitType::Highlight => GitCommitSymbol::Highlight,
    }
}

fn mermaid_branch_order(explicit_order: Option<i64>, declaration_index: usize) -> f64 {
    explicit_order.map(|order| order as f64).unwrap_or_else(|| {
        format!("0.{declaration_index}")
            .parse()
            .expect("fractional Mermaid branch order is valid")
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::*;

    fn simple_gantt() -> TemporalDiagram {
        TemporalDiagram {
            kind: TemporalKind::Gantt,
            title: Some("Project".into()),
            body: TemporalBody::Gantt(GanttDiagram {
                title: Some("Project".into()),
                accessibility_title: None,
                accessibility_description: None,
                date_format: GanttDateFormat::default(),
                config: GanttConfig::default(),
                sections: vec![GanttSection {
                    label: Some("Phase 1".into()),
                    tasks: vec![
                        GanttTask {
                            id: "t1".into(), label: "Design".into(),
                            start: TaskStart::Date("2026-01-01".into()),
                            duration: GanttDuration { value: 5.0, unit: GanttDurationUnit::Days },
                            end: None,
                            tags: GanttTaskTags { done: true, ..GanttTaskTags::default() },
                            dependencies: vec![],
                            link: None,
                            callback: None,
                            callback_args: None,
                        },
                        GanttTask {
                            id: "t2".into(), label: "Build".into(),
                            start: TaskStart::After(vec!["t1".into()]),
                            duration: GanttDuration { value: 3.0, unit: GanttDurationUnit::Days },
                            end: None,
                            tags: GanttTaskTags { active: true, ..GanttTaskTags::default() },
                            dependencies: vec!["t1".into()],
                            link: None,
                            callback: None,
                            callback_args: None,
                        },
                    ],
                }],
            }),
        }
    }

    fn simple_git() -> TemporalDiagram {
        TemporalDiagram {
            kind: TemporalKind::Git,
            title: Some("Release history".into()),
            body: TemporalBody::Git(GitDiagram {
                title: Some("Release history".into()),
                accessibility_title: Some("Accessible release history".into()),
                accessibility_description: Some("Two commits on main".into()),
                direction: DiagramDirection::Lr,
                branches: vec![GitBranch { name: "main".into(), order: None }],
                events: vec![
                    GitEvent::Commit {
                        id: Some("a1".into()), resolved_id: "a1".into(), parents: Vec::new(),
                        message: Some("init".into()),
                        tags: vec!["v1".into(), "stable".into()], branch: "main".into(), type_: GitCommitType::Normal,
                    },
                    GitEvent::Commit {
                        id: Some("a2".into()), resolved_id: "a2".into(), parents: vec!["a1".into()],
                        message: Some("feature".into()),
                        tags: Vec::new(), branch: "main".into(), type_: GitCommitType::Normal,
                    },
                ],
            }),
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(crate::VERSION, "0.34.0");
    }

    #[test]
    fn journey_actor_labels_wrap_to_configured_width() {
        assert_eq!(
            wrap_journey_label("Alice Wonderland", 56.0, 14.0),
            "Alice\nWonder\nland"
        );
    }

    #[test]
    fn gantt_has_task_bars() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        let bars = d.items.iter().filter(|i| matches!(i, LayoutedTemporalItem::TaskBar{..})).count();
        assert_eq!(bars, 2);
    }

    #[test]
    fn gantt_has_axis_spine() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        assert!(d.items.iter().any(|i| matches!(i, LayoutedTemporalItem::TimeAxisSpine{..})));
    }

    #[test]
    fn after_dependency_resolves_correctly() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        // t2 starts after t1 ends (5 days in), so its bar x should be > t1's bar x.
        let bars: Vec<_> = d.items.iter().filter_map(|i| {
            if let LayoutedTemporalItem::TaskBar { x, label, .. } = i { Some((*x, label.clone())) } else { None }
        }).collect();
        let t1_x = bars.iter().find(|(_, l)| l == "Design").unwrap().0;
        let t2_x = bars.iter().find(|(_, l)| l == "Build").unwrap().0;
        assert!(t2_x > t1_x, "Build should start after Design");
    }

    #[test]
    fn gantt_has_section_header() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        assert!(d.items.iter().any(|i| {
            if let LayoutedTemporalItem::SectionHeader { label, .. } = i {
                label == "Phase 1"
            } else { false }
        }));
    }

    #[test]
    fn gantt_multiline_labels_reserve_backend_neutral_rows() {
        let mut diagram = simple_gantt();
        diagram.title = Some("Release\nPlan".into());
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.sections[0].label = Some("Build\nCore".into());
        gantt.sections[0].tasks[0].label = "Parser\nCore".into();

        let layout = layout_temporal_diagram(&diagram, 800.0);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::SectionHeader { label, height, .. }
                if label == "Release\nPlan" && *height == 48.0
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::SectionHeader { label, height, .. }
                if label == "Build\nCore" && *height == 44.0
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::TaskBar { label, height, .. }
                if label == "Parser\nCore" && *height == 34.0
        )));
    }

    #[test]
    fn gantt_layout_preserves_accessibility_metadata() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else {
            unreachable!();
        };
        gantt.accessibility_title = Some("Accessible project".into());
        gantt.accessibility_description = Some("Two delivery tasks".into());
        let layout = layout_temporal_diagram(&diagram, 800.0);
        assert_eq!(layout.accessibility_title.as_deref(), Some("Accessible project"));
        assert_eq!(
            layout.accessibility_description.as_deref(),
            Some("Two delivery tasks")
        );
    }

    #[test]
    fn gantt_layout_resolves_task_interaction_bounds() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else {
            unreachable!();
        };
        gantt.sections[0].tasks[0].link = Some("https://example.com/design".into());
        gantt.sections[0].tasks[0].callback = Some("inspectTask".into());
        let layout = layout_temporal_diagram(&diagram, 800.0);
        let interaction = &layout.interactions[0];
        assert_eq!(interaction.task_id, "t1");
        assert_eq!(interaction.link.as_deref(), Some("https://example.com/design"));
        assert!(interaction.bounds.2 > 0.0);
        assert!(interaction.bounds.3 > 0.0);
    }

    #[test]
    fn gantt_calendar_exclusions_extend_bars_and_dependency_starts() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.config.excludes = vec!["weekends".into()];
        gantt.sections[0].tasks[0].start = TaskStart::Date("2026-01-02".into());
        gantt.sections[0].tasks[0].duration = GanttDuration { value: 2.0, unit: GanttDurationUnit::Days };
        gantt.sections[0].tasks[1].start = TaskStart::After(vec!["t1".into()]);
        let second_duration = gantt.sections[0].tasks[1].duration.days();

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let bars = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TaskBar { x, width, .. } => Some((*x, *width)),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(bars.len(), 2);
        assert!(bars[0].1 > bars[1].1 / second_duration * 2.0);
        assert!((bars[1].0 - (bars[0].0 + bars[0].1)).abs() < 0.01);
    }

    #[test]
    fn gantt_axis_uses_configured_format_and_interval() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.config.axis_format = Some("%m/%d".into());
        gantt.config.tick_interval = Some("1day".into());
        let layout = layout_temporal_diagram(&diagram, 800.0);
        let labels = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TimeAxisTick { label, .. } => Some(label.as_str()),
            _ => None,
        }).collect::<Vec<_>>();
        assert!(labels.len() > 2);
        assert!(labels[0].contains('/'));
    }

    #[test]
    fn gantt_layout_resolves_typed_non_iso_dates_and_times() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.date_format = GanttDateFormat {
            source: "DD MMM YYYY HH:mm".into(),
            parts: vec![GanttDateFormatPart::Day2, GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::MonthShort, GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::Year4, GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::Hour24, GanttDateFormatPart::Literal(":".into()),
                GanttDateFormatPart::Minute],
        };
        gantt.sections[0].tasks[0].start = TaskStart::Date("02 Jan 2026 06:00".into());
        gantt.sections[0].tasks[0].end = Some(TaskEnd::Date("02 Jan 2026 18:00".into()));
        gantt.sections[0].tasks[1].start = TaskStart::Date("03 Jan 2026 06:00".into());
        gantt.sections[0].tasks[1].duration = GanttDuration { value: 0.5, unit: GanttDurationUnit::Days };
        let layout = layout_temporal_diagram(&diagram, 800.0);
        let widths = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TaskBar { width, .. } => Some(*width), _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(widths.len(), 2);
        assert!((widths[0] - widths[1]).abs() < 0.01);
    }

    #[test]
    fn gantt_layout_resolves_unpadded_time_fields() {
        let format = GanttDateFormat {
            source: "YYYY-MM-DD H:m:s".into(),
            parts: vec![
                GanttDateFormatPart::Year4,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Month2,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Day2,
                GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::Hour24Unpadded,
                GanttDateFormatPart::Literal(":".into()),
                GanttDateFormatPart::MinuteUnpadded,
                GanttDateFormatPart::Literal(":".into()),
                GanttDateFormatPart::SecondUnpadded,
            ],
        };
        let first = date_to_days("2026-03-01 4:5:6", &format).expect("valid unpadded time");
        let second = date_to_days("2026-03-01 4:5:7", &format).expect("valid unpadded time");
        assert!(((second - first) * 86_400.0 - 1.0).abs() < 0.000_001);
    }

    #[test]
    fn gantt_layout_resolves_quarter_dates() {
        let format = GanttDateFormat {
            source: "YYYY-Q".into(),
            parts: vec![
                GanttDateFormatPart::Year4,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Quarter,
            ],
        };
        let q1 = date_to_days("2026-1", &format).expect("valid first quarter");
        let q2 = date_to_days("2026-2", &format).expect("valid second quarter");
        assert_eq!(q2 - q1, 90.0);
        assert!(date_to_days("2026-5", &format).is_none());
    }

    #[test]
    fn gantt_layout_normalizes_typed_timezone_offsets() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.date_format = GanttDateFormat {
            source: "YYYY-MM-DD[T]HH:mmZ".into(),
            parts: vec![
                GanttDateFormatPart::Year4,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Month2,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Day2,
                GanttDateFormatPart::Literal("T".into()),
                GanttDateFormatPart::Hour24,
                GanttDateFormatPart::Literal(":".into()),
                GanttDateFormatPart::Minute,
                GanttDateFormatPart::TimezoneOffsetColon,
            ],
        };
        gantt.sections[0].tasks[0].start = TaskStart::Date("2026-01-02T12:00+02:00".into());
        gantt.sections[0].tasks[0].duration = GanttDuration { value: 2.0, unit: GanttDurationUnit::Hours };
        gantt.sections[0].tasks[1].start = TaskStart::Date("2026-01-02T10:00Z".into());
        gantt.sections[0].tasks[1].duration = GanttDuration { value: 2.0, unit: GanttDurationUnit::Hours };

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let bars = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TaskBar { x, width, .. } => Some((*x, *width)),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(bars.len(), 2);
        assert!((bars[0].0 - bars[1].0).abs() < 0.01);
        assert!((bars[0].1 - bars[1].1).abs() < 0.01);
    }

    #[test]
    fn gantt_layout_resolves_seconds_only_ranges() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.date_format = GanttDateFormat {
            source: "ss".into(),
            parts: vec![GanttDateFormatPart::Second],
        };
        gantt.sections[0].tasks[0].start = TaskStart::Date("0".into());
        gantt.sections[0].tasks[0].end = Some(TaskEnd::Date("20".into()));
        gantt.sections[0].tasks[1].start = TaskStart::After(vec!["t1".into()]);
        gantt.sections[0].tasks[1].duration = GanttDuration { value: 5.0, unit: GanttDurationUnit::Seconds };

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let widths = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TaskBar { width, .. } => Some(*width),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(widths.len(), 2);
        assert!((widths[0] / widths[1] - 4.0).abs() < 0.01);
    }

    #[test]
    fn gantt_layout_resolves_twelve_hour_clocks() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.date_format = GanttDateFormat {
            source: "YYYY-MM-DD h:mm A".into(),
            parts: vec![
                GanttDateFormatPart::Year4,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Month2,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Day2,
                GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::Hour12,
                GanttDateFormatPart::Literal(":".into()),
                GanttDateFormatPart::Minute,
                GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::MeridiemUpper,
            ],
        };
        gantt.sections[0].tasks[0].start = TaskStart::Date("2026-03-01 9:30 AM".into());
        gantt.sections[0].tasks[0].duration = GanttDuration { value: 1.0, unit: GanttDurationUnit::Hours };
        gantt.sections[0].tasks[1].start = TaskStart::Date("2026-03-01 2:30 PM".into());
        gantt.sections[0].tasks[1].duration = GanttDuration { value: 1.0, unit: GanttDurationUnit::Hours };

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let bars = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TaskBar { x, width, .. } => Some((*x, *width)),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(bars.len(), 2);
        assert!((bars[0].1 - bars[1].1).abs() < 0.01);
        assert!((bars[1].0 - bars[0].0) / bars[0].1 > 4.9);
    }

    #[test]
    fn gantt_layout_validates_named_weekdays() {
        let format = GanttDateFormat {
            source: "ddd YYYY-MM-DD".into(),
            parts: vec![
                GanttDateFormatPart::WeekdayShort,
                GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::Year4,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Month2,
                GanttDateFormatPart::Literal("-".into()),
                GanttDateFormatPart::Day2,
            ],
        };
        let sunday = date_to_days("Sun 2026-03-01", &format).expect("valid Sunday");
        let monday = date_to_days("Mon 2026-03-02", &format).expect("valid Monday");
        assert_eq!(monday - sunday, 1.0);
        assert!(date_to_days("Mon 2026-03-01", &format).is_none());

        let minimum = GanttDateFormat {
            source: "dd YYYY-MM-DD".into(),
            parts: [GanttDateFormatPart::WeekdayMin]
                .into_iter()
                .chain(format.parts[1..].iter().cloned())
                .collect(),
        };
        assert!(date_to_days("Su 2026-03-01", &minimum).is_some());
        assert!(date_to_days("Mo 2026-03-01", &minimum).is_none());

        let numeric = GanttDateFormat {
            source: "d YYYY-MM-DD".into(),
            parts: [GanttDateFormatPart::WeekdayNumber]
                .into_iter()
                .chain(format.parts[1..].iter().cloned())
                .collect(),
        };
        assert!(date_to_days("0 2026-03-01", &numeric).is_some());
        assert!(date_to_days("1 2026-03-01", &numeric).is_none());
    }

    #[test]
    fn gantt_layout_scales_fractional_second_precision() {
        for (part, value, expected_milliseconds) in [
            (GanttDateFormatPart::Millisecond1, "5", 500.0),
            (GanttDateFormatPart::Millisecond2, "25", 250.0),
            (GanttDateFormatPart::Millisecond, "125", 125.0),
        ] {
            let format = GanttDateFormat { source: "fraction".into(), parts: vec![part] };
            let days = date_to_days(value, &format).expect("valid fractional second");
            assert!((days * 86_400_000.0 - expected_milliseconds).abs() < 0.001);
        }
    }

    #[test]
    fn gantt_layout_resolves_ordinal_days() {
        let format = GanttDateFormat {
            source: "Do MMMM YYYY".into(),
            parts: vec![
                GanttDateFormatPart::DayOrdinal,
                GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::MonthLong,
                GanttDateFormatPart::Literal(" ".into()),
                GanttDateFormatPart::Year4,
            ],
        };
        let first = date_to_days("1st March 2026", &format).expect("valid ordinal day");
        let second = date_to_days("2nd March 2026", &format).expect("valid ordinal day");
        assert_eq!(second - first, 1.0);
        assert!(date_to_days("11st March 2026", &format).is_none());
    }

    #[test]
    fn gantt_layout_resolves_unix_millisecond_dates() {
        let format = GanttDateFormat { source: "x".into(),
            parts: vec![GanttDateFormatPart::UnixMilliseconds] };
        assert_eq!(date_to_days("0", &format), Some(0.0));
        assert_eq!(date_to_days("86400000", &format), Some(1.0));

        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.date_format = format;
        gantt.sections[0].tasks[0].start = TaskStart::Date("0".into());
        gantt.sections[0].tasks[0].duration = GanttDuration {
            value: 20.0, unit: GanttDurationUnit::Milliseconds,
        };
        gantt.sections[0].tasks[1].start = TaskStart::After(vec!["t1".into()]);
        gantt.sections[0].tasks[1].duration = GanttDuration {
            value: 5.0, unit: GanttDurationUnit::Milliseconds,
        };
        let layout = layout_temporal_diagram(&diagram, 800.0);
        let bars = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TaskBar { x, width, .. } => Some((*x, *width)),
            _ => None,
        }).collect::<Vec<_>>();
        assert!((bars[1].0 - (bars[0].0 + bars[0].1)).abs() < 0.01);
        assert!((bars[0].1 / bars[1].1 - 4.0).abs() < 0.01);
    }

    #[test]
    fn gantt_inclusive_end_dates_extend_explicit_date_bars() {
        let mut exclusive = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut exclusive.body else { unreachable!() };
        gantt.sections[0].tasks[0].start = TaskStart::Date("2026-03-01".into());
        gantt.sections[0].tasks[0].end = Some(TaskEnd::Date("2026-03-03".into()));
        gantt.sections[0].tasks[1].start = TaskStart::Date("2026-03-10".into());
        gantt.sections[0].tasks[1].duration = GanttDuration { value: 1.0, unit: GanttDurationUnit::Days };
        let exclusive_layout = layout_temporal_diagram(&exclusive, 800.0);

        let TemporalBody::Gantt(gantt) = &mut exclusive.body else { unreachable!() };
        gantt.config.inclusive_end_dates = true;
        let inclusive_layout = layout_temporal_diagram(&exclusive, 800.0);
        let width = |layout: &LayoutedTemporalDiagram| layout.items.iter().find_map(|item| match item {
            LayoutedTemporalItem::TaskBar { width, .. } => Some(*width),
            _ => None,
        }).unwrap();
        assert!(width(&inclusive_layout) > width(&exclusive_layout));
    }

    #[test]
    fn gantt_top_axis_adds_a_second_resolved_axis() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.config.top_axis = true;
        let layout = layout_temporal_diagram(&diagram, 800.0);
        assert_eq!(layout.items.iter().filter(|item| matches!(
            item, LayoutedTemporalItem::TimeAxisSpine { .. }
        )).count(), 2);
        assert!(layout.items.iter().any(|item| matches!(
            item, LayoutedTemporalItem::TimeAxisTick { label_above: true, .. }
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item, LayoutedTemporalItem::TimeAxisTick { label_above: false, .. }
        )));
    }

    #[test]
    fn gantt_today_marker_resolves_style_without_backend_css() {
        let today = current_epoch_day();
        let iso = |day| {
            let (year, month, date) = civil_from_days(day);
            format!("{year:04}-{month:02}-{date:02}")
        };
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.sections[0].tasks.truncate(1);
        gantt.sections[0].tasks[0].start = TaskStart::Date(iso(today - 1));
        gantt.sections[0].tasks[0].end = Some(TaskEnd::Date(iso(today + 1)));
        gantt.config.today_marker = Some("stroke:#00aa44,stroke-width:5px,stroke-dasharray:none".into());
        let layout = layout_temporal_diagram(&diagram, 800.0);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::TodayMarker { stroke, stroke_width, stroke_dash, .. }
                if stroke == "#00aa44" && *stroke_width == 5.0 && stroke_dash.is_none()
        )));
    }

    #[test]
    fn gantt_resolves_multi_source_after_and_until_ranges() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.sections[0].tasks[0].start = TaskStart::Date("2026-01-01".into());
        gantt.sections[0].tasks[0].duration = GanttDuration { value: 5.0, unit: GanttDurationUnit::Days };
        gantt.sections[0].tasks[1].start = TaskStart::Date("2026-01-03".into());
        gantt.sections[0].tasks[1].duration = GanttDuration { value: 4.0, unit: GanttDurationUnit::Days };
        gantt.sections[0].tasks.push(GanttTask {
            id: "t3".into(), label: "Integrate".into(),
            start: TaskStart::After(vec!["t1".into(), "t2".into()]),
            duration: GanttDuration { value: 2.0, unit: GanttDurationUnit::Days }, end: None, tags: GanttTaskTags::default(),
            dependencies: vec!["t1".into(), "t2".into()], link: None,
            callback: None, callback_args: None,
        });
        gantt.sections[0].tasks.push(GanttTask {
            id: "window".into(), label: "Window".into(),
            start: TaskStart::Date("2025-12-29".into()), duration: GanttDuration::default(),
            end: Some(TaskEnd::Until(vec!["t1".into(), "t2".into()])),
            tags: GanttTaskTags::default(), dependencies: Vec::new(), link: None,
            callback: None, callback_args: None,
        });
        gantt.sections[0].tasks.push(GanttTask {
            id: "gate".into(), label: "Gate".into(),
            start: TaskStart::Date("2025-12-30".into()), duration: GanttDuration::default(),
            end: Some(TaskEnd::Until(vec!["late".into()])),
            tags: GanttTaskTags::default(), dependencies: Vec::new(), link: None,
            callback: None, callback_args: None,
        });
        gantt.sections[0].tasks.push(GanttTask {
            id: "consumer".into(), label: "Consumer".into(),
            start: TaskStart::After(vec!["gate".into()]), duration: GanttDuration { value: 1.0, unit: GanttDurationUnit::Days },
            end: None, tags: GanttTaskTags::default(), dependencies: vec!["gate".into()], link: None,
            callback: None, callback_args: None,
        });
        gantt.sections[0].tasks.push(GanttTask {
            id: "late".into(), label: "Late".into(),
            start: TaskStart::After(vec!["t3".into()]), duration: GanttDuration { value: 1.0, unit: GanttDurationUnit::Days },
            end: None, tags: GanttTaskTags::default(), dependencies: vec!["t3".into()], link: None,
            callback: None, callback_args: None,
        });

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let bars = layout.items.iter().filter_map(|item| match item {
            LayoutedTemporalItem::TaskBar { x, width, label, .. } => {
                Some((label.as_str(), (*x, *width)))
            }
            _ => None,
        }).collect::<HashMap<_, _>>();
        let (build_x, build_width) = bars["Build"];
        let integrate_x = bars["Integrate"].0;
        assert!((integrate_x - (build_x + build_width)).abs() < 0.01);
        let window_end = bars["Window"].0 + bars["Window"].1;
        assert!((window_end - bars["Design"].0).abs() < 0.01);
        assert!((bars["Consumer"].0 - bars["Late"].0).abs() < 0.01);
    }

    #[test]
    fn gantt_combined_tags_and_vertical_markers_survive_layout() {
        let mut diagram = simple_gantt();
        let TemporalBody::Gantt(gantt) = &mut diagram.body else { unreachable!() };
        gantt.sections[0].tasks[0].tags = GanttTaskTags {
            active: true,
            critical: true,
            ..GanttTaskTags::default()
        };
        gantt.sections[0].tasks[1].tags.vertical = true;

        let layout = layout_temporal_diagram(&diagram, 800.0);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::TaskBar { tags, .. }
                if tags.active && tags.critical
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::VerticalMarker { y1, y2, label, .. }
                if y2 > y1 && label == "Build"
        )));
        assert_eq!(layout.items.iter().filter(|item| matches!(
            item,
            LayoutedTemporalItem::TaskBar { .. }
                | LayoutedTemporalItem::MilestoneMarker { .. }
        )).count(), 1);
    }

    #[test]
    fn git_layout_produces_commits() {
        let d = layout_temporal_diagram(&simple_git(), 800.0);
        let commits = d.items.iter().filter(|i| matches!(i, LayoutedTemporalItem::CommitNode{..})).count();
        assert_eq!(commits, 2);
    }

    #[test]
    fn git_layout_preserves_all_commit_tags() {
        let d = layout_temporal_diagram(&simple_git(), 800.0);
        assert!(d.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::CommitNode { tags, .. }
                if tags == &["v1", "stable"]
        )));
    }

    #[test]
    fn git_layout_resolves_distinct_commit_symbols() {
        let mut diagram = simple_git();
        let TemporalBody::Git(git) = &mut diagram.body else {
            unreachable!();
        };
        git.events = vec![
            GitEvent::Commit {
                id: Some("normal".into()), resolved_id: "normal".into(), parents: Vec::new(),
                message: None, tags: Vec::new(),
                branch: "main".into(), type_: GitCommitType::Normal,
            },
            GitEvent::Commit {
                id: Some("reverse".into()), resolved_id: "reverse".into(), parents: vec!["normal".into()],
                message: None, tags: Vec::new(),
                branch: "main".into(), type_: GitCommitType::Reverse,
            },
            GitEvent::Commit {
                id: Some("highlight".into()), resolved_id: "highlight".into(), parents: vec!["reverse".into()],
                message: None, tags: Vec::new(),
                branch: "main".into(), type_: GitCommitType::Highlight,
            },
            GitEvent::Merge {
                from: "main".into(), id: Some("merge".into()), resolved_id: "merge".into(),
                parents: vec!["highlight".into(), "normal".into()], tags: Vec::new(),
                type_: GitCommitType::Normal,
            },
            GitEvent::CherryPick {
                id: "pick".into(), resolved_id: "picked".into(),
                parents: vec!["merge".into(), "pick".into()], tags: Vec::new(),
                parent: None, branch: "main".into(),
            },
        ];

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let symbols = layout.items.iter().filter_map(|item| {
            if let LayoutedTemporalItem::CommitNode { symbol, .. } = item {
                Some(symbol.clone())
            } else {
                None
            }
        }).collect::<Vec<_>>();
        assert_eq!(symbols, [
            GitCommitSymbol::Normal,
            GitCommitSymbol::Reverse,
            GitCommitSymbol::Highlight,
            GitCommitSymbol::Merge,
            GitCommitSymbol::CherryPick,
        ]);
    }

    #[test]
    fn git_layout_applies_explicit_branch_order() {
        let mut diagram = simple_git();
        let TemporalBody::Git(git) = &mut diagram.body else {
            unreachable!();
        };
        git.branches = vec![
            GitBranch { name: "main".into(), order: None },
            GitBranch { name: "test1".into(), order: Some(3) },
            GitBranch { name: "test2".into(), order: Some(2) },
            GitBranch { name: "test3".into(), order: Some(1) },
        ];

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let labels = layout.items.iter().filter_map(|item| {
            if let LayoutedTemporalItem::BranchLane { label, .. } = item {
                Some(label.as_str())
            } else {
                None
            }
        }).collect::<Vec<_>>();
        assert_eq!(labels, ["main", "test3", "test2", "test1"]);
    }

    #[test]
    fn git_layout_keeps_unordered_branches_before_positive_orders() {
        let mut diagram = simple_git();
        let TemporalBody::Git(git) = &mut diagram.body else {
            unreachable!();
        };
        git.branches = vec![
            GitBranch { name: "main".into(), order: None },
            GitBranch { name: "test1".into(), order: Some(1) },
            GitBranch { name: "test2".into(), order: None },
            GitBranch { name: "test3".into(), order: None },
        ];

        let layout = layout_temporal_diagram(&diagram, 800.0);
        let labels = layout.items.iter().filter_map(|item| {
            if let LayoutedTemporalItem::BranchLane { label, .. } = item {
                Some(label.as_str())
            } else {
                None
            }
        }).collect::<Vec<_>>();
        assert_eq!(labels, ["main", "test2", "test3", "test1"]);
    }

    #[test]
    fn git_canvas_width_covers_commits() {
        let d = layout_temporal_diagram(&simple_git(), 800.0);
        assert!(d.width >= 2.0 * COMMIT_SPACING);
    }

    #[test]
    fn git_tb_layout_uses_vertical_lanes_and_downward_commits() {
        let mut diagram = simple_git();
        let TemporalBody::Git(git) = &mut diagram.body else {
            unreachable!();
        };
        git.direction = DiagramDirection::Tb;

        let layout = layout_temporal_diagram(&diagram, 320.0);
        let commits = layout.items.iter().filter_map(|item| {
            if let LayoutedTemporalItem::CommitNode { x, y, .. } = item {
                Some((*x, *y))
            } else {
                None
            }
        }).collect::<Vec<_>>();
        assert_eq!(commits[0].0, commits[1].0);
        assert!(commits[0].1 < commits[1].1);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::BranchLane { x1, x2, y1, y2, .. }
                if x1 == x2 && y1 < y2
        )));
    }

    #[test]
    fn git_bt_layout_places_commits_bottom_to_top() {
        let mut diagram = simple_git();
        let TemporalBody::Git(git) = &mut diagram.body else {
            unreachable!();
        };
        git.direction = DiagramDirection::Bt;

        let layout = layout_temporal_diagram(&diagram, 320.0);
        let commits = layout.items.iter().filter_map(|item| {
            if let LayoutedTemporalItem::CommitNode { y, .. } = item {
                Some(*y)
            } else {
                None
            }
        }).collect::<Vec<_>>();
        assert!(commits[0] > commits[1]);
    }

    #[test]
    fn git_layout_preserves_title_and_accessibility_metadata() {
        let d = layout_temporal_diagram(&simple_git(), 800.0);
        assert!(d.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::TemporalTitle { label, .. }
                if label == "Release history"
        )));
        assert_eq!(
            d.accessibility_title.as_deref(),
            Some("Accessible release history")
        );
        assert_eq!(
            d.accessibility_description.as_deref(),
            Some("Two commits on main")
        );
    }

    #[test]
    fn journey_layout_preserves_score_and_people() {
        let diagram = TemporalDiagram {
            kind: TemporalKind::Journey,
            title: Some("Checkout".into()),
            body: TemporalBody::Journey(Box::new(JourneyDiagram {
                accessibility_title: Some("Checkout journey".into()),
                accessibility_description: Some("Payment experience".into()),
                config: diagram_ir::JourneyConfig {
                    diagram_margin_x: Some(24.0),
                    diagram_margin_y: Some(12.0),
                    task_width: Some(280.0),
                    task_height: Some(52.0),
                    task_margin: Some(18.0),
                    task_font_size: Some(18.0),
                    task_font_family: Some("Avenir Next".into()),
                    title_font_size: Some(22.0),
                    title_font_family: Some("Georgia".into()),
                    title_color: Some("#123456".into()),
                    actor_colors: vec!["#010203".into()],
                    section_fills: vec!["#112233".into(), "#445566".into()],
                    section_colors: vec!["#fefefe".into()],
                    left_margin: Some(120.0),
                    max_label_width: Some(56.0),
                },
                sections: vec![JourneySection {
                    label: "Payment\nflow".into(),
                    tasks: vec![
                        JourneyTask {
                            label: "Pay\nsecurely".into(), score: 2, people: vec!["Bob".into()],
                        },
                        JourneyTask {
                            label: "Confirm".into(), score: 5, people: vec!["Bob".into()],
                        },
                    ],
                }],
            })),
        };
        let layout = layout_temporal_diagram(&diagram, 640.0);
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyTask { score: 2, label, people, .. }
                if label == "Pay\nsecurely" && people == &["Bob"]
        )));
        assert_eq!(layout.accessibility_title.as_deref(), Some("Checkout journey"));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyActor { label, .. } if label == "Bob"
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyTask { x, width, height, .. }
                if *x == 120.0 && *width == 280.0 && *height >= 52.0
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyTask { x, label, .. }
                if *x == 418.0 && label == "Confirm"
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyActivityLine { x1, x2, .. }
                if *x1 == 260.0 && *x2 == 558.0
        )));
        assert_eq!(layout.items.iter().filter(|item| matches!(item,
            LayoutedTemporalItem::JourneyTaskLine { .. }
        )).count(), 2);
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyTask { font_size, font_family, .. }
                if *font_size == Some(18.0) && font_family.as_deref() == Some("Avenir Next")
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyTitle { font_size, font_family, color, .. }
                if *font_size == Some(22.0)
                    && font_family.as_deref() == Some("Georgia")
                    && color.as_deref() == Some("#123456")
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyActor { color, .. } if color == "#010203"
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneyActor { width, label, .. }
                if *width == 56.0 && label == "Bob"
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::JourneySection { fill, text_color, .. }
                if fill == "#112233" && text_color == "#fefefe"
        )));
    }
}
