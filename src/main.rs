use gilrs::{Gilrs, Event, EventType};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use winky::{self, Event as WinkyEvent, Key};

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
        last_input_time: Instant,
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
}

const DEVICE_ID: u32 = 1;
const BUTTON_UP: u32 = 23;
const BUTTON_RIGHT: u32 = 24;
const BUTTON_DOWN: u32 = 25;
const BUTTON_LEFT: u32 = 26;

const TIMEOUT_DURATION: Duration = Duration::from_secs(2);
const LONGPRESS_DURATION: Duration = Duration::from_millis(500);

#[tokio::main]
async fn main() {
    let mut gilrs = Gilrs::new().unwrap();
    let mut app_state = AppState::WaitingForSide {
        mfd: MfdState::LeftMfd,
        last_input_time: Instant::now(),
    };
    let mut button_press_times: HashMap<u32, Instant> = HashMap::new();

    loop {
        while let Some(Event { id, event, .. }) = gilrs.next_event_blocking(Some(Duration::from_millis(100))) {
            let gamepad_id = u32::try_from(usize::from(id)).unwrap();
            if gamepad_id != DEVICE_ID {
                continue;
            }

            match event {
                EventType::ButtonPressed(_, code) => {
                    let index = code.into_u32();
                    button_press_times.insert(index, Instant::now());
                    
                    // Handle immediate button presses
                    if let Some(direction) = map_index_to_direction(index) {
                        // Only process button presses for WaitingForButton state
                        if let AppState::WaitingForButton { .. } = app_state {
                            handle_input(direction, &mut app_state);
                        }
                    }
                }
                EventType::ButtonReleased(_, code) => {
                    let index = code.into_u32();
                    if let Some(press_time) = button_press_times.remove(&index) {
                        if let Some(direction) = map_index_to_direction(index) {
                            let press_duration = press_time.elapsed();
                            
                            if press_duration >= LONGPRESS_DURATION {
                                handle_long_press(direction, &mut app_state);
                            } else {
                                // Handle side selection on release when in WaitingForSide state
                                if let AppState::WaitingForSide { .. } = app_state {
                                    handle_input(direction, &mut app_state);
                                }
                            }
                        }
                    }
                    
                    // Handle button release state
                    if let AppState::ButtonPressed { mfd, button_number } = app_state {
                        println!("Button {} released", button_number);
                        app_state = AppState::WaitingForSide {
                            mfd,
                            last_input_time: Instant::now(),
                        };
                    }
                }
                _ => {}
            }
        }

        match &app_state {
            AppState::WaitingForButton { last_input_time, mfd, .. } => {
                if last_input_time.elapsed() > TIMEOUT_DURATION {
                    println!("Timeout occurred. Resetting to side selection.");
                    app_state = AppState::WaitingForSide {
                        mfd: mfd.clone(),
                        last_input_time: Instant::now(),
                    };
                }
            }
            _ => {}
        }
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
                    last_input_time: Instant::now(),
                };
            }
            _ => {
                println!("Long press detected in invalid direction for MFD selection.");
            }
        }
    }
}

fn handle_input(direction: Direction, app_state: &mut AppState) {
    match app_state {
        AppState::WaitingForSide { mfd, .. } => {
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
            
            if let Some(button_num) = calculate_button_number(mfd.clone(), *side, inputs.as_slice()) {
                println!("Button {} pressed", button_num);
                *app_state = AppState::ButtonPressed {
                    mfd: mfd.clone(),
                    button_number: button_num,
                };
            } else if !could_lead_to_valid_button(*side, inputs.as_slice()) {
                println!("Invalid sequence detected. Resetting to side selection.");
                *app_state = AppState::WaitingForSide {
                    mfd: mfd.clone(),
                    last_input_time: Instant::now(),
                };
            }
        }
        AppState::ButtonPressed { .. } => {
            // Ignore inputs while button is pressed
        }
    }
}

fn calculate_button_number(
    mfd: MfdState,
    side: Direction,
    inputs: &[Direction],
) -> Option<u8> {
    if inputs.is_empty() {
        return None;
    }

    let button_position = calculate_side_button(side, inputs)?;

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

    Some(base_number + button_position + mfd_offset + 1)
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

fn could_lead_to_valid_button(side: Direction, inputs: &[Direction]) -> bool {
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
