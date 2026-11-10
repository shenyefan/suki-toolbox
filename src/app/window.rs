use crate::settings::Backdrop;
use crate::ui::theme::window;
use gpui_kit::{component::TitleBar, *};

/// Native backdrop selection stays at the window boundary. Windows maps the
/// Acrylic material to the compositor's blur path.
pub(crate) fn options(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(
        None,
        size(window::DEFAULT_WIDTH, window::DEFAULT_HEIGHT),
        cx,
    );
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(window::MIN_WIDTH, window::MIN_HEIGHT)),
        window_background: background(crate::settings::read(cx).backdrop),
        app_id: Some("suki-toolbox".into()),
        ..TitleBar::window_options()
    }
}

pub(crate) fn background(backdrop: Backdrop) -> WindowBackgroundAppearance {
    match backdrop {
        Backdrop::Acrylic => WindowBackgroundAppearance::Blurred,
        Backdrop::Solid => WindowBackgroundAppearance::Opaque,
    }
}
