use std::path::Path;

use crate::error::{Result, TetraError};
use crate::fs;
use crate::graphics::{Color, Rectangle, Texture, TextureFormat};
use crate::math::Vec2;
use crate::Context;

/// Raw image data.
///
/// The data can be stored in a variety of formats, as represented by the
/// [`TextureFormat`] enum.
///
/// # Supported File Formats
///
/// Images can be decoded from various common file formats via the [`new`](ImageData::new)
/// and [`from_encoded`](ImageData::from_encoded) constructors. Individual
/// decoders can be enabled or disabled via Cargo feature flags.
///
/// | Format | Cargo feature | Enabled by default? |
/// |-|-|-|
/// | PNG | `texture_png` | Yes |
/// | JPEG | `texture_jpeg` | Yes |
/// | GIF | `texture_gif` | Yes |
/// | BMP | `texture_bmp` | Yes |
/// | TIFF | `texture_tiff` | No |
/// | TGA | `texture_tga` | No |
/// | WebP | `texture_webp` | No |
/// | ICO | `texture_ico` | No |
/// | PNM | `texture_pnm` | No |
/// | DDS/DXT | `texture_dds` | No |
#[derive(Debug, Clone)]
pub struct ImageData {
    data: Vec<u8>,
    width: usize,
    height: usize,
    format: TextureFormat,
}

impl ImageData {
    /// Loads image data from the given file.
    ///
    /// The format will be determined based on the file extension.
    ///
    /// # Errors
    ///
    /// * [`TetraError::FailedToLoadAsset`] will be returned if the file could not be loaded.
    /// * [`TetraError::InvalidTexture`] will be returned if the image data was invalid.
    pub fn new<P>(path: P) -> Result<ImageData>
    where
        P: AsRef<Path>,
    {
        let image = fs::read_to_image(path)?.into_rgba8();
        let width = image.width() as usize;
        let height = image.height() as usize;

        Ok(ImageData {
            data: image.into_raw(),
            width,
            height,
            format: TextureFormat::Rgba8,
        })
    }

    /// Creates an `ImageData` from raw pixel data.
    ///
    /// This function takes `Into<Vec<u8>>`. If you pass a `Vec<u8>`, that `Vec` will
    /// be reused for the created `ImageData` without reallocating. Otherwise, the data
    /// will be copied.
    ///
    /// This function requires you to provide enough data to fill the image's bounds.
    /// If you provide too little data, an error will be returned.
    /// If you provide too much data, it will be truncated.
    ///
    /// # Errors
    ///
    /// * [`TetraError::NotEnoughData`] will be returned if not enough data is provided to fill
    /// the image.
    pub fn from_data<D>(
        width: i32,
        height: i32,
        format: TextureFormat,
        data: D,
    ) -> Result<ImageData>
    where
        D: Into<Vec<u8>>,
    {
        let mut data = data.into();
        let width = width as usize;
        let height = height as usize;

        let expected = width * height * format.stride();
        let actual = data.len();

        if actual < expected {
            return Err(TetraError::NotEnoughData { expected, actual });
        }

        data.truncate(expected);

        Ok(ImageData {
            data,
            width,
            height,
            format,
        })
    }

    /// Decodes image data that is encoded in one of Tetra's supported
    /// file formats (except for TGA).
    ///
    /// This is useful in combination with [`include_bytes`](std::include_bytes), as it
    /// allows you to include your image data directly in the binary.
    ///
    /// The format will be determined based on the 'magic bytes' at the beginning of the
    /// data. Note that TGA files do not have recognizable magic bytes, so this function
    /// will not recognize them.
    ///
    /// # Errors
    ///
    /// * [`TetraError::InvalidTexture`] will be returned if the image data was invalid.
    pub fn from_encoded(data: &[u8]) -> Result<ImageData> {
        let image = image::load_from_memory(data)
            .map_err(TetraError::InvalidTexture)?
            .into_rgba8();

        let width = image.width() as usize;
        let height = image.height() as usize;

        Ok(ImageData {
            data: image.into_raw(),
            width,
            height,
            format: TextureFormat::Rgba8,
        })
    }

    /// Returns the width of the image.
    pub fn width(&self) -> i32 {
        self.width as i32
    }

    /// Returns the height of the image.
    pub fn height(&self) -> i32 {
        self.height as i32
    }

    /// Returns the size of the image.
    pub fn size(&self) -> (i32, i32) {
        (self.width as i32, self.height as i32)
    }

    /// Returns the format of the data contained within the image.
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    /// Returns the image's data, as a slice of raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Returns the image's data, as a mutable slice of raw bytes.
    ///
    /// This is not currently exposed publicly, as some more thought is needed
    /// into whether doing so would cause issues once different pixel formats
    /// are supported.
    pub(crate) fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Returns the image's underlying buffer.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Creates a new `ImageData` from a region.
    ///
    /// This will copy the data into a new buffer - as such, calling this function
    /// can be expensive!
    pub fn region(&self, region: Rectangle<i32>) -> ImageData {
        // TODO: Should this just crop the region if it goes out of bounds?
        assert!(
            region.x >= 0
                && region.y >= 0
                && region.x + region.width <= self.width as i32
                && region.y + region.height <= self.height as i32,
            "tried to read outside of image bounds"
        );

        let region_x = region.x as usize;
        let region_y = region.y as usize;
        let region_width = region.width as usize;
        let region_height = region.height as usize;

        let buffer_size = region_width * region_height * self.format.stride();

        let mut buffer = Vec::with_capacity(buffer_size);

        for scan_y in region_y..region_y + region_height {
            let x_start = (region_x + scan_y * self.width) * self.format.stride();
            let x_end = x_start + (region_width * self.format.stride());

            buffer.extend_from_slice(&self.data[x_start..x_end]);
        }

        ImageData {
            data: buffer,
            width: region_width,
            height: region_height,
            format: self.format,
        }
    }

    /// Creates a new [`Texture`] from the stored data.
    ///
    /// # Errors
    ///
    /// * [`TetraError::PlatformError`] will be returned if the underlying graphics API encounters an error.
    pub fn to_texture(&self, ctx: &mut Context) -> Result<Texture> {
        Texture::from_image_data(ctx, self)
    }

    /// Gets the color of the pixel at the specified location.
    ///
    /// If the image's [`TextureFormat`] does not contain one of the three color channels,
    /// the returned color will have that channel set to zero. Similarly, if the format
    /// does not have an alpha channel, the returned color will have an alpha value of
    /// one.
    ///
    /// # Panics
    ///
    /// Panics if the location is outside the bounds of the image.
    pub fn get_pixel_color(&self, position: Vec2<i32>) -> Color {
        let pixel_idx = position.x as usize + position.y as usize * self.width;
        let idx = pixel_idx * self.format.stride();

        assert!(
            idx + self.format.stride() - 1 < self.data.len(),
            "position was out of bounds"
        );

        match self.format {
            TextureFormat::Rgba8 => Color::rgba8(
                self.data[idx],
                self.data[idx + 1],
                self.data[idx + 2],
                self.data[idx + 3],
            ),
            TextureFormat::R8 => Color::rgba8(self.data[idx], 0, 0, 255),
            TextureFormat::Rg8 => Color::rgba8(self.data[idx], self.data[idx + 1], 0, 255),
            TextureFormat::Rgba16F => unimplemented!(), // TODO
        }
    }

    /// Sets the color of the pixel at the specified location.
    ///
    /// Any channels of the color that are not supported by the image's [`TextureFormat`]
    /// will be ignored. For example, if the image has [`TextureFormat::R8`], only the
    /// red channel of the color will be stored.
    ///
    /// # Panics
    ///
    /// Panics if the location is outside the bounds of the image.
    pub fn set_pixel_color(&mut self, position: Vec2<i32>, color: Color) {
        let pixel_idx = position.x as usize + position.y as usize * self.width;
        let idx = pixel_idx * self.format.stride();

        assert!(
            idx + self.format.stride() - 1 < self.data.len(),
            "position was out of bounds"
        );

        let data: [u8; 4] = color.into();

        match self.format {
            TextureFormat::Rgba8 => {
                self.data[idx..=idx + 3].copy_from_slice(&data);
            }
            TextureFormat::R8 => {
                self.data[idx] = data[0];
            }
            TextureFormat::Rg8 => {
                self.data[idx..=idx + 1].copy_from_slice(&data[..=1]);
            }
            TextureFormat::Rgba16F => unimplemented!(), // TODO
        }
    }

    /// Transforms the image data by applying a function to each pixel.
    ///
    /// If the image's [`TextureFormat`] does not contain one of the three color channels,
    /// the colors provided to the closure will have that channel set to zero. Similarly,
    /// if the format does not have an alpha channel, the returned color will have an
    /// alpha value of one. The unsupported channels will be ignored when writing
    /// back to the image buffer.
    pub fn transform<F>(&mut self, mut func: F)
    where
        F: FnMut(Vec2<i32>, Color) -> Color,
    {
        for (i, data) in self.data.chunks_exact_mut(self.format.stride()).enumerate() {
            let x = i % self.width;
            let y = i / self.width;

            // TODO: This won't be right for packed formats
            let color = Color::rgba8(
                data[0],
                data.get(1).copied().unwrap_or(0),
                data.get(2).copied().unwrap_or(0),
                data.get(3).copied().unwrap_or(255),
            );

            let output: [u8; 4] = func(Vec2::new(x as i32, y as i32), color).into();

            match self.format {
                TextureFormat::Rgba8 => {
                    data.copy_from_slice(&output);
                }
                TextureFormat::R8 => {
                    data[0] = output[0];
                }
                TextureFormat::Rg8 => {
                    data.copy_from_slice(&output[..=1]);
                }
                TextureFormat::Rgba16F => unimplemented!(), // TODO
            }
        }
    }

    /// Multiplies the RGB components of each pixel by the alpha component.
    ///
    /// This can be useful when working with
    /// [premultiplied alpha blending](super::BlendState::alpha).
    ///
    /// If the image's data format does not have an alpha component, this
    /// function will have no effect.
    pub fn premultiply(&mut self) {
        self.transform(|_, color| color.to_premultiplied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region_test(
        format: TextureFormat,
        input: Vec<u8>,
        expected_left: Vec<u8>,
        expected_right: Vec<u8>,
        expected_top: Vec<u8>,
        expected_bottom: Vec<u8>,
    ) {
        let image = ImageData::from_data(2, 2, format, input).unwrap();

        let left = image.region(Rectangle::new(0, 0, 1, 2));
        assert_eq!(left.as_bytes(), &expected_left);

        let right = image.region(Rectangle::new(1, 0, 1, 2));
        assert_eq!(right.as_bytes(), &expected_right);

        let top = image.region(Rectangle::new(0, 0, 2, 1));
        assert_eq!(top.as_bytes(), &expected_top);

        let bottom = image.region(Rectangle::new(0, 1, 2, 1));
        assert_eq!(bottom.as_bytes(), &expected_bottom);
    }

    #[test]
    fn region_rgba8() {
        region_test(
            TextureFormat::Rgba8,
            // Input
            vec![
                0x00, 0x01, 0x02, 0x03, // Pixel 1
                0x04, 0x05, 0x06, 0x07, // Pixel 2
                0x08, 0x09, 0x0A, 0x0B, // Pixel 3
                0x0C, 0x0D, 0x0E, 0x0F, // Pixel 4
            ],
            // Left
            vec![
                0x00, 0x01, 0x02, 0x03, // Pixel 1
                0x08, 0x09, 0x0A, 0x0B, // Pixel 3
            ],
            // Right
            vec![
                0x04, 0x05, 0x06, 0x07, // Pixel 2
                0x0C, 0x0D, 0x0E, 0x0F, // Pixel 4
            ],
            // Top
            vec![
                0x00, 0x01, 0x02, 0x03, // Pixel 1
                0x04, 0x05, 0x06, 0x07, // Pixel 2
            ],
            // Bottom
            vec![
                0x08, 0x09, 0x0A, 0x0B, // Pixel 3
                0x0C, 0x0D, 0x0E, 0x0F, // Pixel 4
            ],
        );
    }

    #[test]
    fn region_r8() {
        region_test(
            TextureFormat::R8,
            // Input
            vec![
                0x00, // Pixel 1
                0x04, // Pixel 2
                0x08, // Pixel 3
                0x0C, // Pixel 4
            ],
            // Left
            vec![
                0x00, // Pixel 1
                0x08, // Pixel 3
            ],
            // Right
            vec![
                0x04, // Pixel 2
                0x0C, // Pixel 4
            ],
            // Top
            vec![
                0x00, // Pixel 1
                0x04, // Pixel 2
            ],
            // Bottom
            vec![
                0x08, // Pixel 3
                0x0C, // Pixel 4
            ],
        );
    }

    #[test]
    fn region_rg8() {
        region_test(
            TextureFormat::Rg8,
            // Input
            vec![
                0x00, 0x01, // Pixel 1
                0x04, 0x05, // Pixel 2
                0x08, 0x09, // Pixel 3
                0x0C, 0x0D, // Pixel 4
            ],
            // Left
            vec![
                0x00, 0x01, // Pixel 1
                0x08, 0x09, // Pixel 3
            ],
            // Right
            vec![
                0x04, 0x05, // Pixel 2
                0x0C, 0x0D, // Pixel 4
            ],
            // Top
            vec![
                0x00, 0x01, // Pixel 1
                0x04, 0x05, // Pixel 2
            ],
            // Bottom
            vec![
                0x08, 0x09, // Pixel 3
                0x0C, 0x0D, // Pixel 4
            ],
        );
    }

    fn get_pixel_color_test(
        format: TextureFormat,
        input: Vec<u8>,
        tl: Color,
        tr: Color,
        bl: Color,
        br: Color,
    ) {
        let image = ImageData::from_data(2, 2, format, input).unwrap();

        assert_eq!(image.get_pixel_color(Vec2::new(0, 0)), tl);
        assert_eq!(image.get_pixel_color(Vec2::new(1, 0)), tr);
        assert_eq!(image.get_pixel_color(Vec2::new(0, 1)), bl);
        assert_eq!(image.get_pixel_color(Vec2::new(1, 1)), br);
    }

    #[test]
    fn get_pixel_color_rgba8() {
        get_pixel_color_test(
            TextureFormat::Rgba8,
            vec![
                0x00, 0x01, 0x02, 0x03, // Pixel 1
                0x04, 0x05, 0x06, 0x07, // Pixel 2
                0x08, 0x09, 0x0A, 0x0B, // Pixel 3
                0x0C, 0x0D, 0x0E, 0x0F, // Pixel 4
            ],
            Color::rgba8(0x00, 0x01, 0x02, 0x03),
            Color::rgba8(0x04, 0x05, 0x06, 0x07),
            Color::rgba8(0x08, 0x09, 0x0A, 0x0B),
            Color::rgba8(0x0C, 0x0D, 0x0E, 0x0F),
        );
    }

    #[test]
    fn get_pixel_color_r8() {
        get_pixel_color_test(
            TextureFormat::R8,
            vec![
                0x00, // Pixel 1
                0x04, // Pixel 2
                0x08, // Pixel 3
                0x0C, // Pixel 4
            ],
            Color::rgba8(0x00, 0x00, 0x00, 0xFF),
            Color::rgba8(0x04, 0x00, 0x00, 0xFF),
            Color::rgba8(0x08, 0x00, 0x00, 0xFF),
            Color::rgba8(0x0C, 0x00, 0x00, 0xFF),
        );
    }

    #[test]
    fn get_pixel_color_rg8() {
        get_pixel_color_test(
            TextureFormat::Rg8,
            vec![
                0x00, 0x01, // Pixel 1
                0x04, 0x05, // Pixel 2
                0x08, 0x09, // Pixel 3
                0x0C, 0x0D, // Pixel 4
            ],
            Color::rgba8(0x00, 0x01, 0x00, 0xFF),
            Color::rgba8(0x04, 0x05, 0x00, 0xFF),
            Color::rgba8(0x08, 0x09, 0x00, 0xFF),
            Color::rgba8(0x0C, 0x0D, 0x00, 0xFF),
        );
    }

    fn set_pixel_color_test(format: TextureFormat, input: Vec<u8>, output: Vec<u8>) {
        let mut image = ImageData::from_data(2, 2, format, input).unwrap();

        image.set_pixel_color(Vec2::new(0, 0), Color::rgba8(0x0F, 0x0E, 0x0D, 0x0C));
        image.set_pixel_color(Vec2::new(1, 0), Color::rgba8(0x0B, 0x0A, 0x09, 0x08));
        image.set_pixel_color(Vec2::new(0, 1), Color::rgba8(0x07, 0x06, 0x05, 0x04));
        image.set_pixel_color(Vec2::new(1, 1), Color::rgba8(0x03, 0x02, 0x01, 0x00));

        assert_eq!(image.as_bytes(), output);
    }

    #[test]
    fn set_pixel_color_rgba8() {
        set_pixel_color_test(
            TextureFormat::Rgba8,
            vec![
                0x00, 0x01, 0x02, 0x03, // Pixel 1
                0x04, 0x05, 0x06, 0x07, // Pixel 2
                0x08, 0x09, 0x0A, 0x0B, // Pixel 3
                0x0C, 0x0D, 0x0E, 0x0F, // Pixel 4
            ],
            vec![
                0x0F, 0x0E, 0x0D, 0x0C, // Pixel 1
                0x0B, 0x0A, 0x09, 0x08, // Pixel 2
                0x07, 0x06, 0x05, 0x04, // Pixel 3
                0x03, 0x02, 0x01, 0x00, // Pixel 4
            ],
        );
    }

    #[test]
    fn set_pixel_color_r8() {
        set_pixel_color_test(
            TextureFormat::R8,
            vec![
                0x00, // Pixel 1
                0x04, // Pixel 2
                0x08, // Pixel 3
                0x0C, // Pixel 4
            ],
            vec![
                0x0F, // Pixel 1
                0x0B, // Pixel 2
                0x07, // Pixel 3
                0x03, // Pixel 4
            ],
        );
    }

    #[test]
    fn set_pixel_color_rg8() {
        set_pixel_color_test(
            TextureFormat::Rg8,
            vec![
                0x00, 0x01, // Pixel 1
                0x04, 0x05, // Pixel 2
                0x08, 0x09, // Pixel 3
                0x0C, 0x0D, // Pixel 4
            ],
            vec![
                0x0F, 0x0E, // Pixel 1
                0x0B, 0x0A, // Pixel 2
                0x07, 0x06, // Pixel 3
                0x03, 0x02, // Pixel 4
            ],
        );
    }

    fn transform_test(format: TextureFormat, input: Vec<u8>, output: Vec<u8>) {
        let mut image = ImageData::from_data(2, 2, format, input).unwrap();

        image.transform(|_, c| c + Color::rgba8(1, 1, 1, 1));

        assert_eq!(image.as_bytes(), &output);
    }

    #[test]
    fn transform_rgba8() {
        transform_test(
            TextureFormat::Rgba8,
            vec![
                0x00, 0x01, 0x02, 0x03, // Pixel 1
                0x04, 0x05, 0x06, 0x07, // Pixel 2
                0x08, 0x09, 0x0A, 0x0B, // Pixel 3
                0x0C, 0x0D, 0x0E, 0x0F, // Pixel 4
            ],
            vec![
                0x01, 0x02, 0x03, 0x04, // Pixel 1
                0x05, 0x06, 0x07, 0x08, // Pixel 2
                0x09, 0x0A, 0x0B, 0x0C, // Pixel 3
                0x0D, 0x0E, 0x0F, 0x10, // Pixel 4
            ],
        );
    }

    #[test]
    fn transform_r8() {
        transform_test(
            TextureFormat::R8,
            vec![
                0x00, // Pixel 1
                0x04, // Pixel 2
                0x08, // Pixel 3
                0x0C, // Pixel 4
            ],
            vec![
                0x01, // Pixel 1
                0x05, // Pixel 2
                0x09, // Pixel 3
                0x0D, // Pixel 4
            ],
        );
    }

    #[test]
    fn transform_rg8() {
        transform_test(
            TextureFormat::Rg8,
            vec![
                0x00, 0x01, // Pixel 1
                0x04, 0x05, // Pixel 2
                0x08, 0x09, // Pixel 3
                0x0C, 0x0D, // Pixel 4
            ],
            vec![
                0x01, 0x02, // Pixel 1
                0x05, 0x06, // Pixel 2
                0x09, 0x0A, // Pixel 3
                0x0D, 0x0E, // Pixel 4
            ],
        );
    }

    fn premultiply_test(format: TextureFormat, input: Vec<u8>, output: Vec<u8>) {
        let mut image = ImageData::from_data(2, 2, format, input).unwrap();

        image.premultiply();

        assert_eq!(image.as_bytes(), output);
    }

    #[test]
    fn premultiply_rgba8() {
        premultiply_test(
            TextureFormat::Rgba8,
            vec![
                0x00, 0x66, 0xCC, 0x00, // Pixel 1
                0x00, 0x66, 0xCC, 0x66, // Pixel 2
                0x00, 0x66, 0xCC, 0xCC, // Pixel 3
                0x00, 0x66, 0xCC, 0xFF, // Pixel 4
            ],
            vec![
                0x00, 0x00, 0x00, 0x00, // Pixel 1
                0x00, 0x28, 0x51, 0x66, // Pixel 2
                0x00, 0x51, 0xA3, 0xCC, // Pixel 3
                0x00, 0x66, 0xCC, 0xFF, // Pixel 4
            ],
        );
    }

    #[test]
    fn premultiply_r8() {
        premultiply_test(
            TextureFormat::R8,
            vec![
                0x01, // Pixel 1
                0x02, // Pixel 2
                0x03, // Pixel 3
                0x04, // Pixel 4
            ],
            vec![
                0x01, // Pixel 1
                0x02, // Pixel 2
                0x03, // Pixel 3
                0x04, // Pixel 4
            ],
        );
    }

    #[test]
    fn premultiply_rg8() {
        premultiply_test(
            TextureFormat::Rg8,
            vec![
                0x01, 0x01, // Pixel 1
                0x02, 0x02, // Pixel 2
                0x03, 0x03, // Pixel 3
                0x04, 0x04, // Pixel 4
            ],
            vec![
                0x01, 0x01, // Pixel 1
                0x02, 0x02, // Pixel 2
                0x03, 0x03, // Pixel 3
                0x04, 0x04, // Pixel 4
            ],
        );
    }
}
