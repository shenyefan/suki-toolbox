//! Lazy rows with one retained native scroll owner.
use gpui_kit::{
    component::{VirtualListScrollHandle, scroll::Scrollbar, v_virtual_list},
    *,
};
use std::rc::Rc;

pub(crate) fn rows<V: Render>(
    owner: Entity<V>,
    id: impl Into<ElementId>,
    heights: Vec<Pixels>,
    handle: &VirtualListScrollHandle,
    render: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) -> AnyElement + 'static,
) -> AnyElement {
    let sizes: Rc<Vec<Size<Pixels>>> = Rc::new(
        heights
            .into_iter()
            .map(|height| size(px(0.), height))
            .collect(),
    );
    let sizes_for_rows = sizes.clone();
    div()
        .relative()
        .size_full()
        .min_h(px(0.))
        .child(
            v_virtual_list(owner, id, sizes, move |view, range, window, cx| {
                range
                    .map(|index| {
                        div()
                            .w_full()
                            .h(sizes_for_rows[index].height)
                            .flex_none()
                            .overflow_hidden()
                            .child(render(view, index, window, cx))
                    })
                    .collect::<Vec<_>>()
            })
            .track_scroll(handle),
        )
        .child(Scrollbar::vertical(handle))
        .into_any_element()
}
