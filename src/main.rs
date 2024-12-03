use gilrs::{Gilrs, Event, EventType};
use std::collections::HashMap;
use std::time::{Duration, Instant};

mod mfd_keys;
use mfd_keys::{press_button, release_button};

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
    WaitingForButton {
        mfd: MfdState,
        side: Direction,
        inputs: Vec<Direction>,
        last_input_time: Instant,
    },
    ButtonPressed {
        mfd: MfdState,
        button_number: u8,
    },
    InvalidSequence {
        mfd: MfdState,
    },
}

const DEVICE_ID: u32 = 1;
const BUTTON_UP: u32 = 23;
const BUTTON_RIGHT: u32 = 24;
const BUTTON_DOWN: u32 = 25;
const BUTTON_LEFT: u32 = 26;

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
) {
    let direction = match map_index_to_direction(button_index) {
        Some(dir) => dir,
        None => return, // Invalid button index
    };

    match (event_type, &*app_state) {
        // Handle initial side selection on button release
        (InputEventType::ButtonUp, AppState::WaitingForSide { .. }) => {
            handle_short_press(direction, app_state)
        },
        // Handle subsequent button presses immediately
        (InputEventType::ButtonDown, AppState::WaitingForButton { .. }) => {
            handle_short_press(direction, app_state)
        },
        // Handle long press for MFD selection
        (InputEventType::LongPress, _) => {
            handle_long_press(direction, app_state)
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
            *app_state = AppState::WaitingForButton {
                mfd: mfd.clone(),
                side: direction,
                inputs: Vec::new(),
                last_input_time: Instant::now(),
            };
        }
        AppState::WaitingForButton { mfd, side, inputs, last_input_time } => {
            *last_input_time = Instant::now();
            inputs.push(direction);
            
            if let Some(button_num) = calculate_osb_number(mfd.clone(), *side, inputs.as_slice()) {
                println!("OSB {} pressed", button_num);
                press_button(button_num);
                *app_state = AppState::ButtonPressed {
                    mfd: mfd.clone(),
                    button_number: button_num,
                };
            } else if !could_lead_to_valid_osb(*side, inputs.as_slice()) {
                println!("Invalid sequence detected. Resetting to side selection.");
                *app_state = AppState::InvalidSequence {
                    mfd: mfd.clone(),
                };
            }
        }
        AppState::ButtonPressed { .. } | AppState::InvalidSequence { .. } => {
            // Ignore inputs while button is pressed or in invalid sequence state
        }
    }
}

fn handle_long_press(direction: Direction, app_state: &mut AppState) {
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
            }
            _ => {
                println!("Long press detected in invalid direction for MFD selection.");
            }
        }
    }
}

fn handle_release(app_state: &mut AppState) {
    match app_state {
        AppState::ButtonPressed { mfd, button_number } => {
            println!("OSB {} released", button_number);
            release_button(*button_number);
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
    if let AppState::WaitingForButton { last_input_time, mfd, .. } = app_state {
        if last_input_time.elapsed() > TIMEOUT_DURATION {
            println!("Timeout occurred. Resetting to side selection.");
            *app_state = AppState::WaitingForSide {
                mfd: mfd.clone(),
            };
        }
    }
}

#[tokio::main]
async fn main() {
    let mut gilrs = Gilrs::new().unwrap();
    let mut app_state = AppState::WaitingForSide {
        mfd: MfdState::LeftMfd,
    };
    let mut button_press_times: HashMap<u32, Instant> = HashMap::new();
    let mut long_press_detected: bool = false;

    loop {
        while let Some(Event { id, event, .. }) = gilrs.next_event_blocking(Some(Duration::from_millis(100))) {
            let gamepad_id = u32::try_from(usize::from(id)).unwrap();
            if gamepad_id != DEVICE_ID {
                continue;
            }

            // Check for long press duration on active buttons
            for (&index, &press_time) in button_press_times.iter() {
                if !long_press_detected && press_time.elapsed() >= LONGPRESS_DURATION {
                    handle_input_event(InputEventType::LongPress, index, &mut app_state);
                    long_press_detected = true;
                }
            }

            match event {
                EventType::ButtonPressed(_, code) => {
                    let index = code.into_u32();
                    button_press_times.insert(index, Instant::now());
                    handle_input_event(InputEventType::ButtonDown, index, &mut app_state);
                }
                EventType::ButtonReleased(_, code) => {
                    let index = code.into_u32();
                    button_press_times.remove(&index);
                    
                    if !long_press_detected {
                        handle_input_event(InputEventType::ButtonUp, index, &mut app_state);
                    }
                    
                    // Reset long press detection when all buttons are released
                    if button_press_times.is_empty() {
                        long_press_detected = false;
                    }
                }
                _ => {}
            }
        }

        check_for_timeouts(&mut app_state);
    }
}

fn map_index_to_direction(index: u32) -> Option<Direction> {
    match index {
        BUTTON_UP => Some(Direction::Up),
        BUTTON_RIGHT => Some(Direction::Right),
        BUTTON_DOWN => Some(Direction::Down),
        BUTTON_LEFT => Some(Direction::Left),
        _ => None,
    }
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
