use std::{env, fs::{self, DirEntry, File}, io::{Error, Read, Write}, path::Path};
use log::{error, info};
use lazy_static::lazy_static;
use rusqlite::{named_params, Connection};
use serde::Serialize;

use crate::crossword::{self, Crossword};

lazy_static! {
    static ref PUZZLE_DIR_PATH: String = env::var("PUZZLE_PATH").unwrap_or("./puzzles".to_string());
    static ref PUZZLE_DB_PATH: String = {
        let puzzle_dir_path = &*PUZZLE_DIR_PATH;
        format!("{}/puzzle.db", puzzle_dir_path)
    };
}


pub fn create_puzzle_dir() -> Result<(),Error> {
    fs::create_dir_all(&*PUZZLE_DIR_PATH)?;
    Ok(())
}

pub fn init_db() -> Result<(), rusqlite::Error> {

    let conn = Connection::open(&*PUZZLE_DB_PATH).unwrap();

    info!("initialising database");
    conn.execute(
        "create table if not exists puzzles (
             id integer primary key,
             name text not null unique,
             file text not null unique
         )",()
    )?;
    Ok(())
}

pub fn add_puzzle_to_db(name: &str, file: &str) -> Result<(), rusqlite::Error> {
    info!("inserting data");
    let conn = Connection::open(&*PUZZLE_DB_PATH).unwrap();

    let mut stmt = conn.prepare(
        "insert into puzzles (name, file) values (:name, :file)"
    )?;
    stmt.execute(named_params! { ":name": name, ":file": file})?;

    Ok(())
}

#[derive(Debug,Serialize)]
pub struct PuzzleDbData {
    id: usize,
    name: String,
    file: String
}

pub fn get_all_puzzle_db() -> Result<Vec<PuzzleDbData>, rusqlite::Error> {
    info!("all data:");
    let conn = Connection::open(&*PUZZLE_DB_PATH).unwrap();

    let mut stmt = conn.prepare(
        "select id, name, file from puzzles"
    )?;
    
    let db_data_iter = stmt.query_map([], |row| {
        Ok(PuzzleDbData {
            id: row.get(0)?,
            name: row.get(1)?,
            file: row.get(2)?,
        })
    })?;

    let data: Vec<PuzzleDbData> = db_data_iter.map(|x| x.unwrap()).collect();
    Ok(data)
}

pub fn get_puzzle_db(id: &str) -> Result<PuzzleDbData, rusqlite::Error> {
    info!("all data:");
    let conn = Connection::open(&*PUZZLE_DB_PATH).unwrap();

    let mut stmt = conn.prepare(
        "select id, name, file from puzzles where id=:id"
    )?;
    
    let db_data_iter = stmt.query_map(&[(":id", id)], |row| {
        Ok(PuzzleDbData {
            id: row.get(0)?,
            name: row.get(1)?,
            file: row.get(2)?,
        })
    })?;

    let data: PuzzleDbData = db_data_iter.map(|x| x.unwrap()).next().unwrap();
    Ok(data)
}


pub fn get_puzzle(id: &str) -> Result<Crossword, Error> {

    let puzzle_dir = Path::new(&*PUZZLE_DIR_PATH);

    let data = get_puzzle_db(id).unwrap();

    let puzzle_path = puzzle_dir.join(format!("{}.json",data.name));

    let mut file = File::open(puzzle_path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    let crossword: Crossword = serde_json::from_str(&data)?;
    Ok(crossword)
}


pub fn save_puzzle(id: &str, cw: &Crossword) -> Result<(), Error> {
    let data = serde_json::to_string(cw)?;

    let puzzle_dir = Path::new(&*PUZZLE_DIR_PATH);

    let puzzle_path = puzzle_dir.join(format!("{id}.json") );
    
    let mut file = File::options().write(true).open(&puzzle_path).unwrap_or_else(|_| {
        match File::create(&puzzle_path) {
            Ok(file) => file,
            Err(e) => panic!("Problem creating file {}", e)
        }
    });
    
    file.set_len(0)?;
    info!("writing crossword to {:?}", file);

    File::write_all(&mut file, data.as_bytes())?;

    Ok(())
}

pub fn create_new_puzzle(id: &str, cw: &Crossword) -> Result<(), Error> {
    let data = serde_json::to_string(cw)?;

    let puzzle_dir = Path::new(&*PUZZLE_DIR_PATH);

    let puzzle_path = puzzle_dir.join(format!("{id}.json") );
    
    let mut file = File::options().write(true).open(&puzzle_path).unwrap_or_else(|_| {
        match File::create(&puzzle_path) {
            Ok(file) => file,
            Err(e) => panic!("Problem creating file {}", e)
        }
    });
    
    file.set_len(0)?;
    info!("writing crossword to {:?}", file);

    File::write_all(&mut file, data.as_bytes())?;

    add_puzzle_to_db(id, puzzle_path.to_str().unwrap()).unwrap();

    save_puzzle(id, &cw).unwrap();
    Ok(())
}