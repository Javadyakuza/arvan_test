pub use rand::Rng;

pub mod models;
pub mod mods;
use mods::*;
use std::{
    collections::HashMap,
    ops::Add,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc, Barrier, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::models::{JobType, JobTypeReceiver, JobTypeSender, Move, MovementAlgorithm, Repairer};
fn main() {
    clear_terminal();
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

    let mut matrix: Vec<Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)>> =
        Vec::with_capacity(rows as usize);

    for _ in 0..rows {
        let mut row: Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)> =
            Vec::with_capacity(columns as usize);

        for _ in 0..columns {
            let tmp_notes = vec![
                Arc::new(Mutex::new("0 repaired 0 times".to_string())),
                Arc::new(Mutex::new("1 repaired 0 times".to_string())),
                Arc::new(Mutex::new("2 repaired 0 times".to_string())),
                Arc::new(Mutex::new("3 repaired 0 times".to_string())),
            ];
            row.push((tmp_notes, AtomicU8::new(0))); // Initialize all elements to false
        }
        matrix.push(row);
    }
    // this matrix is for reading, the arc lets the thread to read the data and not lock it.
    let mut matrix: Arc<Vec<Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)>>> = Arc::new(matrix);

    println!("initial matrix ");
    print_matrix(&matrix, vec![(0, 0), (0, 0), (0, 0), (0, 0)]);
    thread::sleep(Duration::from_secs(1));

    // generating random rows for repairers
    let repairer_rows = gen_rand_index(4, 0, rows as i32);
    // generating random columns for repairers
    let repairer_columns = gen_rand_index(4, 0, columns as i32);

    // adding the broken houses
    for i in 0..total_broken {
        let row_idx = broken_rows[i as usize] as usize;
        let col_idx = broken_columns[i as usize] as usize;

        matrix[row_idx][col_idx]
            .1
            .store(11, std::sync::atomic::Ordering::Relaxed);
    }

    println!("adding broken houses ...");
    thread::sleep(Duration::from_secs(1));
    print_matrix(
        &matrix,
        vec![
            (repairer_rows[0] as u32, repairer_columns[0] as u32),
            (repairer_rows[1] as u32, repairer_columns[1] as u32),
            (repairer_rows[2] as u32, repairer_columns[2] as u32),
            (repairer_rows[3] as u32, repairer_columns[3] as u32),
        ],
    );

    // creating the channels
    let mut channels: Vec<(
        JobTypeSender,   // sender for main
        JobTypeReceiver, // receiver for repairer
    )> = Vec::new();
    for _ in 0..4 {
        let tmp_channel_ms_rr: (Sender<JobType>, Receiver<JobType>) = channel();

        channels.push((
            Arc::new(Mutex::new(tmp_channel_ms_rr.0)),
            Arc::new(Mutex::new(tmp_channel_ms_rr.1)),
        ));
    }
    // creating the initializing algorithms
    let init_algos: Vec<MovementAlgorithm> = vec![
        MovementAlgorithm::BRD,
        MovementAlgorithm::DDL,
        MovementAlgorithm::BLU,
        MovementAlgorithm::DUR,
    ];

    // creating the  barrier

    // creating the repairers state
    let mut repairers_state: Vec<Arc<Mutex<Repairer>>> = Vec::new();
    let mut repairers: Vec<JoinHandle<()>> = Vec::new();

    // creating the state
    for id in 0..4 {
        let mut repairs_track: HashMap<u32, u32> = HashMap::new();
        repairs_track.insert(0, 0);
        repairs_track.insert(1, 0);
        repairs_track.insert(2, 0);
        repairs_track.insert(3, 0);

        let tmp_repairer = Repairer {
            id,
            thread: None,
            total_broken: total_broken as u32,
            total_fixed: 0,
            other_repairers_repairs: repairs_track,
            total_moves: 0,
            current_algorithm: init_algos[id as usize].clone(),
            current_location: (
                repairer_rows[id as usize] as u32,
                repairer_columns[id as usize] as u32,
            ),
            matrix_size: rows as u32,
            decision: Move::Empty,
            move_turn: true, // means the first move
            last_move_rotated: false,
            last_move: Move::Empty,
            result: "".to_string(),
        };
        repairers_state.push(Arc::new(Mutex::new(tmp_repairer)))
    }
    let state0: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[0 as usize]);
    let state1: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[1 as usize]);
    let state2: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[2 as usize]);
    let state3: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[3 as usize]);
    let main_state0: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[0 as usize]);
    let main_state1: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[1 as usize]);
    let main_state2: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[2 as usize]);
    let main_state3: Arc<Mutex<Repairer>> = Arc::clone(&repairers_state[3 as usize]);
    let channel0 = Arc::clone(&channels[0 as usize].1);
    let channel1 = Arc::clone(&channels[1 as usize].1);
    let channel2 = Arc::clone(&channels[2 as usize].1);
    let channel3 = Arc::clone(&channels[3 as usize].1);

    // // spawning the threads
    // for id in 0..4 {
    //     // let tmp_repairer_state = Arc::new(Mutex::new(&mut repairers_state[id as usize]));
    //     let tmp_recv = Arc::clone(&channels[id as usize].1);
    //     // let state2: Arc<Mutex<Vec<Repairer>>> = Arc::clone(&repairer_state);

    repairers.push(thread::spawn(move || loop {
        // setting a listener over the receiver created by the master thread
        let message: JobType = match channel0.lock() {
            Ok(m) => match m.recv() {
                Ok(m) => m,
                Err(e) => {
                    // the channels is closed
                    println!("{}", e.to_string());
                    break;
                }
            },

            Err(e) => {
                println!("{:?}", e);
                panic!()
            }
        };
        // matching the message type
        match message {
            JobType::DecisionMaking(matrix, barrier) => {
                make_decision(state0.clone(), matrix);
                barrier.wait();
            }
            JobType::Execute(matrix, checks, beg_barrier, end_barrier) => {
                // at this stage each thread has decided on its move and they have received a separate execute message and all of them will wait till the barrier hits the threshold and then they all will function together.
                beg_barrier.wait();
                let exe_res = execute(state0.clone(), checks, matrix);
                end_barrier.wait();
                if !exe_res {
                    // at this stage the result message is sent to the master thread and we can kill the thread gracefully
                    break;
                }
            }
        }
    }));
    repairers.push(thread::spawn(move || loop {
        // setting a listener over the receiver created by the master thread
        let message: JobType = match channel1.lock() {
            Ok(m) => match m.recv() {
                Ok(m) => m,
                Err(e) => {
                    // the channels is closed
                    println!("{}", e.to_string());
                    break;
                }
            },

            Err(e) => {
                println!("{:?}", e);
                panic!()
            }
        };
        // // matching the message type
        match message {
            JobType::DecisionMaking(matrix, barrier) => {
                make_decision(state1.clone(), matrix);
                barrier.wait();
            }
            JobType::Execute(matrix, checks, beg_barrier, end_barrier) => {
                // at this stage each thread has decided on its move and they have received a separate execute message and all of them will wait till the barrier hits the threshold and then they all will function together.
                beg_barrier.wait();
                let exe_res = execute(state1.clone(), checks, matrix);
                end_barrier.wait();
                if !exe_res {
                    // at this stage the result message is sent to the master thread and we can kill the thread gracefully
                    break;
                }
            }
        }
    }));
    repairers.push(thread::spawn(move || loop {
        // setting a listener over the receiver created by the master thread
        let message: JobType = match channel2.lock() {
            Ok(m) => match m.recv() {
                Ok(m) => m,
                Err(e) => {
                    // the channels is closed
                    println!("{}", e.to_string());
                    break;
                }
            },

            Err(e) => {
                println!("{:?}", e);
                panic!()
            }
        };
        // // matching the message type
        match message {
            JobType::DecisionMaking(matrix, barrier) => {
                make_decision(state2.clone(), matrix);
                barrier.wait();
            }
            JobType::Execute(matrix, checks, beg_barrier, end_barrier) => {
                // at this stage each thread has decided on its move and they have received a separate execute message and all of them will wait till the barrier hits the threshold and then they all will function together.
                beg_barrier.wait();
                let exe_res = execute(state2.clone(), checks, matrix);
                end_barrier.wait();
                if !exe_res {
                    // at this stage the result message is sent to the master thread and we can kill the thread gracefully
                    break;
                }
            }
        }
    }));
    repairers.push(thread::spawn(move || loop {
        // setting a listener over the receiver created by the master thread
        let message: JobType = match channel3.lock() {
            Ok(m) => match m.recv() {
                Ok(m) => m,
                Err(e) => {
                    // the channels is closed
                    println!("{}", e.to_string());
                    break;
                }
            },

            Err(e) => {
                println!("{:?}", e);
                panic!()
            }
        };
        // // matching the message type
        match message {
            JobType::DecisionMaking(matrix, barrier) => {
                make_decision(state3.clone(), matrix);
                barrier.wait();
            }
            JobType::Execute(matrix, checks, beg_barrier, end_barrier) => {
                // at this stage each thread has decided on its move and they have received a separate execute message and all of them will wait till the barrier hits the threshold and then they all will function together.
                beg_barrier.wait();
                let exe_res = execute(state3.clone(), checks, matrix);
                end_barrier.wait();
                if !exe_res {
                    // at this stage the result message is sent to the master thread and we can kill the thread gracefully
                    break;
                }
            }
        }
    }));

    // start
    // @param dead_repairers will be used to check the end of the repairing progress.

    let dead_repairers: Arc<Vec<AtomicBool>> = Arc::new(vec![
        AtomicBool::new(false),
        AtomicBool::new(false),
        AtomicBool::new(false),
        AtomicBool::new(false),
    ]); // this variable will be incremented by one each time that a thread reaches to the Move `None`; when four the matrix is fully repaired.

    // there is two while loops
    // the first one is the main one is the check for the end of the progress and the inner nested one is for confirming the decision making.
    while !dead_repairers[0].load(Ordering::Relaxed)
        || !dead_repairers[1].load(Ordering::Relaxed)
        || !dead_repairers[2].load(Ordering::Relaxed)
        || !dead_repairers[3].load(Ordering::Relaxed)
    {
        let mut round_barriers: u32 = 1;
        for i in 0..4 {
            if !dead_repairers[i].load(Ordering::Relaxed) {
                round_barriers = round_barriers.add(1);
            }
        }
        let decision_confirmation_barriers = Arc::new(Barrier::new(round_barriers as usize)); // will let the execution part once the decisions are made
        let exe_beginning_barriers = Arc::new(Barrier::new(round_barriers as usize)); // will let all of the threads to start together
        let exe_ending_barriers = Arc::new(Barrier::new(round_barriers as usize)); // will let all of the execution end before the nex decision making round start

        clear_terminal();
        let indexes: Vec<(u32, u32)> = vec![
            Arc::clone(&main_state0).lock().unwrap().current_location,
            Arc::clone(&main_state1).lock().unwrap().current_location,
            Arc::clone(&main_state2).lock().unwrap().current_location,
            Arc::clone(&main_state3).lock().unwrap().current_location,
        ];
        print_matrix(&matrix, indexes);

        thread::sleep(Duration::from_millis(50));

        if !dead_repairers[0].load(Ordering::Relaxed) {
            match channels[0].0.clone().lock() {
                Ok(el) => el
                    .send(JobType::DecisionMaking(
                        Arc::clone(&matrix),
                        Arc::clone(&decision_confirmation_barriers),
                    ))
                    .unwrap(),
                Err(e) => {
                    panic!("{}", e.to_string())
                }
            };
        }
        if !dead_repairers[1].load(Ordering::Relaxed) {
            match channels[1].0.clone().lock() {
                Ok(el) => el
                    .send(JobType::DecisionMaking(
                        Arc::clone(&matrix),
                        Arc::clone(&decision_confirmation_barriers),
                    ))
                    .unwrap(),
                Err(e) => {
                    panic!("{}", e.to_string())
                }
            };
        }
        if !dead_repairers[2].load(Ordering::Relaxed) {
            match channels[2].0.clone().lock() {
                Ok(el) => el
                    .send(JobType::DecisionMaking(
                        Arc::clone(&matrix),
                        Arc::clone(&decision_confirmation_barriers),
                    ))
                    .unwrap(),
                Err(e) => {
                    panic!("{}", e.to_string())
                }
            };
        }
        if !dead_repairers[3].load(Ordering::Relaxed) {
            match channels[3].0.clone().lock() {
                Ok(el) => el
                    .send(JobType::DecisionMaking(
                        Arc::clone(&matrix),
                        Arc::clone(&decision_confirmation_barriers),
                    ))
                    .unwrap(),
                Err(e) => {
                    panic!("{}", e.to_string())
                }
            };
        }
        decision_confirmation_barriers.wait();

        // Faze two: executing
        for channel in channels.iter() {
            channel
                .0
                .lock()
                .unwrap()
                .send(JobType::Execute(
                    Arc::clone(&matrix),
                    Arc::clone(&dead_repairers),
                    Arc::clone(&exe_beginning_barriers),
                    Arc::clone(&exe_ending_barriers),
                ))
                .unwrap();
        }

        // calling the beginning barrier and letting all of the threads to start together
        exe_beginning_barriers.wait();
        // now they have started, we use another barrier to wait until all of the repairers have made their move.
        exe_ending_barriers.wait();
    }

    println!(
        "{} \n{} \n{} \n{} ",
        Arc::clone(&main_state0).lock().unwrap().result,
        Arc::clone(&main_state1).lock().unwrap().result,
        Arc::clone(&main_state2).lock().unwrap().result,
        Arc::clone(&main_state3).lock().unwrap().result,
    );
}
