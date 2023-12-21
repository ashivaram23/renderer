use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::Path,
};

use crate::structures::Film;
use clap::{Arg, Command};
use png::Encoder;

pub fn save_to_png(film: Film, filename: &str) -> Result<(), Box<dyn Error>> {
    let rgb_values: Vec<u8> = film
        .pixel_data
        .into_iter()
        .flat_map(|rgb| {
            [
                (rgb.x * 255.0) as u8,
                (rgb.y * 255.0) as u8,
                (rgb.z * 255.0) as u8,
            ]
        })
        .collect();

    let mut encoder = Encoder::new(
        BufWriter::new(File::create(Path::new(filename))?),
        film.screen_width,
        film.screen_height,
    );

    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgb_values)?;

    Ok(())
}

pub fn read_input(filename: &str) -> Result<(u32, u32), Box<dyn Error>> {
    let mut fields = [640, 480];
    let reader = BufReader::new(File::open(Path::new(filename))?);
    for (line, field) in reader.lines().zip(fields.iter_mut()) {
        *field = line?.parse::<u32>()?;
    }

    Ok((fields[0], fields[1]))
}

pub fn read_args() -> Option<(String, String)> {
    let matches = Command::new("renderer")
        .arg(Arg::new("input").required(true))
        .arg(Arg::new("output").required(true))
        .arg_required_else_help(true)
        .get_matches();

    let input = matches.get_one::<String>("input")?;
    let output = matches.get_one::<String>("output")?;
    Some((input.clone(), output.clone()))
}
