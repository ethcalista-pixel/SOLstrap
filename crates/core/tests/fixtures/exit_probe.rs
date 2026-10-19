//! Opt-in process-observer fixture. Exits with a requested status; never faults.
#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    let code=u32::from_str_radix(&std::env::args().nth(1).unwrap_or_else(||"0".into()),16).expect("hex exit status");
    std::thread::sleep(std::time::Duration::from_secs(2));
    std::process::exit(code as i32);
}
