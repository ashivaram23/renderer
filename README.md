# Path tracer

This program renders 3D scenes with global illumination using Monte Carlo path tracing.

Best image here _____

## Build

[First, install Rust and Cargo if you don't already have them.](https://www.rust-lang.org/learn/get-started)

Build this project with `cargo build --release`. The standalone executable at target/release/renderer is all you need, so you can move it to the main folder and run `cargo clean` to save space.

## Run

To render a scene described at scene.json into a file named output.png, run `./renderer scene.json output.png`.

## Scene format

Describe a scene with a JSON file in the following format. Positions are based on a left-handed coordinate system where +z points forward, +y points up, and +x points to the right.

```json
{
    "camera": {
        "film_dimensions": [1920, 1080], 
        "origin": [0, 0.7, -3],
        "look_at": [0, 0, 2.701],
        "up": [0, 1, 0],
        "field_of_view": 45
    },

    "settings": {
        "environment": [0.4, 0.6, 0.9],
        "samples_per_pixel": 16,
        "max_ray_depth": 8
    },

    "objects": {
        "floor": {
            "type": "mesh",
            "file": "scenes/objects/floor.obj",
            "material": {
                "type": "diffuse",
                "reflectance": [0.05, 0.39, 0.8]
            }
        },
        "dragon": {
            "type": "mesh",
            "file": "scenes/objects/dragon.obj",
            "material": {
                "type": "diffuse",
                "reflectance": [0.8, 0.75, 0.1]
            }
        }
    }
}
```

This example renders a 1920x1080px image from a camera positioned at (0, 0.7, -3) and facing towards (0, 0, 2.701) with a field of view of 25 degrees. The sky color is light blue (RGB 0.6, 0.8, 1.0) and the renderer calculates 64 samples per pixel. There are two objects: a blue floor plane mesh from floor.obj and a yellow dragon mesh from dragon.obj.

Each OBJ file should include one object made of triangles only, and will be read with the +z axis pointing forward and the +y axis pointing up. (OBJ files use right-handed coordinates, which will automatically be converted by the renderer.)

See the scenes folder for more examples.

## Example renders

Text including pictures and render times, comparisons, etc

## Resources

Text
