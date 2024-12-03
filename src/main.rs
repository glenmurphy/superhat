use gilrs::{Gilrs, Event, EventType};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use winky::{self, Key};

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
}

const DEVICE_ID: u32 = 1;
const BUTTON_UP: u32 = 23;
const BUTTON_RIGHT: u32 = 24;
const BUTTON_DOWN: u32 = 25;
const BUTTON_LEFT: u32 = 26;

static MFD_KEYS: &[&[Key]] = &[
    &[Key::Control, Key::Alt, Key::Num1],
    &[Key::Control, Key::Alt, Key::Num2], 
    &[Key::Control, Key::Alt, Key::Num3],
    &[Key::Control, Key::Alt, Key::Num4],
    &[Key::Control, Key::Alt, Key::Num5],

    &[Key::Control, Key::Alt, Key::Num6],
    &[Key::Control, Key::Alt, Key::Num7],
    &[Key::Control, Key::Alt, Key::Num8],
    &[Key::Control, Key::Alt, Key::Num9],
    &[Key::Control, Key::Alt, Key::Num0],

    &[Key::Control, Key::Alt, Key::Numpad1],
    &[Key::Control, Key::Alt, Key::Numpad2], 
    &[Key::Control, Key::Alt, Key::Numpad3],
    &[Key::Control, Key::Alt, Key::Numpad4],
    &[Key::Control, Key::Alt, Key::Numpad5],
    
    &[Key::Control, Key::Alt, Key::Numpad6],
    &[Key::Control, Key::Alt, Key::Numpad7], 
    &[Key::Control, Key::Alt, Key::Numpad8],
    &[Key::Control, Key::Alt, Key::Numpad9],
    &[Key::Control, Key::Alt, Key::Numpad0],

    &[Key::Shift, Key::Alt, Key::Num1],
    &[Key::Shift, Key::Alt, Key::Num2], 
    &[Key::Shift, Key::Alt, Key::Num3],
    &[Key::Shift, Key::Alt, Key::Num4],
    &[Key::Shift, Key::Alt, Key::Num5],

    &[Key::Shift, Key::Alt, Key::Num6],
    &[Key::Shift, Key::Alt, Key::Num7],
    &[Key::Shift, Key::Alt, Key::Num8],
    &[Key::Shift, Key::Alt, Key::Num9],
    &[Key::Shift, Key::Alt, Key::Num0],

    &[Key::Shift, Key::Alt, Key::Numpad1],
    &[Key::Shift, Key::Alt, Key::Numpad2], 
    &[Key::Shift, Key::Alt, Key::Numpad3],
    &[Key::Shift, Key::Alt, Key::Numpad4],
    &[Key::Shift, Key::Alt, Key::Numpad5],
    
    &[Key::Shift, Key::Alt, Key::Numpad6],
    &[Key::Shift, Key::Alt, Key::Numpad7], 
    &[Key::Shift, Key::Alt, Key::Numpad8],
    &[Key::Shift, Key::Alt, Key::Numpad9],
    &[Key::Shift, Key::Alt, Key::Numpad0],
];

const TIMEOUT_DURATION: Duration = Duration::from_millis(1500);
const LONGPRESS_DURATION: Duration = Duration::from_millis(500);

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
                    if let Some(direction) = map_index_to_direction(index) {
                        handle_long_press(direction, &mut app_state);
                        long_press_detected = true;
                    }
                }
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
                    button_press_times.remove(&index);
                    
                    if let Some(direction) = map_index_to_direction(index) {
                        // Only handle short press side selection if no long press was detected
                        if !long_press_detected {
                            if let AppState::WaitingForSide { .. } = app_state {
                                handle_input(direction, &mut app_state);
                            }
                        }
                    }
                    
                    // Reset long press detection when all buttons are released
                    if button_press_times.is_empty() {
                        long_press_detected = false;
                    }
                    
                    // Handle button release state
                    if let AppState::ButtonPressed { mfd, button_number } = app_state {
                        println!("Button {} released", button_number);
                        let key_combo = MFD_KEYS[button_number as usize - 1];
                        for key in key_combo.iter().rev() {
                            winky::release(*key);
                        }
                        app_state = AppState::WaitingForSide {
                            mfd,
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
                let key_combo = MFD_KEYS[button_num as usize - 1];
                for key in key_combo.iter() {
                    winky::press(*key);
                }
                *app_state = AppState::ButtonPressed {
                    mfd: mfd.clone(),
                    button_number: button_num,
                };
            } else if !could_lead_to_valid_osb(*side, inputs.as_slice()) {
                println!("Invalid sequence detected. Resetting to side selection.");
                *app_state = AppState::WaitingForSide {
                    mfd: mfd.clone(),
                };
            }
        }
        AppState::ButtonPressed { .. } => {
            // Ignore inputs while button is pressed
        }
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
