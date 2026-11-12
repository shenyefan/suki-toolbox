//! A complete system-information image, using the page's theme, geometry and icons.
use super::{
    hardware_icon,
    model::{self, Snapshot},
};
use crate::ui::theme::{layout, opacity, preferences, space, system_info_export, text};
use gpui_kit::{
    App, AssetSource, Hsla, Rems, SvgRenderer,
    assets::{AllAssets, IconName},
};
use windows::{
    Win32::{
        Foundation::{COLORREF, RECT},
        Graphics::Gdi::*,
        System::SystemInformation::GetLocalTime,
    },
    core::w,
};

pub(super) struct Appearance {
    background: u32,
    summary: u32,
    icon_background: u32,
    details: u32,
    foreground: u32,
    muted: u32,
    divider: u32,
    padding: i32,
    gap: i32,
    small_gap: i32,
    tiny_gap: i32,
    row_padding: i32,
    section_gap: i32,
    label_width: i32,
    markers_width: i32,
    icon: i32,
    feature_icon: i32,
    icon_box: i32,
    row_height: i32,
    heading_height: i32,
    radius: i32,
    icon_radius: i32,
    fonts: [i32; 4],
}

impl Appearance {
    pub(super) fn current(cx: &App) -> Self {
        let theme = system_info_export::solid_theme(cx);
        let pixels = |value: Rems| {
            (f32::from(value.to_pixels(theme.font_size)) * system_info_export::SCALE).round() as i32
        };
        let background = theme.background;
        let summary = background.blend(theme.muted.opacity(opacity::PANEL));
        let details = background.blend(theme.muted.opacity(if theme.is_dark() {
            opacity::PREFERENCES_DARK
        } else {
            opacity::PREFERENCES_LIGHT
        }));
        Self {
            background: rgb(background),
            summary: rgb(summary),
            icon_background: rgb(summary.blend(theme.muted.opacity(opacity::PANEL))),
            details: rgb(details),
            foreground: rgb(theme.foreground),
            muted: rgb(theme.muted_foreground),
            divider: rgb(details.blend(theme.foreground.opacity(opacity::PREFERENCES_DIVIDER))),
            padding: pixels(space::SECTION),
            gap: pixels(space::LG),
            small_gap: pixels(space::SM),
            tiny_gap: pixels(space::XS),
            row_padding: pixels(space::MD),
            section_gap: pixels(space::SECTION),
            label_width: pixels(layout::SYSTEM_INFO_LABEL_WIDTH),
            markers_width: pixels(layout::SYSTEM_INFO_MARKERS_WIDTH),
            icon: pixels(layout::ICON),
            feature_icon: pixels(layout::ICON_FEATURE),
            icon_box: pixels(layout::SYSTEM_INFO_SUMMARY_ICON),
            row_height: pixels(preferences::ROW_HEIGHT),
            heading_height: pixels(layout::CONTROL_HEIGHT),
            radius: (f32::from(theme.radius_lg) * system_info_export::SCALE).round() as i32,
            icon_radius: (f32::from(theme.radius) * system_info_export::SCALE).round() as i32,
            fonts: [
                pixels(text::BODY),
                pixels(text::BODY),
                pixels(text::CAPTION),
                pixels(text::TITLE),
            ],
        }
    }
}

fn rgb(color: Hsla) -> u32 {
    let color = color.to_rgb();
    ((color.r * 255.).round() as u32) << 16
        | ((color.g * 255.).round() as u32) << 8
        | (color.b * 255.).round() as u32
}

fn native_color(rgb: u32) -> COLORREF {
    COLORREF(((rgb & 255) << 16) | (rgb & 0xff00) | (rgb >> 16))
}

struct Text {
    value: Vec<u16>,
    rect: RECT,
    font: usize,
    color: u32,
}

enum Paint {
    Panel {
        rect: RECT,
        color: u32,
        radius: i32,
    },
    Text(Text),
    Icon {
        icon: IconName,
        x: i32,
        y: i32,
        side: i32,
        color: u32,
    },
}

struct Canvas {
    dc: HDC,
    fonts: [HFONT; 4],
    previous_font: HGDIOBJ,
}

impl Canvas {
    fn new(style: &Appearance) -> Result<Self, String> {
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return Err("无法创建截图绘图上下文".into());
            }
            let fonts = std::array::from_fn(|index| {
                CreateFontW(
                    -style.fonts[index],
                    0,
                    0,
                    0,
                    if index == 1 || index == 3 {
                        FW_SEMIBOLD.0 as i32
                    } else {
                        FW_NORMAL.0 as i32
                    },
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    ANTIALIASED_QUALITY,
                    0,
                    w!("Segoe UI"),
                )
            });
            if fonts.iter().any(|font| font.is_invalid()) {
                for font in fonts {
                    let _ = DeleteObject(font.into());
                }
                let _ = DeleteDC(dc);
                return Err("无法创建截图字体".into());
            }
            let previous_font = SelectObject(dc, fonts[0].into());
            Ok(Self {
                dc,
                fonts,
                previous_font,
            })
        }
    }

    fn text(
        &self,
        value: &str,
        x: i32,
        y: i32,
        width: i32,
        font: usize,
        color: u32,
    ) -> Result<Text, String> {
        let mut text = Text {
            value: value.encode_utf16().collect(),
            rect: RECT {
                left: x,
                top: y,
                right: x + width,
                bottom: y,
            },
            font,
            color,
        };
        unsafe {
            SelectObject(self.dc, self.fonts[font].into());
            if DrawTextW(
                self.dc,
                &mut text.value,
                &mut text.rect,
                DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX,
            ) == 0
            {
                return Err("无法测量截图内容".into());
            }
        }
        Ok(text)
    }

    fn raster(
        &self,
        paints: &mut [Paint],
        height: i32,
        background: u32,
    ) -> Result<image::RgbaImage, String> {
        unsafe {
            let width = system_info_export::WIDTH;
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(Some(self.dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
                .map_err(|error| format!("无法创建截图：{error}"))?;
            let previous_bitmap = SelectObject(self.dc, bitmap.into());
            let result = (|| {
                let len = width as usize * height as usize * 4;
                let pixels = std::slice::from_raw_parts_mut(bits.cast::<u8>(), len);
                for pixel in pixels.as_chunks_mut::<4>().0 {
                    *pixel = [
                        background as u8,
                        (background >> 8) as u8,
                        (background >> 16) as u8,
                        255,
                    ];
                }
                SetBkMode(self.dc, TRANSPARENT);
                for paint in paints {
                    match paint {
                        Paint::Panel {
                            rect,
                            color,
                            radius,
                        } => {
                            let brush = CreateSolidBrush(native_color(*color));
                            if brush.is_invalid() {
                                return Err("无法创建截图画刷".into());
                            }
                            let previous_brush = SelectObject(self.dc, brush.into());
                            let previous_pen = SelectObject(self.dc, GetStockObject(NULL_PEN));
                            let result = RoundRect(
                                self.dc,
                                rect.left,
                                rect.top,
                                rect.right,
                                rect.bottom,
                                *radius * 2,
                                *radius * 2,
                            )
                            .ok();
                            SelectObject(self.dc, previous_pen);
                            SelectObject(self.dc, previous_brush);
                            let _ = DeleteObject(brush.into());
                            result.map_err(|error| format!("无法绘制截图卡片：{error}"))?;
                        }
                        Paint::Text(text) => {
                            SelectObject(self.dc, self.fonts[text.font].into());
                            SetTextColor(self.dc, native_color(text.color));
                            if DrawTextW(
                                self.dc,
                                &mut text.value,
                                &mut text.rect,
                                DT_WORDBREAK | DT_NOPREFIX,
                            ) == 0
                            {
                                return Err("无法绘制截图文字".into());
                            }
                        }
                        Paint::Icon { .. } => {}
                    }
                }
                GdiFlush()
                    .ok()
                    .map_err(|error| format!("无法完成截图：{error}"))?;
                let mut pixels = pixels.to_vec();
                for pixel in pixels.as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                    pixel[3] = 255;
                }
                image::RgbaImage::from_raw(width as u32, height as u32, pixels)
                    .ok_or_else(|| "截图像素尺寸无效".into())
            })();
            SelectObject(self.dc, previous_bitmap);
            let _ = DeleteObject(bitmap.into());
            result
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous_font);
            for font in self.fonts {
                let _ = DeleteObject(font.into());
            }
            let _ = DeleteDC(self.dc);
        }
    }
}

pub(super) fn screenshot(
    snapshot: &Snapshot,
    uptime: u64,
    style: &Appearance,
    renderer: &SvgRenderer,
) -> Result<Vec<u8>, String> {
    let time = unsafe { GetLocalTime() };
    let timestamp = format!(
        "生成时间：{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond,
    );
    let canvas = Canvas::new(style)?;
    let (mut paints, height) = layout_report(&canvas, snapshot, uptime, style, &timestamp)?;
    let mut image = canvas.raster(&mut paints, height, style.background)?;
    // Use the same embedded Lucide SVGs and GPUI renderer as the visible page.
    for paint in &paints {
        if let Paint::Icon {
            icon,
            x,
            y,
            side,
            color,
        } = paint
        {
            let svg = AllAssets
                .load(icon.path().as_ref())
                .map_err(|error| format!("无法读取截图图标：{error}"))?
                .ok_or_else(|| format!("截图图标不存在：{}", icon.path()))?;
            let svg = std::str::from_utf8(&svg)
                .map_err(|error| format!("截图图标格式无效：{error}"))?
                .replace("currentColor", &format!("#{color:06x}"));
            let parsed = renderer
                .parse_svg(svg.as_bytes())
                .map_err(|error| format!("无法解析截图图标：{error}"))?;
            let raster = renderer
                .render_parsed(
                    &parsed,
                    gpui_kit::SvgSize::ExactSize(gpui_kit::size(
                        (*side as u32).into(),
                        (*side as u32).into(),
                    )),
                )
                .map_err(|error| format!("无法绘制截图图标：{error}"))?;
            let mut pixels = raster.as_bytes(0).ok_or("截图图标没有像素数据")?.to_vec();
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
            let icon = image::RgbaImage::from_raw(*side as u32, *side as u32, pixels)
                .ok_or("截图图标尺寸无效")?;
            image::imageops::overlay(&mut image, &icon, *x as i64, *y as i64);
        }
    }
    // The report has an opaque theme backdrop, including under antialiased icons.
    for pixel in image.pixels_mut() {
        pixel[3] = 255;
    }
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|error| format!("无法编码截图：{error}"))?;
    Ok(png.into_inner())
}

fn layout_report(
    canvas: &Canvas,
    snapshot: &Snapshot,
    uptime: u64,
    s: &Appearance,
    timestamp: &str,
) -> Result<(Vec<Paint>, i32), String> {
    let mut paints = Vec::new();
    let width = system_info_export::WIDTH - s.padding * 2;
    let heading = canvas.text("系统信息", s.padding, s.padding, width, 3, s.foreground)?;
    let mut y = heading.rect.bottom + s.gap;
    paints.push(Paint::Text(heading));

    let card_width = (width - s.gap * 2) / 3;
    let mut summaries = Vec::new();
    let mut card_height = 0;
    for (index, (label, value, icon)) in [
        (
            "型号信息",
            if snapshot.model.is_empty() {
                "未知".into()
            } else {
                snapshot.model.clone()
            },
            IconName::Computer,
        ),
        (
            "系统信息",
            if snapshot.os.is_empty() {
                "未知".into()
            } else {
                snapshot.os.clone()
            },
            IconName::SquareTerminal,
        ),
        ("运行时间", model::format_uptime(uptime), IconName::Timer),
    ]
    .into_iter()
    .enumerate()
    {
        let left = s.padding + (card_width + s.gap) * index as i32;
        let x = left + s.gap * 2 + s.icon_box;
        let text_width = card_width - s.gap * 3 - s.icon_box;
        let label = canvas.text(label, x, y + s.gap, text_width, 2, s.muted)?;
        let value = canvas.text(
            &value,
            x,
            label.rect.bottom + s.tiny_gap,
            text_width,
            1,
            s.foreground,
        )?;
        card_height = card_height.max((value.rect.bottom - y + s.gap).max(s.icon_box + s.gap * 2));
        summaries.push((left, icon, label, value));
    }
    for (left, icon, mut label, mut value) in summaries {
        paints.push(Paint::Panel {
            rect: RECT {
                left,
                top: y,
                right: left + card_width,
                bottom: y + card_height,
            },
            color: s.summary,
            radius: s.radius,
        });
        let offset = (card_height - s.gap * 2 - (value.rect.bottom - label.rect.top)) / 2;
        label.rect.top += offset;
        label.rect.bottom += offset;
        value.rect.top += offset;
        value.rect.bottom += offset;
        paints.push(Paint::Text(label));
        paints.push(Paint::Text(value));
        let top = y + (card_height - s.icon_box) / 2;
        paints.push(Paint::Panel {
            rect: RECT {
                left: left + s.gap,
                top,
                right: left + s.gap + s.icon_box,
                bottom: top + s.icon_box,
            },
            color: s.icon_background,
            radius: s.icon_radius,
        });
        paints.push(Paint::Icon {
            icon,
            x: left + s.gap + (s.icon_box - s.feature_icon) / 2,
            y: top + (s.icon_box - s.feature_icon) / 2,
            side: s.feature_icon,
            color: s.foreground,
        });
    }
    y += card_height + s.section_gap;
    let mut label = canvas.text(
        "详细信息",
        s.padding + s.gap,
        y,
        width - s.gap * 2,
        1,
        s.muted,
    )?;
    let offset = (s.heading_height - (label.rect.bottom - label.rect.top)) / 2;
    label.rect.top += offset;
    label.rect.bottom += offset;
    paints.push(Paint::Text(label));
    y += s.heading_height + s.small_gap;
    let panel = paints.len();
    let panel_top = y;
    paints.push(Paint::Panel {
        rect: RECT::default(),
        color: s.details,
        radius: s.radius,
    });
    for (index, row) in snapshot.rows.iter().enumerate() {
        if index > 0 {
            paints.push(Paint::Panel {
                rect: RECT {
                    left: s.padding + s.gap,
                    top: y,
                    right: s.padding + width - s.gap,
                    bottom: y + 2,
                },
                color: s.divider,
                radius: 0,
            });
            y += 2;
        }
        let row_top = y;
        let x = s.padding + s.gap;
        let label = canvas.text(
            row.kind.label(),
            x + s.icon + s.small_gap,
            y + s.row_padding,
            s.label_width - s.icon - s.small_gap,
            0,
            s.muted,
        )?;
        paints.push(Paint::Icon {
            icon: hardware_icon(row.kind),
            x,
            y: y + s.row_padding + (label.rect.bottom - label.rect.top - s.icon) / 2,
            side: s.icon,
            color: s.muted,
        });
        paints.push(Paint::Text(label));
        let value_x = x + s.label_width + s.gap;
        let lane_right = s.padding + width - s.gap;
        let text_width = lane_right - s.markers_width - s.small_gap - value_x;
        y += s.row_padding;
        if row.devices.is_empty() {
            let unknown = canvas.text("未知", value_x, y, text_width, 0, s.muted)?;
            y = unknown.rect.bottom;
            paints.push(Paint::Text(unknown));
        }
        for (index, device) in row.devices.iter().enumerate() {
            if index > 0 {
                y += s.tiny_gap;
            }
            let value = canvas.text(&device.text, value_x, y, text_width, 0, s.foreground)?;
            let icon_y = y + (value.rect.bottom - value.rect.top - s.icon) / 2;
            let mut right = lane_right;
            if device.virtual_device == Some(true) {
                right -= s.icon;
                paints.push(Paint::Icon {
                    icon: IconName::Boxes,
                    x: right,
                    y: icon_y,
                    side: s.icon,
                    color: s.muted,
                });
                right -= s.small_gap;
            }
            if device.primary {
                paints.push(Paint::Icon {
                    icon: IconName::MonitorCheck,
                    x: right - s.icon,
                    y: icon_y,
                    side: s.icon,
                    color: s.muted,
                });
            }
            y = value.rect.bottom;
            paints.push(Paint::Text(value));
        }
        y = (y + s.row_padding).max(row_top + s.row_height);
    }
    if let Paint::Panel { rect, .. } = &mut paints[panel] {
        *rect = RECT {
            left: s.padding,
            top: panel_top,
            right: s.padding + width,
            bottom: y,
        };
    }
    y += s.section_gap;
    let brand = canvas.text(
        &format!("Suki Toolbox  {}", env!("CARGO_PKG_VERSION")),
        s.padding,
        y,
        width,
        1,
        s.foreground,
    )?;
    y = brand.rect.bottom + s.small_gap;
    paints.push(Paint::Text(brand));
    let generated = canvas.text(timestamp, s.padding, y, width, 2, s.muted)?;
    y = generated.rect.bottom + s.padding;
    paints.push(Paint::Text(generated));
    Ok((paints, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{
        TestAppContext,
        component::{Theme, ThemeMode},
    };
    use model::{HardwareDevice, HardwareKind};

    #[gpui_kit::test]
    fn full_report_retains_theme_cards_markers_and_offscreen_devices(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
        });
        let mut snapshot = Snapshot {
            model: "LENOVO 83HF".into(),
            os: "Windows 11 家庭版 中文版 (26H2) | 26300.9818".into(),
            ..Default::default()
        };
        snapshot.add(
            HardwareKind::Memory,
            vec![HardwareDevice::new(
                "Samsung 16.0 GiB DDR5 5200 MT/s（系统报告已使用插槽 1 / 2）".into(),
                None,
            )],
        );
        snapshot.add(
            HardwareKind::Monitor,
            vec![HardwareDevice {
                primary: true,
                ..HardwareDevice::new(
                    "Mi Monitor [27.0″ · 2560 × 1440 · 59.95 Hz]".into(),
                    Some(true),
                )
            }],
        );
        snapshot.add(HardwareKind::Audio, Vec::new());
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let (style, renderer) = cx.update(|cx| {
                Theme::change(mode, None, cx);
                crate::ui::configure_theme(cx);
                (Appearance::current(cx), cx.svg_renderer())
            });
            let canvas = Canvas::new(&style).unwrap();
            let (paints, height) = layout_report(
                &canvas,
                &snapshot,
                6284,
                &style,
                "生成时间：2026-11-12 12:00:00",
            )
            .unwrap();
            assert_eq!(paints.iter().filter(|paint| matches!(paint, Paint::Panel { radius, .. } if *radius == style.radius)).count(), 4);
            assert!(paints.iter().any(|paint| matches!(
                paint,
                Paint::Icon {
                    icon: IconName::MonitorCheck,
                    ..
                }
            )));
            assert!(paints.iter().any(|paint| matches!(
                paint,
                Paint::Icon {
                    icon: IconName::Boxes,
                    ..
                }
            )));
            let png = screenshot(&snapshot, 6284, &style, &renderer).unwrap();
            let image = image::load_from_memory(&png).unwrap().to_rgba8();
            assert_eq!(image.width(), system_info_export::WIDTH as u32);
            assert_eq!(image.height(), height as u32);
            let corner = image.get_pixel(0, 0);
            assert_eq!(corner[0], (style.background >> 16) as u8);
            assert!(image.pixels().all(|pixel| pixel[3] == 255));
            if let Ok(directory) = std::env::var("SUKI_TOOLBOX_EXPORT_PREVIEW") {
                std::fs::write(
                    std::path::Path::new(&directory).join(if mode == ThemeMode::Dark {
                        "system-info-dark.png"
                    } else {
                        "system-info-light.png"
                    }),
                    &png,
                )
                .unwrap();
            }
            let mut long = snapshot.clone();
            long.add(
                HardwareKind::Network,
                (0..100)
                    .map(|index| {
                        HardwareDevice::new(
                            format!("网卡 {index} 中文设备 & Network Adapter"),
                            Some(true),
                        )
                    })
                    .collect(),
            );
            let (paints, long_height) = layout_report(
                &canvas,
                &long,
                6284,
                &style,
                "生成时间：2026-11-12 12:00:00",
            )
            .unwrap();
            assert!(long_height > height + 2000);
            assert!(paints.iter().any(|paint| matches!(paint, Paint::Text(text) if String::from_utf16_lossy(&text.value).contains("网卡 99"))));
        }
    }
}
