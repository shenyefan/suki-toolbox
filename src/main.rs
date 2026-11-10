#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod settings;
mod system_info;
mod tools;
mod ui;

use gpui_kit::{
    base::Root,
    component::{ActiveTheme, Theme, ThemeMode},
    *,
};

fn main() {
    let (preferences, settings_error) = settings::load();
    let (catalog, catalog_error, config_missing) = tools::load();
    application().with_assets(assets::AllAssets).run(move |cx| {
        gpui_kit::init(cx);
        ui::notifications::init(cx);
        settings::init(preferences, cx);
        component::set_locale("zh-CN");
        match preferences.theme {
            settings::ThemeChoice::Light => Theme::change(ThemeMode::Light, None, cx),
            settings::ThemeChoice::System => Theme::sync_system_appearance(None, cx),
            settings::ThemeChoice::Dark => Theme::change(ThemeMode::Dark, None, cx),
        }
        ui::configure_theme(cx);
        cx.set_app_identity("suki-toolbox", "Suki Toolbox");
        cx.bind_keys([KeyBinding::new("ctrl-q", app::Quit, None)]);
        cx.on_action(|_: &app::Quit, cx| cx.quit());
        let options = app::window::options(cx);
        let main_window = cx
            .open_window(options, |window, cx| {
                let view =
                    cx.new(|cx| app::ToolboxApp::new(catalog.clone(), config_missing, window, cx));
                let transparent = cx.theme().background.opacity(0.);
                // Root normally paints an opaque component background. Override
                // only this window surface, keeping menus and dialogs opaque.
                cx.new(|cx| Root::new(view, window, cx).bg(transparent))
            })
            .expect("无法打开 Suki Toolbox 窗口");
        report_startup_errors(
            main_window.into(),
            [settings_error, catalog_error].into_iter().flatten(),
            cx,
        );
    });
}

fn report_startup_errors(
    window: AnyWindowHandle,
    errors: impl IntoIterator<Item = String>,
    cx: &mut App,
) {
    for error in errors {
        // Window-scoped notifications read Root themselves; do not lease Root through WindowHandle::update.
        let _ = cx.update_window(window, |_, window, cx| ui::show_error(error, window, cx));
    }
}

#[cfg(test)]
mod startup_tests {
    use gpui_kit::{AppContext as _, TestAppContext, test::TestWindowExt};

    #[gpui_kit::test]
    fn startup_errors_keep_the_root_and_main_interface_available(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::ui::notifications::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
            cx.set_reduce_motion(true);
        });
        let handle = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let view =
                    cx.new(|cx| crate::app::ToolboxApp::new(Default::default(), false, window, cx));
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            })
            .unwrap()
        });
        cx.update(|cx| {
            super::report_startup_errors(
                handle.into(),
                ["设置格式错误".into(), "配置生成已停止".into()],
                cx,
            )
        });
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert!(crate::ui::notifications::current(window, cx).len() >= 2);
            assert!(window.find("nav-home").visible());
            assert!(window.find("nav-settings").visible());
        })
        .unwrap();
    }
}
