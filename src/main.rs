use speedy2d::{
    color::Color,
    dimen::Vector2,
    shape::Rect,
    window::{WindowHandler, WindowHelper},
    Graphics2D, Window,
};
use std::env;
use crate::image_data::ImageData;

mod wfc;
mod image_data;

const PIXEL_SIZE: f32 = 8.0;
const SPEED: u32 = 6;

//Simple function to display the image onto the window
//x and y are the top left corner of the image
fn display_image(image: &ImageData, graphics: &mut Graphics2D, pixel_size: f32, x: f32, y: f32) {
    image.pixels()
        .iter()
        .enumerate()
        .map(|pixel| {
            let col = image_data::u32_to_color(image.get_pixel(pixel.0 % image.height(), pixel.0 / image.height()));
            (
                Rect::new(
                    Vector2::new(
                        (pixel.0 % image.height()) as f32,
                        (pixel.0 / image.height()) as f32,
                    ) * pixel_size
                        + Vector2::new(x, y),
                    Vector2::new(
                        (pixel.0 % image.height() + 1) as f32,
                        (pixel.0 / image.height() + 1) as f32,
                    ) * pixel_size
                        + Vector2::new(x, y),
                ),
                Color::from_rgb(col.0, col.1, col.2),
            )
        })
        .for_each(|pixel| graphics.draw_rectangle(pixel.0, pixel.1));
}

struct WinHandler {
    input_image: ImageData,
    output_image: ImageData,
    parameters: wfc::WFCParameters,
    lowest_entropy_tiles: Vec<usize>,
    superpositions: Vec<Vec<usize>>,
    not_collapsed: Vec<usize>,
    current_frame: u32
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
            current_frame: 0
        }
    }
}

//Main drawing loop
impl WindowHandler for WinHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::from_rgb(1.0, 1.0, 1.0));
        display_image(&self.input_image, graphics, PIXEL_SIZE, PIXEL_SIZE, PIXEL_SIZE);

        let mut rng = rand::thread_rng();

        self.lowest_entropy_tiles = wfc::lowest_entropy(
            &self.superpositions,
            &self.not_collapsed,
            &self.parameters.wfc_frequency
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
            let x = (rand_tile_index % self.output_image.width()) as isize;
            let y = (rand_tile_index / self.output_image.width()) as isize;
            //Propagate
            let failed = wfc::propagate(
                &mut self.superpositions,
                &self.parameters.wfc_rules,
                x,
                y,
                self.output_image.width(),
                self.output_image.height(),
            );

            if failed {
                eprintln!("FAILED - RESTARTING WFC");
                let w = self.output_image.width();
                let h = self.output_image.height();
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
                &self.parameters.wfc_frequency
            );
        }

        if self.current_frame % SPEED == 0 {
            wfc::copy_superpositions_to_grid(
                self.output_image.pixels_mut(),
                &self.superpositions,
                &self.parameters.wfc_tiles,
            );
        }

        display_image(
            &self.output_image,
            graphics,
            PIXEL_SIZE,
            self.input_image.width() as f32 * PIXEL_SIZE + PIXEL_SIZE + PIXEL_SIZE,
            PIXEL_SIZE,
        );

        helper.request_redraw();
        self.current_frame += 1;
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
