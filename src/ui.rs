use crossterm::{
    cursor,
    style::{self, Color, Stylize},
    terminal, QueueableCommand,
    event,
};
use std::io::{self, Write};
use winapi::um::wincon::{
    COORD, SMALL_RECT, SetConsoleWindowInfo, SetConsoleScreenBufferSize,
};
use winapi::um::processenv::GetStdHandle;
use winapi::um::winbase::STD_OUTPUT_HANDLE;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::winuser::{SetWindowLongA, GetWindowLongA, ShowScrollBar, SetWindowTextA, SB_BOTH, GWL_STYLE, WS_SIZEBOX, WS_MAXIMIZEBOX};
use winapi::um::wincon::GetConsoleWindow;

use crate::{AppState, Direction, MfdState};

const TOP_LEFT: &str = "┌";
const TOP_RIGHT: &str = "┐";
const BOTTOM_LEFT: &str = "└";
const BOTTOM_RIGHT: &str = "┘";
const HORIZONTAL: &str = "─";
const VERTICAL: &str = "│";

// Add this before the Ui implementation
#[derive(Debug)]
struct ButtonPosition {
    x: u16,
    y: u16,
}

// Add this new helper struct to track which button is currently highlighted
#[derive(Debug)]
struct HighlightedButton {
    position: u8,  // 0-4 representing position on the side
    side: Direction,
}

pub struct Ui {
    stdout: io::Stdout,
}

const CONSOLE_WIDTH: u16 = 96;
const CONSOLE_HEIGHT: u16 = 26;

#[derive(Debug)]
struct MfdDisplay {
    active_side: Option<Direction>,
    highlighted_button: Option<HighlightedButton>,
    pressed_osb: Option<u8>,
}

// Add these constants near the top with the other UI constants
const BIND_TEXT_X: u16 = CONSOLE_WIDTH - 5;
const BIND_TEXT_Y: u16 = 1;
const BIND_TEXT: &str = "BIND";

impl Ui {
    pub fn new() -> io::Result<Self> {
        // Set console size before initializing
        set_console_size(CONSOLE_WIDTH as i16, CONSOLE_HEIGHT as i16);

        // Disable window resizing
        unsafe {
            let hwnd = GetConsoleWindow();
            SetWindowLongA(hwnd, GWL_STYLE, GetWindowLongA(hwnd, GWL_STYLE) & !(WS_MAXIMIZEBOX | WS_SIZEBOX) as i32);
            ShowScrollBar(hwnd, SB_BOTH as i32, 0);
            let title = std::ffi::CString::new("Superhat").unwrap();
            SetWindowTextA(hwnd, title.as_ptr());
        }

        let mut stdout = io::stdout();
        terminal::enable_raw_mode()?;
        
        crossterm::execute!(
            stdout,
            terminal::EnterAlternateScreen,
            event::EnableMouseCapture
        )?;
        
        let mut ui = Ui { stdout };
        ui.stdout.queue(cursor::Hide)?;
        ui.stdout.flush()?;

        Ok(ui)
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.stdout.queue(terminal::Clear(terminal::ClearType::All))?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn update(&mut self, app_state: &AppState) -> io::Result<()> {
        self.stdout.queue(cursor::MoveTo(0, 0))?;
        
        // Convert app state into display state
        let (left_mfd, right_mfd) = match app_state {
            AppState::WaitingForSide { mfd } => match mfd {
                MfdState::LeftMfd => (
                    MfdDisplay { active_side: Some(Direction::Up), highlighted_button: None, pressed_osb: None },
                    MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None }
                ),
                MfdState::RightMfd => (
                    MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None },
                    MfdDisplay { active_side: Some(Direction::Up), highlighted_button: None, pressed_osb: None }
                ),
            },
            AppState::SelectingOSB { mfd, side, inputs, .. } => {
                let highlighted = if inputs.is_empty() {
                    Some(HighlightedButton { position: 2, side: *side })
                } else if let Some(last_input) = inputs.last() {
                    let (left_dir, right_dir) = get_relative_directions(*side);
                    let position = match last_input {
                        d if *d == left_dir => 1,
                        d if *d == right_dir => 3,
                        d if *d == *side => 2,
                        _ => 2,
                    };
                    Some(HighlightedButton { position, side: *side })
                } else {
                    None
                };

                match mfd {
                    MfdState::LeftMfd => (
                        MfdDisplay { active_side: Some(*side), highlighted_button: highlighted, pressed_osb: None },
                        MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None }
                    ),
                    MfdState::RightMfd => (
                        MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None },
                        MfdDisplay { active_side: Some(*side), highlighted_button: highlighted, pressed_osb: None }
                    ),
                }
            },
            AppState::OSBPressed { mfd, osb_number } => match mfd {
                MfdState::LeftMfd => (
                    MfdDisplay { active_side: Some(Direction::Up), highlighted_button: None, pressed_osb: Some(*osb_number) },
                    MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None }
                ),
                MfdState::RightMfd => (
                    MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None },
                    MfdDisplay { active_side: Some(Direction::Up), highlighted_button: None, pressed_osb: Some(*osb_number) }
                ),
            },
            _ => (
                MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None },
                MfdDisplay { active_side: None, highlighted_button: None, pressed_osb: None }
            ),
        };

        // Render both MFDs
        self.render_mfd(3, 1, &left_mfd, false)?;
        self.render_mfd(51, 1, &right_mfd, true)?;

        // Render status line
        self.render_status_line(app_state)?;

        // Draw the bind button
        self.draw_bind_button()?;

        self.stdout.flush()?;
        Ok(())
    }

    fn draw_button(&mut self, number: u8, pos: ButtonPosition, highlighted: bool, active: bool, pressed: bool) -> io::Result<()> {
        // Helper to get the character style based on button state
        fn get_colors(pressed: bool, highlighted: bool, active: bool) -> (Color, Option<Color>) {
            match (pressed, highlighted, active) {
                (true, _, _) => (Color::Red, Some(Color::White)),
                (_, true, _) => (Color::Black, Some(Color::White)),
                (_, _, true) => (Color::Yellow, None),
                _ => (Color::White, None),
            }
        }

        // Helper to get the character at a specific position
        fn get_char(dx: u16, dy: u16, number: u8) -> String {
            match (dx, dy) {
                (0, 0) => TOP_LEFT.to_string(),
                (5, 0) => TOP_RIGHT.to_string(),
                (0, 2) => BOTTOM_LEFT.to_string(),
                (5, 2) => BOTTOM_RIGHT.to_string(),
                (_, 0) | (_, 2) => HORIZONTAL.to_string(),
                (0, _) | (5, _) => VERTICAL.to_string(),
                (2, 1) => format!("{:02}", number).chars().nth(0).unwrap().to_string(),
                (3, 1) => format!("{:02}", number).chars().nth(1).unwrap().to_string(),
                _ => " ".to_string()
            }
        }

        // Draw the 3x6 button box
        for dy in 0..3 {
            self.stdout.queue(cursor::MoveTo(pos.x, pos.y + dy))?;
            
            for dx in 0..6 {
                let char = get_char(dx, dy, number);
                let (fg_color, bg_color) = get_colors(pressed, highlighted, active);
                
                let styled = match bg_color {
                    Some(bg) => style::style(char).with(fg_color).on(bg),
                    None => style::style(char).with(fg_color),
                };
                
                write!(self.stdout, "{}", styled)?;
            }
        }
        Ok(())
    }

    fn render_mfd(
        &mut self,
        start_x: u16,
        start_y: u16,
        display: &MfdDisplay,
        is_right_mfd: bool,
    ) -> io::Result<()> {
        let base_number = if is_right_mfd { 20 } else { 0 };
        
        for (i, (rel_x, rel_y)) in BUTTON_POSITIONS.iter().enumerate() {
            let button_num = (i as u8) + 1;
            let pos = ButtonPosition {
                x: start_x + rel_x,
                y: start_y + rel_y,
            };

            let is_highlighted = display.highlighted_button.as_ref().map_or(false, |hb| {
                let button_side = match i {
                    0..=4 => Direction::Up,
                    5..=9 => Direction::Right,
                    10..=14 => Direction::Down,
                    _ => Direction::Left,
                };
                
                let position_in_side = match i {
                    0..=4 => i,
                    5..=9 => i - 5,
                    10..=14 => i - 10,
                    _ => i - 15,
                };

                hb.side == button_side && position_in_side as u8 == hb.position
            });

            let is_pressed = display.pressed_osb.map_or(false, |osb| osb == button_num + base_number);
            let is_active = display.active_side.is_some();

            self.draw_button(
                button_num + base_number,
                pos,
                is_highlighted,
                is_active,
                is_pressed,
            )?;
        }
        Ok(())
    }

    fn render_status_line(&mut self, app_state: &AppState) -> io::Result<()> {
        let status_line_y = CONSOLE_HEIGHT - 2;
        self.stdout.queue(cursor::MoveTo(0, status_line_y))?;
        self.stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;

        // Get the status message based on app state
        let status = match app_state {
            AppState::WaitingForSide { mfd } => {
                format!("{} MFD SELECTED", 
                    if matches!(mfd, MfdState::LeftMfd) { "LEFT" } else { "RIGHT" })
            }
            AppState::SelectingOSB { mfd, side, .. } => {
                format!("Selecting OSB on {} MFD, {} side", 
                    if matches!(mfd, MfdState::LeftMfd) { "LEFT" } else { "RIGHT" },
                    format!("{:?}", side).to_uppercase())
            }
            AppState::OSBPressed { mfd, osb_number } => {
                format!("OSB {} pressed on {} MFD", 
                    osb_number,
                    if matches!(mfd, MfdState::LeftMfd) { "LEFT" } else { "RIGHT" })
            }
            AppState::InvalidSequence { .. } => {
                "Invalid sequence".to_string()
            }
            AppState::BindingMode { waiting_for } => {
                format!("Binding mode: Press button for {:?}", waiting_for)
            }
        };

        // Calculate padding for centering
        let padding = (CONSOLE_WIDTH as usize - status.len()) / 2;
        self.stdout.queue(cursor::MoveTo(padding as u16, status_line_y))?;
        write!(self.stdout, "{}", status)?;

        Ok(())
    }

    pub fn handle_resize(&mut self, width: u16, height: u16, app_state: &AppState) -> io::Result<()> {
        // Force it back
        if width == CONSOLE_WIDTH && height == CONSOLE_HEIGHT { 
            // Re-render the entire UI
            self.clear()?;
            self.update(app_state)?;
            return Ok(())
         }

        set_console_size(CONSOLE_WIDTH as i16, CONSOLE_HEIGHT as i16);
        
        // Re-render the entire UI
        self.clear()?;
        self.update(app_state)?;
        Ok(())
    }

    // Simplified bind button drawing
    fn draw_bind_button(&mut self) -> io::Result<()> {
        self.stdout.queue(cursor::MoveTo(BIND_TEXT_X, BIND_TEXT_Y))?;
        write!(self.stdout, "{}", style::style(BIND_TEXT).with(Color::Yellow))?;
        Ok(())
    }

    // Simplified click detection
    pub fn is_bind_button_click(&self, x: u16, y: u16) -> bool {
        x >= BIND_TEXT_X && x < BIND_TEXT_X + BIND_TEXT.len() as u16 &&
        y == BIND_TEXT_Y
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            self.stdout,
            event::DisableMouseCapture,
            cursor::Show
        );
    }
}

fn set_console_size(width: i16, height: i16) {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle == INVALID_HANDLE_VALUE {
            return;
        }

        // First set buffer size
        let buffer_size = COORD {
            X: width,
            Y: height,
        };
        SetConsoleScreenBufferSize(handle, buffer_size);

        // Then set window size
        let window_size = SMALL_RECT {
            Left: 0,
            Top: 0,
            Right: width - 1,
            Bottom: height - 1,
        };
        SetConsoleWindowInfo(handle, 1, &window_size);
    }
}

// Helper function to get relative directions (if not already defined)
fn get_relative_directions(side: Direction) -> (Direction, Direction) {
    match side {
        Direction::Up => (Direction::Left, Direction::Right),
        Direction::Right => (Direction::Up, Direction::Down),
        Direction::Down => (Direction::Right, Direction::Left),
        Direction::Left => (Direction::Down, Direction::Up),
    }
}

// Define button positions as a constant
const BUTTON_POSITIONS: [(u16, u16); 20] = [
    // Top row (1-5)
    (6, 0), (12, 0), (18, 0), (24, 0), (30, 0),
    // Right side (6-10)
    (36, 3), (36, 6), (36, 9), (36, 12), (36, 15),
    // Bottom row (11-15) - reversed order
    (30, 18), (24, 18), (18, 18), (12, 18), (6, 18),
    // Left side (16-20) - reversed order
    (0, 15), (0, 12), (0, 9), (0, 6), (0, 3),
]; 