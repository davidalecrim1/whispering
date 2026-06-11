// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::var("WHISPERING_TEST_MODE").as_deref() == Ok("startup-probe") {
        match whispering_lib::startup_probe() {
            Ok(output) => {
                println!("{output}");
                return;
            }
            Err(err) => {
                eprintln!("startup probe failed: {err}");
                std::process::exit(1);
            }
        }
    }

    whispering_lib::run();
}
