use std::{error::Error, fs::File, io::BufWriter, path::Path};

use clap::{Arg, Command};
use png::Encoder;

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

struct Image {
    width: u32,
    height: u32,
    data: Vec<Color>,
}

impl Image {
    fn new(width: u32, height: u32) -> Self {
        Image {
            width,
            height,
            data: vec![Color { r: 0, g: 0, b: 0 }; (width * height).try_into().unwrap()],
        }
    }
}

fn read_args() -> (String, String, u32, u32) {
    let matches = Command::new("renderer")
        .arg(Arg::new("input").required(true))
        .arg(Arg::new("output").long("output").short('o').required(true))
        .arg(
            Arg::new("width")
                .long("width")
                .short('w')
                .default_value("640")
                .value_parser(clap::value_parser!(u32).range(8..5000)),
        )
        .arg(
            Arg::new("height")
                .long("height")
                .short('h')
                .default_value("480")
                .value_parser(clap::value_parser!(u32).range(8..5000)),
        )
        .arg_required_else_help(true)
        .disable_help_flag(true)
        .get_matches();

    const ARGS_ERROR: &str = "Error reading arguments";
    let input = matches.get_one::<String>("input").expect(ARGS_ERROR);
    let output = matches.get_one::<String>("output").expect(ARGS_ERROR);
    let width = matches.get_one::<u32>("width").expect(ARGS_ERROR);
    let height = matches.get_one::<u32>("height").expect(ARGS_ERROR);

    (input.clone(), output.clone(), *width, *height)
}

fn save_to_png(image: Image, filename: &str) -> Result<(), Box<dyn Error>> {
    let rgb_values: Vec<u8> = image
        .data
        .into_iter()
        .flat_map(|c| [c.r, c.g, c.b])
        .collect();

    let mut encoder = Encoder::new(
        BufWriter::new(File::create(Path::new(filename))?),
        image.width,
        image.height,
    );

    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgb_values)?;

    Ok(())
}

fn main() {
    let (_input, output, width, height) = read_args();

    let image = Image::new(width, height);
    save_to_png(image, &output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
