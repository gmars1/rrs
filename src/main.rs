#[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use audio_controller::{AudioController, DefaultController};
    use std::io::{self, Write};

    println!("=== Audio Controller ===\n");

    let mut controller =
        DefaultController::new().map_err(|e| format!("Failed to create controller: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        eprintln!("[DEBUG] Windows controller created successfully");
    }

    controller
        .refresh_sessions()
        .map_err(|e| format!("Failed to refresh sessions: {}", e))?;

    let mut sessions = controller
        .list_sessions()
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

    if sessions.is_empty() {
        #[cfg(target_os = "windows")]
        {
            eprintln!("\n[DEBUG] No sessions found. Make sure:");
            eprintln!("  - Audio applications are playing sound");
            eprintln!("  - Running with appropriate permissions");
            eprintln!("  - Windows Audio Service is running");
            eprintln!("  - Using eConsole role (for better compatibility)");
            eprintln!("  - Try running as Administrator (some apps are protected)");
            if let Ok(ctrl) = DefaultController::new() {
                eprintln!("  - Device: {}", ctrl.device_name());
            }
        }
        #[cfg(target_os = "linux")]
        {
            eprintln!("\n[DEBUG] No sessions found. Make sure:");
            eprintln!("  - pulseaudio is running");
            eprintln!("  - pactl is installed (pulseaudio-utils)");
            eprintln!("  - User is in pulse-access group or run with sudo");
        }
        println!("No active audio sessions found.");
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        eprintln!("[DEBUG] Found {} sessions", sessions.len());
        eprintln!("[DEBUG] Device: {}", controller.device_name());
    }

    println!("\nActive sessions:");
    println!(
        "{:<4} {:<12} {:<20} {}",
        "#", "PID", "Device", "Application"
    );
    println!("{}", "-".repeat(60));
    for (i, session) in sessions.iter().enumerate() {
        let pid = if session.pid > 0 {
            session.pid.to_string()
        } else {
            "system".to_string()
        };
        let device = session.device.as_deref().unwrap_or("unknown");
        let volume = format!("{:.0}%", session.volume * 100.0);
        let mute = if session.mute { "MUTED" } else { "audible" };
        println!(
            "{:<4} {:<12} {:<20} {} (Volume: {}, {})",
            i + 1,
            pid,
            device,
            session.name,
            volume,
            mute
        );
    }

    loop {
        println!("\n=== Actions ===");
        println!("1. Set volume 50%");
        println!("2. Toggle mute");
        println!("3. Set volume 100%");
        println!("4. Refresh sessions");
        println!("5. Show session details");
        println!("0. Exit");
        print!("Choice: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        if choice == "0" {
            println!("Exiting...");
            break;
        }

        if choice == "4" {
            controller
                .refresh_sessions()
                .map_err(|e| format!("Failed to refresh sessions: {}", e))?;

            sessions = controller
                .list_sessions()
                .map_err(|e| format!("Failed to list sessions: {}", e))?;
            if sessions.is_empty() {
                println!("No sessions found after refresh.");
            } else {
                println!("Sessions refreshed. Found {} sessions.", sessions.len());

                println!("\nActive sessions:");
                println!(
                    "{:<4} {:<12} {:<20} {}",
                    "#", "PID", "Device", "Application"
                );
                println!("{}", "-".repeat(60));
                for (i, session) in sessions.iter().enumerate() {
                    let pid = if session.pid > 0 {
                        session.pid.to_string()
                    } else {
                        "system".to_string()
                    };
                    let device = session.device.as_deref().unwrap_or("unknown");
                    let volume = format!("{:.0}%", session.volume * 100.0);
                    let mute = if session.mute { "MUTED" } else { "audible" };
                    println!(
                        "{:<4} {:<12} {:<20} {} (Volume: {}, {})",
                        i + 1,
                        pid,
                        device,
                        session.name,
                        volume,
                        mute
                    );
                }
            }
            continue;
        }

        if choice == "5" {
            println!("\nDetailed session info:");
            for session in &sessions {
                println!(
                    "  {} (ID: {}, PID: {}, Device: {:?}, Volume: {:.2}, Mute: {})",
                    session.name,
                    session.id,
                    session.pid,
                    session.device,
                    session.volume,
                    session.mute
                );
            }
            continue;
        }

        if sessions.is_empty() {
            println!("No sessions to act on.");

            continue;
        }

        print!("Select session number (1-{}): ", sessions.len());

        io::stdout().flush()?;

        let mut session_input = String::new();

        io::stdin().read_line(&mut session_input)?;

        let session_index: usize = match session_input.trim().parse() {
            Ok(num) if (1..=sessions.len()).contains(&num) => num - 1,

            Ok(_) => {
                println!(
                    "Invalid session number. Must be between 1 and {}.",
                    sessions.len()
                );

                continue;
            }

            Err(_) => {
                println!("Please enter a valid number.");

                continue;
            }
        };

        let session = &sessions[session_index];

        let session_id = session.id;

        match choice {
            "1" => {
                if let Err(e) = controller.set_volume(session_id, 0.5, 0.5) {
                    println!("Failed to set volume: {}", e);
                } else {
                    println!("Volume set to 50%");
                }
            }
            "2" => {
                if let Err(e) = controller.set_mute(session_id, !session.mute) {
                    println!("Failed to toggle mute: {}", e);
                } else {
                    println!(
                        "Mute toggled (now {})",
                        if session.mute { "OFF" } else { "ON" }
                    );
                }
            }
            "3" => {
                if let Err(e) = controller.set_volume(session_id, 1.0, 1.0) {
                    println!("Failed to set volume: {}", e);
                } else {
                    println!("Volume set to 100%");
                }
            }
            _ => {
                println!("Unknown action");
                continue;
            }
        }

        match controller.list_sessions() {
            Ok(updated_sessions) => {
                if let Some(updated) = updated_sessions.iter().find(|s| s.id == session_id) {
                    sessions[session_index] = updated.clone();
                    println!(
                        "Updated state: Volume: {:.1}%, Mute: {}",
                        updated.volume * 100.0,
                        if updated.mute { "ON" } else { "OFF" }
                    );
                } else {
                    println!("Warning: Session no longer exists (application may have closed)");

                    sessions.remove(session_index);
                    if sessions.is_empty() {
                        println!("No more sessions. Exiting...");
                        break;
                    }
                }
            }
            Err(e) => {
                println!("Warning: Failed to refresh session state: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "android")]
fn main() {
    eprintln!("This is a library for Android. Use JNI to call from Java/Kotlin.");
    eprintln!("Build with: cargo build --release --target aarch64-linux-android");
    eprintln!("See examples/ for JNI usage patterns.");
}
