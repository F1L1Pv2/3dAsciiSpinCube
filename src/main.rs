use std::thread;

fn draw_grid(grid: &Vec<Vec<String>>) {
    for row in grid {
        for cell in row {
            // Print with space padding (for alignment)
            print!("{} ", cell);
        }
        // Print newline
        println!();
    }
}

fn change_cell(grid: &mut [Vec<String>], x: usize, y: usize, new_value: String) {
    grid[y][x] = new_value;
}

fn draw_line((x1, y1): (usize, usize), (x2, y2): (usize, usize), grid: &mut [Vec<String>]) {
    let mut x = x1;
    let mut y = y1;
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = (y2 as i32 - y1 as i32).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
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
    // Constant values for testing
    const WIDTH: usize = 32;
    const HEIGHT: usize = 32;
    const BOX_WIDTH: f32 = 10.0;
    const BOX_HEIGHT: f32 = 10.0;
    const BOX_DEPTH: f32 = 10.0;
    const ROTATE_SPEED: f32 = 0.025;

    // Create a 3D grid of strings
    let mut grid = vec![vec![" ".to_string(); WIDTH]; HEIGHT];

    const FOCAL_LENGTH: f32 = 64.0;

    // Define the 3D points of the box
    let mut verticies = vec![vec![0; 3]; 8];
    verticies[0] = vec![0, 0, 0];
    verticies[1] = vec![0, 0, 1];
    verticies[2] = vec![0, 1, 0];
    verticies[3] = vec![0, 1, 1];
    verticies[4] = vec![1, 0, 0];
    verticies[5] = vec![1, 0, 1];
    verticies[6] = vec![1, 1, 0];
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
        let mut projected_verticies = vec![vec![0 as f32; 2]; verticies.len()];

        for (i, vertex) in verticies.iter().enumerate() {
            let x = (vertex[0] as f32 - 0.5) * BOX_WIDTH;
            let y = (vertex[1] as f32 - 0.5) * BOX_HEIGHT;
            let z = (vertex[2] as f32 - 0.5) * BOX_DEPTH;

            //rotate around y axis
            let new_x = x * angle.cos() as f32 - z * angle.sin() as f32;
            let new_z = x * angle.sin() as f32 + z * angle.cos() as f32;

            //rotate around x axis
            let new_y = y * angle.cos() as f32 - new_z * angle.sin() as f32;
            let new_z = y * angle.sin() as f32 + new_z * angle.cos() as f32;

            //rotate around z axis
            //let new_x = new_x * angle.cos() as f32 - new_y * angle.sin() as f32;
            //let new_y = new_x * angle.sin() as f32 + new_y * angle.cos() as f32;

            let x_projected = new_x * FOCAL_LENGTH / (new_z + FOCAL_LENGTH) + (WIDTH as f32 / 2.0);
            let y_projected = new_y * FOCAL_LENGTH / (new_z + FOCAL_LENGTH) + (HEIGHT as f32 / 2.0);

            //let x_projected = new_x * focal_length / (new_z + focal_length) + (width as f32 / 2.0);
            //let y_projected = y * focal_length / (new_z + focal_length)  +(height as f32 / 2.0);

            //let x_projected = x * focal_length / (z + focal_length);
            //let y_projected = y * focal_length / (z + focal_length);

            projected_verticies[i] = vec![x_projected, y_projected];
        }
        // End of fancy math

        // Draw lines between verticies
        for edge in &edges {
            let x1 = projected_verticies[edge[0]][0] as usize;
            let y1 = projected_verticies[edge[0]][1] as usize;
            let x2 = projected_verticies[edge[1]][0] as usize;
            let y2 = projected_verticies[edge[1]][1] as usize;
            draw_line((x1, y1), (x2, y2), &mut grid);
        }

        // 
        for vertex in projected_verticies {
            let x = vertex[0] as usize;
            let y = vertex[1] as usize;
            change_cell(&mut grid, x, y, "O".to_string());
            //println!("Drawing point at {}, {}", x, y);
        }

        draw_grid(&grid);

        // Sleep for 10 miliseconds
        thread::sleep(std::time::Duration::from_millis(10));
        angle += ROTATE_SPEED;

        // Clear the screen
        print!("\x1B[2J\x1B[1;1H");
    }
}
