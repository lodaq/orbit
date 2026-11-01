use slint::ComponentHandle;
use std::{path::Path, time::Duration};

slint::slint! {
    import { WidgetView, InternetUsageWidget } from "../ui/local-widget.slint";
    import { Theme } from "../ui/theme.slint";

    export component UsageProbe inherits Window {
        in property <WidgetView> view;
        width: 364px;
        height: 170px;
        background: transparent;
        Rectangle {
            width: parent.width;
            height: parent.height;
            background: Theme.widget-background;
            border-width: Theme.border-width;
            border-color: Theme.widget-border;
            border-radius: Theme.radius-widget;
            InternetUsageWidget { width: parent.width; height: parent.height; view: root.view; }
        }
    }
}

fn capture(ui: &UsageProbe, name: &str) -> Vec<u8> {
    let image = ui.window().take_snapshot().unwrap();
    image::save_buffer(
        Path::new("target/ui-check").join(name),
        image.as_bytes(),
        image.width(),
        image.height(),
        image::ColorType::Rgba8,
    )
    .unwrap();
    image.as_bytes().to_vec()
}

#[test]
fn renders_usage_card_in_the_shared_widget_design() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            renderer_name: Some("software".into()),
            ..Default::default()
        },
    )))
    .unwrap();
    std::fs::create_dir_all("target/ui-check").unwrap();
    let ui = UsageProbe::new().unwrap();
    let mut view = WidgetView::default();
    view.total_value = "1.31".into();
    view.total_unit = "GiB".into();
    view.down_value = "910.12".into();
    view.down_unit = "MiB".into();
    view.up_value = "428.61".into();
    view.up_unit = "MiB".into();
    view.updated = "01:19".into();
    view.down_ratio = 0.68;
    ui.set_view(view);
    ui.show().unwrap();
    i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
    assert!(!ui.window().has_active_animations());
    capture(&ui, "internet-usage.png");
    ui.set_view({
        let mut view = ui.get_view();
        view.down_ratio = 0.72;
        view
    });
    i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
    capture(&ui, "internet-usage-ratio-start.png");
    assert!(
        ui.window().has_active_animations(),
        "capsule width animates"
    );
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(400));
    assert!(!ui.window().has_active_animations());
}
