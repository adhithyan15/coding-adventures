"""DRC: geometric design-rule checking.

Operates on simple rectangle data (per layer). v0.1.0 implements:
- min-width
- min-spacing (per layer; pairwise)
- min-area

Uses naive O(n²) pairwise checks; fine for 4-bit-adder smoke test, future
rounds add an R-tree for scale.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True, slots=True)
class Rect:
    layer: str
    x1: float
    y1: float
    x2: float
    y2: float

    def width(self) -> float:
        return self.x2 - self.x1

    def height(self) -> float:
        return self.y2 - self.y1

    def area(self) -> float:
        return self.width() * self.height()


@dataclass(frozen=True, slots=True)
class Violation:
    rule: str
    severity: str  # "error" | "warning"
    layer: str
    location_x: float
    location_y: float
    description: str


@dataclass(frozen=True, slots=True)
class Rule:
    name: str
    layer: str
    kind: str  # 'min_width' | 'min_spacing' | 'min_area'
    value: float
    severity: str = "error"


@dataclass
class DrcReport:
    violations: list[Violation] = field(default_factory=list)
    rules_checked: int = 0

    @property
    def clean(self) -> bool:
        return not any(v.severity == "error" for v in self.violations)


def run_drc(rects: list[Rect], rules: list[Rule]) -> DrcReport:
    """Run all rules against the given rectangles. Returns DrcReport."""
    report = DrcReport(rules_checked=len(rules))
    by_layer: dict[str, list[Rect]] = {}
    for r in rects:
        by_layer.setdefault(r.layer, []).append(r)

    for rule in rules:
        layer_rects = by_layer.get(rule.layer, [])
        if rule.kind == "min_width":
            _check_min_width(rule, layer_rects, report)
        elif rule.kind == "min_spacing":
            _check_min_spacing(rule, layer_rects, report)
        elif rule.kind == "min_area":
            _check_min_area(rule, layer_rects, report)
        else:
            report.violations.append(Violation(
                rule=rule.name, severity="warning", layer=rule.layer,
                location_x=0, location_y=0,
                description=f"unknown rule kind: {rule.kind}",
            ))

    return report


def _check_min_width(rule: Rule, rects: list[Rect], report: DrcReport) -> None:
    for r in rects:
        if r.width() < rule.value or r.height() < rule.value:
            report.violations.append(Violation(
                rule=rule.name, severity=rule.severity, layer=rule.layer,
                location_x=r.x1, location_y=r.y1,
                description=f"min_width {rule.value} violated: {r.width()}x{r.height()}",
            ))


def _check_min_spacing(rule: Rule, rects: list[Rect], report: DrcReport) -> None:
    for i, a in enumerate(rects):
        for b in rects[i + 1:]:
            spacing = _rect_spacing(a, b)
            # spacing = -1 if rectangles overlap (degenerate, treat as 0 spacing).
            if 0 <= spacing < rule.value:
                report.violations.append(Violation(
                    rule=rule.name, severity=rule.severity, layer=rule.layer,
                    location_x=(a.x1 + b.x1) / 2,
                    location_y=(a.y1 + b.y1) / 2,
                    description=f"min_spacing {rule.value} violated: {spacing}",
                ))


def _check_min_area(rule: Rule, rects: list[Rect], report: DrcReport) -> None:
    for r in rects:
        if r.area() < rule.value:
            report.violations.append(Violation(
                rule=rule.name, severity=rule.severity, layer=rule.layer,
                location_x=r.x1, location_y=r.y1,
                description=f"min_area {rule.value} violated: {r.area()}",
            ))


def _rect_spacing(a: Rect, b: Rect) -> float:
    """Return the minimum distance between two rectangles. 0 if touching;
    -1 if overlapping."""
    # Check overlap
    if not (a.x2 <= b.x1 or b.x2 <= a.x1 or a.y2 <= b.y1 or b.y2 <= a.y1):
        # Overlap or one contains the other
        return -1.0
    dx = max(0.0, max(b.x1 - a.x2, a.x1 - b.x2))
    dy = max(0.0, max(b.y1 - a.y2, a.y1 - b.y2))
    if dx == 0 and dy == 0:
        return 0.0
    if dx == 0:
        return dy
    if dy == 0:
        return dx
    return (dx * dx + dy * dy) ** 0.5
