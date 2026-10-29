#[path = "../src/search-cursor.rs"]
mod search_cursor;
use slint::ComponentHandle;
use std::{path::Path, time::Duration};
slint::slint! {
    export { Launcher, PrefixMatch } from "../ui/launcher.slint";
    import { SmoothCaret } from "../ui/controls.slint";
    import { Theme } from "../ui/theme.slint";
    export component NativeCaretProbe inherits Window {
        width: 80px; height: 40px; background: Theme.field;
        input := TextInput {
            x: 10px; y: 10px; width: 60px; height: 20px;
            color: Theme.text; font-size: 16px;
        }
        init => { input.focus(); }
    }
    export component Capture inherits Window {
        width: 560px; height: 80px; background: Theme.field;
        in-out property <length> target: 24px;
        in-out property <bool> active: true;
        in-out property <bool> shown: true;
        in-out property <bool> enabled: true;
        Text { x: 24px; y: 24px; text: "Search anything — smooth cursor"; color: Theme.text; font-size: 20px; }
        if root.shown && root.enabled: SmoothCaret {
            target-x: root.target; target-y: 24px;
            caret-width: 11px; caret-height: 24px; active: root.active;
        }
        if !root.enabled: Rectangle { x: root.target; y: 24px; width: 1px; height: 24px; background: Theme.text; }
    }
}
fn advance(ms: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(ms));
}
fn capture(ui: &Capture, name: &str) {
    save_window(ui.window(), name);
}
fn save_window(window: &slint::Window, name: &str) {
    let snapshot = window.take_snapshot().unwrap();
    image::save_buffer(
        Path::new("target/ui-check").join(name),
        snapshot.as_bytes(),
        snapshot.width(),
        snapshot.height(),
        image::ColorType::Rgba8,
    )
    .unwrap();
}
#[test]
fn cursor_captures_and_search_wiring() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            renderer_name: Some("software".into()),
            ..Default::default()
        },
    )))
    .unwrap();
    std::fs::create_dir_all("target/ui-check").unwrap();
    let probe = NativeCaretProbe::new().unwrap();
    probe.show().unwrap();
    advance(0);
    search_cursor::sync(probe.window(), true);
    for _ in 0..6 {
        advance(250);
        let hidden = probe.window().take_snapshot().unwrap();
        assert!(
            !hidden
                .as_slice()
                .iter()
                .any(|pixel| pixel.r > 200 && pixel.g > 200 && pixel.b > 200),
            "native caret must stay hidden across its blink cycle"
        );
    }
    search_cursor::sync(probe.window(), false);
    advance(0);
    let native = probe.window().take_snapshot().unwrap();
    assert!(
        native
            .as_slice()
            .iter()
            .any(|pixel| pixel.r > 200 && pixel.g > 200 && pixel.b > 200),
        "turning Smooth cursor off must restore the native caret"
    );
    probe.hide().unwrap();
    drop(probe);
    let ui = Capture::new().unwrap();
    ui.show().unwrap();
    advance(0);
    capture(&ui, "cursor-rest.png");
    ui.set_target(250.);
    capture(&ui, "cursor-move-start.png");
    advance(40);
    capture(&ui, "cursor-stretch.png");
    ui.set_active(false);
    advance(0);
    advance(100);
    capture(&ui, "cursor-selection.png");
    ui.set_shown(false);
    capture(&ui, "cursor-unfocused.png");
    advance(0);
    assert!(slint::platform::duration_until_next_timer_update().is_none());
    advance(1);
    capture(&ui, "cursor-unfocused.png");
    assert!(!ui.window().has_active_animations());
    ui.set_enabled(false);
    capture(&ui, "cursor-disabled.png");
    ui.set_enabled(true);
    ui.set_active(true);
    ui.set_shown(true);
    advance(0);
    capture(&ui, "cursor-reopened.png");
    assert!(slint::platform::duration_until_next_timer_update().is_some());
    ui.set_target(500.);
    capture(&ui, "cursor-hide-start.png");
    advance(40);
    ui.set_shown(false);
    capture(&ui, "cursor-hide-end.png");
    advance(1);
    capture(&ui, "cursor-hide-end.png");
    assert!(slint::platform::duration_until_next_timer_update().is_none());
    assert!(!ui.window().has_active_animations());
    ui.hide().unwrap();

    let launcher = Launcher::new().unwrap();
    let weak = launcher.as_weak();
    launcher.on_sync_native_search_cursor(move |hidden| {
        if let Some(ui) = weak.upgrade() {
            search_cursor::sync(ui.window(), hidden);
        }
    });
    launcher.show().unwrap();
    launcher.set_launcher_visible(true);
    launcher.invoke_focus_search();
    advance(0);
    assert!(launcher.get_search_focused());
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: "Orbit".into(),
        });
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: "Orbit".into(),
        });
    advance(0);
    assert_eq!(launcher.get_search_text(), "Orbit");
    advance(200);
    save_window(launcher.window(), "cursor-search-typed.png");
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Control.into(),
        });
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: "a".into() });
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: "a".into() });
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Control.into(),
        });
    advance(0);
    advance(100);
    save_window(launcher.window(), "cursor-search-selection.png");
    launcher.set_smooth_cursor(false);
    advance(0);
    save_window(launcher.window(), "cursor-search-disabled.png");
    launcher.set_smooth_cursor(true);
    launcher.set_search_text("".into());
    advance(0);
    save_window(launcher.window(), "cursor-search-cleared.png");
    launcher.set_search_text(
        "A long query that wraps and scrolls vertically. "
            .repeat(30)
            .into(),
    );
    advance(0);
    save_window(launcher.window(), "cursor-search-wrapped.png");
    launcher.on_match_prefix(|_| PrefixMatch {
        action_id: "ticktick-task".into(),
        name: "TickTick".into(),
        badge_width: 72,
        ..Default::default()
    });
    launcher.set_search_text("t".into());
    launcher.invoke_focus_search();
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Tab.into(),
        });
    launcher
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Tab.into(),
        });
    advance(0);
    assert_eq!(launcher.get_activated_prefix_name(), "TickTick");
    for (index, query) in [
        "i".repeat(185),
        "i love to be strong and healthy ya man hhahahahhah fsj fkljklajd fkljasdkjf lkajsd fkljadksj fkljdsak jkldsajf lkjasdkjf lj".into(),
        "A long query that wraps and scrolls vertically. ".repeat(30),
    ].into_iter().enumerate() {
        launcher.set_search_text(query.into());
        advance(0);
        for (key, pressed) in [
            (slint::platform::Key::Control, true),
            (slint::platform::Key::End, true),
            (slint::platform::Key::End, false),
            (slint::platform::Key::Control, false),
        ] {
            launcher.window().dispatch_event(if pressed {
                slint::platform::WindowEvent::KeyPressed { text: key.into() }
            } else {
                slint::platform::WindowEvent::KeyReleased { text: key.into() }
            });
        }
        advance(0);
        advance(200);
        let snapshot = launcher.window().take_snapshot().unwrap();
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for y in 39..210 {
            for x in 445..995 {
                let pixel = snapshot.as_slice()[(y * snapshot.width() + x) as usize];
                if pixel.b > 140 && pixel.b.saturating_sub(pixel.r) > 80 && pixel.r < 120 && pixel.g < 160 {
                    bounds = Some(bounds.map_or((x,y,x,y), |(left,top,right,bottom)| (left.min(x),top.min(y),right.max(x),bottom.max(y))));
                }
            }
        }
        let (left, top, right, bottom) = bounds.expect("full blue block must be visible at query end");
        assert!(right - left + 1 >= 7, "cursor clipped horizontally: {bounds:?}");
        assert!(bottom - top + 1 >= 16, "cursor clipped vertically: {bounds:?}");
        save_window(launcher.window(), &format!("cursor-prefix-overflow-{index}.png"));
    }
    launcher.set_launcher_visible(false);
    advance(0);
    assert!(!launcher.get_launcher_visible());
    launcher.hide().unwrap();
    std::fs::write("target/ui-check/cursor-report.txt", "Headless software-renderer captures use mocked Slint time.\nRun: cargo test --test caret-ui-check\nCaptures: cursor-rest.png, cursor-move-start.png, cursor-stretch.png, cursor-selection.png, cursor-unfocused.png, cursor-disabled.png.\nSelection/unfocused/disabled images exercise isolated cursor states; desktop captures validate real search integration.\n").unwrap();
}

#[test]
fn horizontal_touchpad_scroll_changes_one_page_per_swipe() {
    i_slint_backend_testing::init_no_event_loop();
    let launcher = Launcher::new().unwrap();
    launcher
        .window()
        .set_size(slint::LogicalSize::new(1600., 1000.));
    launcher.set_page_count(3);
    launcher.set_current_page(1);
    launcher.set_launcher_visible(true);
    let weak = launcher.as_weak();
    launcher.on_open_page(move |index| {
        let ui = weak.upgrade().unwrap();
        ui.set_current_page(index);
        ui.set_busy(false);
    });
    launcher.show().unwrap();
    let scroll = |x, y| {
        launcher
            .window()
            .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position: slint::LogicalPosition::new(800., 400.),
                delta_x: x,
                delta_y: y,
            });
    };
    advance(0);
    scroll(0., -120.);
    scroll(-10., 1.);
    assert_eq!(launcher.get_current_page(), 1);
    scroll(-35., 1.);
    assert_eq!(launcher.get_current_page(), 2);
    advance(240);
    scroll(-120., 0.);
    assert_eq!(launcher.get_current_page(), 2);
    advance(400);
    scroll(60., 0.);
    assert_eq!(launcher.get_current_page(), 1);
    advance(400);
    scroll(60., 0.);
    assert_eq!(launcher.get_current_page(), 0);
    advance(400);
    scroll(60., 0.);
    assert_eq!(launcher.get_current_page(), 0);
}
