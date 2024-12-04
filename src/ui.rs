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
use winapi::um::winuser::{SetWindowLongA, GetWindowLongA, GWL_STYLE, WS_SIZEBOX, WS_MAXIMIZEBOX};
use winapi::um::wincon::GetConsoleWindow;

use crate::{AppState, Direction, MfdState};

const TOP_LEFT: &str = "┌";
const TOP_RIGHT: &str = "┐";
const BOTTOM_LEFT: &str = "└";
const BOTTOM_RIGHT: &str = "┘";
const HORIZONTAL: &str = "─";
const VERTICAL: &str = "│";

// Add this before the Ui implementation
struct ButtonPosition {
    x: u16,
    y: u16,
}

pub struct Ui {
    stdout: io::Stdout,
}

const CONSOLE_WIDTH: u16 = 96;
const CONSOLE_HEIGHT: u16 = 26;

impl Ui {
    pub fn new() -> io::Result<Self> {
        // Set console size before initializing
        set_console_size(CONSOLE_WIDTH as i16, CONSOLE_HEIGHT as i16);

        // Disable window resizing
        unsafe {
            let hwnd = GetConsoleWindow();
            SetWindowLongA(hwnd, GWL_STYLE, GetWindowLongA(hwnd, GWL_STYLE) & !(WS_MAXIMIZEBOX | WS_SIZEBOX) as i32);
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
        
        let (left_highlight, right_highlight) = match app_state {
            AppState::WaitingForSide { mfd } => {
                match mfd {
                    MfdState::LeftMfd => (true, false),
                    MfdState::RightMfd => (false, true),
                }
            }
            AppState::SelectingOSB { mfd, .. } |
            AppState::OSBPressed { mfd, .. } |
            AppState::InvalidSequence { mfd } => {
                (matches!(mfd, MfdState::LeftMfd), matches!(mfd, MfdState::RightMfd))
            }
            AppState::BindingMode { .. } => (false, false),
        };

        let (left_active_side, right_active_side) = match app_state {
            AppState::SelectingOSB { mfd, side, .. } => {
                match mfd {
                    MfdState::LeftMfd => (Some(side), None),
                    MfdState::RightMfd => (None, Some(side)),
                }
            }
            _ => (None, None),
        };

        self.render_mfds(
            left_highlight,
            right_highlight,
            left_active_side,
            right_active_side,
        )?;

        // Replace the status line rendering with the new method
        self.render_status_line(app_state)?;

        self.stdout.flush()?;
        Ok(())
    }

    fn draw_button(&mut self, number: u8, pos: ButtonPosition, highlighted: bool, active: bool) -> io::Result<()> {
        // Draw the 3x6 button box
        for dy in 0..3 {
            self.stdout.queue(cursor::MoveTo(pos.x, pos.y + dy))?;
            
            for dx in 0..6 {
                let char = match (dx, dy) {
                    (0, 0) => TOP_LEFT,
                    (5, 0) => TOP_RIGHT,
                    (0, 2) => BOTTOM_LEFT,
                    (5, 2) => BOTTOM_RIGHT,
                    (_, 0) | (_, 2) => HORIZONTAL,
                    (0, _) | (5, _) => VERTICAL,
                    (1..=4, 1) if dx == 2 => &format!("{:2}", number)[0..1],
                    (1..=4, 1) if dx == 3 => &format!("{:2}", number)[1..],
                    _ => " "
                };

                if active {
                    write!(
                        self.stdout,
                        "{}",
                        style::style(char).with(Color::Black).on(Color::White)
                    )?;
                } else if highlighted {
                    write!(
                        self.stdout,
                        "{}",
                        style::style(char).with(Color::Yellow)
                    )?;
                } else {
                    write!(self.stdout, "{}", char)?;
                }
            }
        }
        Ok(())
    }

    fn render_mfd(
        &mut self,
        start_x: u16,
        start_y: u16,
        highlighted: bool,
        active_side: Option<&Direction>,
        is_right_mfd: bool,
    ) -> io::Result<()> {
        let base_number = if is_right_mfd { 20 } else { 0 };
        
        // Define button positions
        let positions = [
            // Top row (1-5)
            (6, 0), (12, 0), (18, 0), (24, 0), (30, 0),
            // Right side (6-10)
            (36, 3), (36, 6), (36, 9), (36, 12), (36, 15),
            // Bottom row (11-15) - reversed order
            (30, 18), (24, 18), (18, 18), (12, 18), (6, 18),
            // Left side (16-20) - reversed order
            (0, 15), (0, 12), (0, 9), (0, 6), (0, 3),
        ];

        for (i, (rel_x, rel_y)) in positions.iter().enumerate() {
            let button_num = (i as u8) + 1;
            let pos = ButtonPosition {
                x: start_x + *rel_x,
                y: start_y + *rel_y,
            };

            let is_active = active_side.map_or(false, |side| {
                match side {
                    Direction::Up => i < 5,
                    Direction::Right => (5..10).contains(&i),
                    Direction::Down => (10..15).contains(&i),
                    Direction::Left => i >= 15,
                }
            });

            self.draw_button(
                button_num + base_number,
                pos,
                highlighted,
                is_active,
            )?;
        }
        Ok(())
    }

    fn render_mfds(
        &mut self,
        left_highlight: bool,
        right_highlight: bool,
        left_active_side: Option<&Direction>,
        right_active_side: Option<&Direction>,
    ) -> io::Result<()> {
        // Render left MFD
        self.render_mfd(3, 1, left_highlight, left_active_side, false)?;
        
        // Render right MFD
        self.render_mfd(51, 1, right_highlight, right_active_side, true)?;
        
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