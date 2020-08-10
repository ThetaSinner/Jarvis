mod validate;
mod config;
mod build;

pub fn build_project(project_path: std::path::PathBuf) {
    return build::build_project(project_path);
}

pub fn validate_project(project_path: std::path::PathBuf) -> Result<validate::ValidationMessages, validate::ValidationError> {
    return validate::validate_project(project_path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
