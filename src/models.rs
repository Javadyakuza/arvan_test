use std::{
    collections::HashMap,
    str::FromStr,
    sync::{atomic::AtomicU8, mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{execute, make_decision};

// type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct RepairerResult {
    pub id: u32,
    pub repairs: u32,
    pub moves: u32,
    pub goal: u32,
    pub all_players_repairs: Vec<u32>,
}
impl RepairerResult {
    pub fn to_string(&self) -> String {
        format!(
            "id: {}, repairs: {}, moves: {}, all_player_repairs: {:?}, goal: {}",
            self.id, self.repairs, self.moves, self.all_players_repairs, self.goal
        )
    }
}
pub enum JobType {
    DecisionMaking(Arc<Vec<Vec<(Vec<String>, AtomicU8)>>>),
    Execute(Arc<Vec<Vec<(Vec<String>, AtomicU8)>>>),
    DecisionMade,
    Executed,
    End(RepairerResult),
}
pub struct Command {
    pub recipient_repairer: u32, // 7 is the master thread
    pub job_type: JobType,
}
pub struct Repairer {
    pub id: u32,                                    // not going to be changed
    pub thread: Option<JoinHandle<()>>,             // not going to be changed
    pub total_broken: u32,                          // not going to be changed
    pub total_fixed: u32,                           // ▶️ will be changed in the execute
    pub other_repairers_repairs: HashMap<u32, u32>, // ⏸️  ▶️ will be changed in the execute and decision making
    pub total_moves: u32,                           // ▶️ will be changed in the execute
    pub sender: Arc<Mutex<mpsc::Sender<Command>>>,  // not going to be changed
    // pub receiver: Arc<Mutex<mpsc::Receiver<Command>>>, // not going to be changed // the spawned thread will only need that so we do not save this value in the thread stat
    pub current_algorithm: MovementAlgorithm, // ⏸️ change in decision making
    pub current_location: (u32, u32),         // ▶️ chang in execute
    pub matrix_size: u32,                     // not going to be changed
    pub decision: Option<Move>,               // ⏸️  ▶️ change in decision making and in execute
    pub move_turn: bool,                      // ▶️ change in execute
}

impl Repairer {
    pub fn new_repairer(
        self, // we will build a instance manually first and then we will call this new function that will create a new instance for us.
        receiver: Arc<Mutex<mpsc::Receiver<Command>>>,
    ) -> Self {
        // -> Self
        // @notice @dev we do not actually need to return anything since we are only communicating with the messages through a
        // channel but since the data built in the function during the function call will be saved into the call stack then after
        // the function call the data will be gone so we will save the information if that thread that we spawned in a unnamed variable
        // but we do not use that.
        let thread_id = self.id;
        let matrix_size = self.matrix_size;
        let id = self.id;
        let sender = Arc::clone(&self.sender);
        let total_broken = self.total_broken;
        let current_algorithm = self.current_algorithm.clone();
        let current_location = self.current_location;
        let decision = self.decision.clone();
        let move_turn = self.move_turn;
        let other_repairers_repairs = self.other_repairers_repairs.clone();
        let repairer_state = Arc::new(Mutex::new(self));
        let repairer_thread = thread::spawn(move || loop {
            // setting a listener over the receiver created by the master thread
            let message: Command = receiver.lock().unwrap().recv().unwrap();
            if message.recipient_repairer == thread_id {
                // matching the message type
                match message.job_type {
                    JobType::DecisionMaking(matrix) => {
                        make_decision(repairer_state.clone(), matrix);
                    }
                    JobType::Execute(matrix) => {
                        let exe_res = execute(repairer_state.clone(), matrix);
                        if !exe_res {
                            // at this stage the result message is sent to the master thread and we can kill the thread gracefully
                            break;
                        }
                    }
                    // not related command, impossible
                    _ => panic!("invalid command received"),
                }
            }
        });

        Repairer {
            id,
            thread: Some(repairer_thread),
            total_broken,
            total_fixed: 0,
            other_repairers_repairs,
            total_moves: 0,
            sender,
            current_algorithm,
            current_location,
            matrix_size,
            decision,
            move_turn, // means the first move
        }
    }
    pub fn get_total_fixes_from_notes(&self) -> u32 {
        let mut tmp_total_fix: u32 = 0;
        let _ = self
            .other_repairers_repairs
            .values()
            .map(|v| tmp_total_fix += *v);
        tmp_total_fix + self.total_fixed
    }
}

#[derive(PartialEq, Clone)]
pub enum Move {
    Up,
    Down,
    Right,
    Left,
    Fix,
    None,
}

impl Move {
    pub fn is_horizontal(&self) -> bool {
        match self {
            Self::Left | Self::Right => true,
            _ => false,
        }
    }

    pub fn rotate_dir(&mut self) {
        match self {
            Self::Right => *self = Self::Left,
            Self::Left => *self = Self::Right,
            Self::Up => *self = Self::Down,
            Self::Down => *self = Self::Up,
            _ => { // do nothing}
            }
        }
    }
    pub fn apply_on_index(&self, index: (u32, u32)) -> (u32, u32) {
        match self {
            Self::Right => (index.0 + 1, index.1),
            Self::Left => (index.0 - 1, index.1),
            Self::Up => (index.0, index.1 + 1),
            Self::Down => (index.0, index.1 - 1),
            _ => panic!("incorrect move to be applied !!"), // impossible
        }
    }
}
#[derive(Clone)]
pub enum MovementAlgorithm {
    BRD, // BFS right and down
    BLD, // BFS left and down
    BRU, // BFS right and up
    BLU, // BFS left and down
    DDR, // DFS down and right
    DDL, // DFS down and left
    DUR, // DFS up and right
    DUL, // DFS up and left
}

impl MovementAlgorithm {
    pub fn get_move(&self, first: bool) -> Move {
        match self {
            MovementAlgorithm::BLD => {
                if first {
                    Move::Left
                } else {
                    Move::Down
                }
            }
            MovementAlgorithm::BLU => {
                if first {
                    Move::Left
                } else {
                    Move::Up
                }
            }
            MovementAlgorithm::BRD => {
                if first {
                    Move::Right
                } else {
                    Move::Down
                }
            }
            MovementAlgorithm::BRU => {
                if first {
                    Move::Right
                } else {
                    Move::Up
                }
            }
            MovementAlgorithm::DDL => {
                if first {
                    Move::Down
                } else {
                    Move::Left
                }
            }
            MovementAlgorithm::DDR => {
                if first {
                    Move::Down
                } else {
                    Move::Right
                }
            }
            MovementAlgorithm::DUL => {
                if first {
                    Move::Up
                } else {
                    Move::Left
                }
            }
            MovementAlgorithm::DUR => {
                if first {
                    Move::Up
                } else {
                    Move::Right
                }
            }
        }
    }
    pub fn rotate_algo(&mut self, current_mv: &Move) {
        if current_mv.is_horizontal() {
            match self {
                // first move for sure
                Self::BRD => *self = Self::BLD,
                Self::BLD => *self = Self::BRD,
                Self::BRU => *self = Self::BLU,
                Self::BLU => *self = Self::BRU,
                // second for sure
                Self::DDL => *self = Self::DDR,
                Self::DDR => *self = Self::DDL,
                Self::DUL => *self = Self::DUR,
                Self::DUR => *self = Self::DUL,
            }
        } else {
            match self {
                // first move for sure
                Self::DDL => *self = Self::DUL,
                Self::DDR => *self = Self::DUR,
                Self::DUL => *self = Self::DDL,
                Self::DUR => *self = Self::DDR,
                // second move for sure
                Self::BRD => *self = Self::BRU,
                Self::BLD => *self = Self::BLU,
                Self::BRU => *self = Self::BRD,
                Self::BLU => *self = Self::BLD,
            }
        }
    }
}

pub struct Note {
    pub id: u32,
    pub num_repairs: u32,
}

impl Note {
    pub fn parse(raw_string: &String) -> Self {
        let parts: Vec<&str> = raw_string.split_whitespace().collect();
        // Handle invalid string format,
        if parts.len() != 5 || parts[1] != "repaired" || parts[3] != "times" {
            panic!("unexpected format of string detected as a note !!") // Almost impossible panic
        }
        let id = u32::from_str(parts[0])
            .ok()
            .expect("couldn't parse the repairer id from note !");
        let num_repairs =
            u32::from_str(parts[2]).expect("couldn't parse the repair times from note !");

        Self { id, num_repairs }
    }

    pub fn to_string(&self) -> String {
        format!("{} repaired {} times", self.id, self.num_repairs)
    }
}
