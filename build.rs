use std::fs;
use std::process::{Command};
use std::path::{Path, PathBuf};
use std::io::Write;

static DEBUG: bool = true;

fn main() {
    let mut log_file = open_log();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    
    let dir_iter = std::fs::read_dir(manifest_dir);
    if dir_iter.is_err() {
        println!("failed to read manifest dir: {}", dir_iter.err().unwrap());
        return;
    }
    
    // Create a directory to store the changelog files
    let changelog_path = Path::new(manifest_dir).join(PathBuf::from("changelog"));
    // Will error if the directory already exists, but that's okay; we'll just ignore it.
    let created = std::fs::create_dir(changelog_path.clone());
    if !created.is_err() {
        // Initialize the git repo
        let init = Command::new("git")
                                    .args(["init"])
                                    .current_dir(changelog_path.clone())
                                    .output()
                                    .expect("failed to execute git init");
    }
    
    copy_files_to_changelog(&mut log_file, dir_iter.unwrap(), &changelog_path);

    commit_to_git(&mut log_file, &changelog_path);

    // just logging
    let err = fs::write("/tmp/foo.txt", manifest_dir);
    match err {
        Ok(_) => println!("ok"),
        Err(e) => println!("err: {}", e),
    }
}

fn copy_files_to_changelog(log_file: &mut Option<std::fs::File>, dir_iter: std::fs::ReadDir, changelog_path: &PathBuf) {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"));


    for (_i, entry) in dir_iter.enumerate() {
        if entry.is_ok() {
            let dir_entry = entry.unwrap();
            if dir_entry.path().is_dir() { // this is a directory
                if !dir_entry.path().ends_with(".git") {
                    let iterator = std::fs::read_dir(dir_entry.path());
                    copy_files_to_changelog(log_file, iterator.unwrap(), changelog_path);
                }
            }
            else { // this is just a file
                let path = dir_entry.path();
                let pruned_path = path.strip_prefix(manifest_path);
                // log(log_file, pruned_path.clone().unwrap().to_str().unwrap());
                let is_ignore = Command::new("git")
                                                    .args(["check-ignore", "-q", pruned_path.unwrap().to_str().unwrap()])
                                                    .current_dir(manifest_path)
                                                    .output()
                                                    .expect("failed to execute git");


                let ignored = is_ignore.status.success();
                if  !ignored { // file is not in .gitignore
                    log(log_file, path.to_str().unwrap());

                    let stripped_prefix = path.strip_prefix(manifest_path);
                    if stripped_prefix.is_ok() {
                        let dest_path = changelog_path.join(stripped_prefix.unwrap());
                        //log(log_file, dest_path.to_str().unwrap());
                        let copy_result = std::fs::copy(&path, dest_path);
                        if copy_result.is_err() {
                            println!("failed to copy file: {}", copy_result.as_ref().err().unwrap());
                            let err_text = copy_result.err().unwrap().to_string();
                            log(log_file, &err_text);
                        }
                    }
                }
            }                                
        }
    }
} 

fn commit_to_git(log_file: &mut Option<std::fs::File>, changelog_path: &PathBuf) {
    let add = Command::new("git")
                                .args(["add", "*"])
                                .current_dir(changelog_path.clone())
                                .output()
                                .expect("failed to execute git add");
    if !add.status.success() {
        log(log_file, "failed to add files to git");
    }

    let commit = Command::new("git")
                                 .args(["commit", "-a", "-m", "changelog update"])
                                 .current_dir(changelog_path.clone())
                                 .output()
                                 .expect("failed to execute git add");
    if !commit.status.success() {
        log(log_file, "failed to commit files to git");
    }
}

fn open_log() -> Option<std::fs::File> {
    if DEBUG {
        Some(fs::File::create("/tmp/log.txt").expect("Couldn't open log file for writing"))
    }
    else {
        None
    }
    
}

fn log(log_file: &mut Option<std::fs::File>, msg: &str) {
    if log_file.is_some() {
        let err = log_file.as_mut().unwrap().write_all(msg.as_bytes());
        match err {
            Ok(_) => println!("ok"),
            Err(e) => println!("err: {}", e),
        }

        let err = log_file.as_mut().unwrap().write_all("\n".as_bytes());
        match err {
            Ok(_) => println!("ok"),
            Err(e) => println!("err: {}", e),
        }
    }
}