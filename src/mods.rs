use std::{ops::Add, process::Command, sync::{
    atomic::{AtomicBool, AtomicU8, Ordering}, Arc, Mutex
}};

use rand::{prelude::SliceRandom, thread_rng, Rng};

use crate::models::{ Move, Note, Repairer };

pub fn gen_rand_index(amount: i32, min: i32, max: i32) -> Vec<i32> {
    let mut rng = rand::thread_rng();
    let mut numbers: Vec<i32> = (min..max).collect();
    numbers.shuffle(&mut rng);
    numbers.iter().take(amount as usize).cloned().collect()
}

pub fn print_matrix(matrix: &Arc<Vec<Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)>>>, repairers: Vec<(u32, u32)>) {
    println!("   repairer 1    |    repairer 2    |    repairer 3    |    repairer 4    ");
    println!("     {:?}             {:?}            {:?}            {:?}        ", repairers[0], repairers[1], repairers[2], repairers[3]);
    println!();
    for row in matrix.iter() {
        for element in row.iter() {
            print!("{:?} | ", element.1);
        }
        println!();
        for _ in row.iter() {
            print!("-   ");
        }
        println!();
    }
}

pub fn make_decision(
    repairer: Arc<Mutex<Repairer>>,
    // barrier: Arc<Barrier>,
    matrix: Arc<Vec<Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)>>>,
) -> bool {
    let mut repairer = match repairer.lock() {
        Ok(r) => {
            r}, 
        Err(e) => {
            println!("{}", e.to_string());
            panic!("{}", e.to_string())
        }
    };
    // based on the turn which will either be a breath or depth move we will find the sensitive houses that the algorithm must be rotated.
    // the rotation is applied on the algorithm of the specific thread,
    // if the thread is on a BFS turn and the current index is a sensitive index we rotate the BFS direction and will update the new algo on the threads state.
    
    // getting the next move in condition that nothing is checked
    let mut rng = thread_rng();

    let mut n_move: Move = repairer.current_algorithm.get_move(rng.gen_bool(1.0 / 3.0));
    if repairer.last_move_rotated {
        n_move = repairer.last_move.clone();
        repairer.last_move_rotated = false;
    }
    // checking the conditions that may change the algo

    // checking the current index status -> might change to Move::Fix
    let current_value = matrix[repairer.current_location.0 as usize]
        [repairer.current_location.1 as usize].1
        .load(Ordering::Relaxed);
    if current_value == 11 {
        n_move = Move::Fix;

        repairer.decision = n_move.clone();   
                                            
        if repairer.last_move_rotated {
          repairer.last_move_rotated = false;
        }

        return true; // return true because the first priority is the fixing
    }

    // reading the notes // might change to Move::None
    for (note_idx, note) in matrix[repairer.current_location.0 as usize]
    [repairer.current_location.1 as usize].0.iter().enumerate() {
        let num_repairs = Note::parse(match &note.lock() {Ok(n) => n, Err(e) => panic!("{}",e.to_string())}).num_repairs;
        // checking with the previous value of the repairers value
        if **(repairer
            .other_repairers_repairs
            .get(&(note_idx as u32))
            .as_ref()
            .unwrap())
            < num_repairs // the number of the fixes can not be reduced so != will do the job and there is no need for greater and smaller than sign.
            && 
            // the current threads state was updated in the last round of the execute function
            note_idx as u32 != repairer.id
        {   
            // updating the specific repairer total repairs
            repairer
                .other_repairers_repairs
                .insert(note_idx as u32, num_repairs).unwrap();

        }
    }
        if repairer.get_total_fixes_from_notes() == repairer.total_broken  {
            n_move = Move::None;
            repairer.decision = n_move.clone();
            return true;
        }

    // checking the index // might rotate tha algo
    // case 1 => corners
    let corners = [
        (0, 0),
        (0, repairer.matrix_size - 1),
        (repairer.matrix_size - 1, repairer.matrix_size - 1),
        (repairer.matrix_size - 1, 0),
    ];
    if  repairer.current_location == corners[0] && (n_move == Move::Left || n_move == Move::Up ){
        // updating the threads state
        repairer.current_algorithm.rotate_algo(&n_move);
        repairer.last_move_rotated = true;
        n_move.rotate_dir();
        repairer.last_move = n_move.clone();
        repairer.decision = n_move.clone();
    } else if  repairer.current_location == corners[1] && (n_move == Move::Right || n_move == Move::Up ){
        // updating the threads state
        repairer.current_algorithm.rotate_algo(&n_move);
        repairer.last_move_rotated = true;
        n_move.rotate_dir();
        repairer.last_move = n_move.clone();
        repairer.decision = n_move.clone();
    } else if  repairer.current_location == corners[2] && (n_move == Move::Right || n_move == Move::Down ){
        // updating the threads state
        repairer.current_algorithm.rotate_algo(&n_move);
        repairer.last_move_rotated = true;
        n_move.rotate_dir();
        repairer.last_move = n_move.clone();
        repairer.decision = n_move.clone();
    } else if repairer.current_location == corners[3] && (n_move == Move::Left || n_move == Move::Down ){
        // updating the threads state
        repairer.current_algorithm.rotate_algo(&n_move);
        repairer.last_move_rotated = true;
        n_move.rotate_dir();
        repairer.last_move = n_move.clone();
        repairer.decision = n_move.clone();
    } else { 
        if !repairer.last_move_rotated {
        // case 2 => edges
        if n_move.is_horizontal() {
            // checking the right and the left edges
            // checking if the col value is 0 or <matrix_size - 1>
            if repairer.current_location.1 == 0 {
                // on the left edge, changing if next move is left
                if n_move == Move::Left {
                    repairer.current_algorithm.rotate_algo(&n_move);
                    n_move.rotate_dir();
                    repairer.decision = n_move.clone();
                }
            } else if repairer.current_location.1 == repairer.matrix_size - 1 {
                // on the right edge, changing if next move is right
                if n_move == Move::Right {
                    repairer.current_algorithm.rotate_algo(&n_move);
                    n_move.rotate_dir();
                    repairer.decision = n_move.clone();
                }
            }
        } else {
            // checking the bottom and the top edges
            // checking if the row value is 0 or <matrix_size - 1>
            if repairer.current_location.0 == 0 {
                // on the upper edge, changing if next move is up
                if n_move == Move::Up {
                    repairer.current_algorithm.rotate_algo(&n_move);
                    n_move.rotate_dir();
                    repairer.decision = n_move.clone();
                }
            } else if repairer.current_location.0 == repairer.matrix_size - 1 {
                // on the bottom edge, changing if next move is down
                if n_move == Move::Down {
                    repairer.current_algorithm.rotate_algo(&n_move);
                    n_move.rotate_dir();
                    repairer.decision = n_move.clone();
                }
            }
        }
    }
    }
    // sending the confirmation

    if repairer.decision == Move::Empty {
        repairer.decision = n_move.clone();
    }

    true // incase none of the if clauses returned the true we do the least move detected above by the <Move::get_move()> fn.
}

pub fn execute(
    repairer: Arc<Mutex<Repairer>>,
    // barrier: Arc<Barrier>,
    checks : Arc<Vec<AtomicBool>>,
    matrix: Arc<Vec<Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)>>>,
) -> bool {
    let mut repairer = repairer.lock().unwrap();

    // applying the move
    match repairer.decision {
        Move::Empty => panic!("decision making round didn't make any decisions"),
        Move::None => {
             repairer.total_moves += 1;
            checks[repairer.id as usize].store(true, Ordering::Relaxed);
            repairer.result = format!("repairer id: {}, repairs: {}, moves: {}, all_players_repairs: {:?}, goal: {}", repairer.id, repairer.total_fixed, repairer.total_moves, repairer.other_repairers_repairs, repairer.total_broken);
            false
        }
        Move::Fix => {
            // move is fix
            // fixing
            let exchange_res = matrix[repairer.current_location.0 as usize][repairer.current_location.1 as usize].1
                .compare_exchange(11, 0, Ordering::Acquire, Ordering::Relaxed);

            // updating the decision
            repairer.decision = Move::Empty.clone();

            match exchange_res { Ok(_) => {
                 // updating the total fixed
                repairer.total_fixed = repairer.total_fixed.add(1);             
                // updating the other repairers 
                let tmp_tf = repairer.total_fixed;
                let tmp_id = repairer.id;               
                repairer.other_repairers_repairs.insert(tmp_id, tmp_tf).unwrap();

            }, Err(_) => {
                // nothing
            }}
           
            // adding the total moves
            repairer.total_moves = repairer.total_moves.add(1);

            // leaving the note

             let mut note = matrix[repairer.current_location.0 as usize]
            [repairer.current_location.1 as usize].0[repairer.id as usize].lock().unwrap();
            *note = format!("{} repaired {} times", repairer.id, repairer.total_fixed);
            



            // updating the move turn
            repairer.move_turn = !repairer.move_turn;


            true
        }
        _ => {
            // move is actual move, changing the thread state

            // updating the current location
            repairer.current_location = repairer
                .decision
                .apply_on_index(repairer.current_location);

            // updating the decision
            repairer.decision = Move::Empty.clone();


            //updating the move turn
            repairer.move_turn = !repairer.move_turn;

            // adding the total moves
            repairer.total_moves = repairer.total_moves.add(1);


            let mut note = matrix[repairer.current_location.0 as usize]
            [repairer.current_location.1 as usize].0[repairer.id as usize].lock().unwrap();
            *note = format!("{} repaired {} times", repairer.id, repairer.total_fixed);

            true
        }
    }
}

pub fn clear_terminal() {
    let _ = Command::new("clear")
    .status()
    .expect("failed to execute ls");
}
