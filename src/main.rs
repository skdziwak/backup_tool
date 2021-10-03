use std::fs::{canonicalize, File, read_to_string};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use zip::ZipWriter;
use std::env;
use chrono;
use chrono::{Datelike, Timelike};

#[derive(Serialize, Deserialize)]
struct Config {
    input_paths: Vec<String>,
    output_path: String,
}

fn get_config(path : &Path) -> Result<Config, String> {
    let config_string = read_to_string(path);
    match config_string {
        Ok(s) => {
            let config = serde_json::from_str(s.as_str());
            match config {
                Ok(conf) => Ok(conf),
                Err(error) => {
                    let mut msg = String::from("Config parsing failed.");
                    msg.push_str("\n");
                    msg.push_str(&*error.to_string());
                    Err(msg)
                }
            }
        }
        Err(error) => {
            let mut msg = String::from("Config loading failed.");
            msg.push_str("\n");
            msg.push_str(&*error.to_string());
            Err(msg)
        }
    }
}

fn crawl(results: &mut Vec<PathBuf>, path_buf: PathBuf) {
    let path = path_buf.as_path();
    if path.is_file() {
        results.push(path_buf);
    } else if path.is_dir() {
        let read_dir = path.read_dir();
        if read_dir.is_ok() {
            for p in read_dir.unwrap() {
                if p.is_ok() {
                    let p = p.unwrap();
                    let pb = p.path();
                    crawl(results, pb);
                }
            }
        } else {
            println!("WARNING: Unable to read directory {}", path.display());
        }
    }
}

fn get_all_input_paths(config: &Config) -> Vec<PathBuf> {
    let mut all_paths: Vec<PathBuf> = Vec::new();
    let input_paths = &config.input_paths;
    for index in 0..input_paths.len() {
        let path = input_paths.get(index).unwrap();
        let path_buf = PathBuf::from(path);
        crawl(&mut all_paths, path_buf);
    }
    all_paths
}

fn get_absolute_path_string(path_buf: &PathBuf) -> Option<String> {
    match canonicalize(path_buf) {
        Ok(absolute) => {
            match absolute.to_str() {
                Some(s) => Option::Some(String::from(s)),
                None => Option::None
            }
        }
        Err(_) => Option::None
    }
}

fn copy_files_to_zip(zip_writer: &mut ZipWriter<File>, files: &Vec<PathBuf>) {
    let mut buffer: [u8; 8196] = [0; 8196];

    for path in files {
        let absolute_path = get_absolute_path_string(path);
        if !absolute_path.is_some() {
            println!("Unable to extract absolute path of {}", path.display());
            continue;
        }
        let absolute_path = absolute_path.unwrap();
        match File::open(path) {
            Ok(mut file) => {
                let zip_file_name = &absolute_path[1..];

                if zip_writer.start_file(zip_file_name, Default::default()).is_ok() {
                    loop {
                        let size = file.read(&mut buffer);
                        if !size.is_ok() {
                            println!("Error occurred while reading file");
                            break;
                        }
                        let size = size.unwrap();
                        if size == 0 { break; }
                        if !zip_writer.write(&buffer[..size]).is_ok() {
                            println!("Error occurred while writing to zip file");
                            break;
                        };
                    }
                } else {
                    println!("Unable to create zip entry")
                }
            }
            Err(_) => {}
        }
    }
}

fn args_as_vec() -> Vec<String> {
    let mut  vec : Vec<String> = Vec::new();
    for arg in env::args() {
        vec.push(arg);
    }
    return vec;
}

fn get_timestamp_name() -> String {
    let now = chrono::offset::Local::now();
    format!("backup_{}_{}_{} {}-{}.zip", now.year(), now.month(), now.day(), now.hour(), now.minute())
}

fn main() {
    let args = args_as_vec();
    match args.get(1) {
        Some(config_path) => {
            println!("Loading config: {}", config_path);
            let config_path = Path::new(config_path);
            let config = get_config(config_path);
            match config {
                Ok(config) => {
                    let all_paths = get_all_input_paths(&config);

                    let zip_dir = Path::new(config.output_path.as_str());
                    let zip_path = zip_dir.join(get_timestamp_name());
                    let zip_path = zip_path.as_path();
                    let zip_file = File::create(&zip_path).unwrap();
                    let mut zip = ZipWriter::new(zip_file);

                    copy_files_to_zip(&mut zip, &all_paths);

                    match zip.finish() {
                        Ok(_) => {
                            println!("Backup generated: {}", zip_path.display())
                        }
                        Err(_) => {
                            println!("ERROR: zip.finish() returned error.")
                        }
                    };
                }
                Err(err) => {
                    println!("{}", err)
                }
            }
        },
        None => {
            println!("Usage: backup_tool CONFIG_PATH");
            println!("Config format: {{\"input_paths\": [\"FILE TO BACKUP\", \"DIRECTORY TO BACKUP\"], \"output_path\": \"OUTPUT DIRECTORY\"}}");
        }
    }
}
