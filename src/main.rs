use std::thread;
use std::time;

use config::Config;


fn draw_grid(grid: &Vec<Vec<String>>) {
    let mut out_str = String::new();
    for row in grid {
        for cell in row {
            // Print with space padding (for alignment)
            // Color the wall corner in red
            if cell == "O" {
                out_str.push_str(&format!("\x1b[31m{}\x1b[0m ", cell));
            } else {
                out_str.push_str(&format!("\x1b[33m{}\x1b[0m ", cell));
            }
            //out_str += outcell.as_str();
        }
        out_str += "\n";
    }
    println!("{}", out_str);
}

fn change_cell(grid: &mut [Vec<String>], x: usize, y: usize, new_value: String) {
    // Check if cell is in bounds
    if x < grid.len() && y < grid[0].len() {
        grid[x][y] = new_value;
    }
}

fn draw_line((x1, y1): (usize, usize), (x2, y2): (usize, usize), grid: &mut [Vec<String>]) {
    let mut x: usize = x1;
    let mut y: usize = y1;
    let dx: i32 = (x2 as i32 - x1 as i32).abs();
    let dy: i32 = (y2 as i32 - y1 as i32).abs();
    let sx: i32 = if x1 < x2 { 1 } else { -1 };
    let sy: i32 = if y1 < y2 { 1 } else { -1 };
    let mut err = dx - dy;

    loop {
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x = (x as i32 + sx) as usize;
        }
        if e2 < dx {
            err += dx;
            y = (y as i32 + sy) as usize;
        }

        change_cell(grid, x, y, "X".to_string());
    }
}

fn main() {
    // Check if the yaml settings file exists
    const CONFIGPATH: &str = "Settings.toml";
    if !std::path::Path::new(CONFIGPATH).exists() {
        println!("\nWARNING! No settings file found. Creating one now... \n");
        let content = r#"
# This is the config file. If you want to change values, remember to keep the .0 at the end of the number.
WIDTH = 25
HEIGHT = 25
BOX_WIDTH = 10.0
BOX_HEIGHT = 10.0
BOX_DEPTH = 5.0
ROTATE_SPEED = 3.0
FOCAL_LENGTH = 64.0

# Experimental options
BETA_SCREEN = true
FPS = 30
"#;
        match std::fs::write(CONFIGPATH, content) {
            Ok(_) => (),
            Err(e) => panic!("Error creating {} file: {}", CONFIGPATH, e),
        };
    }

    // Load toml config file
    let config: Config = config::Config::builder()
        .add_source(config::File::with_name(CONFIGPATH))
        .build()
        .unwrap();

    // Set variable consts from config file
    let width: usize = config.get_int("WIDTH").unwrap() as usize;
    let height: usize = config.get_int("HEIGHT").unwrap() as usize;
    let box_width: f32 = config.get_float("BOX_WIDTH").unwrap() as f32;
    let box_height: f32 = config.get_float("BOX_HEIGHT").unwrap() as f32;
    let box_depth: f32 = config.get_float("BOX_DEPTH").unwrap() as f32;
    let rotate_speed: f32 = config.get_float("ROTATE_SPEED").unwrap() as f32;
    let focal_length: f32 = config.get_float("FOCAL_LENGTH").unwrap() as f32;

    let beta_screen: bool = config.get_bool("BETA_SCREEN").unwrap();
    let fps: u64 = config.get_int("FPS").unwrap() as u64;

    // Calculate rotation speed as fps non dependent
    let rotate_speed = rotate_speed / fps as f32;

    // Create a 3D grid of strings
    let mut grid = vec![vec![" ".to_string(); width]; height];

    // Define the 3D points of the box
    let mut verticies = vec![vec![0; 3]; 8];
    verticies[0] = vec![-1, -1, -1];
    verticies[1] = vec![1, -1, -1];
    verticies[2] = vec![-1, 1, -1];
    verticies[3] = vec![1, 1, -1];
    verticies[4] = vec![-1, -1, 1];
    verticies[5] = vec![1, -1, 1];
    verticies[6] = vec![-1, 1, 1];
    verticies[7] = vec![1, 1, 1];

    // Define the 3D edges of the box
    let mut edges = vec![vec![0; 2]; 12];
    edges[0] = vec![0, 1];
    edges[1] = vec![0, 2];
    edges[2] = vec![0, 4];
    edges[3] = vec![1, 3];
    edges[4] = vec![1, 5];
    edges[5] = vec![2, 3];
    edges[6] = vec![2, 6];
    edges[7] = vec![3, 7];
    edges[8] = vec![4, 5];
    edges[9] = vec![4, 6];
    edges[10] = vec![5, 7];
    edges[11] = vec![6, 7];

    let mut angle: f32 = 0.0;

    loop {
        // Clear grid
        for row in grid.iter_mut() {
            for cell in row.iter_mut() {
                *cell = " ".to_string();
            }
        }

        // Start fancy math
        let mut projected_verticies: Vec<Vec<f32>> = vec![vec![0.0; 2]; verticies.len()];

        for (i, vertex) in verticies.iter().enumerate() {
            let x = vertex[0] as f32 * box_width;
            let y = vertex[1] as f32 * box_height;
            let z = vertex[2] as f32 * box_depth;

            // Rotate around Y axis
            let new_x = x * angle.cos() - z * angle.sin();
            let new_z = x * angle.sin() + z * angle.cos();

            // Rotate around X axis
            let new_y = y * angle.cos() - new_z * angle.sin();
            let new_z = y * angle.sin() + new_z * angle.cos();

            // Project 3D to 2D
            let x_projected = new_x * focal_length / (new_z + focal_length) + (width as f32 / 2.0);
            let y_projected = new_y * focal_length / (new_z + focal_length) + (height as f32 / 2.0);

            // Add projected points to projected_verticies
            projected_verticies[i] = vec![x_projected, y_projected];
        }
        // End of fancy math

        // Draw lines between verticies
        for edge in &edges {
            let x1: usize = projected_verticies[edge[0]][0] as usize;
            let y1: usize = projected_verticies[edge[0]][1] as usize;
            let x2: usize = projected_verticies[edge[1]][0] as usize;
            let y2: usize = projected_verticies[edge[1]][1] as usize;
            draw_line((x1, y1), (x2, y2), &mut grid);
        }

        // Set project verticies to grid
        for vertex in projected_verticies {
            let x = vertex[0] as usize;
            let y = vertex[1] as usize;
            change_cell(&mut grid, x, y, "O".to_string());
            //println!("Drawing point at {}, {}", x, y);
        }

        draw_grid(&grid);

        // Update every x fps
        thread::sleep(time::Duration::from_millis(1000 / fps));

        // Add to angle
        angle += rotate_speed;

        // Clear the screen
        if beta_screen {
            print!("\x1B[2J\x1B[1;1H");
        }
    }
}
