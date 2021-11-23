use std::error::Error;

#[repr(C, align(4))]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

pub trait Image: Sized {
    fn width(&self) -> usize;

    fn height(&self) -> usize;

    fn pixels(&self) -> &[Pixel];
}

pub struct DefaultImage {
    width: usize,
    height: usize,
    data: Vec<Pixel>
}

impl Image for DefaultImage {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn pixels(&self) -> &[Pixel] {
        &self.data
    }
}

pub trait ScreenHandle: Sized {
    type Image: Image;
    type Error: Error;

    fn new() -> Result<Self, Self::Error>;

    fn capture(&mut self) -> Result<Self::Image, Self::Error>;

    fn update(&mut self, image: &mut Self::Image) -> Result<(), Self::Error> {
        *image = self.capture()?;
        Ok(())
    }

    fn close(&mut self);
}