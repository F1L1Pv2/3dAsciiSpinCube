use std::thread;
use std::time;

use crossterm::{
    cursor,
    style::{self, Stylize},
    terminal, ExecutableCommand, QueueableCommand,
};
use std::io::{stdout, Write};

use config::Config;
use device_query::{DeviceQuery, DeviceState, Keycode};

fn draw_grid(
    grid: &[Vec<String>],
    color: &bool,
    legacy_mode: &bool,
    pitch: &f32,
    yaw: &f32,
    roll: &f32,
    focal_length: &f32,
) {
    match legacy_mode {
        true => {
            // Slow mode
            let mut out_str = String::new();
            for row in grid {
                for cell in row {
                    let outcell = cell.clone() + " ";
                    out_str += outcell.as_str();
                }
                out_str += "\n";
            }
            println!("{}", out_str)
        }
        false => {
            // Fast mode
            let mut stdout = stdout();
            stdout
                .queue(cursor::MoveTo(0, 0_u16))
                .unwrap()
                .queue(style::PrintStyledContent(
                    format!(
                        "Focal Length: {:.2}, Pitch: {:.2}, Yaw: {:.2}, Roll: {:.2}\n",
                        focal_length,
                        // Convert to degrees
                        pitch * (180.0 / std::f64::consts::PI) as f32,
                        yaw * (180.0 / std::f64::consts::PI) as f32,
                        roll * (180.0 / std::f64::consts::PI) as f32,
                    )
                    .as_str()
                    .red(),
                ))
                .unwrap();
            // Ignore the object, pass the index as y
            for (y, _) in grid.iter().enumerate() {
                for x in 0..grid[y].len() {
                    // Print with space padding (for alignment)
                    // Color the wall corner in red
                    if *color {
                        if grid[y][x] == "X" {
                            stdout
                                .queue(cursor::MoveTo(x as u16 * 2, y as u16 + 1))
                                .unwrap()
                                .queue(style::PrintStyledContent(grid[y][x].as_str().yellow()))
                                .unwrap();
                        } else {
                            stdout
                                .queue(cursor::MoveTo(x as u16 * 2, y as u16 + 1))
                                .unwrap()
                                .queue(style::PrintStyledContent(grid[y][x].as_str().red()))
                                .unwrap();
                        }
                    } else {
                        stdout
                            .queue(cursor::MoveTo(x as u16 * 2, y as u16 + 1))
                            .unwrap()
                            .queue(style::Print(grid[y][x].as_str()))
                            .unwrap();
                    }
                }
                stdout.flush().unwrap();
            }
        }
    }
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
    // Start listening for key presses
    let device_state = DeviceState::new();

    // Check if the yaml settings file exists
    const CONFIGPATH: &str = "Settings.toml";
    if !std::path::Path::new(CONFIGPATH).exists() {
        let content = r#"
# This is the config file. If you want to change values, remember to keep the .0 at the end of the number.
VIEW_WIDTH = 25
VIEW_HEIGHT = 25
WIDTH = 5.0
HEIGHT = 5.0
DEPTH = 5.0
ROTATE_SPEED = 3.0
FOCAL_LENGTH = 64.0
    
# Experimental options
# Change these if you're having problems
LEGACY_MODE = false
CLEAR_SCREEN = true
FPS = 60
COLOR = true
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
    let view_width: usize = config.get_int("VIEW_WIDTH").unwrap() as usize;
    let view_height: usize = config.get_int("VIEW_HEIGHT").unwrap() as usize;
    let width: f32 = config.get_float("WIDTH").unwrap() as f32;
    let height: f32 = config.get_float("HEIGHT").unwrap() as f32;
    let depth: f32 = config.get_float("DEPTH").unwrap() as f32;
    let rotate_speed: f32 = config.get_float("ROTATE_SPEED").unwrap() as f32;
    let mut focal_length: f32 = config.get_float("FOCAL_LENGTH").unwrap() as f32;

    let legacy_mode: bool = config.get_bool("LEGACY_MODE").unwrap();
    let clear_screen: bool = config.get_bool("CLEAR_SCREEN").unwrap();
    let fps: u64 = config.get_int("FPS").unwrap() as u64;
    let color: bool = config.get_bool("COLOR").unwrap();

    let mut animation = false;

    // Calculate rotation speed as fps non dependent
    let rotate_speed = rotate_speed / fps as f32;

    // Create a 3D grid of strings
    let mut grid = vec![vec![" ".to_string(); view_width]; view_height];

    // Define the 3D points of the box
    let mut verticies = vec![vec![0; 3]; 8];
    // BOX
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
    // BOX
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

    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut roll: f32 = 0.0;

    'main: loop {
        // Clear grid
        for row in grid.iter_mut() {
            for cell in row.iter_mut() {
                *cell = " ".to_string();
            }
        }

        // Start fancy math
        let mut projected_verticies: Vec<Vec<f32>> = vec![vec![0.0; 2]; verticies.len()];

        for (i, vertex) in verticies.iter().enumerate() {
            let x = vertex[0] as f32 * width;
            let y = vertex[1] as f32 * height;
            let z = vertex[2] as f32 * depth;

            let beta = -pitch;
            let gamma = yaw;
            let alpha = roll;

            let new_x = x * (alpha.cos() * beta.cos())
                + y * (alpha.cos() * beta.sin() * gamma.sin() - alpha.sin() * gamma.cos())
                + z * (alpha.cos() * beta.sin() * gamma.cos() + alpha.sin() * gamma.sin());
            let new_y = x * (alpha.sin() * beta.cos())
                + y * (alpha.sin() * beta.sin() * gamma.sin() + alpha.cos() * gamma.cos())
                + z * (alpha.sin() * beta.sin() * gamma.cos() - alpha.cos() * gamma.sin());
            let new_z =
                -x * beta.sin() + y * beta.cos() * gamma.sin() + z * beta.cos() * gamma.cos();

            // Project 3D to 2D
            let x_projected =
                new_x * focal_length / (new_z + focal_length) + (view_width as f32 / 2.0);
            let y_projected =
                new_y * focal_length / (new_z + focal_length) + (view_height as f32 / 2.0);

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

        draw_grid(
            &grid,
            &color,
            &legacy_mode,
            &pitch,
            &yaw,
            &roll,
            &focal_length,
        );

        let keys: Vec<Keycode> = device_state.get_keys();

        // Match the keyboard input to the correct action
        if !animation {
            for key in keys {
                match key {
                    Keycode::W | Keycode::Up => pitch += rotate_speed,
                    Keycode::A | Keycode::Left => yaw += rotate_speed,
                    Keycode::S | Keycode::Down => pitch -= rotate_speed,
                    Keycode::D | Keycode::Right => yaw -= rotate_speed,
                    Keycode::Q => roll += rotate_speed,
                    Keycode::E => roll -= rotate_speed,
                    // Check if focal length is not too small
                    Keycode::Z => {
                        if focal_length > 10.0 {
                            focal_length -= 1.0
                        }
                    }
                    // Check if focal length is not too big
                    Keycode::X => {
                        if focal_length < 100.0 {
                            focal_length += 1.0
                        }
                    }
                    Keycode::R => {
                        pitch = 0.0;
                        yaw = 0.0;
                        roll = 0.0;
                        focal_length = config.get_float("FOCAL_LENGTH").unwrap() as f32;
                    }
                    Keycode::Space => {
                        // Toggle animation
                        if animation {
                            animation = false;
                        } else {
                            animation = true;
                        }
                    }
                    Keycode::Escape => break 'main,
                    _ => (),
                }
            }
        } else {
            // Check if animation is toggled
            if !keys.contains(&Keycode::Space) {
                animation = false;
            }
            pitch += rotate_speed;
            yaw += rotate_speed;
            roll += rotate_speed;
        }

        // Update every x fps
        thread::sleep(time::Duration::from_millis(1000 / fps));

        // Clear the screen
        // Check for legacy mode and clear the screen accordingly
        match (legacy_mode, clear_screen) {
            (true, true) => print!("\x1B[2J\x1B[1;1H"),
            (true, false) => (),
            (false, true) => {
                let mut stdout = stdout();
                stdout
                    .execute(terminal::Clear(terminal::ClearType::All))
                    .unwrap();
            }
            (false, false) => (),
        }
    }
}
