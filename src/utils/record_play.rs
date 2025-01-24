use std::io;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// Function "record" facilitates audio recording from a specified Bluetooth audio card.
// It leverages the external binary "parecord" via Command and awaits a SIGINT signal from the user to terminate the recording.
pub fn record(bt_addr: String, filename: String) -> io::Result<()> {
    let bluez_addr = bt_addr.replace(":", "_");
    let card_name = format!("bluez_card.{}", bluez_addr);
    let source_name = format!("bluez_input.{}.0", bluez_addr);

    // Once connected to target, you may need to run 'pactl list cards' to list bluetooth audio cards
    // and choose the right one. For me, it was standing under the name : headset-head-unit
    let set_profile_status = Command::new("pactl")
        .args(["set-card-profile", &card_name, "headset-head-unit"])
        .status()?;

    if !set_profile_status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to set card profile.",
        ));
    }

    // Prepare for SIGINT !
    let running = Arc::new(AtomicBool::new(true));
    let signal_handle = running.clone();
    ctrlc::set_handler(move || {
        signal_handle.store(false, Ordering::SeqCst);
        println!("\nInterrupt signal received. Stopping...");
    })
    .expect("Failed to set Ctrl+C handler");

    // Launch `parecord`
    let mut child = Command::new("parecord")
        .args(["-d", &source_name, &filename])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    while running.load(Ordering::SeqCst) {
        if let Ok(Some(_)) = child.try_wait() {
            break; // Normal termination of process
        }
        thread::sleep(Duration::from_millis(100)); // Avoid CPU overload 
    }

    // Kill running process if still alive !
    if running.load(Ordering::SeqCst) {
        let _ = child.kill();
        println!("Recording process terminated.");
    }

    println!("Recorded under filename : {} !", filename);

    Ok(())
}

// The "playback" function is responsible for playing back audio recordings on a specified audio output device.
// It utilizes the external binary "paplay" through Command to achieve this functionality.
pub fn playback(sink: Option<String>, filename: String) {

    // Check if sink was specified, otherwise, we use a default one.
    let out_sink = sink.unwrap_or_else(|| {
        String::from("alsa_card.pci-0000_00_05.0.analog-stereo")
    });

    println!("Playing back audio on sink {} now!", out_sink);

    // Launch 'paplay' !
    let result = Command::new("paplay")
        .args(["-d", &out_sink, &filename])
        .output();

    // Checking status of Command.
    match result {
        Ok(output) => {
            if !output.status.success() {
                eprintln!(
                    "paplay failed:\nstatus: {}\nstderr: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
            } else {
                println!("Audio played successfully!");
            }
        }
        Err(error) => {
            eprintln!("Failed to execute paplay: {}", error);
        }
    }
}