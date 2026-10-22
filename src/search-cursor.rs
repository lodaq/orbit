use slint::private_unstable_api::re_exports::{TextInput, WindowInner};

// Width zero still draws a hairline in FemtoVG. Stop the native blink binding
// on input events instead; Slint keeps editing, selection and IME ownership.
pub fn sync(window: &slint::Window, hidden: bool) {
    let window = WindowInner::from_pub(window);
    let Some(item) = window.focus_item.borrow().upgrade() else {
        return;
    };
    let Some(input) = item.downcast::<TextInput>() else {
        return;
    };
    let input = input.as_pin_ref();
    if hidden
        && TextInput::FIELD_OFFSETS
            .preedit_text()
            .apply_pin(input)
            .get()
            .is_empty()
    {
        input.cursor_visible.set(false);
    } else if TextInput::FIELD_OFFSETS.has_focus().apply_pin(input).get() {
        window.set_cursor_blink_binding(&input.cursor_visible);
    }
}
