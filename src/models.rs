use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU8},
        mpsc::{self, Receiver, Sender},
        Arc, Barrier, Mutex,
    },
    thread::{self, JoinHandle},
};

use crate::{execute, make_decision};

pub type JobTypeSender = Arc<Mutex<Sender<JobType>>>;
pub type JobTypeReceiver = Arc<Mutex<Receiver<JobType>>>;

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
#[derive(Debug)]
pub enum JobType {
    DecisionMaking(
        Arc<Vec<Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)>>>,
        Arc<Barrier>,
    ),
    Execute(
        Arc<Vec<Vec<(Vec<Arc<Mutex<String>>>, AtomicU8)>>>, // the matrix
        Arc<Vec<AtomicBool>>,                               // the explore end check
        Arc<Barrier>,                                       // the beginning barrier
        Arc<Barrier>,                                       // the ending barrier
    ),
    // DecisionMade,
    // Executed,
    // End(RepairerResult),
}
#[derive(Debug)]
pub struct Repairer {
    pub id: u32,                                    // not going to be changed
    pub thread: Option<JoinHandle<()>>,             // not going to be changed
    pub total_broken: u32,                          // not going to be changed
    pub total_fixed: u32,                           // ▶️ will be changed in the execute
    pub other_repairers_repairs: HashMap<u32, u32>, // ⏸️  ▶️ will be changed in the execute and decision making
    pub total_moves: u32,                           // ▶️ will be changed in the execute
    // pub receiver: Arc<Mutex<Receiver<Command>>>,// the spawned thread will only need that so we do not save this value in the thread state
    pub current_algorithm: MovementAlgorithm, // ⏸️ change in decision making
    pub current_location: (u32, u32),         // ▶️ chang in execute
    pub matrix_size: u32,                     // not going to be changed
    pub decision: Move,                       // ⏸️  ▶️ change in decision making and in execute
    pub move_turn: bool,                      // ▶️ change in execute
    pub last_move_rotated: bool,
    pub last_move: Move,
}

impl Repairer {
    pub fn get_total_fixes_from_notes(&self) -> u32 {
        let mut tmp_total_fix = 0;
        for v in self.other_repairers_repairs.values() {
            tmp_total_fix += v;
        }
        tmp_total_fix
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Move {
    Up,
    Down,
    Right,
    Left,
    Fix,
    None,  // this means the end of the explore and there is no more move available
    Empty, // the actual None value, no moves for now
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
            Self::Right => (index.0, index.1 + 1),
            Self::Left => (index.0, index.1 - 1),
            Self::Up => (index.0 - 1, index.1),
            Self::Down => (index.0 + 1, index.1),
            _ => panic!("incorrect move to be applied !!"), // impossible
        }
    }
}
#[derive(Clone, Debug)]
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
                Self::BRD => *self = Self::BLU,
                Self::BLD => *self = Self::BRU,
                Self::BRU => *self = Self::BLD,
                Self::BLU => *self = Self::BRD,
                // second for sure
                Self::DDL => *self = Self::DUR,
                Self::DDR => *self = Self::DUL,
                Self::DUL => *self = Self::DDR,
                Self::DUR => *self = Self::DDL,
            }
        } else {
            match self {
                // first move for sure
                Self::DDL => *self = Self::DUR,
                Self::DDR => *self = Self::DUL,
                Self::DUL => *self = Self::DDR,
                Self::DUR => *self = Self::DDL,
                // second move for sure
                Self::BRD => *self = Self::BLU,
                Self::BLD => *self = Self::BRU,
                Self::BRU => *self = Self::BLD,
                Self::BLU => *self = Self::BRD,
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
        if parts.len() != 4 || parts[1] != "repaired" || parts[3] != "times" {
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
