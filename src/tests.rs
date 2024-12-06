use super::*;

fn setup_test_config() {
    let mut config = Config::default();
    config.button_bindings = ButtonBindings {
        up: (1, 1),
        right: (1, 2),
        down: (1, 3),
        left: (1, 4),
    };
    *CONFIG.lock().unwrap() = Some(config);
}

fn simulate_button_event(
    event_type: InputEventType,
    direction: Direction,
    app_state: &mut AppState,
    long_press_detected: bool,
) {
    let (device_id, button_id) = match direction {
        Direction::Up => (1, 1),
        Direction::Right => (1, 2),
        Direction::Down => (1, 3),
        Direction::Left => (1, 4),
    };

    handle_input_event(event_type, button_id, device_id, app_state, long_press_detected);
}

#[test]
fn test_long_press_mfd_selection() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Simulate long press of right button
    simulate_button_event(InputEventType::ButtonDown, Direction::Right, &mut app_state, false);
    simulate_button_event(InputEventType::LongPress, Direction::Right, &mut app_state, true);
    
    // Should switch to right MFD
    assert!(matches!(app_state, AppState::WaitingForSide { mfd: MfdState::RightMfd }));
    
    // Release should not trigger side selection after long press
    simulate_button_event(InputEventType::ButtonUp, Direction::Right, &mut app_state, true);
    assert!(matches!(app_state, AppState::WaitingForSide { mfd: MfdState::RightMfd }));
}

#[test]
fn test_long_press_during_osb_selection() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Start OSB selection with short press
    simulate_button_event(InputEventType::ButtonDown, Direction::Up, &mut app_state, false);
    simulate_button_event(InputEventType::ButtonUp, Direction::Up, &mut app_state, false);
    
    // Should be in SelectingOSB state
    assert!(matches!(app_state, AppState::SelectingOSB { side: Direction::Up, .. }));
    
    // Long press during selection should be ignored
    simulate_button_event(InputEventType::ButtonDown, Direction::Right, &mut app_state, false);
    simulate_button_event(InputEventType::LongPress, Direction::Right, &mut app_state, true);
    
    // Should still be in SelectingOSB state
    assert!(matches!(app_state, AppState::SelectingOSB { side: Direction::Up, .. }));
}

#[test]
fn test_timeout_during_long_press() {
    setup_test_config();
    let mut app_state = AppState::SelectingOSB {
        mfd: MfdState::LeftMfd,
        side: Direction::Up,
        inputs: vec![],
        last_input_time: Instant::now() - TIMEOUT_DURATION - Duration::from_millis(100),
    };
    let mut ui = Ui::new().unwrap();
    
    // Check timeout
    check_for_timeouts(&mut app_state, &mut ui).unwrap();
    
    // Should reset to WaitingForSide
    assert!(matches!(app_state, AppState::WaitingForSide { mfd: MfdState::LeftMfd }));
}

#[test]
fn test_short_press_after_long_press() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Long press right to select right MFD
    simulate_button_event(InputEventType::ButtonDown, Direction::Right, &mut app_state, false);
    simulate_button_event(InputEventType::LongPress, Direction::Right, &mut app_state, true);
    simulate_button_event(InputEventType::ButtonUp, Direction::Right, &mut app_state, true);
    
    // Short press up to start OSB selection
    simulate_button_event(InputEventType::ButtonDown, Direction::Up, &mut app_state, false);
    simulate_button_event(InputEventType::ButtonUp, Direction::Up, &mut app_state, false);
    
    // Should be selecting OSB on right MFD
    assert!(matches!(app_state, AppState::SelectingOSB { 
        mfd: MfdState::RightMfd,
        side: Direction::Up,
        ..
    }));
}

#[test]
fn test_osb_selection_sequence() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Select top row, middle OSB (OSB 3)
    simulate_button_event(InputEventType::ButtonDown, Direction::Up, &mut app_state, false);
    simulate_button_event(InputEventType::ButtonUp, Direction::Up, &mut app_state, false);
    
    assert!(matches!(app_state, AppState::SelectingOSB { 
        mfd: MfdState::LeftMfd,
        side: Direction::Up,
        ..
    }));
    
    // Press Up again to select middle button
    simulate_button_event(InputEventType::ButtonDown, Direction::Up, &mut app_state, false);
    
    assert!(matches!(app_state, AppState::OSBPressed { 
        mfd: MfdState::LeftMfd,
        osb_number: 3
    }));
    
    // Release button
    simulate_button_event(InputEventType::ButtonUp, Direction::Up, &mut app_state, false);
    
    assert!(matches!(app_state, AppState::WaitingForSide { 
        mfd: MfdState::LeftMfd
    }));
}

#[test]
fn test_complex_osb_sequence() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Select OSB 10 on left MFD
    simulate_button_event(InputEventType::ButtonDown, Direction::Right, &mut app_state, false);
    simulate_button_event(InputEventType::ButtonUp, Direction::Right, &mut app_state, false);
    
    simulate_button_event(InputEventType::ButtonDown, Direction::Down, &mut app_state, false);
    simulate_button_event(InputEventType::ButtonUp, Direction::Down, &mut app_state, false);
    
    simulate_button_event(InputEventType::ButtonDown, Direction::Down, &mut app_state, false);
    
    assert!(matches!(app_state, AppState::OSBPressed { 
        mfd: MfdState::LeftMfd,
        osb_number: 10
    }));
    
    simulate_button_event(InputEventType::ButtonUp, Direction::Down, &mut app_state, false);
}

#[test]
fn test_complex_mfd_switching_sequence() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Long press to switch to right MFD
    simulate_button_event(InputEventType::ButtonDown, Direction::Right, &mut app_state, false);
    simulate_button_event(InputEventType::LongPress, Direction::Right, &mut app_state, true);
    simulate_button_event(InputEventType::ButtonUp, Direction::Right, &mut app_state, true);
    
    assert!(matches!(app_state, AppState::WaitingForSide { 
        mfd: MfdState::RightMfd
    }));
    
    // Press an OSB on right MFD (OSB 3 - top middle)
    simulate_button_event(InputEventType::ButtonDown, Direction::Up, &mut app_state, false);
    simulate_button_event(InputEventType::ButtonUp, Direction::Up, &mut app_state, false);
    
    simulate_button_event(InputEventType::ButtonDown, Direction::Up, &mut app_state, false);
    
    assert!(matches!(app_state, AppState::OSBPressed { 
        mfd: MfdState::RightMfd,
        osb_number: 23  // OSB 3 + 20 for right MFD
    }));
    
    // Release OSB
    simulate_button_event(InputEventType::ButtonUp, Direction::Up, &mut app_state, false);
    
    // Switch back to left MFD
    simulate_button_event(InputEventType::ButtonDown, Direction::Left, &mut app_state, false);
    simulate_button_event(InputEventType::LongPress, Direction::Left, &mut app_state, true);
    simulate_button_event(InputEventType::ButtonUp, Direction::Left, &mut app_state, true);
    
    assert!(matches!(app_state, AppState::WaitingForSide { 
        mfd: MfdState::LeftMfd
    }));
}

#[test]
fn test_mixed_long_press_and_osb_sequence() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Press OSB on left MFD (middle left OSB - OSB 18)
    simulate_button_event(InputEventType::ButtonDown, Direction::Left, &mut app_state, false);
    simulate_button_event(InputEventType::ButtonUp, Direction::Left, &mut app_state, false);
    
    simulate_button_event(InputEventType::ButtonDown, Direction::Left, &mut app_state, false);
    
    assert!(matches!(app_state, AppState::OSBPressed { 
        mfd: MfdState::LeftMfd,
        osb_number: 18  // Middle left OSB
    }));
    
    // Release button
    simulate_button_event(InputEventType::ButtonUp, Direction::Left, &mut app_state, false);
    
    // Long press to switch to right MFD
    simulate_button_event(InputEventType::ButtonDown, Direction::Right, &mut app_state, false);
    simulate_button_event(InputEventType::LongPress, Direction::Right, &mut app_state, true);
    simulate_button_event(InputEventType::ButtonUp, Direction::Right, &mut app_state, true);
    
    assert!(matches!(app_state, AppState::WaitingForSide { 
        mfd: MfdState::RightMfd
    }));
    
    // Try to press an OSB during long press (should be ignored)
    simulate_button_event(InputEventType::ButtonDown, Direction::Up, &mut app_state, false);
    simulate_button_event(InputEventType::LongPress, Direction::Up, &mut app_state, true);
    
    assert!(matches!(app_state, AppState::WaitingForSide { 
        mfd: MfdState::RightMfd
    }));
}

#[test]
fn test_osb_numbering() {
    setup_test_config();
    let mut app_state = AppState::WaitingForSide { mfd: MfdState::LeftMfd };
    
    // Test each side's middle button
    let test_cases = [
        (Direction::Up, 3),      // Top middle
        (Direction::Right, 8),   // Right middle
        (Direction::Down, 13),   // Bottom middle
        (Direction::Left, 18),   // Left middle
    ];
    
    for (side, expected_osb) in test_cases {
        // Select side
        simulate_button_event(InputEventType::ButtonDown, side, &mut app_state, false);
        simulate_button_event(InputEventType::ButtonUp, side, &mut app_state, false);
        
        // Press middle button
        simulate_button_event(InputEventType::ButtonDown, side, &mut app_state, false);
        
        assert!(matches!(app_state, AppState::OSBPressed { 
            mfd: MfdState::LeftMfd,
            osb_number: n
        } if n == expected_osb));
        
        // Release and reset
        simulate_button_event(InputEventType::ButtonUp, side, &mut app_state, false);
    }
}