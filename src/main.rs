pub use rand::Rng;

pub mod models;
pub mod mods;
use mods::*;
use std::{
    process::Command,
    sync::{atomic::AtomicU8, Arc},
    thread,
    time::Duration,
};
fn main() {
    // creating a square matrix of the atomic bool's
    // rows and columns are pre-known for this specific example
    let rows: u8 = 7;
    let columns: u8 = 7;
    // let total: u8 = rows * columns;
    // generating some random indexes to be chosen as the broken elements, the value of the broken elements are 11 while the normal ones are 0.

    // how many broken houses ? between 3 to 6 for the sake of simplicity
    let total_broken: i32 = rand::thread_rng().gen_range(3..7);

    // <total_broken> rows for broken houses
    let broken_rows = gen_rand_index(total_broken, 0, rows as i32);

    // <total_broken> column for broken houses
    let broken_columns = gen_rand_index(total_broken, 0, columns as i32);

    // Creating the matrix

    let mut matrix: Vec<Vec<(Vec<String>, AtomicU8)>> = Vec::with_capacity(rows as usize);

    for _ in 0..rows {
        let mut row: Vec<(Vec<String>, AtomicU8)> = Vec::with_capacity(columns as usize);
        for _ in 0..columns {
            row.push((Vec::with_capacity(4), AtomicU8::new(0))); // Initialize all elements to false
        }
        matrix.push(row);
    }
    // this matrix is for reading, the arc lets the thread to read the data and not lock it.
    let mut matrix: Arc<Vec<Vec<(Vec<String>, AtomicU8)>>> = Arc::new(matrix);

    println!("initial matrix ");
    print_matrix(&matrix);
    thread::sleep(Duration::from_secs(2));

    // adding the broken houses
    for i in 0..total_broken {
        let row_idx = broken_rows[i as usize] as usize;
        let col_idx = broken_columns[i as usize] as usize;

        matrix[row_idx][col_idx]
            .1
            .store(11, std::sync::atomic::Ordering::Relaxed);
    }

    let _ = Command::new("clear")
        .status()
        .expect("failed to execute ls");

    println!("adding broken houses ...");
    thread::sleep(Duration::from_secs(2));
    print_matrix(&matrix);
}
