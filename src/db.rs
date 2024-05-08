use std::{env, fs::{self, File}, io::{Error, ErrorKind, Read, Write}, path::Path};
use log::{error, info, warn};
use lazy_static::lazy_static;
use rusqlite::{named_params, Connection};
use serde::Serialize;
use sha256::digest;

use crate::crossword::Crossword;

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

    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    info!("initialising database");
    conn.execute(
        "create table if not exists puzzles (
             id integer primary key,
             name text not null unique,
             file text not null unique
         )",()
    )?;

    conn.execute(
        "create table if not exists users (
             id integer primary key,
             username text not null unique,
             password text not null,
             session integer
         )",()
    )?;

    Ok(())
}

pub fn add_puzzle_to_db(name: &str, file: &str) -> Result<(), rusqlite::Error> {
    info!("inserting puzzle data");
    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    let mut stmt = conn.prepare(
        "insert into puzzles (name, file) values (:name, :file)"
    )?;
    stmt.execute(named_params! { ":name": name, ":file": file})?;

    Ok(())
}

pub fn add_user(username: &str, password: &str) -> Result<i64, rusqlite::Error> {
    info!("inserting data");
    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    let hash = digest(password);

    let mut stmt = conn.prepare(
        "insert into users (username, password) values (:username, :password)"
    )?;
    stmt.execute(named_params! { ":username": username, ":password": hash})?;

    Ok(conn.last_insert_rowid())
}

pub struct SignIn {
    pub id: i64,
    pub password: String
}

pub fn get_user_password(username: &str) -> Result<SignIn, rusqlite::Error> {
    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    let mut stmt = conn.prepare(
        "select id, password from users where username=(:username)"
    )?;
    
    let rows: Result<SignIn, rusqlite::Error> = stmt.query_row(&[(":username", username)], |row| {
        let id = row.get(0)?;
        let password = row.get(1)?;
        Ok(SignIn { id, password})
    });
    rows
}

pub fn set_session(user_id: i64) -> Result<i64, rusqlite::Error> {
    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    let session: i64 = rand::random();

    let mut stmt = conn.prepare(
        "update users set session=(:session) where id=(:user_id)"
    )?;

    stmt.execute(named_params! { ":session": session, ":user_id": user_id})?;

    Ok(session)
}

pub fn check_session(id: i64, session: i64) -> Result<bool, rusqlite::Error> {
    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    let mut stmt = conn.prepare(
        "select session from users where id=(:user_id)"
    )?;

    let session_db: i64 = stmt.query_row(&[(":user_id", &id.to_string())], |row| {
        row.get(0)
    })?;

    Ok(session_db == session)
}

pub fn validate_password(plain: &str, hashed: &str) -> Result<(), () > {
    let hash = digest(plain);
    if hashed == hash {Ok(())} else { Err(()) }
}


#[derive(Debug,Serialize)]
pub struct PuzzleDbData {
    id: usize,
    pub name: String,
    file: String
}

impl PuzzleDbData {

    fn from_row(row: &rusqlite::Row<'_>) ->Result<PuzzleDbData, rusqlite::Error> {
        let id = match row.get(0){
            Ok(val) => val,
            Err(rusqlite::Error::InvalidColumnIndex(idx)) => {
                error!("While trying to parse a row from the database into a PuzzleDbData struct, we attempted to find 'ID' at Column index {0}, but {0} is an invalid Column Index", idx);
                return Err(rusqlite::Error::InvalidColumnIndex(idx))
            },
            Err(err) => {
                error!("{0}",err);
                return Err(err)
            }
        };
        let name = match row.get(1){
            Ok(val) => val,
            Err(rusqlite::Error::InvalidColumnIndex(idx)) => {
                error!("While trying to parse a row from the database into a PuzzleDbData struct, we attempted to find 'name' at Column index {0}, but {0} is an invalid Column Index", idx);
                return Err(rusqlite::Error::InvalidColumnIndex(idx))
            },
            Err(err) => {
                error!("{0}",err);
                return Err(err)
            }
        };
        let file = match row.get(2){
            Ok(val) => val,
            Err(rusqlite::Error::InvalidColumnIndex(idx)) => {
                error!("While trying to parse a row from the database into a PuzzleDbData struct, we attempted to find 'file' at Column index {0}, but {0} is an invalid Column Index", idx);
                return Err(rusqlite::Error::InvalidColumnIndex(idx))
            },
            Err(err) => {
                error!("{0}",err);
                return Err(err)
            }
        };
        Ok(PuzzleDbData {id, name, file})
    }
}

pub fn get_all_puzzle_db() -> Result<Vec<PuzzleDbData>, rusqlite::Error> {
    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    let mut stmt = conn.prepare(
        "select id, name, file from puzzles"
    )?;
    
    let rows = stmt.query_map([], |row| {
        PuzzleDbData::from_row(row)
    })?
    .collect();

   rows
}

pub fn get_puzzle_db(id: &str) -> Result<PuzzleDbData, rusqlite::Error> {
    let conn = Connection::open(&*PUZZLE_DB_PATH)?;

    let mut stmt = conn.prepare(
        "select id, name, file from puzzles where id=:id"
    )?;
    
    let rows = stmt.query_row(&[(":id", id)], |row| {
        PuzzleDbData::from_row(row)
    });

    rows
    
}


pub fn get_puzzle(id: &str) -> Result<Option<Crossword>, Error> {

    let puzzle_dir = Path::new(&*PUZZLE_DIR_PATH);

    match get_puzzle_db(id) {
        Ok(data) => {
            let puzzle_path = puzzle_dir.join(format!("{}.json",data.name));
            let mut file = File::open(puzzle_path)?;
            let mut data = String::new();
            file.read_to_string(&mut data)?;
            let crossword: Crossword = serde_json::from_str(&data)?;
            Ok(Some(crossword))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            warn!("No crossword with ID {id} exists.");
            Ok(None)
        },
        Err(e) => {
            Err(Error::new(ErrorKind::Other, format!("Database error: {}", e)))
        }
    }
}


pub fn save_puzzle(id: &str, cw: &Crossword) -> Result<(), Error> {

    let puzzle_dir = Path::new(&*PUZZLE_DIR_PATH);

    match get_puzzle_db(id) {
        Ok(puzzle_meta_data) => {
            let puzzle_path = puzzle_dir.join(format!("{}.json",puzzle_meta_data.name));
                
            let mut file = File::options().write(true).open(&puzzle_path).unwrap_or_else(|_| {
                match File::create(&puzzle_path) {
                    Ok(file) => file,
                    Err(e) => panic!("Problem creating file {}", e)
                }
            });
            file.set_len(0)?;
            info!("writing crossword to {:?}", file);
            let cw_data = serde_json::to_string(cw)?;
            File::write_all(&mut file, cw_data.as_bytes())?;
            Ok(())
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            warn!("No crossword with ID {id} exists.");
            Err(Error::new(ErrorKind::NotFound, format!("No crossword with ID {id} exists.")))
        },
        Err(e) => {
            Err(Error::new(ErrorKind::Other, format!("Database error: {}", e)))
        }
    }

}

pub fn create_new_puzzle(id: &str, cw: &Crossword) -> Result<(), Error> {

    let data = serde_json::to_string(cw)?;

    let puzzle_dir = Path::new(&*PUZZLE_DIR_PATH);

    let puzzle_path = puzzle_dir.join(format!("{id}.json") );
    
    let puzzle_path_str = puzzle_path.to_str().ok_or_else(
        || Error::new(ErrorKind::Other, format!("Path must be valid utf-8"))
    )?;

    let mut file = match File::options().write(true).create(true).open(&puzzle_path) {
        Ok(file) => {
            file
        },
        Err(ref e) if e.kind() == ErrorKind::PermissionDenied => {
            return Err(Error::new(ErrorKind::PermissionDenied, format!("Cannot save puzzle data to file. Ensure you have permission to create files.")))
        }
        Err(e) => {
            return Err(e)
        }
    };
    
    file.set_len(0)?;
    info!("writing crossword to {:?}", file);
    File::write_all(&mut file, data.as_bytes())?;

    add_puzzle_to_db(id, puzzle_path_str).map_err(|e| Error::new(ErrorKind::Other, format!("Database error: {}", e)))?;

    Ok(())
}