use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

fn main() {
    let script_folder = Path::new("scripts");

    let mut concatenated_content = String::new();

    if let Ok(file_content) =
        fs::read_to_string(&format!("{}/general.js", script_folder.to_str().unwrap()))
    {
        concatenated_content.push_str(&file_content);
        concatenated_content.push('\n');
    }

    for entry in fs::read_dir(script_folder).unwrap() {
        let entry = entry.unwrap();
        let file_path = entry.path();
        if file_path.is_file() {
            let file_name = file_path.file_name().unwrap().to_str().unwrap();
            if file_name == "general.js" {
                continue;
            }
            if file_name.ends_with(".js") {
                let file_content = fs::read_to_string(&file_path).unwrap();
                concatenated_content.push_str(&file_content);
                concatenated_content.push('\n'); // Add a newline between files
            }
        }
    }

    let output_file_path = Path::new("script.js");
    let mut output_file = File::create(output_file_path).unwrap();
    output_file
        .write_all(concatenated_content.as_bytes())
        .unwrap();
}
