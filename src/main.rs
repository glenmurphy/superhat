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
        while let Some(Event { event, .. }) = gilrs.next_event_blocking(Some(Duration::from_millis(100))) {
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

        let AppState::MfdSelected { last_input_time, mfd, side, .. } = &app_state;
        
        if side.is_some() && last_input_time.elapsed() > Duration::from_secs(2) {
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
    let AppState::MfdSelected {
        ref mfd,
        ref mut side,
        ref mut inputs,
        ref mut last_input_time,
    } = app_state;

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
        MfdState::Inactive => return None,
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
