use image::RgbImage;
use std::error::Error;

pub(crate) trait NativeCapture: Sized {
    type Error: Error;

    fn new() -> Result<Self, Self::Error>;

    fn capture_screen(&self) -> Result<RgbImage, Self::Error>;

    fn update_screen_capture(&self, cap: &mut RgbImage) -> Result<(), Self::Error> {
        *cap = self.capture_screen()?;
        Ok(())
    }

    fn pointer_position(&self) -> Result<(u32, u32), Self::Error>;

    fn key_toggle(&self, keysym: u32);
}
