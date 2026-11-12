mod export;
mod mi;
mod model;
mod windows;

use self::model::{HardwareKind, HardwareRow};
use crate::ui::{
    self,
    theme::{control, layout, preferences, space, text},
};
use gpui_kit::{
    assets::IconName,
    base::TestSupportExt as _,
    component::{
        ActiveTheme, Disableable, Icon, Sizable,
        button::{Button, DropdownButton},
        h_flex,
        scroll::ScrollableElement,
        tooltip::Tooltip,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

actions!(system_information, [SaveScreenshot]);

fn hardware_icon(kind: HardwareKind) -> IconName {
    match kind {
        HardwareKind::Mainboard => IconName::CircuitBoard,
        HardwareKind::Processor => IconName::Cpu,
        HardwareKind::Memory => IconName::MemoryStick,
        HardwareKind::Graphics => IconName::Gpu,
        HardwareKind::Monitor => IconName::Monitor,
        HardwareKind::Disk => IconName::HardDrive,
        HardwareKind::Battery => IconName::Battery,
        HardwareKind::Audio => IconName::AudioLines,
        HardwareKind::Network => IconName::Network,
    }
}
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub(crate) struct SystemInfoPage {
    snapshot: model::Snapshot,
    loading: bool,
    request: u64,
    cancelled: Arc<AtomicBool>,
    task: Option<Task<()>>,
    export_task: Option<Task<()>>,
    exporting: bool,
    _uptime_task: Task<()>,
    uptime: u64,
}

impl SystemInfoPage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let uptime_task = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this
                    .update_in(cx, |this, _, cx| {
                        this.uptime = windows::uptime();
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut page = Self {
            snapshot: model::Snapshot::default(),
            loading: false,
            request: 0,
            cancelled: Arc::new(AtomicBool::new(false)),
            task: None,
            export_task: None,
            exporting: false,
            _uptime_task: uptime_task,
            uptime: windows::uptime(),
        };
        page.refresh(window, cx);
        page
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.cancelled = Arc::new(AtomicBool::new(false));
        let cancelled = self.cancelled.clone();
        self.request += 1;
        let request = self.request;
        self.loading = true;
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let snapshot = cx
                .background_spawn(async move { windows::read(&cancelled) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.request != request {
                    return;
                }
                if !snapshot.errors.is_empty() {
                    ui::show_error(
                        format!("部分系统信息读取失败\n{}", snapshot.errors.join("\n")),
                        window,
                        cx,
                    );
                }
                this.snapshot = snapshot;
                this.loading = false;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn screenshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.exporting || self.loading || self.snapshot.rows.is_empty() {
            return;
        }
        let mut directory = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                ui::show_error(format!("无法获取截图保存目录：{error}"), window, cx);
                return;
            }
        };
        directory.pop();
        let destination = cx.prompt_for_new_path(&directory, Some("SukiToolbox-system-info.png"));
        let snapshot = self.snapshot.clone();
        let uptime = self.uptime;
        let appearance = export::Appearance::current(cx);
        let svg_renderer = cx.svg_renderer();
        self.exporting = true;
        self.export_task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = match destination.await {
                Ok(Ok(Some(path))) => {
                    cx.background_spawn(async move {
                        let png =
                            export::screenshot(&snapshot, uptime, &appearance, &svg_renderer)?;
                        std::fs::write(&path, png).map_err(|error| {
                            format!("截图导出失败（{}）：{error}", path.display())
                        })?;
                        Ok(Some(path))
                    })
                    .await
                }
                Ok(Ok(None)) => Ok(None),
                Ok(Err(error)) => Err(format!("无法选择截图保存路径：{error}")),
                Err(error) => Err(format!("无法选择截图保存路径：{error}")),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.exporting = false;
                match result {
                    Ok(Some(_)) => {
                        ui::notifications::push(
                            window,
                            component::notification::Notification::success("已导出系统信息截图"),
                            cx,
                        );
                    }
                    Ok(None) => {}
                    Err(error) => ui::show_error(error, window, cx),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn hardware_row(&self, row: &HardwareRow, cx: &App) -> impl IntoElement {
        let id = match row.kind {
            HardwareKind::Mainboard => "system-info-mainboard",
            HardwareKind::Processor => "system-info-processor",
            HardwareKind::Memory => "system-info-memory",
            HardwareKind::Graphics => "system-info-graphics",
            HardwareKind::Monitor => "system-info-monitor",
            HardwareKind::Disk => "system-info-disk",
            HardwareKind::Battery => "system-info-battery",
            HardwareKind::Audio => "system-info-audio",
            HardwareKind::Network => "system-info-network",
        };
        let icon = hardware_icon(row.kind);
        let devices = if row.devices.is_empty() {
            v_flex().child(ui::muted("未知", cx))
        } else {
            v_flex()
                .gap(space::XS)
                .children(row.devices.iter().map(|device| {
                    let marker = |id, label: &'static str, icon| {
                        h_flex()
                            .id(id)
                            .test_support()
                            .role(Role::Image)
                            .aria_label(label)
                            .flex_shrink_0()
                            .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
                            .child(
                                Icon::new(icon)
                                    .size(layout::ICON)
                                    .text_color(cx.theme().muted_foreground),
                            )
                    };
                    h_flex()
                        .id(SharedString::from(device.id.to_string()))
                        .w_full()
                        .min_w_0()
                        .gap(space::SM)
                        .child(div().flex_1().min_w_0().child(device.text.clone()))
                        .child(
                            h_flex()
                                .id("system-info-markers")
                                .test_support()
                                .w(layout::SYSTEM_INFO_MARKERS_WIDTH)
                                .flex_shrink_0()
                                .justify_end()
                                .gap(space::SM)
                                .when(device.primary, |group| {
                                    group.child(marker(
                                        "system-info-primary",
                                        "主屏",
                                        IconName::MonitorCheck,
                                    ))
                                })
                                .when(device.virtual_device == Some(true), |group| {
                                    group.child(marker(
                                        "system-info-virtual",
                                        "虚拟设备",
                                        IconName::Boxes,
                                    ))
                                }),
                        )
                }))
        };
        // There are at most nine hardware categories. Device text wraps naturally
        // inside each category instead of using measured or truncated list rows.
        h_flex()
            .id(id)
            .w_full()
            .min_w_0()
            .min_h(preferences::ROW_HEIGHT)
            .px(space::LG)
            .py(space::MD)
            .gap(space::LG)
            .text_size(text::BODY)
            .child(
                h_flex()
                    .w(layout::SYSTEM_INFO_LABEL_WIDTH)
                    .flex_shrink_0()
                    .gap(space::SM)
                    .child(
                        Icon::new(icon)
                            .size(layout::ICON)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(ui::muted(row.kind.label(), cx)),
            )
            .child(
                devices
                    .id(SharedString::from(format!("{id}-values")))
                    .test_support()
                    .flex_1()
                    .min_w_0(),
            )
            .test_support()
    }
}

impl Drop for SystemInfoPage {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Render for SystemInfoPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let summary = |id: &'static str, icon: IconName, label: &'static str, value: String| {
            h_flex()
                .id(id)
                .test_support()
                .flex_1()
                .min_w_0()
                .gap(space::LG)
                .p(space::LG)
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().muted.opacity(ui::theme::opacity::PANEL))
                .child(
                    h_flex()
                        .justify_center()
                        .flex_shrink_0()
                        .size(layout::SYSTEM_INFO_SUMMARY_ICON)
                        .rounded(cx.theme().radius)
                        .bg(cx.theme().muted.opacity(ui::theme::opacity::PANEL))
                        .child(Icon::new(icon).size(layout::ICON_FEATURE)),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(space::XS)
                        .child(ui::muted(label, cx).text_size(text::CAPTION))
                        .child(
                            div()
                                .text_size(text::BODY)
                                .font_weight(text::HEADING)
                                .child(if value.is_empty() {
                                    if self.loading {
                                        "正在读取".into()
                                    } else {
                                        "未知".into()
                                    }
                                } else {
                                    value
                                }),
                        ),
                )
        };
        let details = if self.snapshot.rows.is_empty() {
            ui::empty(
                if self.loading {
                    IconName::LoaderCircle
                } else {
                    IconName::Monitor
                },
                if self.loading {
                    "正在读取系统信息"
                } else {
                    "暂无系统信息"
                },
                cx,
            )
            .into_any_element()
        } else {
            ui::preferences_group(
                "详细信息",
                self.snapshot
                    .rows
                    .iter()
                    .map(|row| self.hardware_row(row, cx).into_any_element())
                    .collect(),
                cx,
            )
            .into_any_element()
        };
        ui::workspace()
            .on_action(cx.listener(|this, _: &SaveScreenshot, window, cx| {
                this.screenshot(window, cx);
            }))
            .child(
                ui::workspace_header("系统信息", "", cx).child(
                    h_flex()
                        .gap(space::SM)
                        .child(
                            Button::new("refresh-system-information")
                                .outline()
                                .label("刷新")
                                .icon(IconName::RefreshCw)
                                .with_size(control::STANDARD)
                                .loading(self.loading)
                                .disabled(self.loading)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.refresh(window, cx)),
                                ),
                        )
                        .child(
                            DropdownButton::new("system-information-copy-menu")
                                .outline()
                                .with_size(control::STANDARD)
                                .menu_accessibility_label("系统信息复制选项")
                                .disabled(
                                    self.loading || self.exporting || self.snapshot.rows.is_empty(),
                                )
                                .dropdown_menu(|menu, _, _| {
                                    menu.menu("保存截图", Box::new(SaveScreenshot))
                                })
                                .button(
                                    Button::new("copy-system-information")
                                        .outline()
                                        .label("复制")
                                        .icon(IconName::Copy)
                                        .loading(self.exporting)
                                        .tooltip("复制全部系统信息")
                                        .with_size(control::STANDARD)
                                        .disabled(self.loading || self.snapshot.rows.is_empty())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                this.snapshot.copy_text(this.uptime),
                                            ));
                                            ui::notifications::push(
                                                window,
                                                component::notification::Notification::success(
                                                    "已复制系统信息",
                                                ),
                                                cx,
                                            );
                                        })),
                                ),
                        ),
                ),
            )
            .child(if self.snapshot.rows.is_empty() {
                details
            } else {
                div()
                    .id("system-information-scroll")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap(space::SECTION)
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap(space::LG)
                                    .items_stretch()
                                    .child(summary(
                                        "system-summary-model",
                                        IconName::Computer,
                                        "型号信息",
                                        self.snapshot.model.clone(),
                                    ))
                                    .child(summary(
                                        "system-summary-os",
                                        IconName::SquareTerminal,
                                        "系统信息",
                                        self.snapshot.os.clone(),
                                    ))
                                    .child(summary(
                                        "system-summary-uptime",
                                        IconName::Timer,
                                        "运行时间",
                                        model::format_uptime(self.uptime),
                                    )),
                            )
                            .child(details),
                    )
                    .into_any_element()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemInfoPage, model};
    use crate::ui::theme::{layout, preferences};
    use gpui_kit::{
        AppContext as _, Bounds, ClipboardItem, TestAppContext, WindowBounds, WindowOptions,
        component::{Theme, ThemeMode},
        point, px, size,
        test::{TestEventExt, TestQueryExt, TestWindowExt},
    };
    use model::{HardwareDevice, HardwareKind};
    use std::sync::{Arc, atomic::AtomicBool};

    #[gpui_kit::test]
    fn overview_wraps_devices_and_keeps_summary_cards_aligned(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
            cx.set_reduce_motion(true);
        });
        let (handle, page) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(620.), px(600.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|cx| {
                        let mut snapshot = model::Snapshot {
                            model: "LENOVO 83HF".into(),
                            os: "Windows 11 家庭版 中文版 (26H2) | 26300.9818".into(),
                            ..Default::default()
                        };
                        snapshot.add(
                            HardwareKind::Memory,
                            vec![HardwareDevice::new(
                                "Samsung 16 GiB DDR5 5200 MT/s ".repeat(8),
                                None,
                            )],
                        );
                        snapshot.add(
                            HardwareKind::Monitor,
                            vec![
                                HardwareDevice {
                                    primary: true,
                                    ..HardwareDevice::new(
                                        "Mi Monitor [27.0″ · 2560 × 1440 · 59 Hz]".into(),
                                        Some(true),
                                    )
                                },
                                HardwareDevice::new(
                                    "Lenovo [15.3″ · 1920 × 1080 · 60 Hz]".into(),
                                    Some(false),
                                ),
                                HardwareDevice::new("第二个虚拟显示器".into(), Some(true)),
                            ],
                        );
                        snapshot.add(HardwareKind::Audio, Vec::new());
                        // Keep system queries outside this layout test.
                        SystemInfoPage {
                            snapshot,
                            loading: false,
                            request: 0,
                            cancelled: Arc::new(AtomicBool::new(false)),
                            task: None,
                            export_task: None,
                            exporting: false,
                            _uptime_task: cx.background_spawn(async {}),
                            uptime: 6284,
                        }
                    })
                },
            )
            .unwrap()
        });
        for (mode, width) in [
            (ThemeMode::Light, 620.),
            (ThemeMode::Dark, 620.),
            (ThemeMode::Light, 900.),
            (ThemeMode::Dark, 900.),
        ] {
            cx.simulate_window_resize(handle, size(px(width), px(600.)));
            cx.update(|cx| Theme::change(mode, None, cx));
            cx.run_until_parked();
            cx.update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                let model = window.find("system-summary-model").bounds();
                let os = window.find("system-summary-os").bounds();
                let uptime = window.find("system-summary-uptime").bounds();
                assert!((model.size.width - os.size.width).abs() <= px(1.));
                assert!((model.size.width - uptime.size.width).abs() <= px(1.));
                assert_eq!(model.origin.y, os.origin.y);
                assert_eq!(model.size.height, os.size.height);
                let memory = window.find("system-info-memory").bounds();
                let values = window.find("system-info-memory-values").bounds();
                assert!(values.size.height > preferences::ROW_HEIGHT.to_pixels(window.rem_size()));
                assert!(values.right() <= memory.right());
                assert!(values.bottom() <= memory.bottom());
                let monitors = window.find("system-info-monitor-values").bounds();
                assert_eq!(values.origin.x, monitors.origin.x);
                let marker_lanes = window
                    .within("system-info-monitor-values")
                    .find_all("system-info-markers");
                assert_eq!(marker_lanes.len(), 3);
                for lane in &marker_lanes {
                    assert_eq!(lane.bounds().right(), monitors.right());
                    assert_eq!(
                        lane.bounds().size.width,
                        layout::SYSTEM_INFO_MARKERS_WIDTH.to_pixels(window.rem_size())
                    );
                }
                let primary = window.find_by_label("主屏").bounds();
                let virtual_markers = window.find_all_by_label("虚拟设备");
                assert_eq!(virtual_markers.len(), 2);
                assert!(primary.right() < virtual_markers[0].bounds().origin.x);
                assert_eq!(primary.center().y, virtual_markers[0].bounds().center().y);
                assert!(primary.origin.y > marker_lanes[0].bounds().origin.y);
                for marker in virtual_markers {
                    assert_eq!(marker.bounds().right(), monitors.right());
                }
                assert!(window.find_all("system-info-battery").is_empty());
                window.click("copy-system-information", cx);
                let copied = cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .unwrap();
                assert!(copied.contains("型号信息：LENOVO 83HF"));
                assert!(copied.contains("系统信息：Windows 11 家庭版 中文版 (26H2) | 26300.9818"));
                assert!(copied.contains("运行时间：0天1小时44分钟44秒"));
                assert!(copied.contains(&"Samsung 16 GiB DDR5 5200 MT/s ".repeat(8)));
                assert!(copied.contains("Lenovo [15.3″ · 1920 × 1080 · 60 Hz]"));
                assert!(
                    copied.contains("主屏 Mi Monitor [27.0″ · 2560 × 1440 · 59 Hz]（虚拟设备）")
                );
                assert!(copied.find("Lenovo [").unwrap() < copied.find("主屏 Mi Monitor").unwrap());
                assert!(copied.contains("声卡：\n  未知"));
            })
            .unwrap();
        }
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("系统信息.png");
        for save in [false, true] {
            cx.update_window(handle, |_, window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string("保持剪贴板内容".into()));
                window.render_frame(cx);
                let trigger = window.find_by_label("系统信息复制选项").bounds().center();
                window.pointer_down(trigger, gpui_kit::MouseButton::Left, cx);
                window.pointer_up(trigger, gpui_kit::MouseButton::Left, cx);
                assert_eq!(window.find_all_by_role(gpui_kit::Role::Menu).len(), 1);
                if save {
                    window.press("down", cx);
                    window.press("enter", cx);
                } else {
                    let item = window.find_by_label("保存截图").bounds().center();
                    window.pointer_down(item, gpui_kit::MouseButton::Left, cx);
                    window.pointer_up(item, gpui_kit::MouseButton::Left, cx);
                }
            })
            .unwrap();
            assert!(cx.did_prompt_for_new_path());
            assert!(!output.exists());
            cx.simulate_new_path_selection(|_| save.then(|| output.clone()));
            cx.run_until_parked();
            assert!(!page.read_with(cx, |page, _| page.exporting));
            cx.update(|cx| {
                assert_eq!(
                    cx.read_from_clipboard()
                        .and_then(|item| item.text())
                        .as_deref(),
                    Some("保持剪贴板内容")
                );
            });
        }
        let image = image::open(&output).unwrap();
        assert!(image.width() >= 1200);
        assert!(image.height() > 600);
        cx.update_window(handle, |_, window, cx| {
            page.update(cx, |page, cx| {
                page.loading = true;
                cx.notify();
            });
            cx.write_to_clipboard(ClipboardItem::new_string("保持原内容".into()));
            window.render_frame(cx);
            assert_eq!(
                window.find("copy-system-information").disabled(),
                Some(true)
            );
            window.click("copy-system-information", cx);
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("保持原内容")
            );
        })
        .unwrap();
    }
}
