use crate::config::get_project_config;

pub fn build_project(project_path: std::path::PathBuf) {
    let project_config_result = get_project_config(project_path);


}