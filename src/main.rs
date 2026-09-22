#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ifc_converter;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct ConvertResult {
    temp_path: String,
    suggested_name: String,
    repaired: usize,
}

#[tauri::command]
fn convert_file(path: String) -> Result<ConvertResult, String> {
    let p = Path::new(&path);
    let original_name = p.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    
    let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let (new_text, repaired) = ifc_converter::convert_ifc(&text)?;
    
    let temp_dir = std::env::temp_dir().join("orthowan_temp");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    
    let temp_path = temp_dir.join(format!("{}_Allplan.ifc", original_name));
    fs::write(&temp_path, new_text).map_err(|e| e.to_string())?;
    
    Ok(ConvertResult {
        temp_path: temp_path.to_string_lossy().to_string(),
        suggested_name: format!("{}_Allplan.ifc", original_name),
        repaired,
    })
}

#[derive(Deserialize)]
struct FileToSave {
    temp_path: String,
    new_name: String,
}

#[tauri::command]
fn save_files(files: Vec<FileToSave>, out_dir: String) -> Result<(), String> {
    let out_p = Path::new(&out_dir);
    if !out_p.exists() {
        return Err("Zielordner existiert nicht.".into());
    }
    
    for file in files {
        let tp = Path::new(&file.temp_path);
        if !tp.exists() { continue; }
        let target = out_p.join(&file.new_name);
        fs::copy(tp, target).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![convert_file, save_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
