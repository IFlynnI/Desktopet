//! `pet-pack validate <pack folder>` checks a Pack the way the app does, and prints every error, one per line. It exits
//! with 1 when there are any, and with 2 when it wasn't asked properly.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: pet-pack validate <pack folder>";

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(command), Some(folder), None) = (args.next(), args.next(), args.next()) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if command != "validate" {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    let folder = PathBuf::from(folder);
    match pet_content::load(&folder) {
        Ok(pack) => {
            let name = &pack.definition.name;
            println!("{}: \"{name}\" is a good Pack", folder.display());
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in &errors {
                println!("{error}");
            }
            eprintln!("{} errors in {}", errors.len(), folder.display());
            ExitCode::FAILURE
        }
    }
}
