use speedy2d::{
    color::Color,
    dimen::Vector2,
    shape::Rect,
    window::{WindowHandler, WindowHelper},
    Graphics2D, Window,
};
use std::{env, fs::File};

mod wfc;

#[derive(Clone)]
struct ImageData {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
}

const PIXEL_SIZE: f32 = 8.0;

fn wrap_value(v: isize, max: usize) -> usize {
    if v < 0 {
        max - ((-v) as usize % max)
    } else {
        v as usize % max
    }
}

impl ImageData {
    fn new(w: usize, h: usize) -> Self {
        Self {
            pixels: vec![0; w * h],
            width: w,
            height: h,
        }
    }

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
        let wrapped_x = wrap_value(x, self.width);
        let wrapped_y = wrap_value(y, self.height);
        self.pixels[wrapped_x + wrapped_y * self.width]
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
    output_image: ImageData,
    parameters: wfc::WFCParameters,
    lowest_entropy_tiles: Vec<usize>,
    superpositions: Vec<Vec<usize>>,
    not_collapsed: Vec<usize>,
}

impl WinHandler {
    fn new(input_img: &ImageData, wfc_parameters: &wfc::WFCParameters) -> Self {
        let w = 64;
        let h = 64;

        let superpos = {
            let id_list: Vec<usize> = (0..wfc_parameters.wfc_tiles.len()).collect();
            vec![id_list; w * h]
        };

        Self {
            input_image: input_img.clone(),
            output_image: ImageData::new(w, h),
            parameters: wfc_parameters.clone(),
            lowest_entropy_tiles: vec![],
            superpositions: superpos.clone(),
            not_collapsed: (0..superpos.len()).collect(),
        }
    }
}

//Main drawing loop
impl WindowHandler for WinHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::from_rgb(1.0, 1.0, 1.0));
        self.input_image
            .display_image(graphics, PIXEL_SIZE, 0.0, 0.0);

        let mut rng = rand::thread_rng();

        self.lowest_entropy_tiles = wfc::lowest_entropy(
            &self.superpositions,
            &self.not_collapsed,
            self.parameters.wfc_tiles.len(),
        );
        //Repeat until we have collapsed each tile into a single state
        if !self.lowest_entropy_tiles.is_empty() {
            //Find the tile with the lowest "entropy"
            let rand_tile_index =
                wfc::random_element(&self.lowest_entropy_tiles, &mut rng, None).unwrap_or(0);
            //Collapse that tile into a random state that is allowed
            let weights: Vec<u32> = self.superpositions[rand_tile_index].iter()
                .map(|tile| self.parameters.wfc_frequency[*tile])
                .collect();
            self.superpositions[rand_tile_index] =
                vec![
                    wfc::random_element(&self.superpositions[rand_tile_index], &mut rng, Some(&weights))
                        .unwrap_or(0),
                ];
            //Update surrounding tiles to only have valid tiles in the superposition
            let x = (rand_tile_index % self.output_image.width) as isize;
            let y = (rand_tile_index / self.output_image.width) as isize;
            //Propagate
            let failed = wfc::propagate(
                &mut self.superpositions,
                &self.parameters.wfc_rules,
                x,
                y,
                self.output_image.width,
                self.output_image.height,
            );

            if failed {
                eprintln!("FAILED - RESTARTING WFC");
                let w = self.output_image.width;
                let h = self.output_image.height;
                self.output_image = ImageData::new(w, h);
                self.lowest_entropy_tiles.clear();
                self.superpositions = {
                    let id_list: Vec<usize> = (0..self.parameters.wfc_tiles.len()).collect();
                    vec![id_list; w * h]
                };

                self.not_collapsed = (0..self.superpositions.len()).collect();
                helper.request_redraw();
                return;
            }

            self.not_collapsed
                .retain(|index| self.superpositions[*index].len() > 1);
            self.lowest_entropy_tiles = wfc::lowest_entropy(
                &self.superpositions,
                &self.not_collapsed,
                self.parameters.wfc_tiles.len(),
            );
        }

        wfc::copy_superpositions_to_grid(
            &mut self.output_image.pixels,
            &self.superpositions,
            &self.parameters.wfc_tiles,
        );

        self.output_image.display_image(
            graphics,
            PIXEL_SIZE,
            self.input_image.width as f32 * PIXEL_SIZE + PIXEL_SIZE,
            0.0,
        );

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

            /*let start = ::std::time::Instant::now();
            let _generated = wfc_parameters.generate_grid(64, 64);
            let seconds = start.elapsed().as_secs_f64();
            eprintln!("Took {} sec to generate image", seconds);*/

            let window = Window::new_centered("wave function collapse demo", (800, 640)).unwrap();
            window.run_loop(WinHandler::new(&data, &wfc_parameters));
        }
        Err(msg) => {
            eprintln!("failed to open file: {}", args[1]);
            eprintln!("{msg}");
        }
    }
}
