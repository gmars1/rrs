#![allow(clippy::print_literal)]

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
        println!("No active audio sessions found.");
        return Ok(());
    }

    print_sessions(&sessions);

    loop {
        println!("\n=== Actions ===");
        println!("1. Set volume 50%");
        println!("2. Toggle mute");
        println!("3. Set volume 100%");
        println!("4. Refresh sessions");
        println!("5. Show session details");
        println!("6. Set balance (L/R)");
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
                print_sessions(&sessions);
            }
            continue;
        }

        if choice == "6" {
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
                    println!("Invalid session number.");
                    continue;
                }
                Err(_) => {
                    println!("Please enter a valid number.");
                    continue;
                }
            };

            let session = &sessions[session_index];
            let cur_l = (session.left_volume * 100.0).round() as u32;
            let cur_r = (session.right_volume * 100.0).round() as u32;
            let ch = if session.channel_count <= 1 {
                "mono"
            } else {
                "stereo"
            };
            println!(
                "Selected: {} (current: L={}{}% R={}{}%, {})",
                session.name, cur_l, "%", cur_r, "%", ch
            );

            print!("Left volume (0-100) [{}]: ", cur_l);
            io::stdout().flush()?;
            let mut left_input = String::new();
            io::stdin().read_line(&mut left_input)?;
            let left: u32 = if left_input.trim().is_empty() {
                cur_l
            } else {
                match left_input.trim().parse() {
                    Ok(v) if v <= 100 => v,
                    Ok(_) => {
                        println!("Invalid value. Must be 0-100.");
                        continue;
                    }
                    Err(_) => {
                        println!("Please enter a valid number.");
                        continue;
                    }
                }
            };

            print!("Right volume (0-100) [{}]: ", cur_r);
            io::stdout().flush()?;
            let mut right_input = String::new();
            io::stdin().read_line(&mut right_input)?;
            let right: u32 = if right_input.trim().is_empty() {
                cur_r
            } else {
                match right_input.trim().parse() {
                    Ok(v) if v <= 100 => v,
                    Ok(_) => {
                        println!("Invalid value. Must be 0-100.");
                        continue;
                    }
                    Err(_) => {
                        println!("Please enter a valid number.");
                        continue;
                    }
                }
            };

            let session_id = sessions[session_index].id;
            let left_f = left as f32 / 100.0;
            let right_f = right as f32 / 100.0;
            if let Err(e) = controller.set_volume(session_id, left_f, right_f) {
                println!("Failed to set balance: {}", e);
            } else {
                println!("Balance set: L={}% R={}", left, right);
            }
            continue;
        }

        if choice == "5" {
            println!("\nDetailed session info:");
            for session in &sessions {
                let l = (session.left_volume * 100.0).round() as u32;
                let r = (session.right_volume * 100.0).round() as u32;
                let ch = if session.channel_count <= 1 {
                    "mono"
                } else {
                    "stereo"
                };
                println!(
                    "  {} (ID: {}, PID: {}, Device: {:?}, L:{}% R:{}% {}, Mute: {})",
                    session.name, session.id, session.pid, session.device, l, r, ch, session.mute
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

fn print_sessions(sessions: &[audio_controller::Session]) {
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
        let l = (session.left_volume * 100.0).round() as u32;
        let r = (session.right_volume * 100.0).round() as u32;
        let ch = if session.channel_count <= 1 {
            "mono"
        } else {
            "stereo"
        };
        let mute = if session.mute { "MUTED" } else { "audible" };
        println!(
            "{:<4} {:<12} {:<20} {} (L:{}% R:{}% {}, {})",
            i + 1,
            pid,
            device,
            session.name,
            l,
            r,
            ch,
            mute
        );
    }
}

#[cfg(target_os = "android")]
fn main() {
    eprintln!("This is a library for Android. Use JNI to call from Java/Kotlin.");
    eprintln!("Build with: cargo build --release --target aarch64-linux-android");
    eprintln!("See examples/ for JNI usage patterns.");
}
