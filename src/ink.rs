use crate::config::{InkPoint, InkStroke};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Style {
    pub color: String,
    pub size: f32,
    pub opacity: f32,
    pub highlighter: bool,
    pub arrow: bool,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            color: "#e8e5ef".into(),
            size: 2.5,
            opacity: 1.0,
            highlighter: false,
            arrow: false,
        }
    }
}
impl Style {
    pub fn valid(&self) -> bool {
        self.color.len() == 7
            && self.color.starts_with('#')
            && self.color[1..].bytes().all(|b| b.is_ascii_hexdigit())
            && self.size.is_finite()
            && (1.0..=64.0).contains(&self.size)
            && self.opacity.is_finite()
            && (0.1..=1.0).contains(&self.opacity)
    }
}

// Bounds config size and the work needed to rebuild saved stroke paths.
pub const MAX_POINTS: usize = 100_000;

#[cfg(test)]
pub fn view(points: &[InkPoint]) -> crate::CanvasInk {
    view_with_padding(points, 2.0)
}

fn view_with_padding(points: &[InkPoint], padding: f32) -> crate::CanvasInk {
    let mut left = f32::INFINITY;
    let mut top = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for point in points {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    if points.is_empty() {
        return crate::CanvasInk::default();
    }
    // Slint culls Paths by their geometry, including the stroke outside its centerline.
    left -= padding;
    top -= padding;
    let mut path = String::with_capacity(points.len() * 18);
    let first = &points[0];
    let _ = write!(path, "M {:.1} {:.1}", first.x - left, first.y - top);
    // Midpoint quadratic curves avoid sharp joins without delaying the pointer.
    for pair in points.windows(2) {
        let _ = write!(
            path,
            " Q {:.1} {:.1} {:.1} {:.1}",
            pair[0].x - left,
            pair[0].y - top,
            (pair[0].x + pair[1].x) * 0.5 - left,
            (pair[0].y + pair[1].y) * 0.5 - top
        );
    }
    let last = points.last().unwrap();
    let _ = write!(
        path,
        " L {:.1} {:.1}",
        last.x - left + 0.01,
        last.y - top + 0.01
    );
    crate::CanvasInk {
        x: left,
        y: top,
        width: right - left + padding,
        height: bottom - top + padding,
        path: path.into(),
        samples: Default::default(),
        ..Default::default()
    }
}

pub fn samples(points: &[InkPoint], style: &Style) -> Vec<InkPoint> {
    let mut samples = points.to_vec();
    if style.arrow
        && let Some((a, b)) = points.first().zip(points.last())
    {
        let angle = (b.y - a.y).atan2(b.x - a.x);
        let head = (style.size * 5.0).max(14.0);
        for turn in [-0.5_f32, 0.5] {
            samples.push(InkPoint {
                x: b.x - head * (angle + turn).cos(),
                y: b.y - head * (angle + turn).sin(),
            });
            samples.push(*b);
        }
    }
    samples
}

pub fn styled_view(points: &[InkPoint], style: &Style) -> crate::CanvasInk {
    let padding = if style.arrow {
        (style.size * 5.0).max(14.0) + style.size
    } else {
        style.size * if style.highlighter { 2.0 } else { 0.5 } + 1.0
    };
    let mut result = view_with_padding(points, padding);
    if style.arrow && points.len() >= 2 {
        let a = points[0];
        let b = points[points.len() - 1];
        let mut path = String::new();
        let _ = write!(
            path,
            "M {:.1} {:.1} L {:.1} {:.1}",
            a.x - result.x,
            a.y - result.y,
            b.x - result.x,
            b.y - result.y
        );
        for point in samples(points, style).iter().skip(points.len()).step_by(2) {
            let _ = write!(
                path,
                " M {:.1} {:.1} L {:.1} {:.1}",
                point.x - result.x,
                point.y - result.y,
                b.x - result.x,
                b.y - result.y
            );
        }
        result.path = path.into();
    }
    result
}

pub fn normalize(points: &mut Vec<InkPoint>) {
    let mut deduped = Vec::with_capacity(points.len().min(2000));
    for point in points.drain(..) {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x.abs() > 10_000_000.0
            || point.y.abs() > 10_000_000.0
        {
            continue;
        }
        let far = deduped.last().is_none_or(|last: &InkPoint| {
            (last.x - point.x).abs() >= 0.01 || (last.y - point.y).abs() >= 0.01
        });
        if far && deduped.len() < 2000 {
            deduped.push(point);
        }
    }
    // A click without movement still draws a dot once scaled by the round line cap.
    if deduped.len() == 1 {
        deduped.push(InkPoint {
            x: deduped[0].x + 0.1,
            y: deduped[0].y + 0.1,
        });
    }
    *points = deduped;
}

pub fn connect(launcher: &crate::Launcher) {
    launcher.on_ink_view(|points, _count, mode, size| {
        use slint::Model;
        let points = points
            .iter()
            .map(|point| InkPoint {
                x: point.x,
                y: point.y,
            })
            .collect::<Vec<_>>();
        styled_view(
            &points,
            &Style {
                size,
                arrow: mode == 7,
                highlighter: mode == 8,
                ..Default::default()
            },
        )
    });
    launcher.on_preview_erase_items(|items, targets, x1, y1, x2, y2| {
        use slint::Model;
        let mut targets: Vec<_> = targets.iter().collect();
        for item in items.iter().filter(|item| item.annotation > 0) {
            if !targets.contains(&item.id)
                && hits_rect(
                    &[InkPoint { x: x1, y: y1 }, InkPoint { x: x2, y: y2 }],
                    crate::spatial::Rect {
                        col: item.col,
                        row: item.row,
                        width: item.span_x as usize,
                        height: item.span_y as usize,
                    },
                )
            {
                targets.push(item.id);
            }
        }
        std::rc::Rc::new(slint::VecModel::from(targets)).into()
    });
    launcher.on_preview_erase(|strokes, x1, y1, x2, y2| {
        use slint::Model;
        let from = InkPoint { x: x1, y: y1 };
        let to = InkPoint { x: x2, y: y2 };
        let visible = strokes
            .iter()
            .map(|mut stroke| {
                if stroke
                    .samples
                    .iter()
                    .zip(stroke.samples.iter().skip(1))
                    .any(|(a, b)| {
                        segments_hit(
                            &from,
                            &to,
                            &InkPoint { x: a.x, y: a.y },
                            &InkPoint { x: b.x, y: b.y },
                            (12.0 + stroke.size as f64 / 2.0).powi(2),
                        )
                    })
                {
                    stroke.dimmed = true;
                }
                stroke
            })
            .collect::<Vec<_>>();
        std::rc::Rc::new(slint::VecModel::from(visible)).into()
    });
}

// Erasing removes a whole stroke when any part of it crosses the eraser path.
pub fn erase(strokes: &mut Vec<InkStroke>, points: &[InkPoint], radius: f32) -> bool {
    let radius = radius.clamp(1.0, 200.0) as f64;
    let before = strokes.len();
    strokes.retain(|stroke| {
        let samples = samples(&stroke.points, &stroke.style);
        let mut previous = points.first();
        !points.iter().any(|eraser| {
            let from = previous.unwrap_or(eraser);
            previous = Some(eraser);
            samples.windows(2).any(|pair| {
                segments_hit(
                    from,
                    eraser,
                    &pair[0],
                    &pair[1],
                    (radius
                        + stroke.style.size as f64
                            * if stroke.style.highlighter { 2.0 } else { 0.5 })
                    .powi(2),
                )
            })
        })
    });
    strokes.len() != before
}

pub fn hits_rect(points: &[InkPoint], rect: crate::spatial::Rect) -> bool {
    let left = rect.col as f32;
    let top = rect.row as f32;
    let right = left + rect.width as f32;
    let bottom = top + rect.height as f32;
    let corners = [
        InkPoint { x: left, y: top },
        InkPoint { x: right, y: top },
        InkPoint {
            x: right,
            y: bottom,
        },
        InkPoint { x: left, y: bottom },
        InkPoint { x: left, y: top },
    ];
    points.iter().enumerate().any(|(i, p)| {
        (p.x >= left && p.x <= right && p.y >= top && p.y <= bottom)
            || corners.windows(2).any(|edge| {
                segments_hit(&points[i.saturating_sub(1)], p, &edge[0], &edge[1], 144.0)
            })
    })
}

fn segments_hit(a: &InkPoint, b: &InkPoint, c: &InkPoint, d: &InkPoint, radius2: f64) -> bool {
    fn side(a: &InkPoint, b: &InkPoint, p: &InkPoint) -> f64 {
        (f64::from(b.x) - f64::from(a.x)) * (f64::from(p.y) - f64::from(a.y))
            - (f64::from(b.y) - f64::from(a.y)) * (f64::from(p.x) - f64::from(a.x))
    }
    let intersect = a.x.min(b.x) <= c.x.max(d.x)
        && c.x.min(d.x) <= a.x.max(b.x)
        && a.y.min(b.y) <= c.y.max(d.y)
        && c.y.min(d.y) <= a.y.max(b.y)
        && side(a, b, c) * side(a, b, d) <= 0.0
        && side(c, d, a) * side(c, d, b) <= 0.0;
    intersect
        || segment_distance2(a, c, d) <= radius2
        || segment_distance2(b, c, d) <= radius2
        || segment_distance2(c, a, b) <= radius2
        || segment_distance2(d, a, b) <= radius2
}

fn segment_distance2(point: &InkPoint, a: &InkPoint, b: &InkPoint) -> f64 {
    let (px, py) = (f64::from(point.x), f64::from(point.y));
    let (ax, ay) = (f64::from(a.x), f64::from(a.y));
    let (bx, by) = (f64::from(b.x), f64::from(b.y));
    let (dx, dy) = (bx - ax, by - ay);
    let length2 = dx * dx + dy * dy;
    let t = if length2 == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / length2).clamp(0.0, 1.0)
    };
    let (x, y) = (ax + t * dx - px, ay + t * dy - py);
    x * x + y * y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> InkPoint {
        InkPoint { x, y }
    }

    #[test]
    fn path_geometry_contains_the_stroke_at_negative_coordinates() {
        let ink = view(&[point(1.0, 2.0), point(3.5, -4.25)]);
        assert_eq!(
            (ink.x, ink.y, ink.width, ink.height),
            (-1.0, -6.25, 6.5, 10.25)
        );
        assert!(ink.path.contains(" Q "));
        assert!(view(&[]).path.is_empty());
    }

    #[test]
    fn normalize_turns_a_click_into_a_dot_and_caps_long_strokes() {
        let mut dot = vec![point(10.0, 10.0)];
        normalize(&mut dot);
        assert_eq!(dot.len(), 2);
        let mut long = (0..3000)
            .map(|index| point(index as f32, index as f32))
            .collect();
        normalize(&mut long);
        assert_eq!(long.len(), 2000);
        let mut invalid = vec![point(f32::NAN, 0.0), point(1.0, 1.0)];
        normalize(&mut invalid);
        assert_eq!(invalid, vec![point(1.0, 1.0), point(1.1, 1.1)]);
    }

    #[test]
    fn eraser_removes_whole_strokes_that_cross_its_path() {
        let mut strokes = vec![
            InkStroke {
                id: "a".into(),
                style: Default::default(),
                points: vec![point(0.0, 0.0), point(100.0, 0.0)],
            },
            InkStroke {
                id: "b".into(),
                style: Default::default(),
                points: vec![point(0.0, 100.0), point(100.0, 100.0)],
            },
        ];
        assert!(erase(&mut strokes, &[point(50.0, 8.0)], 10.0));
        assert_eq!(strokes.len(), 1);
        assert_eq!(strokes[0].id, "b");
        assert!(!erase(&mut strokes, &[point(50.0, 60.0)], 10.0));
        assert!(erase(
            &mut strokes,
            &[point(50.0, 50.0), point(50.0, 150.0)],
            10.0
        ));
        assert!(strokes.is_empty());
    }
}
