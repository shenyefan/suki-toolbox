//! One native notification list per window, painted above modal popups.
use gpui_kit::{
    base::{POPUP_PRIORITY, Root, RootPlugin},
    component::notification::{Notification, NotificationList},
    *,
};
struct ToastLayer {
    list: Entity<NotificationList>,
    _list_subscription: Subscription,
}
impl ToastLayer {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let list = cx.new(|cx| NotificationList::new(window, cx));
        let subscription = cx.observe(&list, |_, _, cx| cx.notify());
        Self {
            list,
            _list_subscription: subscription,
        }
    }
}
impl RootPlugin for ToastLayer {}
impl Render for ToastLayer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        deferred(div().absolute().inset_0().child(self.list.clone()))
            .with_priority(POPUP_PRIORITY + 10)
    }
}
pub(crate) fn init(cx: &mut App) {
    Root::register_plugin(cx, ToastLayer::new);
}
fn list(window: &Window, cx: &App) -> Option<Entity<NotificationList>> {
    Root::read(window, cx)
        .plugin::<ToastLayer>()
        .map(|layer| layer.read(cx).list.clone())
}
pub(crate) fn push(
    window: &mut Window,
    note: impl Into<Notification>,
    cx: &mut App,
) -> Option<Entity<Notification>> {
    let list = list(window, cx)?;
    list.update(cx, |list, cx| {
        list.push(note.into().autohide(true), window, cx)
    });
    list.read(cx).notifications().last().cloned()
}

#[cfg(test)]
pub(crate) fn current(window: &Window, cx: &App) -> Vec<Entity<Notification>> {
    list(window, cx)
        .map(|list| list.read(cx).notifications())
        .unwrap_or_default()
}
