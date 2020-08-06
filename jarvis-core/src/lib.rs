use std::fs;
use std::path::PathBuf;

pub fn validate_project(project_path: std::path::PathBuf) -> (&'static str, bool) {
    let project_dir = find_project_dir(project_path);
    if project_dir.is_none() {
        return ("No .jarvis directory found in the project root", false);
    }

    let build_file = project_dir.unwrap().join("build.yaml");
    if !build_file.exists() {
        return ("build.yaml file not found in .jarvis directory", false);
    }

    return ("Successfully validated", true);
}

pub fn find_project_dir(project_path: std::path::PathBuf) -> Option<PathBuf> {
    let dir = fs::read_dir(project_path);
    match dir {
        Ok(paths) => {
            for path in paths {
                if path.is_ok() {
                    let entry = path.unwrap();
                    let entry_type = entry.file_type();
                    if entry_type.is_ok() && entry_type.unwrap().is_dir() && ".jarvis" == entry.file_name() {
                        return Option::Some(entry.path())
                    }
                }
            }
        },
        Err(error) => {
            println!("Error locating project directory: {}", error);
            return Option::None;
        }
    }

    return Option::None;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
