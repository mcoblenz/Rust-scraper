use std::fs;
use std::process::{Command};
use std::path::{Path, PathBuf};
use std::io::{Read, Write};

static DEBUG: bool = true;

fn main() {
    let mut log_file = open_log();
    if log_file.is_none() {
        panic!("failed to open log file");
    };
    log(&mut log_file, "opened log...");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    
    let dir_iter = std::fs::read_dir(manifest_dir);
    if dir_iter.is_err() {
        println!("failed to read manifest dir: {}", dir_iter.err().unwrap());
        return;
    }
    
    log(&mut log_file, "creating directory...");
    // Create a directory to store the changelog files
    let changelog_path = Path::new(manifest_dir).join(PathBuf::from("changelog"));
    // Will error if the directory already exists, but that's okay; we'll just ignore it.
    let created = std::fs::create_dir(changelog_path.clone());
    if !created.is_err() {
        // Initialize the git repo
       Command::new("git")
                .args(["init"])
                .current_dir(changelog_path.clone())
                .output()
                .expect("failed to execute git init");

        let config = read_config(&mut log_file);
        if config.is_none() {
            let _ = std::fs::remove_dir(changelog_path.clone());
            panic!("failed to read config");
        }
 
        let pid = &config.as_ref().unwrap().participant_id.to_owned();
        let project: &String = &config.as_ref().unwrap().project.to_owned();
        let pwd = &config.unwrap().git_password.to_owned();

        log(&mut log_file, "project: ");
        log(&mut log_file, project);

        if project.len() == 0 {
            let _ = std::fs::remove_dir(changelog_path.clone());
            panic!("Project not specified in config.txt");
        } 

        let repo = "https://".to_owned() + &pid + ":" + pwd + "@git.goto.ucsd.edu/" + &pid + "/" + project + ".git";
 
        Command::new("git")
                .args(["remote", "add", "origin", &repo])
                .current_dir(changelog_path.clone())
                .output()
                .expect("failed to execute git remote add");
 
    }
    
    log(&mut log_file, "copying files...");
    copy_files_to_changelog(&mut log_file, dir_iter.unwrap(), &changelog_path);

    log(&mut log_file, "committing to git...");
    commit_to_git(&mut log_file, &changelog_path);

    log(&mut log_file, "pushing...");
    git_push(&mut log_file, &changelog_path);
}

fn copy_files_to_changelog(log_file: &mut Option<std::fs::File>, dir_iter: std::fs::ReadDir, changelog_path: &PathBuf) {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    for (_i, entry) in dir_iter.enumerate() {
        if entry.is_ok() {

            let dir_entry = entry.unwrap();

            if !dir_entry.path().ends_with(".git") && !dir_entry.path().ends_with("changelog") {
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
                    if stripped_prefix.is_err() {
                        continue;
                    }

                    let dest_path = changelog_path.join(stripped_prefix.unwrap());
                    
                    if dir_entry.path().is_dir() { // this is a directory
                        let inner_iterator = std::fs::read_dir(dir_entry.path());
                        // maybe create a directory
                        let creation_err = std::fs::DirBuilder::new().create(dest_path);       
                        if creation_err.is_err() {
                            // do nothing; errors are expected for dirs that aren't new
                        }                 

                        copy_files_to_changelog(log_file, inner_iterator.unwrap(), changelog_path);
                    }
                    else { // this is a file
                        //log(log_file, dest_path.to_str().unwrap());
                        let copy_result = std::fs::copy(&path, dest_path);
                        if copy_result.is_err() {
                            log(log_file, "failed to copy file: ");
                            log(log_file, &copy_result.as_ref().err().unwrap().to_string());
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
        log(log_file, &commit.status.to_string());
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
        let _ = log_file.as_mut().unwrap().sync_all();        
    }
    else {
        panic!("failed to write log");
    }
}

struct Config {
    participant_id: String,
    git_password: String,
    project: String,
}

fn read_config(log_file: &mut Option<std::fs::File>) -> Option<Config> {
    // read config.txt
    let config_file = fs::File::open("config.txt");
    if config_file.is_err() {
        log(log_file, "failed to open config.txt");
        return None;
    }

    let mut contents = String::new();
    let read_result = config_file.unwrap().read_to_string(&mut contents);

    if read_result.is_err() {
        let str = std::format!("failed to read config.txt: {}", read_result.err().unwrap());
        log(log_file, &str);
        return None;
    }

    parse_config(log_file, &contents)
}

fn parse_config(log_file: &mut Option<std::fs::File>, text: &str) -> Option<Config> {
    let mut id = None;
    let mut pwd: Option<&str> = None;
    let mut proj: Option<&str> = None;

    // Custom parsing logic to avoid making clients depend on serde.
    let comma_split = text.split(',');

    for elt in comma_split {
        let assign_split: Vec<&str> = elt.split(':').collect();
        if assign_split.len() != 2 {
            log(log_file, "failed to parse config.txt");
            log(log_file, &assign_split.join(":"));
            return None; 
        }

        if assign_split[0].trim().eq("participant_id") {
            id = Some(assign_split[1].trim());
        } 
        if assign_split[0].trim().eq("git_password") {
            pwd = Some(assign_split[1].trim());
        }
        if assign_split[0].trim().eq("project") {
            proj = Some(assign_split[1].trim());
        } 
    } 

    if id.is_none(){
        log(log_file, "failed to parse config.txt: missing participant_id");
        None
    } 
    else if pwd.is_none() {
        log(log_file, "failed to parse config.txt: missing git_password");
        None
    }
    else if proj.is_none() {
        log(log_file, "failed to parse config.txt: missing project");
        None
    } 
    else {
        // log(log_file, "participant id:");
        // log(log_file, id.unwrap());
        // log(log_file, "password:");
        // log(log_file, pwd.unwrap());
        // log(log_file, "project:");
        // log(log_file, proj.unwrap());

        Some (Config{participant_id: id.unwrap().to_owned(), git_password: pwd.unwrap().to_owned(), project: proj.unwrap().to_owned()})
    }
}



// Pushes any committed changes to the remote server.
fn git_push(log_file: &mut Option<std::fs::File>, changelog_path: &PathBuf) {
    let push_success = Command::new("git")
                .args(["push", "--set-upstream", "origin", "main"])
                .current_dir(changelog_path)
                .output();
     
    if push_success.is_err() {
        log(log_file, "failed to push");
        log(log_file, &push_success.err().unwrap().to_string());
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config() {
        let text =     
            "\"participant_id\": \"592089\",
            \"git_password\": \"985613\",
            \"project\":\"p1\"";
        let mut opt: Option<std::fs::File> = None;
        let config = parse_config(&mut opt, text);
        assert!(config.is_some());
    }   
}