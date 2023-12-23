use speedy2d::{
    color::Color,
    dimen::Vector2,
    shape::Rect,
    window::{WindowHandler, WindowHelper},
    Graphics2D, Window,
};
use std::{env, fs::File};

mod wfc;

struct ImageData {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
}

const PIXEL_SIZE: f32 = 8.0;

fn wrap_value(v: isize, max: usize) -> usize {
    (v % max as isize + max as isize) as usize % max
}

impl ImageData {
    //Load the image data from a png
    fn load_png(path: &str) -> Result<Self, String> {
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
    fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }

        self.pixels[y * self.width + x]
    } 

    //Same as get_pixel but wraps around the edges
    fn get_pixel_wrap(&self, x: isize, y: isize) -> u32 {
        self.pixels[wrap_value(x, self.width) + wrap_value(y, self.height) * self.width]
    }

    //Simple function to display the image onto the window
    //x and y are the top left corner of the image
    fn display_image(&self, graphics: &mut Graphics2D, pixel_size: f32, x: f32, y: f32) {
        self.pixels
            .iter()
            .enumerate()
            .map(|pixel| {
                (
                    Rect::new(
                        Vector2::new(
                            (pixel.0 % self.height) as f32,
                            (pixel.0 / self.height) as f32,
                        ) * pixel_size
                            + Vector2::new(x, y),
                        Vector2::new(
                            (pixel.0 % self.height + 1) as f32,
                            (pixel.0 / self.height + 1) as f32,
                        ) * pixel_size
                            + Vector2::new(x, y),
                    ),
                    u32_to_color(self.get_pixel(pixel.0 % self.height, pixel.0 / self.height)),
                )
            })
            .for_each(|pixel| graphics.draw_rectangle(pixel.0, pixel.1));
    }
}

//Converts a u32 into a color struct (r, g, b)
fn u32_to_color(pixel: u32) -> Color {
    let b = ((pixel >> 16) & 0xff) as f32;
    let g = ((pixel >> 8) & 0xff) as f32;
    let r = (pixel & 0xff) as f32;
    Color::from_rgb(r / 255.0, g / 255.0, b / 255.0)
}

struct WinHandler {
    input_image: ImageData,
    parameters: wfc::WFCParameters,
}

//Main drawing loop
impl WindowHandler for WinHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::from_rgb(1.0, 1.0, 1.0));
        self.input_image
            .display_image(graphics, PIXEL_SIZE, 0.0, 0.0);

        let mut ix = self.input_image.width as f32 * PIXEL_SIZE;
        let mut iy = 0.0f32;
        for tile in &self.parameters.wfc_tiles {
            let tile_img = ImageData {
                pixels: tile.clone(),
                width: self.parameters.wfc_tile_sz,
                height: self.parameters.wfc_tile_sz,
            };
            tile_img.display_image(graphics, PIXEL_SIZE, ix, iy);
            ix += self.parameters.wfc_tile_sz as f32 * PIXEL_SIZE + PIXEL_SIZE;

            if ix > 768.0 {
                ix = self.input_image.width as f32 * PIXEL_SIZE;
                iy += self.parameters.wfc_tile_sz as f32 * PIXEL_SIZE + PIXEL_SIZE;
            }
        }

        helper.request_redraw();
    }
}

fn main() {
    //Get command line arguments
    let args: Vec<String> = env::args().collect();

    //If we have no arguments, exit program
    if args.len() == 1 {
        eprintln!("usage: {} [input file]", args[0]);
        std::process::exit(1);
    }

    //Otherwise, attempt to open the png file that was provided as an argument
    let img_data = ImageData::load_png(&args[1]);

    match img_data {
        Ok(data) => {
            let wfc_parameters = wfc::WFCParameters::from_image_data(&data, 3);
            let window = Window::new_centered("wave function collapse demo", (800, 600)).unwrap();
            window.run_loop(WinHandler { 
                input_image: data,
                parameters: wfc_parameters
            });
        }
        Err(msg) => {
            eprintln!("failed to open file: {}", args[1]);
            eprintln!("{msg}");
        }
    }
}
