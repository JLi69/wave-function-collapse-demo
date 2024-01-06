use crate::image_data::ImageData;
use sdl2::{
    event::Event,
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    video::{Window, WindowContext},
    EventPump,
};
use std::{env, path::Path};

mod image_data;
mod wfc;

const PIXEL_SIZE: f32 = 8.0;

//Process events
struct ProcessedEvents {
    can_quit: bool,
}

fn process_events(event_pump: &mut EventPump) -> ProcessedEvents {
    let mut processed = ProcessedEvents { can_quit: false };

    for event in event_pump.poll_iter() {
        if let Event::Quit { .. } = event {
            processed.can_quit = true;
        }
    }

    processed
}

fn texture_from_image<'a>(
    data: &ImageData,
    texture_creator: &'a TextureCreator<WindowContext>,
) -> Result<Texture<'a>, String> {
    //Create the texture
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::BGRA8888,
            data.width() as u32,
            data.height() as u32,
        )
        .map_err(|e| e.to_string())?;

    texture
        .with_lock(None, |pixels: &mut [u8], _pitch: usize| {
            for y in 0..data.height() {
                for x in 0..data.width() {
                    let pixel = data.get_pixel(x, y);
                    let col = image_data::u32_to_color(pixel);
                    pixels[y * data.width() * 4 + x * 4 + 1] = (col.0 * 255.0) as u8;
                    pixels[y * data.width() * 4 + x * 4 + 2] = (col.1 * 255.0) as u8;
                    pixels[y * data.width() * 4 + x * 4 + 3] = (col.2 * 255.0) as u8;
                    pixels[y * data.width() * 4 + x * 4] = 0xff;
                }
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(texture)
}

fn display_loop(
    canvas: &mut Canvas<Window>,
    input_texture: &Texture,
    output_texture: &Texture,
) -> Result<(), String> {
    canvas.clear();

    canvas.copy(
        input_texture,
        None,
        Rect::new(
            PIXEL_SIZE as i32,
            PIXEL_SIZE as i32,
            PIXEL_SIZE as u32 * input_texture.query().width,
            PIXEL_SIZE as u32 * input_texture.query().height,
        ),
    )?;

    canvas.copy(
        output_texture,
        None,
        Rect::new(
            PIXEL_SIZE as i32 * 2 + input_texture.query().width as i32 * PIXEL_SIZE as i32,
            PIXEL_SIZE as i32,
            PIXEL_SIZE as u32 * output_texture.query().width,
            PIXEL_SIZE as u32 * output_texture.query().height,
        ),
    )?;

    canvas.present();

    Ok(())
}

fn main_loop(data: &ImageData, wfc_parameters: &wfc::WFCParameters) -> Result<(), String> {
    //Init sdl
    let ctx = sdl2::init()?;
    let video_subsystem = ctx.video()?;
    let window = video_subsystem
        .window("wave function collapse demo", 800, 640)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let mut event_pump = ctx.event_pump()?;

    let mut events = ProcessedEvents { can_quit: false };

    let input_texture = texture_from_image(data, &texture_creator)?;
    let w = 64;
    let h = 64;
    let mut output_image = ImageData::new(w, h);
    let mut output_texture = texture_from_image(&output_image, &texture_creator)?;

    let mut rng = rand::thread_rng();
    let mut wfc_state = wfc::WFCState::new(
        w,
        h,
        &wfc_parameters.wfc_tiles,
        &wfc_parameters.wfc_frequency,
    );
    while !events.can_quit {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        display_loop(&mut canvas, &input_texture, &output_texture)?;

        if !wfc_state.done() {
            match wfc_parameters.step(w, h, &mut wfc_state, &mut rng) {
                Ok(()) => {}
                Err(msg) => {
                    eprintln!("{msg}");
                    //Reset the state
                    wfc_state = wfc::WFCState::new(
                        w,
                        h,
                        &wfc_parameters.wfc_tiles,
                        &wfc_parameters.wfc_frequency,
                    );
                }
            }

            wfc::copy_superpositions_to_grid(
                output_image.pixels_mut(),
                wfc_state.superpositions(),
                &wfc_parameters.wfc_tiles,
            );

            output_texture = texture_from_image(&output_image, &texture_creator)?;
        }

        events = process_events(&mut event_pump);
    }

    Ok(())
}

fn parse_args(args: Vec<String>) -> (String, isize) {
    let mut path = "".to_string();
    let mut n = 3;

    for arg in &args {
        let tmp_n: isize = arg.parse().unwrap_or(-1);

        if tmp_n > 4 {
            continue;
        }

        if tmp_n > 0 {
            n = tmp_n;
        } else {
            path = (*arg).clone();
        }
    }

    let file_path = Path::new(&path);
    if path.is_empty() {
        eprintln!("No input file specified!");
        std::process::exit(1);
    }
    if !file_path.is_file() {
        eprintln!("{path} does not exist!");
        std::process::exit(1);
    }

    (path, n)
}

fn main() -> Result<(), String> {
    //Get command line arguments
    let args: Vec<String> = env::args().collect();

    //If we have no arguments, exit program
    if args.len() == 1 {
        eprintln!("usage: {} [input file] [n]", args[0]);
        std::process::exit(1);
    }

    //Otherwise, attempt to open the png file that was provided as an argument
    let parsed_args = parse_args(args);
    let img_data = ImageData::load_png(&parsed_args.0);

    match img_data {
        Ok(data) => {
            let wfc_parameters = wfc::WFCParameters::from_image_data(&data, parsed_args.1);

            /*let start = ::std::time::Instant::now();
            let _generated = wfc_parameters.generate_grid(64, 64).unwrap();
            let seconds = start.elapsed().as_secs_f64();
            eprintln!("Took {} sec to generate image", seconds);*/

            main_loop(&data, &wfc_parameters)?;
        }
        Err(msg) => {
            eprintln!("failed to open file: {}", parsed_args.0);
            eprintln!("{msg}");
            return Err(msg);
        }
    }

    Ok(())
}
