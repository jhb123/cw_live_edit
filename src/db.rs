use std::{env, fs::{self, DirEntry, File, FileType}, io::Error, path::Path};



pub fn init_db() -> Result<(),Error> {
    let path = env::var("PUZZLE_PATH").unwrap_or("./puzzles".to_string());
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn get_all_puzzles() -> Result<Vec<DirEntry>,Error>  {
    let path = env::var("PUZZLE_PATH").unwrap_or("./puzzles".to_string());

    let puzzles = fs::read_dir(path)?
        .filter(|res| {
            res.as_ref().is_ok_and(|x|{
                x.path()
                .extension()
                .is_some_and(|y| {
                    y.eq("json")
                })
            })
                
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(puzzles)

}

pub fn get_puzzle(id: &str) -> Result<File, Error> {
    let p = env::var("PUZZLE_PATH").unwrap_or("./puzzles".to_string());

    let path = Path::new(&p);

    let puzzle_path = path.join(id);

    let file = File::open(puzzle_path)?;
    Ok(file)
}