use crossbeam_channel as channel;
use std::fs::File;
use std::path::PathBuf;
use std::thread;
use treestate::{FileState, State, TreeState};

use ignore::{DirEntry, WalkBuilder};

use anyhow::Result;
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
            let (tx, rx) = channel::unbounded::<(PathBuf, FileState)>();

            let collect_thread = thread::spawn(move || {
                let res: TreeState<FileState, PathBuf> = TreeState::from(rx.iter());
                res
            });

            let walker = WalkBuilder::new(".").build_parallel();

            walker.run(|| {
                let tx = tx.clone();
                Box::new(move |result| {
                    use ignore::WalkState::*;

                    if let Ok(result) = result {
                        if result.file_type().unwrap().is_file() {
                            let path_buf = result.into_path();
                            let state = State::from(&path_buf).unwrap();
                            tx.send((path_buf, state)).unwrap();
                        }
                    }
                    Continue
                })
            });

            drop(tx);

            let mut treestate = collect_thread.join().unwrap();
            treestate.ignore(&PathBuf::from(statefile));
            let mut file = File::create(&statefile)?;
            treestate.dump(&mut file)?;
        }
        ("status", Some(_matches)) => {
            use std::time::Instant;
            let start = Instant::now();
            let file = std::fs::read(&statefile)?;
            let mut treestate = TreeState::<FileState, PathBuf>::load_vec(&file)?;
            println!("{:?}", start.elapsed());
            treestate.ignore(&PathBuf::from(statefile));
            std::process::exit(treestate.has_changed() as i32);
        }
        _ => {
            println!("treestate: no arguments given. try \"treestate-cli --help\"");
        }
    }
    Ok(0)
}
