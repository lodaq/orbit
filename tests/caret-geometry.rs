use i_slint_backend_testing::{init_no_event_loop, mock_elapsed_time};
use slint::ComponentHandle;
slint::slint! {
    import { SmoothCaret } from "../ui/controls.slint";
    export component Harness inherits Window {
        in-out property <length> target;
        in-out property <length> line;
        in-out property <bool> active: true;
        in-out property <bool> animated: true;
        in-out property <bool> trail: true;
        in-out property <int> generation;
        out property <length> left: caret.block-x;
        out property <length> span: caret.block-width;
        out property <float> alpha: caret.opacity;
        out property <float> idle-alpha: caret.idle-opacity;
        out property <bool> running: caret.idle-running;
        caret := SmoothCaret {
            target-x: root.target; target-y: root.line;
            caret-width: 7px; caret-height: 16px;
            active: root.active; animated: root.animated; trail: root.trail;
            edit-generation: root.generation;
        }
    }
}
fn advance(ms: u64) {
    mock_elapsed_time(std::time::Duration::from_millis(ms));
}
fn harness() -> Harness {
    init_no_event_loop();
    let ui = Harness::new().unwrap();
    ui.show().unwrap();
    advance(0);
    let _ = (ui.get_left(), ui.get_span());
    ui
}
#[test]
fn settles_and_stretches_in_both_directions() {
    let ui = harness();
    ui.set_target(100.);
    let _ = ui.get_left();
    advance(40);
    assert!(ui.get_span() > 17., "span {}", ui.get_span());
    advance(500);
    assert!((ui.get_left() - 100.).abs() < 0.5);
    assert!((ui.get_span() - 7.).abs() < 0.5);
    ui.set_target(0.);
    let _ = ui.get_left();
    advance(40);
    assert!(ui.get_span() > 17.);
    assert!(ui.get_left() < 50.);
}
#[test]
fn clamps_and_retargets_continuously() {
    let ui = harness();
    ui.set_target(1000.);
    let _ = ui.get_left();
    advance(40);
    assert!(ui.get_span() <= 167.);
    let left = ui.get_left();
    ui.set_target(1200.);
    assert!((ui.get_left() - left).abs() < 0.5);
    advance(40);
    assert!(ui.get_left() >= left);
    advance(500);
    assert!((ui.get_left() - 1200.).abs() < 0.5);
}
#[test]
fn snaps_and_glides_without_trail() {
    let ui = harness();
    ui.set_animated(false);
    ui.set_target(300.);
    assert!((ui.get_left() - 300.).abs() < 0.5);
    ui.set_trail(false);
    ui.set_animated(true);
    ui.set_target(100.);
    let _ = ui.get_left();
    advance(40);
    assert!((ui.get_span() - 7.).abs() < 0.5);
}
#[test]
fn selection_stops_idle_and_fades() {
    let ui = harness();
    ui.set_active(false);
    advance(0);
    advance(100);
    assert_eq!(ui.get_alpha(), 0.);
    assert!(!ui.get_running());
}
#[test]
fn idle_resets_on_edit_without_movement() {
    let ui = harness();
    advance(449);
    assert_eq!(ui.get_idle_alpha(), 1.);
    advance(1);
    advance(550);
    assert!(ui.get_idle_alpha() < 0.9, "{}", ui.get_idle_alpha());
    ui.set_generation(1);
    advance(0);
    assert_eq!(ui.get_idle_alpha(), 1.);
    ui.set_active(false);
    advance(0);
    assert!(!ui.get_running());
}

#[test]
fn line_change_snaps_instead_of_crossing_text() {
    let ui = harness();
    ui.set_target(250.);
    let _ = ui.get_left();
    advance(40);
    ui.set_line(18.);
    ui.set_target(0.);
    advance(0);
    assert!(ui.get_left().abs() < 0.5, "left {}", ui.get_left());
    assert!((ui.get_span() - 7.).abs() < 0.5);
}

#[test]
fn inactive_component_starts_without_idle_timer() {
    init_no_event_loop();
    let ui = Harness::new().unwrap();
    ui.set_active(false);
    ui.show().unwrap();
    advance(0);
    assert!(!ui.get_running());
}
