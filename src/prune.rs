use clap::{Args, ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Restore a puzzle.
    Restore(SingleArgs),
    /// Permenantly delete a puzzle.
    Delete(SingleArgs),
    /// Restore all puzzles that have been soft deleted.
    BatchRestore(BatchArgs),
    /// Permenantly delete all puzzles that have been soft deleted.
    BatchDelete(BatchArgs),
}

#[derive(Args)]
struct BatchArgs {
    #[arg(short, long, action=ArgAction::SetTrue)]
    /// Perform the operation. (default behaviour is a dry run)
    live: bool,
}

#[derive(Args)]
struct SingleArgs {
    #[arg(short, long, action=ArgAction::SetTrue)]
    /// Perform the operation. (default behaviour is a dry run)  
    live: bool,
    #[arg(short, long)]
    /// The ID of the puzzle the operation will be performed on.
    id: i64
}

fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::BatchRestore(args)=> batch_restore(args),
        Commands::BatchDelete(args) => batch_delete(args),
        Commands::Restore(args) => restore(args),
        Commands::Delete(args) => delete(args),
    }
}

fn batch_restore(args: &BatchArgs) {
    match args.live {
        true => println!("Starting restoration"),
        false => println!("This is a dry run"),
    }
    let puzzles = cw_grid_server::db::get_soft_delete_puzzles().unwrap();
    puzzles.iter().for_each(|el| {
        println!("Restoring {:?}",el);
    });
    if args.live {
        cw_grid_server::db::batch_restore().unwrap();
    }
    match args.live {
        true => println!("Completed restoration"),
        false => println!("Finished dry run"),
    }
}

fn restore(args: &SingleArgs) {
    match args.live {
        true => println!("Starting restoration"),
        false => println!("This is a dry run"),
    }
    let puzzle = match find_puzzle(args) {
        Some(value) => value,
        None => return,
    };
    
    println!("Restoring {:?}",puzzle);
    if args.live {
        cw_grid_server::db::restore_puzzle(&args.id).unwrap();
    }
    match args.live {
        true => println!("Completed restoration"),
        false => println!("Finished dry run"),
    }
}

fn batch_delete(args: &BatchArgs) {
    match args.live {
        true => println!("Starting batch delete"),
        false => println!("This is a dry run"),
    }
    let puzzles = cw_grid_server::db::get_soft_delete_puzzles().unwrap();

    puzzles.iter().for_each(|el| {
        println!("Deleting {:?}",el);
    });
    if args.live {
        cw_grid_server::db::batch_delete().unwrap();
    }
    match args.live {
        true => println!("Completed batch delete"),
        false => println!("Finished dry run"),
    }
}

fn delete(args: &SingleArgs) {
    match args.live {
        true => println!("Starting deletion"),
        false => println!("This is a dry run"),
    }
    let puzzle = match find_puzzle(args) {
        Some(value) => value,
        None => return,
    };

    println!("Deleting {:?}",puzzle);
    if args.live {
        cw_grid_server::db::delete_puzzle(&args.id).unwrap();
    }
    match args.live {
        true => println!("Completed deletion"),
        false => println!("Finished dry run"),
    }
}

fn find_puzzle(args: &SingleArgs) -> Option<cw_grid_server::db::PuzzleDbData> {
    let puzzle = match cw_grid_server::db::get_puzzle_db(&args.id){
        Ok(puzzles) =>  puzzles,
        Err(rusqlite::Error::QueryReturnedNoRows) =>{
            println!("No puzzle with id {}", args.id);
            return None
        },
        Err(e) => {
            eprintln!("{e}");
            return None 
        } 
    };
    Some(puzzle)
}