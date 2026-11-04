// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // A browser starts this executable as its native-messaging host, passing
    // the extension origin as the first argument. That process must never
    // create a window or take the single-instance slot: it is a headless relay
    // that forwards messages to the running app over a loopback socket.
    let args: Vec<String> = std::env::args().collect();
    if vidaro_lib::is_native_host_invocation(&args) {
        vidaro_lib::run_native_host();
    }

    vidaro_lib::run()
}
