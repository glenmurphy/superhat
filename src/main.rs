use gilrs::{Gilrs, Button, Event, ev::Code, EventType};
use std::time::{Instant, Duration};

// Define our MFD states
#[derive(Debug, PartialEq, Clone)]
enum MfdState {
    Inactive,
    LeftMfd,
    RightMfd,
}

#[derive(Debug, PartialEq)]
enum ButtonSelectionState {
    Inactive,
    ButtonSelection { 
        side: u8, 
        first_press: Option<Direction> 
    },
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

fn main() {
    let mut gilrs = Gilrs::new().unwrap();
    
    // State tracking
    let mut mfd_state = MfdState::Inactive;
    let mut button_state = ButtonSelectionState::Inactive;
    let mut _last_press = Instant::now();
    let mut hat_held_time = None;
    
    // Iterate over all connected gamepads
    /*
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }
    */
    
    loop {
        while let Some(Event { id, event, time, .. }) = gilrs.next_event_blocking(Some(Duration::from_millis(1000))) {
            match event {
                EventType::ButtonPressed(_, code) => {
                    let index = code.into_u32();
                    if (23..=26).contains(&index) {
                        println!("Hat pressed {:?}", index);
                        hat_held_time = Some(Instant::now());
                    }
                }
                EventType::ButtonReleased(_, code) => {
                    let index = code.into_u32();
                    if (23..=26).contains(&index) {
                        println!("Hat released {:?}", index);
                        if let Some(held_time) = hat_held_time {
                            let hold_duration = held_time.elapsed();
                            
                            // Long press handling for MFD selection
                            if hold_duration >= Duration::from_millis(1000) {
                                match index {
                                    24 => mfd_state = MfdState::RightMfd,
                                    26 => mfd_state = MfdState::LeftMfd,
                                    _ => {}
                                }
                                println!("Selected {:?}", mfd_state);
                                button_state = ButtonSelectionState::Inactive;
                            } else {
                                let button = match index {
                                    23 => Button::DPadUp,
                                    24 => Button::DPadRight,
                                    25 => Button::DPadDown,
                                    26 => Button::DPadLeft,
                                    _ => return,
                                };
                                handle_hat_press(button, &mut mfd_state, &mut button_state, _last_press);
                            }
                            hat_held_time = None;
                        }
                        _last_press = Instant::now();
                    }
                }
                _ => {}
            }
        }
        
        // Reset button selection if timeout occurred
        if _last_press.elapsed() > Duration::from_secs(2) {
            button_state = ButtonSelectionState::Inactive;
        }
    }
}

fn handle_hat_press(
    button: Button,
    mfd_state: &mut MfdState,
    button_state: &mut ButtonSelectionState,
    _last_press: Instant,
) {
    if *mfd_state == MfdState::Inactive {
        return;
    }

    let direction = match button {
        Button::DPadUp => Some(Direction::Up),
        Button::DPadRight => Some(Direction::Right),
        Button::DPadDown => Some(Direction::Down),
        Button::DPadLeft => Some(Direction::Left),
        _ => None,
    };

    let Some(direction) = direction else { return };

    match button_state {
        ButtonSelectionState::Inactive => {
            // Initial press selects a side
            let side = match direction {
                Direction::Up => 0,    // Top side
                Direction::Right => 1,  // Right side
                Direction::Down => 2,   // Bottom side
                Direction::Left => 3,   // Left side
            };
            *button_state = ButtonSelectionState::ButtonSelection { 
                side, 
                first_press: None 
            };
        }

        ButtonSelectionState::ButtonSelection { side, first_press } => {
            if let None = first_press {
                // Check if this is a middle button press (same direction as side selection)
                let is_middle_press = match (*side, direction) {
                    (0, Direction::Up) => true,       // Top side, press up
                    (1, Direction::Right) => true,    // Right side, press right
                    (2, Direction::Down) => true,     // Bottom side, press down
                    (3, Direction::Left) => true,     // Left side, press left
                    _ => false,
                };

                if is_middle_press {
                    // Calculate middle button number directly
                    let base = 3; // Middle button is always position 3
                    let offset = match *mfd_state {
                        MfdState::LeftMfd => 0,
                        MfdState::RightMfd => 20,
                        MfdState::Inactive => return,
                    };

                    let button_num = offset + match *side {
                        0 => base,                    // Top side: 3
                        1 => 5 + base,                // Right side: 8
                        2 => 10 + base,               // Bottom side: 13
                        3 => 15 + base,               // Left side: 18
                        _ => return,
                    };

                    println!("Pressed button {}", button_num);
                    *button_state = ButtonSelectionState::Inactive;
                } else {
                    // Not a middle button press, store for second press
                    *button_state = ButtonSelectionState::ButtonSelection {
                        side: *side,
                        first_press: Some(direction),
                    };
                }
            } else {
                // This is the second press, time to determine which button
                let button_num = calculate_button_number(
                    mfd_state.clone(),
                    *side,
                    first_press.unwrap(),
                    direction
                );
                if let Some(num) = button_num {
                    println!("Pressed button {}", num);
                }
                *button_state = ButtonSelectionState::Inactive;
            }
        }
    }
}

fn calculate_button_number(
    mfd: MfdState,
    side: u8,
    first_press: Direction,
    second_press: Direction,
) -> Option<u8> {
    // Calculate base number (1-5) based on the side's perspective
    let base = match side {
        0 => { // Top side - uses original up/down/left/right logic
            match (first_press, second_press) {
                (Direction::Left, Direction::Left) => Some(1),
                (Direction::Left, Direction::Up) => Some(2),
                (Direction::Right, Direction::Up) => Some(4),
                (Direction::Right, Direction::Right) => Some(5),
                _ => None,
            }
        },
        1 => { // Right side - rotated 90° clockwise
            match (first_press, second_press) {
                (Direction::Down, Direction::Down) => Some(1),
                (Direction::Down, Direction::Right) => Some(2),
                (Direction::Up, Direction::Right) => Some(4),
                (Direction::Up, Direction::Up) => Some(5),
                _ => None,
            }
        },
        2 => { // Bottom side - rotated 180°
            match (first_press, second_press) {
                (Direction::Right, Direction::Right) => Some(1),
                (Direction::Right, Direction::Down) => Some(2),
                (Direction::Left, Direction::Down) => Some(4),
                (Direction::Left, Direction::Left) => Some(5),
                _ => None,
            }
        },
        3 => { // Left side - rotated 90° counterclockwise
            match (first_press, second_press) {
                (Direction::Up, Direction::Up) => Some(1),
                (Direction::Up, Direction::Left) => Some(2),
                (Direction::Down, Direction::Left) => Some(4),
                (Direction::Down, Direction::Down) => Some(5),
                _ => None,
            }
        },
        _ => None,
    }?;

    // Convert to global button number (1-20)
    let offset = match mfd {
        MfdState::LeftMfd => 0,
        MfdState::RightMfd => 20,
        MfdState::Inactive => return None,
    };

    // Calculate final button number based on side and offset
    Some(offset + match side {
        0 => base,                   // Top side: 1-5
        1 => 11 - base,               // Right side: 6-10
        2 => 10 + base,               // Bottom side: 11-15
        3 => 21 - base,              // Left side: 16-20
        _ => return None,
    })
}
