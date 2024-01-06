use std::fs::File;

#[derive(Clone)]
pub struct ImageData {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
}

pub fn wrap_value(v: isize, max: usize) -> usize {
    if v < 0 {
        max - ((-v) as usize % max)
    } else {
        v as usize % max
    }
}

impl ImageData {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            pixels: vec![0; w * h],
            width: w,
            height: h,
        }
    }

    pub fn from_pixels(grid: &[u32], w: usize, h: usize) -> Self {
        ImageData {
            pixels: Vec::from(grid),
            width: w,
            height: h,
        }
    }

    //Load the image data from a png
    pub fn load_png(path: &str) -> Result<Self, String> {
        let decoder = png::Decoder::new(File::open(path).map_err(|e| e.to_string())?);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;

        let mut pix_data = vec![0; buf.len() / 4];

        for i in 0..(buf.len() / 4) {
            if i * 4 + 3 >= buf.len() {
                break;
            }
            pix_data[i] = (buf[4 * i] as u32)
                | (buf[4 * i + 1] as u32) << 8
                | (buf[4 * i + 2] as u32) << 16
                | (buf[4 * i + 3] as u32) << 24;
        }

        Ok(Self {
            pixels: pix_data,
            width: info.width as usize,
            height: info.height as usize,
        })
    }

    //Get pixel data, if it is out of bounds return 0
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }

        self.pixels[y * self.width + x]
    }

    //Same as get_pixel but wraps around the edges
    pub fn get_pixel_wrap(&self, x: isize, y: isize) -> u32 {
        let wrapped_x = wrap_value(x, self.width);
        let wrapped_y = wrap_value(y, self.height);
        self.pixels[wrapped_x + wrapped_y * self.width]
    } 

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn pixels_mut(&mut self) -> &mut [u32] {
        &mut self.pixels
    }
}

//Converts a u32 into a color struct (r, g, b)
pub fn u32_to_color(pixel: u32) -> (f32, f32, f32) {
    let b = ((pixel >> 16) & 0xff) as f32;
    let g = ((pixel >> 8) & 0xff) as f32;
    let r = (pixel & 0xff) as f32;
    (r / 255.0, g / 255.0, b / 255.0)
}
