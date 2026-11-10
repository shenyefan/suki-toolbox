//! Native file/folder prompts, with lifetime owned by the form that opened them.
use super::{
    show_error,
    theme::{control, space},
};
use gpui_kit::{
    assets::IconName,
    component::{
        Disableable, Sizable,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputState},
    },
    *,
};

#[derive(Clone, Copy)]
pub(crate) enum PathKind {
    File,
    Folder,
    Image,
}

/// Shared icon form row: editable value followed by an interactive preview.
pub(crate) fn preview_field(
    id: &'static str,
    label: &'static str,
    input: &Entity<InputState>,
    pending: bool,
    preview: impl IntoElement,
) -> Div {
    h_flex()
        .w_full()
        .min_w_0()
        .gap(space::SM)
        .child(
            Input::new(input)
                .id(id)
                .aria_label(label)
                .with_size(control::STANDARD)
                .flex_1()
                .min_w_0()
                .disabled(pending),
        )
        .child(preview)
}

pub(crate) fn field(
    id: &'static str,
    label: &'static str,
    input: &Entity<InputState>,
    pending: bool,
    choose: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    h_flex()
        .w_full()
        .min_w_0()
        .gap(space::SM)
        .child(
            Input::new(input)
                .id(id)
                .aria_label(label)
                .with_size(control::STANDARD)
                .flex_1()
                .min_w_0()
                .disabled(pending),
        )
        .child(
            Button::new((ElementId::from(id), "choose"))
                .ghost()
                .with_size(control::STANDARD)
                .icon(IconName::FolderOpen)
                .accessibility_label(format!("选择{label}"))
                .tooltip(format!("选择{label}…"))
                .disabled(pending)
                .on_click(choose),
        )
}

pub(crate) fn choose<T: 'static>(
    input: Entity<InputState>,
    kind: PathKind,
    format_path: impl FnOnce(
        &mut T,
        std::path::PathBuf,
        &mut Window,
        &mut Context<T>,
    ) -> Result<String, String>
    + 'static,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Task<()> {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: !matches!(kind, PathKind::Folder),
        directories: matches!(kind, PathKind::Folder),
        multiple: false,
        prompt: Some(
            match kind {
                PathKind::File => "选择工具文件",
                PathKind::Folder => "选择分类目录",
                PathKind::Image => "选择图标图片",
            }
            .into(),
        ),
    });
    let input = input.downgrade();
    cx.spawn_in(window, async move |owner, cx| {
        let result = match receiver.await {
            Ok(Ok(Some(paths))) => paths.into_iter().next().map(Ok),
            Ok(Ok(None)) => None,
            Ok(Err(error)) => Some(Err(error.to_string())),
            Err(error) => Some(Err(error.to_string())),
        };
        if let Some(result) = result {
            let _ = owner.update_in(cx, |owner, window, cx| match result {
                Ok(path) => {
                    if matches!(kind, PathKind::Image)
                        && !path.extension().is_some_and(|ext| {
                            ["png", "jpg", "jpeg", "ico", "bmp", "gif", "webp"]
                                .iter()
                                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
                        })
                    {
                        show_error(
                            "请选择 PNG、JPEG、ICO、BMP、GIF 或 WebP 图片".into(),
                            window,
                            cx,
                        );
                        return;
                    }
                    let value = match format_path(owner, path, window, cx) {
                        Ok(value) => value,
                        Err(error) => {
                            show_error(error, window, cx);
                            return;
                        }
                    };
                    let _ = input.update(cx, |input, cx| {
                        input.set_value(value, window, cx);
                        if !matches!(kind, PathKind::Image) {
                            input.focus(window, cx);
                        }
                    });
                }
                Err(error) => show_error(format!("无法选择文件：{error}"), window, cx),
            });
        }
    })
}
