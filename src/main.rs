use gilrs::{Gilrs, Event, EventType};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::sync::Mutex;

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
    button_bindings: ButtonBindings,
}

#[derive(Debug, Serialize, Deserialize)]
struct ButtonBindings {
    up: (u32, u32),    // (device_id, button_code)
    right: (u32, u32),
    down: (u32, u32),
    left: (u32, u32),
}

impl Default for Config {
    fn default() -> Self {
        Config {
            button_bindings: ButtonBindings {
                up: (1, 23),
                right: (1, 24),
                down: (1, 25),
                left: (1, 26),
            }
        }
    }
}

static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

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
        // Handle long press for MFD selection - only in WaitingForSide state
        (InputEventType::LongPress, AppState::WaitingForSide { .. }) => {
            if let Direction::Left | Direction::Right = direction {
                let selected_mfd = match direction {
                    Direction::Left => MfdState::LeftMfd,
                    Direction::Right => MfdState::RightMfd,
                    _ => unreachable!(),
                };
                println!("MFD Selected: {:?}", selected_mfd);
                *app_state = AppState::WaitingForSide {
                    mfd: selected_mfd,
                };
            }
        },
        // Handle button releases in WaitingForSide state
        (InputEventType::ButtonUp, AppState::WaitingForSide { .. }) => {
            // Only process as a short press if no long press was detected
            if !long_press_detected {
                handle_short_press(direction, app_state);
            }
        },
        // Handle subsequent button presses
        (InputEventType::ButtonDown, AppState::SelectingOSB { .. }) => {
            handle_short_press(direction, app_state);
        },
        // Handle button releases
        (InputEventType::ButtonUp, _) => {
            handle_release(app_state);
        },
        // Ignore initial button presses in WaitingForSide state
        (InputEventType::ButtonDown, AppState::WaitingForSide { .. }) => {},
        // Handle other button presses
        (InputEventType::ButtonDown, _) => {
            handle_short_press(direction, app_state);
        },
        _ => {},
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

fn handle_binding(button_id: u32, device_id: u32, app_state: &mut AppState) {
    if let AppState::BindingMode { waiting_for } = app_state {
        if let Ok(mut config) = CONFIG.lock() {
            if let Some(config) = config.as_mut() {
                match waiting_for {
                    Direction::Up => {
                        config.button_bindings.up = (device_id, button_id);
                        println!("UP bound to device {} button {}. Press button for RIGHT", device_id, button_id);
                        *app_state = AppState::BindingMode { waiting_for: Direction::Right };
                    },
                    Direction::Right => {
                        config.button_bindings.right = (device_id, button_id);
                        println!("RIGHT bound to device {} button {}. Press button for DOWN", device_id, button_id);
                        *app_state = AppState::BindingMode { waiting_for: Direction::Down };
                    },
                    Direction::Down => {
                        config.button_bindings.down = (device_id, button_id);
                        println!("DOWN bound to device {} button {}. Press button for LEFT", device_id, button_id);
                        *app_state = AppState::BindingMode { waiting_for: Direction::Left };
                    },
                    Direction::Left => {
                        config.button_bindings.left = (device_id, button_id);
                        println!("LEFT bound to device {} button {}. Configuration complete!", device_id, button_id);
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
    // Initialize CONFIG at startup
    *CONFIG.lock().unwrap() = Some(load_config());

    let mut gilrs = Gilrs::new().unwrap();
    let mut app_state = AppState::WaitingForSide {
        mfd: MfdState::LeftMfd,
    };
    let mut button_press_times: HashMap<(u32, u32), Instant> = HashMap::new();  // (device_id, button_code) -> press_time
    let mut long_press_detected: bool = false;

    // Clear out any events that occurred before we started
    while let Some(Event { .. }) = gilrs.next_event() {}

    loop {
        // Non-blocking event check
        while let Some(Event { id, event, .. }) = gilrs.next_event() {
            match event {
                EventType::ButtonPressed(_, code) => {
                    let button_id = code.into_u32();
                    let device_id = u32::try_from(usize::from(id)).unwrap();

                    if let AppState::BindingMode { .. } = app_state {
                        handle_binding(button_id, device_id, &mut app_state);
                        continue;
                    }

                    if let Some(_) = map_button_to_direction(device_id, button_id) {
                        button_press_times.insert((device_id, button_id), Instant::now());
                        long_press_detected = false; // Reset long press flag on new press
                        handle_input_event(InputEventType::ButtonDown, button_id, device_id, &mut app_state, long_press_detected);
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
                    handle_input_event(
                        InputEventType::LongPress,
                        button_id,
                        device_id,
                        &mut app_state,
                        true
                    );
                    long_press_detected = true;
                }
            }
        }

        // Process keyboard events
        while crossterm::event::poll(Duration::ZERO).unwrap() {
            if let Ok(crossterm::event::Event::Key(key_event)) = crossterm::event::read() {
                if key_event.code == crossterm::event::KeyCode::Char('b') 
                    && key_event.kind == crossterm::event::KeyEventKind::Press {
                    enter_binding_mode(&mut app_state);
                    break;
                }
            }
        }

        check_for_timeouts(&mut app_state);

        // Small sleep to prevent CPU spinning
        std::thread::sleep(Duration::from_millis(100));
    }
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
