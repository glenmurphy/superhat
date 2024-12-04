use gilrs::{Gilrs, Event, EventType};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use gilrs::{Button, GamepadId};

mod mfd_keys;
use mfd_keys::{press_osb, release_osb};

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    device_id: u32,
    button_bindings: ButtonBindings,
}

#[derive(Debug, Serialize, Deserialize)]
struct ButtonBindings {
    up: u32,
    right: u32,
    down: u32,
    left: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            device_id: 1,
            button_bindings: ButtonBindings {
                up: 23,
                right: 24,
                down: 25,
                left: 26,
            }
        }
    }
}

static mut CONFIG: Option<Config> = None;

const TIMEOUT_DURATION: Duration = Duration::from_millis(1500);
const LONGPRESS_DURATION: Duration = Duration::from_millis(500);

enum InputEventType {
    ButtonDown,    // When button is first pressed
    ButtonUp,      // When button is released
    LongPress,     // When button has been held long enough
}

fn handle_input_event(
    event_type: InputEventType,
    button_index: u32,
    app_state: &mut AppState,
    long_press_for_mfd: &mut bool,
) {
    let direction = match map_index_to_direction(button_index) {
        Some(dir) => dir,
        None => return, // Invalid button index
    };

    match (event_type, &*app_state) {
        // Handle initial side selection on button release (because we need to distinguish between
        // a long press for MFD selection and a short press for side selection)
        (InputEventType::ButtonUp, AppState::WaitingForSide { .. }) => {
            if !*long_press_for_mfd {
                handle_short_press(direction, app_state)
            }
        },
        // Handle subsequent button presses immediately
        (InputEventType::ButtonDown, AppState::SelectingOSB { .. }) => {
            handle_short_press(direction, app_state)
        },
        // Handle button presses in other states
        (InputEventType::ButtonDown, _) => {
            handle_short_press(direction, app_state)
        },
        // Handle long press for MFD selection
        (InputEventType::LongPress, _) => {
            *long_press_for_mfd = handle_long_press(direction, app_state)
        },
        // Handle button release cleanup
        (InputEventType::ButtonUp, _) => {
            handle_release(app_state)
        },
        _ => {} // Ignore other state combinations
    }
}

fn handle_short_press(direction: Direction, app_state: &mut AppState) {
    match app_state {
        AppState::WaitingForSide { mfd } => {
            println!("Side Selected: {:?}", direction);
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
                println!("OSB {} pressed", osb_num);
                press_osb(osb_num);
                *app_state = AppState::OSBPressed {
                    mfd: mfd.clone(),
                    osb_number: osb_num,
                };
            } else if !could_lead_to_valid_osb(*side, inputs.as_slice()) {
                println!("Invalid sequence detected. Resetting to side selection.");
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

fn handle_long_press(direction: Direction, app_state: &mut AppState) -> bool {
    if let AppState::WaitingForSide { .. } = app_state {
        match direction {
            Direction::Left | Direction::Right => {
                let selected_mfd = match direction {
                    Direction::Left => MfdState::LeftMfd,
                    Direction::Right => MfdState::RightMfd,
                    _ => unreachable!(),
                };
                println!("MFD Selected: {:?}", selected_mfd);
                *app_state = AppState::WaitingForSide {
                    mfd: selected_mfd,
                };
                return true; // Long press was for MFD selection
            }
            _ => {
                println!("Long press detected in invalid direction for MFD selection.");
            }
        }
    }
    false // Long press was not for MFD selection
}

fn handle_release(app_state: &mut AppState) {
    match app_state {
        AppState::OSBPressed { mfd, osb_number: button_number } => {
            println!("OSB {} released", button_number);
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

fn check_for_timeouts(app_state: &mut AppState) {
    if let AppState::SelectingOSB { last_input_time, mfd, .. } = app_state {
        if last_input_time.elapsed() > TIMEOUT_DURATION {
            println!("Timeout occurred. Resetting to side selection.");
            *app_state = AppState::WaitingForSide {
                mfd: mfd.clone(),
            };
        }
    }
}

fn enter_binding_mode(app_state: &mut AppState) {
    println!("Entering binding mode. Press the button you want to use for UP");
    *app_state = AppState::BindingMode {
        waiting_for: Direction::Up,
    };
}

fn handle_binding(button_code: u32, gamepad_id: GamepadId, app_state: &mut AppState) {
    if let AppState::BindingMode { waiting_for } = app_state {
        unsafe {
            if let Some(config) = &mut CONFIG {
                // Store the device ID of the gamepad being bound
                config.device_id = u32::try_from(usize::from(gamepad_id)).unwrap();
                
                match waiting_for {
                    Direction::Up => {
                        config.button_bindings.up = button_code;
                        println!("UP bound. Press button for RIGHT");
                        *app_state = AppState::BindingMode { waiting_for: Direction::Right };
                    },
                    Direction::Right => {
                        config.button_bindings.right = button_code;
                        println!("RIGHT bound. Press button for DOWN");
                        *app_state = AppState::BindingMode { waiting_for: Direction::Down };
                    },
                    Direction::Down => {
                        config.button_bindings.down = button_code;
                        println!("DOWN bound. Press button for LEFT");
                        *app_state = AppState::BindingMode { waiting_for: Direction::Left };
                    },
                    Direction::Left => {
                        config.button_bindings.left = button_code;
                        println!("Configuration complete! Using device {}", config.device_id);
                        save_config(&config);
                        *app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
                    },
                }
            }
        }
    }
}

fn save_config(config: &Config) {
    let config_str = toml::to_string(config).unwrap();
    fs::write("superhat.cfg", config_str).expect("Failed to write config file");
}

fn load_config() -> Config {
    if Path::new("superhat.cfg").exists() {
        let config_str = fs::read_to_string("superhat.cfg").expect("Failed to read config file");
        toml::from_str(&config_str).unwrap_or_default()
    } else {
        Config::default()
    }
}

#[tokio::main]
async fn main() {
    // Load config at startup
    unsafe {
        CONFIG = Some(load_config());
    }

    let mut gilrs = Gilrs::new().unwrap();
    let mut app_state = AppState::WaitingForSide {
        mfd: MfdState::LeftMfd,
    };
    let mut button_press_times: HashMap<u32, Instant> = HashMap::new();
    let mut long_press_detected: bool = false;
    let mut long_press_for_mfd: bool = false;

    loop {
        while let Some(Event { id, event, .. }) = gilrs.next_event_blocking(Some(Duration::from_millis(100))) {
            let gamepad_id = u32::try_from(usize::from(id)).unwrap();
            
            match event {
                EventType::ButtonPressed(button, code) => {
                    let index = code.into_u32();
                    println!("Button pressed: {:?} (code: {})", button, index);

                    if let AppState::BindingMode { .. } = app_state {
                        handle_binding(index, id, &mut app_state);
                        continue;
                    }

                    unsafe {
                        if let Some(config) = &CONFIG {
                            if gamepad_id != config.device_id {
                                continue;
                            }
                        }
                    }
                    
                    button_press_times.insert(index, Instant::now());
                    handle_input_event(InputEventType::ButtonDown, index, &mut app_state, &mut long_press_for_mfd);
                }
                EventType::ButtonReleased(_, code) => {
                    if let AppState::BindingMode { .. } = app_state {
                        continue;
                    }
                    let index = code.into_u32();
                    button_press_times.remove(&index);
                    
                    if !long_press_detected || !long_press_for_mfd {
                        handle_input_event(InputEventType::ButtonUp, index, &mut app_state, &mut long_press_for_mfd);
                    }
                    
                    // Reset long press detection when all buttons are released
                    if button_press_times.is_empty() {
                        long_press_detected = false;
                        long_press_for_mfd = false;
                    }
                }
                EventType::Connected => {
                    println!("Gamepad {} connected", gamepad_id);
                }
                EventType::Disconnected => {
                    println!("Gamepad {} disconnected", gamepad_id);
                }
                _ => {}
            }
        }

        // Check for 'b' key press to enter binding mode
        if let Ok(true) = crossterm::event::poll(std::time::Duration::from_millis(0)) {
            if let Ok(crossterm::event::Event::Key(key_event)) = crossterm::event::read() {
                if key_event.code == crossterm::event::KeyCode::Char('b') {
                    enter_binding_mode(&mut app_state);
                }
            }
        }

        check_for_timeouts(&mut app_state);
    }
}

fn map_index_to_direction(index: u32) -> Option<Direction> {
    unsafe {
        if let Some(config) = &CONFIG {
            return match index {
                i if i == config.button_bindings.up => Some(Direction::Up),
                i if i == config.button_bindings.right => Some(Direction::Right),
                i if i == config.button_bindings.down => Some(Direction::Down),
                i if i == config.button_bindings.left => Some(Direction::Left),
                _ => None,
            };
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
