use image::RgbImage;
use std::{error::Error, fmt::Debug};

pub(crate) trait NativeCapture: Sized {
    type Error: Error;

    fn new() -> Result<Self, Self::Error>;

    fn capture_screen(&self) -> Result<RgbImage, Self::Error>;

    fn update_screen_capture(&self, cap: &mut RgbImage) -> Result<(), Self::Error> {
        *cap = self.capture_screen()?;
        Ok(())
    }

    fn wait_for_event(&self) -> Result<Event, Self::Error>;
}

pub struct Event(pub Box<dyn Debug>);
