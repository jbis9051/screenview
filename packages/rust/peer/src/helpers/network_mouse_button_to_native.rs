use common::messages::rvd::ButtonsMask;
use native::api::MouseButton;

macro_rules! map_button {
    ($button: ident,
        $(
          $network: expr => $native: expr
        ),*
    ) => {
        {
            $(
               if $button.contains($network){
                   return $native;
               }
            )*
        }
    };
}

/// network MUST only contain one flag
pub fn network_mouse_button_to_native(button: &ButtonsMask) -> MouseButton {
    map_button!(button,
        ButtonsMask::LEFT => MouseButton::Left,
        ButtonsMask::RIGHT => MouseButton::Right,
        ButtonsMask::MIDDLE => MouseButton::Center,
        ButtonsMask::SCROLL_RIGHT => MouseButton::ScrollRight,
        ButtonsMask::SCROLL_LEFT => MouseButton::ScrollLeft,
        ButtonsMask::SCROLL_DOWN => MouseButton::ScrollDown,
        ButtonsMask::SCROLL_UP => MouseButton::ScrollUp
    );
    panic!("No match found for button");
}
