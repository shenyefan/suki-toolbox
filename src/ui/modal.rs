use super::theme::{control, layout, space};
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Disableable, Sizable, WindowExt,
        button::{Button, ButtonVariants},
        dialog::{Dialog, DialogAction, DialogContent, DialogEntrance, DialogFooter},
        scroll::ScrollableElement,
    },
    *,
};

pub(crate) fn frame(
    dialog: Dialog,
    title: impl Into<SharedString>,
    window: &Window,
    cx: &App,
) -> Dialog {
    dialog
        .title(title.into())
        .entrance(DialogEntrance::FadeSlide)
        .width(layout::DIALOG_WIDTH.to_pixels(window.rem_size()))
        .bg(cx.theme().popover)
        .text_color(cx.theme().popover_foreground)
        .rounded(cx.theme().radius_lg)
        .p(space::SECTION)
        .gap(space::LG)
        .overlay_closable(false)
}

pub(crate) fn content(
    id: &'static str,
    body: Entity<impl Render>,
) -> impl Fn(DialogContent, &mut Window, &mut App) -> DialogContent {
    move |content, window, _| {
        let height = (window.viewport_size().height
            - layout::DIALOG_RESERVED_HEIGHT.to_pixels(window.rem_size()))
        .max(Pixels::ZERO);
        content.flex_initial().min_h_0().max_h(height).child(
            div()
                .id(id)
                .min_h_0()
                .max_h(height)
                .overflow_y_scrollbar()
                .child(body.clone()),
        )
    }
}

pub(crate) fn footer(pending: bool) -> DialogFooter {
    DialogFooter::new()
        .justify_end()
        .gap(space::SM)
        .child(
            Button::new("dialog-cancel")
                .icon(IconName::X)
                .with_size(control::STANDARD)
                .outline()
                .label("取消")
                .disabled(pending)
                .on_click(|_, window, cx| window.close_dialog(cx)),
        )
        .child(
            div().flex_none().child(
                DialogAction::new().child(
                    Button::new("dialog-save")
                        .icon(IconName::Save)
                        .with_size(control::STANDARD)
                        .primary()
                        .label("保存")
                        .loading(pending)
                        .disabled(pending),
                ),
            ),
        )
}
