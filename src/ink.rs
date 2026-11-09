use crate::config::{InkPoint, InkStroke};
use std::fmt::Write;

// Bounds config size and the work needed to rebuild saved stroke paths.
pub const MAX_POINTS: usize = 100_000;

pub fn view(points: &[InkPoint]) -> crate::CanvasInk {
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
    left -= 2.0;
    top -= 2.0;
    let mut path = String::with_capacity(points.len() * 18);
    for (index, point) in points.iter().enumerate() {
        let _ = write!(
            path,
            "{}{:.1} {:.1}",
            if index == 0 { "M " } else { " L " },
            point.x - left,
            point.y - top
        );
    }
    if points.len() == 1 {
        path.push_str(" L 2.1 2.1");
    }
    crate::CanvasInk {
        x: left,
        y: top,
        width: right - left + 2.0,
        height: bottom - top + 2.0,
        path: path.into(),
        samples: Default::default(),
    }
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
    launcher.on_ink_view(|points, _count| {
        use slint::Model;
        let points = points
            .iter()
            .map(|point| InkPoint {
                x: point.x,
                y: point.y,
            })
            .collect::<Vec<_>>();
        view(&points)
    });
    launcher.on_preview_erase(|strokes, x1, y1, x2, y2| {
        use slint::Model;
        let from = InkPoint { x: x1, y: y1 };
        let to = InkPoint { x: x2, y: y2 };
        let visible = strokes
            .iter()
            .filter(|stroke| {
                !stroke
                    .samples
                    .iter()
                    .zip(stroke.samples.iter().skip(1))
                    .any(|(a, b)| {
                        segments_hit(
                            &from,
                            &to,
                            &InkPoint { x: a.x, y: a.y },
                            &InkPoint { x: b.x, y: b.y },
                            144.0,
                        )
                    })
            })
            .collect::<Vec<_>>();
        std::rc::Rc::new(slint::VecModel::from(visible)).into()
    });
}

// Erasing removes a whole stroke when any part of it crosses the eraser path.
pub fn erase(strokes: &mut Vec<InkStroke>, points: &[InkPoint], radius: f32) -> bool {
    let radius = radius.clamp(1.0, 200.0) as f64;
    let radius2 = radius * radius;
    let before = strokes.len();
    strokes.retain(|stroke| {
        let mut previous = points.first();
        !points.iter().any(|eraser| {
            let from = previous.unwrap_or(eraser);
            previous = Some(eraser);
            stroke
                .points
                .windows(2)
                .any(|pair| segments_hit(from, eraser, &pair[0], &pair[1], radius2))
        })
    });
    strokes.len() != before
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
        assert_eq!(ink.path, "M 2.0 8.2 L 4.5 2.0");
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
                points: vec![point(0.0, 0.0), point(100.0, 0.0)],
            },
            InkStroke {
                id: "b".into(),
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
