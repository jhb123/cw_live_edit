use std::collections::HashMap;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    Across,
    Down
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Crossword {
    across: HashMap<String, Clue>,
    down: HashMap<String, Clue>
}

impl Crossword {

    pub fn demo_grid() -> Self {

        let clue_1 = Clue::new(8, (0,0), "For all the money that e'er I had", Direction::Across);
        let clue_2 = Clue::new(8, (0,4), "I spent it in good company", Direction::Across);

        let clue_3 = Clue::new(8, (0,0), "And for all the harm that ever I've done", Direction::Down);
        let clue_4 = Clue::new(8, (4,0), "I've done to none but me.", Direction::Down);

        let across = HashMap::from([
            ("1a".to_string(), clue_1),
            ("3a".to_string(), clue_2)
            ]);

        let down = HashMap::from([
            ("1d".to_string(), clue_3),
            ("2d".to_string(), clue_4)
            ]);

        Self{across, down}

    }

}

#[derive(Serialize, Deserialize, Debug)]
pub struct Clue {
    hint: String,
    cells: Vec<(usize,usize)>
}

impl Clue {
    fn new(len:usize, start: (usize,usize), hint: &str, direction: Direction) -> Self {
        let mut cells = vec![(0,0); len];

        match direction {
            Direction::Across => {
                for i in 0..len {
                    cells[i] = (start.0 + i, start.1)
                };
            },
            Direction::Down => {
                for i in 0..len {
                    cells[i] = (start.0, start.1+i)
                };
            }
        }

        Self{ hint: hint.to_string(),  cells }

    }
}