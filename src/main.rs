use gilrs::{Gilrs, Event, EventType};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone)]
enum MfdState {
    Inactive,
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
    MfdSelected {
        mfd: MfdState,
        side: Option<Direction>,
        inputs: Vec<Direction>,
        last_input_time: Instant,
    }
}

fn main() {
    let mut gilrs = Gilrs::new().unwrap();
    let mut app_state = AppState::MfdSelected {
        mfd: MfdState::LeftMfd,
        side: None,
        inputs: Vec::new(),
        last_input_time: Instant::now(),
    };
    let mut button_press_times: HashMap<u32, Instant> = HashMap::new();

    loop {
        while let Some(Event { event, .. }) = gilrs.next_event() {
            match event {
                EventType::ButtonPressed(_, code) => {
                    let index = code.into_u32();
                    button_press_times.insert(index, Instant::now());
                    if let Some(direction) = map_index_to_direction(index) {
                        handle_input(direction, &mut app_state);
                    }
                }
                EventType::ButtonReleased(_, code) => {
                    let index = code.into_u32();
                    if let Some(press_time) = button_press_times.remove(&index) {
                        let duration = press_time.elapsed();
                        if let Some(direction) = map_index_to_direction(index) {
                            if duration >= Duration::from_millis(500) {
                                handle_long_press(direction, &mut app_state);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if let AppState::MfdSelected { last_input_time, mfd, .. } = &app_state {
            if last_input_time.elapsed() > Duration::from_secs(2) {
                println!("Timeout occurred. Resetting selection.");
                app_state = AppState::MfdSelected {
                    mfd: mfd.clone(),
                    side: None,
                    inputs: Vec::new(),
                    last_input_time: Instant::now(),
                };
            }
        }
    }
}

fn map_index_to_direction(index: u32) -> Option<Direction> {
    match index {
        23 => Some(Direction::Up),
        24 => Some(Direction::Right),
        25 => Some(Direction::Down),
        26 => Some(Direction::Left),
        _ => None,
    }
}

fn handle_long_press(direction: Direction, app_state: &mut AppState) {
    match direction {
        Direction::Left | Direction::Right => {
            let selected_mfd = match direction {
                Direction::Left => MfdState::LeftMfd,
                Direction::Right => MfdState::RightMfd,
                _ => unreachable!(),
            };
            println!("MFD Selected: {:?}", selected_mfd);
            *app_state = AppState::MfdSelected {
                mfd: selected_mfd,
                side: None,
                inputs: Vec::new(),
                last_input_time: Instant::now(),
            };
        }
        _ => {
            println!("Long press detected in invalid direction for MFD selection.");
        }
    }
}

fn handle_input(direction: Direction, app_state: &mut AppState) {
    if let AppState::MfdSelected {
        ref mfd,
        ref mut side,
        ref mut inputs,
        ref mut last_input_time,
    } = app_state {
        *last_input_time = Instant::now();
        
        if side.is_none() {
            *side = Some(direction);
            println!("Side Selected: {:?}", direction);
        } else {
            inputs.push(direction);
            if let Some(button_num) = calculate_button_number(mfd.clone(), side.unwrap(), inputs.as_slice()) {
                println!("Pressed button {}", button_num);
                *side = None;
                inputs.clear();
            } else if !could_lead_to_valid_button(side.unwrap(), inputs.as_slice()) {
                println!("Invalid sequence detected. Resetting selection.");
                *side = None;
                inputs.clear();
            }
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

    let button_position = match side {
        Direction::Up => calculate_top_side_button(inputs),
        Direction::Right => calculate_right_side_button(inputs),
        Direction::Down => calculate_bottom_side_button(inputs),
        Direction::Left => calculate_left_side_button(inputs),
    }?;

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
        MfdState::Inactive => return None,
    };

    Some(base_number + button_position + mfd_offset + 1) // +1 to make it 1-indexed
}

fn calculate_top_side_button(inputs: &[Direction]) -> Option<u8> {
    match inputs {
        [Direction::Left, Direction::Left] => Some(0),
        [Direction::Left, Direction::Up] => Some(1),
        [Direction::Up] => Some(2),
        [Direction::Right, Direction::Up] => Some(3),
        [Direction::Right, Direction::Right] => Some(4),
        _ => None,
    }
}

fn calculate_right_side_button(inputs: &[Direction]) -> Option<u8> {
    match inputs {
        [Direction::Up, Direction::Up] => Some(0),
        [Direction::Up, Direction::Right] => Some(1),
        [Direction::Right] => Some(2),
        [Direction::Down, Direction::Right] => Some(3),
        [Direction::Down, Direction::Down] => Some(4),
        _ => None,
    }
}

fn calculate_bottom_side_button(inputs: &[Direction]) -> Option<u8> {
    match inputs {
        [Direction::Left, Direction::Left] => Some(0),
        [Direction::Left, Direction::Down] => Some(1),
        [Direction::Down] => Some(2),
        [Direction::Right, Direction::Down] => Some(3),
        [Direction::Right, Direction::Right] => Some(4),
        _ => None,
    }
}

fn calculate_left_side_button(inputs: &[Direction]) -> Option<u8> {
    match inputs {
        [Direction::Up, Direction::Up] => Some(0),
        [Direction::Up, Direction::Left] => Some(1),
        [Direction::Left] => Some(2),
        [Direction::Down, Direction::Left] => Some(3),
        [Direction::Down, Direction::Down] => Some(4),
        _ => None,
    }
}

fn could_lead_to_valid_button(side: Direction, inputs: &[Direction]) -> bool {
    match side {
        Direction::Up => could_be_valid_top_button(inputs),
        Direction::Right => could_be_valid_right_button(inputs),
        Direction::Down => could_be_valid_bottom_button(inputs),
        Direction::Left => could_be_valid_left_button(inputs),
    }
}

fn could_be_valid_top_button(inputs: &[Direction]) -> bool {
    match inputs {
        [] => true, // Empty sequence could still lead to valid button
        [Direction::Left] => true, // Could be left-left or left-up
        [Direction::Left, Direction::Left] => true, // Valid button
        [Direction::Left, Direction::Up] => true, // Valid button
        [Direction::Up] => true, // Valid button (middle)
        [Direction::Right] => true, // Could be right-right or right-up
        [Direction::Right, Direction::Right] => true, // Valid button
        [Direction::Right, Direction::Up] => true, // Valid button
        _ => false, // Any other sequence cannot lead to valid button
    }
}

fn could_be_valid_right_button(inputs: &[Direction]) -> bool {
    match inputs {
        [] => true,
        [Direction::Up] => true,
        [Direction::Up, Direction::Up] => true,
        [Direction::Up, Direction::Right] => true,
        [Direction::Right] => true,
        [Direction::Down] => true,
        [Direction::Down, Direction::Down] => true,
        [Direction::Down, Direction::Right] => true,
        _ => false,
    }
}

fn could_be_valid_bottom_button(inputs: &[Direction]) -> bool {
    match inputs {
        [] => true,
        [Direction::Left] => true,
        [Direction::Left, Direction::Left] => true,
        [Direction::Left, Direction::Down] => true,
        [Direction::Down] => true,
        [Direction::Right] => true,
        [Direction::Right, Direction::Right] => true,
        [Direction::Right, Direction::Down] => true,
        _ => false,
    }
}

fn could_be_valid_left_button(inputs: &[Direction]) -> bool {
    match inputs {
        [] => true,
        [Direction::Up] => true,
        [Direction::Up, Direction::Up] => true,
        [Direction::Up, Direction::Left] => true,
        [Direction::Left] => true,
        [Direction::Down] => true,
        [Direction::Down, Direction::Down] => true,
        [Direction::Down, Direction::Left] => true,
        _ => false,
    }
}
