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
pub fn continue_scroll(
    launcher: &crate::Launcher,
    event: &slint::winit_030::winit::event::WindowEvent,
) -> bool {
    use slint::ComponentHandle;
    use slint::winit_030::winit::event::{MouseScrollDelta, TouchPhase, WindowEvent};
    let (delta, phase) = match event {
        WindowEvent::MouseInput { .. } | WindowEvent::Focused(false) => {
            launcher.set_canvas_scrolling(false);
            launcher.set_canvas_scroll_gesture(false);
            return false;
        }
        WindowEvent::MouseWheel { delta, phase, .. } => (delta, phase),
        _ => return false,
    };
    match phase {
        TouchPhase::Started => {
            launcher.set_canvas_scrolling(false);
            launcher.set_canvas_scroll_gesture(true);
            return false;
        }
        TouchPhase::Ended | TouchPhase::Cancelled => {
            let active = launcher.get_canvas_scrolling();
            launcher.set_canvas_scrolling(false);
            launcher.set_canvas_scroll_gesture(false);
            return active;
        }
        TouchPhase::Moved => {}
    }
    if !launcher.get_canvas_scrolling() {
        return false;
    }
    // Match Slint's wheel conversion before bypassing widget hit testing.
    let (dx, dy) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x * 60., y * 60.),
        MouseScrollDelta::PixelDelta(delta) => {
            let delta = delta.to_logical::<f32>(launcher.window().scale_factor() as f64);
            (delta.x, delta.y)
        }
    };
    launcher.invoke_continue_canvas_scroll(dx, dy)
}

#[cfg(test)]
mod tests {
    #[test]
    fn canvas_scroll_continues_when_a_widget_moves_under_the_pointer() {
        use slint::winit_030::winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};
        use slint::{ComponentHandle, ModelRc, VecModel};
        i_slint_backend_testing::init_no_event_loop();
        let ui = crate::Launcher::new().unwrap();
        ui.window().set_size(slint::LogicalSize::new(1600., 1000.));
        ui.set_launcher_visible(true);
        crate::spatial::connect(&ui);
        ui.set_canvas_speed(1);
        ui.set_items(ModelRc::new(VecModel::from(vec![crate::AppTile {
            id: "widget".into(),
            widget: true,
            col: 650,
            row: 600,
            span_x: 400,
            span_y: 300,
            ..Default::default()
        }])));
        ui.show().unwrap();
        let position = slint::LogicalPosition::new(800., 450.);
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::PointerMoved { position });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position,
                delta_x: 0.,
                delta_y: -200.,
            });
        let state = ui.global::<crate::CanvasState>();
        assert_eq!(state.get_pan_y(), -200.);
        let scroll = |delta, phase| {
            super::continue_scroll(
                &ui,
                &WindowEvent::MouseWheel {
                    device_id: DeviceId::dummy(),
                    delta,
                    phase,
                },
            )
        };
        // The widget is now beneath the pointer; the same gesture keeps panning.
        assert!(scroll(
            MouseScrollDelta::LineDelta(0., -1.),
            TouchPhase::Moved
        ));
        assert_eq!(state.get_pan_y(), -260.);
        assert!(scroll(
            MouseScrollDelta::LineDelta(0., 0.),
            TouchPhase::Ended
        ));
        assert!(!scroll(
            MouseScrollDelta::LineDelta(0., -1.),
            TouchPhase::Started
        ));
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position,
                delta_x: 0.,
                delta_y: -60.,
            });
        assert_eq!(
            state.get_pan_y(),
            -260.,
            "a new scroll over the widget belongs to its content"
        );
    }

    #[test]
    fn scroll_sequence_respects_touchpad_end_mouse_idle_and_layers() {
        use slint::ComponentHandle;
        use slint::winit_030::winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};
        i_slint_backend_testing::init_no_event_loop();
        let ui = crate::Launcher::new().unwrap();
        ui.window().set_size(slint::LogicalSize::new(1600., 1000.));
        ui.set_launcher_visible(true);
        ui.set_canvas_speed(1);
        ui.show().unwrap();
        let scroll = |phase| {
            super::continue_scroll(
                &ui,
                &WindowEvent::MouseWheel {
                    device_id: DeviceId::dummy(),
                    delta: MouseScrollDelta::PixelDelta(
                        slint::winit_030::winit::dpi::PhysicalPosition::new(0., -40.),
                    ),
                    phase,
                },
            )
        };
        let saves = std::rc::Rc::new(std::cell::Cell::new(0));
        let recorded = saves.clone();
        ui.global::<crate::CanvasState>()
            .on_save_view(move |_, _, _| recorded.set(recorded.get() + 1));
        let begin = || {
            ui.window()
                .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                    position: slint::LogicalPosition::new(800., 450.),
                    delta_x: 0.,
                    delta_y: -10.,
                })
        };
        let pause = || {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(300));
            slint::platform::update_timers_and_animations();
        };
        assert!(!scroll(TouchPhase::Started));
        begin();
        pause();
        assert!(
            ui.get_canvas_scrolling(),
            "a paused touchpad gesture still owns the Canvas"
        );
        assert!(scroll(TouchPhase::Moved));
        assert_eq!(ui.global::<crate::CanvasState>().get_pan_y(), -50.);
        assert!(scroll(TouchPhase::Ended));
        assert!(!ui.get_canvas_scrolling());
        let before = saves.get();
        begin();
        pause();
        assert!(
            !ui.get_canvas_scrolling(),
            "wheel ownership ends after idle"
        );
        assert!(
            saves.get() > before,
            "ending wheel ownership must not cancel the view save"
        );
        begin();
        ui.set_settings(true);
        assert!(!scroll(TouchPhase::Moved));
        assert!(!ui.get_canvas_scrolling());
    }

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
