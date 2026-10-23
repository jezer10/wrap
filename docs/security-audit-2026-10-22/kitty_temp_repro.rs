extern crate self as log;
#[macro_export]
macro_rules! warn { ($($arg:tt)*) => { eprintln!($($arg)*); }; }
use std::{env,fs};
#[cfg(feature = "local_fs")]
fn safe_delete_temp_file(path: &str) {
    if is_path_in_temp_dir(path)
        && path.contains("tty-graphics-protocol")
        && let Err(err) = fs::remove_file(path)
    {
        log::warn!("Failed to delete kitty temporary file (path = {path}): {err:#}");
    }
}

#[cfg(feature = "local_fs")]
fn is_path_in_temp_dir(path: &str) -> bool {
    let temp_dirs = vec!["/tmp", "/var/tmp", "/dev/shm"];

    for temp_dir in temp_dirs {
        if path.starts_with(temp_dir) {
            return true;
        }
    }

    if let Ok(temp_dir) = env::var("TMPDIR")
        && path.starts_with(&temp_dir)
    {
        return true;
    }

    false
}


fn main() {
 let path=env::args().nth(1).unwrap();
 println!("lexical_temp={}",is_path_in_temp_dir(&path));
 println!("synthetic_read={}",fs::read(&path).unwrap().len());
 safe_delete_temp_file(&path);
 println!("target_exists={}",std::path::Path::new(&path).exists());
}
