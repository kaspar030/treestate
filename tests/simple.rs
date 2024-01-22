use std::fs::File;
use std::path::PathBuf;
use treestate::{FileState, TreeState};

#[test]
fn basic() {
    let paths = vec![PathBuf::from("target/foo"), PathBuf::from("target/bar")];
    for path in &paths {
        File::create(path).unwrap();
    }
    let treestate: TreeState<FileState, PathBuf> = TreeState::new(paths.iter());
    println!("{:#?}", treestate);
    assert!(!treestate.has_changed());

    let (accessed, mut modified) = utime::get_file_times(&paths[0]).unwrap();
    modified += 1;
    utime::set_file_times(&paths[0], accessed, modified).unwrap();
    println!("{:#?}", treestate);

    assert!(treestate.has_changed());
}
