use speedy2d::{
    color::Color,
    dimen::Vector2,
    shape::Rect,
    window::{WindowHandler, WindowHelper},
    Graphics2D, Window,
};
use std::{fs::File, env};

struct ImageData {
    pixels: Vec<u32>,
    width: usize,
    height: usize
}

const PIXEL_SIZE: f32 = 8.0;

impl ImageData {
    fn load_png(path: &str) -> Result<Self, String> {
        let decoder = png::Decoder::new(File::open(path).map_err(|e| e.to_string())?);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    
        let mut pix_data = vec![0; buf.len() / 4 ];
    
        for i in 0..(buf.len() / 4) {
            if i * 4 + 3 >= buf.len() {
                break;
            }
            pix_data[i] =
                (buf[4 * i] as u32) |
                (buf[4 * i + 1] as u32) << 8 |
                (buf[4 * i + 2] as u32) << 16 |
                (buf[4 * i + 3] as u32) << 24;
        }
    
        Ok(
            ImageData { 
                pixels: pix_data, 
                width: info.width as usize, 
                height: info.height as usize
            }
        )
    }

    fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        
        self.pixels[y * self.width + x]
    }
}

fn u32_to_color(pixel: u32) -> Color {
    let b = ((pixel >> 16) & 0xff) as f32;
    let g = ((pixel >> 8) & 0xff) as f32;
    let r = (pixel & 0xff) as f32;
    Color::from_rgb(r / 255.0, g / 255.0, b / 255.0)
}

struct WinHandler {
    input_image: ImageData
}

impl WindowHandler for WinHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::from_rgb(1.0, 1.0, 1.0));

        for y in 0..self.input_image.height {
            for x in 0..self.input_image.width {
                let col = u32_to_color(self.input_image.get_pixel(x, y));
                graphics.draw_rectangle(
                    Rect::new(
                        Vector2::new(x as f32, y as f32) * PIXEL_SIZE,
                        Vector2::new(x as f32 + 1.0, y as f32 + 1.0) * PIXEL_SIZE
                    ),
                    col
                );
            }
        }

        helper.request_redraw();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        eprintln!("usage: {} [input file]", args[0]);
        std::process::exit(1);
    }

    let img_data = ImageData::load_png(&args[1]);
    match img_data {
        Ok(data) => {
            let window = Window::new_centered(
                "wave function collapse demo", 
                (800, 600)
            ).unwrap();
            window.run_loop(WinHandler {
                input_image: data
            });
        }
        Err(msg) => {
            eprintln!("failed to open file: {}", args[1]);
            eprintln!("{msg}");
        }
    }
}
