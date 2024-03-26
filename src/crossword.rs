use std::collections::{HashSet,HashMap};

use log::info;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    Across,
    Down
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Crossword {
    across: HashMap<String, Clue>,
    down: HashMap<String, Clue>,
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

    pub fn update_cell(&mut self, incoming_cell: Cell) {
        self.across.iter_mut().for_each(|(_, clue)| {
            clue.cells.iter_mut()
            .find( |cell| cell.x == incoming_cell.x && cell.y == incoming_cell.y)
            .and_then(|cell | {
                info!("updating across clue cell");
                Some(cell.c = incoming_cell.c)
            });
        });
        self.down.iter_mut().for_each(|(_, clue)| {
            clue.cells.iter_mut()
            .find( |cell| cell.x == incoming_cell.x && cell.y == incoming_cell.y)
            .and_then(|cell | {
                info!("updating down clue cell");
                Some(cell.c = incoming_cell.c)
            });
        });
    }

}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cell {
    x: usize,
    y: usize,
    #[serde(deserialize_with = "char_deserialise")]
    c: char,
}

fn char_deserialise<'de, D>(deserializer: D) -> Result<char, D::Error> where D: Deserializer<'de> {
    let c_str: String = Deserialize::deserialize(deserializer)?;
    let c = c_str.chars().next().unwrap_or(' ');
    Ok(c)
}

impl Cell {
    fn default() -> Self {
        Self { x: 0, y: 0, c: ' ' }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Clue {
    hint: String,
    cells: Vec<Cell>
}

impl Clue {
    fn new(len:usize, start: (usize,usize), hint: &str, direction: Direction) -> Self {
        let mut cells = vec![Cell::default(); len];

        match direction {
            Direction::Across => {
                for i in 0..len {
                    // cells[i] = (start.0 + i, start.1)
                    cells[i].x = start.0 + i;
                    cells[i].y = start.1;

                };
            },
            Direction::Down => {
                for i in 0..len {
                    cells[i].x = start.0;
                    cells[i].y = start.1 + i;
                };
            }
        }

        Self{ hint: hint.to_string(),  cells }

    }
}