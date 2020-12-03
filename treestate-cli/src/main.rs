use std::fs::File;
use std::path::{Path, PathBuf};
use treestate::{FileState, TreeState};

use walkdir::WalkDir;

use anyhow::{Context as _, Error, Result};
use clap::{crate_version, App, AppSettings, Arg, SubCommand};

fn main() {
    let result = try_main();
    match result {
        Err(e) => {
            eprintln!("treestate: error: {:#}", e);
            std::process::exit(1);
        }
        Ok(code) => std::process::exit(code),
    };
}

fn try_main() -> Result<i32> {
    let matches = App::new("treestate")
        .version(crate_version!())
        .author("Kaspar Schleiser <kaspar@schleiser.de>")
        .about("watch folder for changes since last checkpoint")
        .setting(AppSettings::InferSubcommands)
        .arg(
            Arg::with_name("chdir")
                .short("C")
                .long("chdir")
                .help("change working directory before doing anything else")
                .global(true)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("statefile")
                .short("f")
                .long("state-file")
                .takes_value(true)
                .value_name("FILE")
                .default_value("./.treestate")
                .help("specify file to store state in"),
        )
        .subcommand(SubCommand::with_name("store").about("record current tree state"))
        .subcommand(SubCommand::with_name("status").about("check current tree state"))
        .get_matches();

    let statefile = matches.value_of("statefile").unwrap();

    match matches.subcommand() {
        ("store", Some(_matches)) => {
            let walker = WalkDir::new(".")
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_type().is_dir())
                .map(|e| e.into_path())
                .collect::<Vec<_>>();
            let mut treestate: TreeState<FileState, PathBuf> = TreeState::new(walker.iter());
            treestate.ignore(&PathBuf::from(statefile));
            let file = File::create(&statefile)?;
            treestate.dump(file)?;
        }
        ("status", Some(_matches)) => {
            let file = File::open(&statefile)?;
            let mut treestate = TreeState::<FileState, PathBuf>::load(file)?;
            treestate.ignore(&PathBuf::from(statefile));
            std::process::exit(treestate.has_changed() as i32);
        }
        _ => {
            println!("treestate: no arguments given. try \"treestate-cli --help\"");
        }
    }
    Ok(0)
}
