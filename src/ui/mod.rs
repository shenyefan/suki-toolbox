use self::theme::{layout, opacity, space, stroke, text};
pub(crate) mod file_field;
pub(crate) mod modal;
pub(crate) mod notifications;
pub(crate) mod theme;
pub(crate) mod virtual_rows;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Icon, Sizable,
        empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};
pub(crate) use theme::{configure_theme, window_tint, workspace_tint};

pub(crate) fn muted(value: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_size(text::BODY)
        .text_color(cx.theme().muted_foreground)
        .child(value.into())
}

pub(crate) fn section(title: impl Into<SharedString>, cx: &App) -> Div {
    h_flex()
        .h(layout::CONTROL_HEIGHT)
        .flex_shrink_0()
        .font_weight(text::EMPHASIS)
        .text_color(cx.theme().foreground)
        .child(title.into())
}

/// Validation failures use the shared native notification stack.
pub(crate) fn show_error(message: String, window: &mut Window, cx: &mut App) {
    notifications::push(
        window,
        component::notification::Notification::error(message),
        cx,
    );
}

pub(crate) fn empty(icon: IconName, heading: &str, cx: &App) -> Div {
    v_flex()
        .items_center()
        .justify_center()
        .gap(space::SM)
        .py(space::REGION)
        .size_full()
        .min_h(layout::EMPTY_STATE_HEIGHT)
        .px(space::LG)
        .child(
            Empty::new().p(space::NONE).header(
                EmptyHeader::new()
                    .max_w_full()
                    .gap(space::SM)
                    .media(EmptyMedia::new().mb(space::NONE).child(
                        if icon == IconName::LoaderCircle {
                            component::spinner::Spinner::new()
                                .icon(IconName::LoaderCircle)
                                .with_size(layout::ICON_FEATURE.to_pixels(cx.theme().font_size))
                                .color(cx.theme().muted_foreground)
                                .into_any_element()
                        } else {
                            Icon::new(icon)
                                .size(layout::ICON_FEATURE)
                                .text_color(cx.theme().muted_foreground)
                                .into_any_element()
                        },
                    ))
                    .title(
                        EmptyTitle::new()
                            .text_size(text::BODY)
                            .font_weight(text::EMPHASIS)
                            .child(heading.to_owned()),
                    ),
            ),
        )
}

/// Fit cards to the actual viewport; keep at least two columns without an upper cap.
pub(crate) fn adaptive_columns(width: Pixels, minimum: Pixels, gap: Pixels) -> usize {
    (((width + gap) / (minimum + gap)).floor() as usize).max(2)
}

/// Shared page frame: fixed heading/toolbar/footer, with the body owning scrolling.
pub(crate) fn workspace() -> Div {
    v_flex()
        .size_full()
        .min_w(space::NONE)
        .min_h(space::NONE)
        .px(space::SECTION)
        .pt(space::MD)
        .pb(space::LG)
        .gap(space::LG)
}

pub(crate) fn workspace_toolbar() -> Div {
    h_flex().w_full().flex_shrink_0().gap(space::SM)
}

pub(crate) fn workspace_body() -> Div {
    div().flex_1().min_w(space::NONE).min_h(space::NONE)
}

pub(crate) fn workspace_footer() -> Div {
    h_flex()
        .min_h(layout::FOOTER_HEIGHT)
        .flex_shrink_0()
        .gap(space::SM)
        .text_size(text::CAPTION)
}

/// Shared title geometry for every destination.
pub(crate) fn workspace_header(title: &str, detail: &str, cx: &App) -> Div {
    h_flex()
        .min_h(layout::TITLE_HEIGHT)
        .gap(space::MD)
        .flex_shrink_0()
        .child(
            div()
                .text_size(text::TITLE)
                .font_weight(text::HEADING)
                .child(title.to_owned()),
        )
        .child(
            muted(detail.to_owned(), cx)
                .text_size(text::CAPTION)
                .flex_1()
                .min_w_0()
                .truncate(),
        )
}

/// Native preferences geometry shared by the settings page.
pub(crate) fn preference_row(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
    cx: &App,
) -> Div {
    let description = description.into();
    h_flex()
        .w_full()
        .flex_shrink_0()
        .min_h(theme::preferences::ROW_HEIGHT)
        .px(space::LG)
        .py(space::MD)
        .gap(space::LG)
        .child(
            v_flex()
                .flex_1()
                .min_w(space::NONE)
                .gap(space::XS)
                .child(div().child(label.into()))
                .when(!description.is_empty(), |label| {
                    label.child(muted(description, cx).text_size(text::CAPTION))
                }),
        )
        .child(control)
}

pub(crate) fn preferences_group(
    title: impl Into<SharedString>,
    rows: Vec<AnyElement>,
    cx: &App,
) -> Div {
    let mut content = v_flex()
        .rounded(cx.theme().radius_lg)
        .bg(theme::preferences_tint(cx));
    for (ix, row) in rows.into_iter().enumerate() {
        if ix > 0 {
            content = content.child(
                div()
                    .mx(space::LG)
                    .h(stroke::HAIRLINE)
                    .bg(cx.theme().foreground.opacity(opacity::PREFERENCES_DIVIDER)),
            );
        }
        content = content.child(row);
    }
    v_flex()
        .gap(space::SM)
        .child(section(title, cx).px(space::LG))
        .child(content)
}

/// Cards keep the focus border inside the virtual row's clipping boundary.
pub(crate) fn selectable_card(
    card: component::button::Button,
    selected: bool,
    cx: &App,
) -> component::button::Button {
    use component::{FocusableExt as _, Selectable as _};
    card.selected(selected)
        .focus_ring(false)
        .border(stroke::HAIRLINE)
        .border_color(transparent_black())
        .bg(cx.theme().muted.opacity(opacity::PANEL))
        .when(selected, |card| card.bg(cx.theme().list_active))
        .focus_visible(|card| card.border_color(cx.theme().ring))
}
