use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Mutex,
};

use rand::prelude::SliceRandom;

use crate::models::{Command, Move, Note, Repairer, RepairerResult};

pub fn gen_rand_index(amount: i32, min: i32, max: i32) -> Vec<i32> {
    let mut rng = rand::thread_rng();
    let mut numbers: Vec<i32> = (min..max).collect();
    numbers.shuffle(&mut rng);
    numbers.iter().take(amount as usize).cloned().collect()
}

pub fn print_matrix(matrix: &Arc<Vec<Vec<(Vec<String>, AtomicU8)>>>) {
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
    matrix: Arc<Vec<Vec<(Vec<String>, AtomicU8)>>>,
) -> bool {
    let mut repairer = repairer.lock().unwrap();
    // // based on the turn which will either be a breath or depth move we will find the sensitive houses that the algorithm must be rotated.
    // // the rotation is applied on the algorithm of the specific thread,
    // // if the thread is on a BFS turn and the current index is a sensitive index we rotate the BFS direction and will update the new algo on the threads state.
    // let mut top_rotators: Vec<(u32, u32)> = (0..matrix_size).map(|col| (0, col)).collect();
    // let mut right_rotators: Vec<(u32, u32)> =
    //     (0..matrix_size).map(|row| (row, matrix_size - 1)).collect();
    // let mut bottom_rotators: Vec<(u32, u32)> = (0..matrix_size)
    //     .rev()
    //     .map(|col| (matrix_size - 1, col))
    //     .collect();
    // let mut left_rotators: Vec<(u32, u32)> = (0..matrix_size).rev().map(|row| (row, 0)).collect();

    // getting the next move in condition that nothing is checked
    let mut n_move: Move = repairer.current_algorithm.get_move(repairer.move_turn);

    // checking the conditions that may change the algo

    // checking the current index status -> might change to Move::Fix
    let current_value = matrix[repairer.current_location.0 as usize]
        [repairer.current_location.1 as usize].1
        .load(Ordering::Relaxed);
    if current_value == 11 {
        n_move = Move::Fix;

        repairer.decision = Some(n_move); // saving the fixing op into the threads task queue // todo
                                          // saving the move int he threads move queue // todo
                                          // sending the confirmation
        repairer
            .sender
            .lock()
            .unwrap()
            .send(Command {
                recipient_repairer: 7,
                job_type: crate::models::JobType::DecisionMade,
            })
            .unwrap();
        return true; // return true because the first priority is the fixing
    }

    // reading the notes // might change to Move::None
    for (repairer_id, note) in matrix[repairer.current_location.0 as usize]
    [repairer.current_location.1 as usize].0.iter().enumerate() {
        let num_repairs = Note::parse(&note).num_repairs;
        // checking with the previous value of the repairers value
        if **(repairer
            .other_repairers_repairs
            .get(&(repairer_id as u32))
            .as_ref()
            .unwrap())
            != num_repairs // the number of the fixes can not be reduced so != will do the job and there is no need for greater and smaller than sign.
            && 
            // the current threads state was updated in the last round of the execute function
            repairer_id as u32 != repairer.id
        {
            // updating the specific repairer total repairs
            repairer
                .other_repairers_repairs
                .insert(repairer_id as u32, num_repairs);
        }
        
        if repairer.get_total_fixes_from_notes() == repairer.total_broken  {
            n_move = Move::None;
            repairer.decision = Some(n_move);
            // killing the thread // todo
            // sending the confirmation
            repairer
                .sender
                .lock()
                .unwrap()
                .send(Command {
                    recipient_repairer: 7,
                    job_type: crate::models::JobType::DecisionMade,
                })
                .unwrap();
            return true;
        }
    }

    // checking the index // might rotate tha algo

    // case 1 => corners
    let corners = [
        (0, 0),
        (0, repairer.matrix_size - 1),
        (repairer.matrix_size - 1, repairer.matrix_size - 1),
        (repairer.matrix_size - 1, 0),
    ];
    if corners
        .iter()
        .any(|corner| *corner == repairer.current_location)
    {
        // updating the threads state
        repairer.current_algorithm.rotate_algo(&n_move);
        n_move.rotate_dir();
        repairer.decision = Some(n_move);
    } else {
        // case 2 => edges
        if n_move.is_horizontal() {
            // checking the right and the left edges
            // checking if the col value is 0 or <matrix_size - 1>
            if repairer.current_location.1 == 0 {
                // on the left edge, changing if next move is left
                if n_move == Move::Left {
                    repairer.current_algorithm.rotate_algo(&n_move);
                    n_move.rotate_dir();
                    repairer.decision = Some(n_move);
                }
            } else if repairer.current_location.1 == repairer.matrix_size - 1 {
                // on the right edge, changing if next move is right
                if n_move == Move::Right {
                    repairer.current_algorithm.rotate_algo(&n_move);
                    n_move.rotate_dir();
                    repairer.decision = Some(n_move);
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
                    repairer.decision = Some(n_move);
                }
            } else if repairer.current_location.0 == repairer.matrix_size - 1 {
                // on the bottom edge, changing if next move is down
                if n_move == Move::Down {
                    repairer.current_algorithm.rotate_algo(&n_move);
                    n_move.rotate_dir();
                    repairer.decision = Some(n_move);
                }
            }
        }
    }
    // sending the confirmation
    repairer
        .sender
        .lock()
        .unwrap()
        .send(Command {
            recipient_repairer: 7,
            job_type: crate::models::JobType::DecisionMade,
        })
        .unwrap();
    true // incase none of the if clauses returned the true we do the least move detected above by the <Move::get_move()> fn.
}

pub fn execute(
    repairer: Arc<Mutex<Repairer>>,
    matrix: Arc<Vec<Vec<(Vec<String>, AtomicU8)>>>,
) -> bool {
    let mut repairer = repairer.lock().unwrap();
    // applying the move

    match repairer.decision.as_ref().unwrap() {
        Move::None => {
            repairer.total_moves += 1;


            // sending the confirmation
            repairer
                .sender
                .lock()
                .unwrap()
                .send(Command {
                    recipient_repairer: 7,
                    job_type: crate::models::JobType::End(RepairerResult {
                        id: repairer.id,
                        repairs: repairer.total_fixed,
                        moves: repairer.total_moves, 
                        goal: repairer.total_broken,
                        all_players_repairs: repairer.other_repairers_repairs.values().map(|&x| x).collect()        
                    }),
                })
                .unwrap();
            false
        }
        Move::Fix => {
            // move is fix
            // fixing
            matrix[repairer.current_location.0 as usize][repairer.current_location.1 as usize].1
                .store(0, Ordering::Relaxed);

            // updating the decision
            repairer.decision = None;

            // updating the total fixed
            repairer.total_fixed += 1;

            // adding the total moves
            repairer.total_moves += 1;

            // leaving the note
            // getting the old note
            let old_note = matrix[repairer.current_location.0 as usize]
            [repairer.current_location.1 as usize].0[repairer.id as usize].clone();
            
            // replacing with the new note
            let _ = matrix[repairer.current_location.0 as usize]
            [repairer.current_location.1 as usize].0[repairer.id as usize].replace(
                &old_note,
                 format!("{} repaired {} times", repairer.id, repairer.total_fixed
                ).as_str());  
            
            // sending the confirmation
            repairer
                .sender
                .lock()
                .unwrap()
                .send(Command {
                    recipient_repairer: 7,
                    job_type: crate::models::JobType::Executed,
                })
                .unwrap();

            true
        }
        _ => {
            // move is actual move, changing the thread state

            // updating the current location
            repairer.current_location = repairer
                .decision
                .as_ref()
                .unwrap()
                .apply_on_index(repairer.current_location);

            // updating the decision
            repairer.decision = None;

            //updating the move turn
            repairer.move_turn = !repairer.move_turn;

            // adding the total moves
            repairer.total_moves += 1;

            // sending the confirmation
            repairer
                .sender
                .lock()
                .unwrap()
                .send(Command {
                    recipient_repairer: 7,
                    job_type: crate::models::JobType::Executed,
                })
                .unwrap();

            true
        }
    }
}
