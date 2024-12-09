use gilrs::{Gilrs, Event as GilrsEvent, EventType};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use std::io;

mod mfd_keys;
use mfd_keys::{press_osb, release_osb};

mod ui;
use ui::Ui;

#[cfg(test)]
mod tests;
mod winstance;
mod sound;
use sound::{ClickSound, play_click};

mod config;
use config::{CONFIG, save_config, load_config, save_mfd_state};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
enum MfdState {
    LeftMfd,
    RightMfd,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Debug)]
enum AppState {
    WaitingForSide {
        mfd: MfdState,
    },
    SelectingOSB {
        mfd: MfdState,
        side: Direction,
        inputs: Vec<Direction>,
        last_input_time: Instant,
    },
    OSBPressed {
        mfd: MfdState,
        osb_number: u8,
    },
    InvalidSequence {
        mfd: MfdState,
    },
    BindingMode {
        waiting_for: Direction,
    },
}

static SOUND_ENABLED: Mutex<bool> = Mutex::new(true);

const TIMEOUT_DURATION: Duration = Duration::from_millis(1500);
const LONGPRESS_DURATION: Duration = Duration::from_millis(500);

enum InputEventType {
    ButtonDown,    // When button is first pressed
    ButtonUp,      // When button is released
    LongPress,     // When button has been held long enough
}

fn handle_input_event(
    event_type: InputEventType,
    button_id: u32,
    device_id: u32,
    app_state: &mut AppState,
    long_press_detected: bool,
) {
    let direction = match map_button_to_direction(device_id, button_id) {
        Some(dir) => dir,
        None => return, // Invalid button index
    };

    match (event_type, &*app_state) {
        // Handle long press for MFD selection
        (InputEventType::LongPress, AppState::WaitingForSide { .. }) => {
            if let Direction::Left | Direction::Right = direction {
                let selected_mfd = match direction {
                    Direction::Left => {
                        if *SOUND_ENABLED.lock().unwrap() {
                            play_click(ClickSound::Left);
                        }
                        MfdState::LeftMfd
                    },
                    Direction::Right => {
                        if *SOUND_ENABLED.lock().unwrap() {
                            play_click(ClickSound::Right);
                        }
                        MfdState::RightMfd
                    },
                    _ => unreachable!(),
                };
                
                // Save MFD state to config
                save_mfd_state(selected_mfd.clone());
                
                *app_state = AppState::WaitingForSide {
                    mfd: selected_mfd,
                };
            }
        },
        // Handle button releases in WaitingForSide state - ONLY if no long press was detected
        (InputEventType::ButtonUp, AppState::WaitingForSide { .. }) => {
            if !long_press_detected {
                handle_short_press(direction, app_state);
            }
        },
        // Ignore button down events in WaitingForSide state to prevent accidental triggers
        (InputEventType::ButtonDown, AppState::WaitingForSide { .. }) => {},
        // Rest of the cases remain the same
        (InputEventType::ButtonDown, AppState::SelectingOSB { .. }) => {
            handle_short_press(direction, app_state);
        },
        (InputEventType::ButtonUp, _) => {
            handle_release(app_state);
        },
        (InputEventType::ButtonDown, _) => {
            handle_short_press(direction, app_state);
        },
        _ => {},
    }
}

fn handle_short_press(direction: Direction, app_state: &mut AppState) {
    match app_state {
        AppState::WaitingForSide { mfd } => {
            // println!("Side Selected: {:?}", direction);
            *app_state = AppState::SelectingOSB {
                mfd: mfd.clone(),
                side: direction,
                inputs: Vec::new(),
                last_input_time: Instant::now(),
            };
        }
        AppState::SelectingOSB { mfd, side, inputs, last_input_time } => {
            *last_input_time = Instant::now();
            inputs.push(direction);
            
            if let Some(osb_num) = calculate_osb_number(mfd.clone(), *side, inputs.as_slice()) {
                // println!("OSB {} pressed", osb_num);
                press_osb(osb_num);
                *app_state = AppState::OSBPressed {
                    mfd: mfd.clone(),
                    osb_number: osb_num,
                };
            } else if !could_lead_to_valid_osb(*side, inputs.as_slice()) {
                // println!("Invalid sequence detected. Resetting to side selection.");
                *app_state = AppState::InvalidSequence {
                    mfd: mfd.clone(),
                };
            }
        }
        AppState::OSBPressed { .. } | AppState::InvalidSequence { .. } => {
            // Ignore inputs while button is pressed or in invalid sequence state
        }
        AppState::BindingMode { .. } => {
            // Ignore short presses while in binding mode
        }
    }
}

fn handle_release(app_state: &mut AppState) {
    match app_state {
        AppState::OSBPressed { mfd, osb_number: button_number } => {
            // println!("OSB {} released", button_number);
            release_osb(*button_number);
            *app_state = AppState::WaitingForSide {
                mfd: mfd.clone(),
            };
        }
        AppState::InvalidSequence { mfd } => {
            // Reset to waiting for side after handling release
            *app_state = AppState::WaitingForSide {
                mfd: mfd.clone(),
            };
        }
        _ => {}
    }
}

fn check_for_timeouts(app_state: &mut AppState, ui: &mut Ui) -> io::Result<()> {
    if let AppState::SelectingOSB { last_input_time, mfd, .. } = app_state {
        if last_input_time.elapsed() > TIMEOUT_DURATION {
            //  println!("Timeout occurred. Resetting to side selection.");
            *app_state = AppState::WaitingForSide {
                mfd: mfd.clone(),
            };
            ui.update(&app_state)?;
        }
    }
    Ok(())
}

fn enter_binding_mode(app_state: &mut AppState, ui: &mut Ui) -> io::Result<()> {
    // println!("Entering binding mode. Press the button you want to use for UP");
    *app_state = AppState::BindingMode {
        waiting_for: Direction::Up,
    };
    ui.update(app_state)?;
    Ok(())
}

fn handle_binding(button_id: u32, device_id: u32, app_state: &mut AppState, ui: &mut Ui) {
    let AppState::BindingMode { waiting_for } = app_state else { return };
    
    let mut config_lock = match CONFIG.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    
    let Some(config) = config_lock.as_mut() else { return };
    
    match waiting_for {
        Direction::Up => {
            config.button_bindings.up = (device_id, button_id);
            *app_state = AppState::BindingMode { waiting_for: Direction::Right };
            ui.update(app_state).unwrap();
        },
        Direction::Right => {
            config.button_bindings.right = (device_id, button_id);
            *app_state = AppState::BindingMode { waiting_for: Direction::Down };
            ui.update(app_state).unwrap();
        },
        Direction::Down => {
            config.button_bindings.down = (device_id, button_id);
            *app_state = AppState::BindingMode { waiting_for: Direction::Left };
            ui.update(app_state).unwrap();
        },
        Direction::Left => {
            config.button_bindings.left = (device_id, button_id);
            save_config(&config);
            *app_state = AppState::InvalidSequence { mfd: MfdState::LeftMfd };
            ui.update(app_state).unwrap();
        },
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Create UI first - this handles single instance check
    let mut ui = match Ui::new() {
        Ok(ui) => ui,
        Err(e) => {
            if e.kind() == io::ErrorKind::Other {
                return Ok(()); // Exit quietly if another instance is running
            }
            return Err(e);    // Propagate other errors
        }
    };

    let mut gilrs = Gilrs::new().unwrap();

    // Only do the startup delay in release builds, not during tests
    #[cfg(not(test))]
    {
        // Wait 200ms and flush any pending events
        tokio::time::sleep(Duration::from_millis(200)).await;
        while gilrs.next_event().is_some() {}
    }

    // Load config and check if controls are bound
    let config = load_config();
    let controls_bound = config.button_bindings.up != (0, 0) 
        && config.button_bindings.right != (0, 0)
        && config.button_bindings.down != (0, 0)
        && config.button_bindings.left != (0, 0);

    // Initialize sound state from config
    *SOUND_ENABLED.lock().unwrap() = config.sound_enabled;

    *CONFIG.lock().unwrap() = Some(config.clone());  // Clone if needed

    let mut app_state = if !controls_bound {
        AppState::BindingMode {
            waiting_for: Direction::Up,
        }
    } else {
        AppState::WaitingForSide {
            mfd: config.selected_mfd,
        }
    };
    
    let mut button_press_times: HashMap<(u32, u32), Instant> = HashMap::new();
    let mut long_press_detected: bool = false;

    // flush any events that happened before we started
    std::thread::sleep(Duration::from_millis(100));
    while let Some(GilrsEvent { .. }) = gilrs.next_event() {}
    std::thread::sleep(Duration::from_millis(100));
    
    ui.update(&app_state)?;
    
    let mut running = true;
    while running {
        // Need to keep an eye on this blocking code - in some situations it blocks indefinitely but is
        // masked by axis events coming in causing it to carry through
        while let Some(GilrsEvent { id, event, .. }) = gilrs.next_event() {
            match event {
                EventType::ButtonPressed(_, code) => {
                    let button_id = code.into_u32();
                    let device_id = u32::try_from(usize::from(id)).unwrap();

                    if let AppState::BindingMode { .. } = app_state {
                        handle_binding(button_id, device_id, &mut app_state, &mut ui);
                        ui.update(&app_state).unwrap();
                        continue;
                    }

                    if let Some(_) = map_button_to_direction(device_id, button_id) {
                        button_press_times.insert((device_id, button_id), Instant::now());
                        long_press_detected = false; // Reset long press flag on new press
                        handle_input_event(InputEventType::ButtonDown, button_id, device_id, &mut app_state, long_press_detected);
                        ui.update(&app_state).unwrap();
                    }
                }
                EventType::ButtonReleased(_, code) => {
                    let button_id = code.into_u32();
                    let device_id = u32::try_from(usize::from(id)).unwrap();

                    if let AppState::BindingMode { .. } = app_state {
                        continue;
                    }

                    if let Some(_) = map_button_to_direction(device_id, button_id) {
                        // Store the current long_press_detected state before removing from press_times
                        let was_long_press = long_press_detected;
                        button_press_times.remove(&(device_id, button_id));
                        
                        // Only process button release if it wasn't a long press or if we're in OSBPressed state
                        if !was_long_press || matches!(app_state, AppState::OSBPressed { .. }) {
                            handle_input_event(InputEventType::ButtonUp, button_id, device_id, &mut app_state, was_long_press);
                            ui.update(&app_state).unwrap();
                        }
                        
                        if button_press_times.is_empty() {
                            long_press_detected = false;
                        }
                    }
                }
                _ => {}
            }
        }

        // Check for long presses on every iteration
        for (&(device_id, button_id), &press_time) in button_press_times.iter() {
            if !long_press_detected && press_time.elapsed() >= LONGPRESS_DURATION {
                if let Some(_) = map_button_to_direction(device_id, button_id) {
                    long_press_detected = true;  // Set this before handling the event
                    handle_input_event(
                        InputEventType::LongPress,
                        button_id,
                        device_id,
                        &mut app_state,
                        true
                    );
                    ui.update(&app_state).unwrap();
                }
            }
        }

        // Process other events
        while crossterm::event::poll(Duration::ZERO)? {
            match crossterm::event::read()? {
                Event::Key(KeyEvent { code: KeyCode::Char(c), kind: KeyEventKind::Press, .. }) => {
                    match c.to_ascii_lowercase() {
                        'b' => {
                            enter_binding_mode(&mut app_state, &mut ui)?;
                        }
                        'q' => {
                            running = false;
                        }
                        _ => {}
                    }
                }
                Event::Resize(width, height) => {
                    ui.handle_resize(width, height, &app_state)?;
                }
                Event::Mouse(MouseEvent { kind, column, row, .. }) => {
                    match kind {
                        MouseEventKind::Down(_) => {
                            if ui.is_bind_button_click(column, row) {
                                match app_state {
                                    AppState::BindingMode { .. } => {
                                        // Exit binding mode (TODO: don't reset the MFD)
                                        app_state = AppState::WaitingForSide { 
                                            mfd: MfdState::LeftMfd 
                                        };
                                        ui.update(&app_state)?;
                                    },
                                    _ => {
                                        // Enter binding mode
                                        enter_binding_mode(&mut app_state, &mut ui)?;
                                    }
                                }
                            } else if ui.is_sound_button_click(column, row) {
                                // Scope the lock to ensure it's released before calling update
                                {
                                    let mut sound_enabled = SOUND_ENABLED.lock().unwrap();
                                    *sound_enabled = !*sound_enabled;
                                    
                                    // Save sound state to config
                                    if let Ok(mut config_lock) = CONFIG.lock() {
                                        if let Some(config) = config_lock.as_mut() {
                                            config.sound_enabled = *sound_enabled;
                                            save_config(&config);
                                        }
                                    }
                                } // Lock is released here
                                
                                ui.update(&app_state)?;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        check_for_timeouts(&mut app_state, &mut ui)?;
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

fn map_button_to_direction(device_id: u32, button_id: u32) -> Option<Direction> {
    if let Ok(config) = CONFIG.lock() {
        if let Some(config) = config.as_ref() {
            let direction = match (device_id, button_id) {
                (dev, code) if (dev, code) == config.button_bindings.up => Some(Direction::Up),
                (dev, code) if (dev, code) == config.button_bindings.right => Some(Direction::Right),
                (dev, code) if (dev, code) == config.button_bindings.down => Some(Direction::Down),
                (dev, code) if (dev, code) == config.button_bindings.left => Some(Direction::Left),
                _ => None,
            };

            return direction;
        }
    }
    None
}

fn calculate_osb_number(
    mfd: MfdState,
    side: Direction,
    inputs: &[Direction],
) -> Option<u8> {
    if inputs.is_empty() {
        return None;
    }

    let osb_position = calculate_side_button(side, inputs)?;

    // Calculate global button number
    let base_number = match side {
        Direction::Up => 0,
        Direction::Right => 5,
        Direction::Down => 10,
        Direction::Left => 15,
    };

    let mfd_offset = match mfd {
        MfdState::LeftMfd => 0,
        MfdState::RightMfd => 20,
    };

    Some(base_number + osb_position + mfd_offset + 1)
}

fn calculate_side_button(side: Direction, inputs: &[Direction]) -> Option<u8> {
    // For any side, the middle button is always a single press in that direction
    if inputs.len() == 1 && inputs[0] == side {
        return Some(2);
    }

    // The "left" and "right" directions relative to the selected side
    let (left_dir, right_dir) = get_relative_directions(side);

    match (inputs.get(0), inputs.get(1)) {
        // Outer buttons using relative directions
        (Some(&d1), Some(&d2)) if d1 == left_dir && d2 == left_dir => Some(0),
        (Some(&d1), Some(&d2)) if d1 == left_dir && d2 == side => Some(1),
        (Some(&d1), Some(&d2)) if d1 == right_dir && d2 == side => Some(3),
        (Some(&d1), Some(&d2)) if d1 == right_dir && d2 == right_dir => Some(4),
        _ => None,
    }
}

fn could_lead_to_valid_osb(side: Direction, inputs: &[Direction]) -> bool {
    if inputs.is_empty() {
        return true;
    }

    // Single press of the side direction is always valid (middle button)
    if inputs.len() == 1 && inputs[0] == side {
        return true;
    }

    let (left_dir, right_dir) = get_relative_directions(side);

    match (inputs.get(0), inputs.get(1)) {
        // Single press that could lead to valid double press
        (Some(&d), None) if d == left_dir || d == right_dir => true,
        
        // Valid double presses
        (Some(&d1), Some(&d2)) if d1 == left_dir && (d2 == left_dir || d2 == side) => true,
        (Some(&d1), Some(&d2)) if d1 == right_dir && (d2 == right_dir || d2 == side) => true,
        
        _ => false,
    }
}

fn get_relative_directions(side: Direction) -> (Direction, Direction) {
    match side {
        Direction::Up => (Direction::Left, Direction::Right),
        Direction::Right => (Direction::Up, Direction::Down),
        Direction::Down => (Direction::Right, Direction::Left),
        Direction::Left => (Direction::Down, Direction::Up),
    }
}
