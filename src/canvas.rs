use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct View {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}
impl Default for View {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}
impl View {
    pub fn world(self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.x) / self.zoom, (y - self.y) / self.zoom)
    }
    pub fn zoom_at(self, x: f32, y: f32, factor: f32) -> Self {
        if !self.valid() || !x.is_finite() || !y.is_finite() || !factor.is_finite() || factor <= 0.0
        {
            return self;
        }
        let (wx, wy) = self.world(x, y);
        // A fitted view may be below the normal limit; zoom in from that scale without a jump.
        let minimum = if self.zoom < 0.25 {
            self.zoom.min(self.zoom * factor)
        } else {
            0.25
        };
        let zoom = (self.zoom * factor).clamp(minimum.max(f32::MIN_POSITIVE), 2.0);
        Self {
            x: x - wx * zoom,
            y: y - wy * zoom,
            zoom,
        }
    }
    pub fn valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.zoom.is_finite()
            && self.zoom > 0.0
            && self.zoom <= 2.0
    }
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Canvas {
    pub view: View,
}
impl Canvas {
    pub fn valid(&self) -> bool {
        self.view.valid()
    }
}

pub fn fit(
    positions: &std::collections::BTreeMap<String, crate::spatial::Rect>,
    width: f32,
    height: f32,
) -> Option<View> {
    if positions.is_empty()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 48.0
        || height <= 266.0
    {
        return None;
    }
    let left = positions
        .values()
        .map(|r| f64::from(r.col))
        .fold(f64::INFINITY, f64::min);
    let top = positions
        .values()
        .map(|r| f64::from(r.row))
        .fold(f64::INFINITY, f64::min);
    let right = positions
        .values()
        .map(|r| f64::from(r.col) + r.width as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = positions
        .values()
        .map(|r| f64::from(r.row) + r.height as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let available_width = f64::from(width - 48.0);
    let available_height = f64::from(height - 266.0);
    let zoom = (available_width / (right - left))
        .min(available_height / (bottom - top))
        .min(2.0);
    let view = View {
        x: (24.0 + (available_width - (right - left) * zoom) / 2.0 - left * zoom) as f32,
        y: (110.0 + (available_height - (bottom - top) * zoom) / 2.0 - top * zoom) as f32,
        zoom: zoom as f32,
    };
    view.valid().then_some(view)
}

pub enum Mutation {
    View(View),
}
pub fn apply(config: &mut Config, change: Mutation) -> Result<(), String> {
    let Mutation::View(view) = change;
    if !view.valid() {
        return Err("Invalid Canvas position".into());
    }
    if config.canvas.view != view {
        let mut next = config.clone();
        next.canvas.view = view;
        next.save()
            .map_err(|error| format!("Couldn't save Canvas: {error}"))?;
        *config = next;
    }
    Ok(())
}
pub fn sync(launcher: &crate::Launcher, canvas: &Canvas, restore: bool) {
    use slint::ComponentHandle;
    if restore {
        let state = launcher.global::<crate::CanvasState>();
        state.set_pan_x(canvas.view.x);
        state.set_pan_y(canvas.view.y);
        state.set_zoom(canvas.view.zoom);
    }
}
pub fn connect(launcher: &crate::Launcher, send: impl Fn(Mutation) + Clone + 'static) {
    use slint::ComponentHandle;
    let state = launcher.global::<crate::CanvasState>();
    state.on_save_view(move |x, y, zoom| send(Mutation::View(View { x, y, zoom })));
    let weak = launcher.as_weak();
    state.on_zoom_at(move |x, y, factor| {
        if let Some(ui) = weak.upgrade() {
            let state = ui.global::<crate::CanvasState>();
            let view = View {
                x: state.get_pan_x(),
                y: state.get_pan_y(),
                zoom: state.get_zoom(),
            }
            .zoom_at(x, y, factor);
            if view.valid() {
                state.set_pan_x(view.x);
                state.set_pan_y(view.y);
                state.set_zoom(view.zoom);
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_zoom_is_stable_and_fitted_view_zooms_smoothly() {
        let view = View {
            x: -120.0,
            y: 70.0,
            zoom: 0.5,
        };
        assert_eq!(
            view.world(310.0, 220.0),
            view.zoom_at(310.0, 220.0, 1.5).world(310.0, 220.0)
        );
        assert_eq!(view.zoom_at(0.0, 0.0, 20.0).zoom, 2.0);
        assert_eq!(view.zoom_at(0.0, 0.0, 0.01).zoom, 0.25);
        let positions = std::collections::BTreeMap::from([(
            "a".into(),
            crate::spatial::Rect {
                col: -20_000,
                row: 20_000,
                width: 40_000,
                height: 100,
            },
        )]);
        let fitted = fit(&positions, 1000.0, 700.0).unwrap();
        assert!(fitted.zoom < 0.25);
        assert!((fitted.zoom_at(400.0, 300.0, 1.1).zoom - fitted.zoom * 1.1).abs() < 0.00001);
        assert!(fit(&Default::default(), 1000.0, 700.0).is_none());
    }
    #[test]
    fn invalid_view_leaves_locked_configuration_unchanged() {
        let mut config = Config::default();
        config.layout_locked = true;
        assert!(
            apply(
                &mut config,
                Mutation::View(View {
                    x: f32::NAN,
                    ..Default::default()
                })
            )
            .is_err()
        );
        assert_eq!(config.canvas.view, View::default());
    }
}
