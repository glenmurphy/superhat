use crossterm::{
    cursor,
    style::{self, Color, Stylize},
    terminal, QueueableCommand,
};
use std::io::{self, Write};

use crate::{AppState, Direction, MfdState};

const MFD_WIDTH: usize = 33;
const MFD_HEIGHT: usize = 21;
const MFD_SPACING: usize = 3;

const TOP_LEFT: &str = "┌";
const TOP_RIGHT: &str = "┐";
const BOTTOM_LEFT: &str = "└";
const BOTTOM_RIGHT: &str = "┘";
const HORIZONTAL: &str = "─";
const VERTICAL: &str = "│";
const T_DOWN: &str = "┬";
const T_UP: &str = "┴";
const T_RIGHT: &str = "├";
const T_LEFT: &str = "┤";
const CROSS: &str = "┼";

// Add this before the Ui implementation
struct ButtonPosition {
    x: u16,
    y: u16,
}

pub struct Ui {
    stdout: io::Stdout,
}

impl Ui {
    pub fn new() -> io::Result<Self> {
        let mut stdout = io::stdout();
        terminal::enable_raw_mode()?;
        stdout.queue(terminal::Clear(terminal::ClearType::All))?;
        stdout.queue(cursor::Hide)?;
        Ok(Self { stdout })
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

        // Render status line
        self.stdout.queue(cursor::MoveTo(0, MFD_HEIGHT as u16 + 2))?;
        self.stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
        match app_state {
            AppState::WaitingForSide { mfd } => {
                write!(self.stdout, "Waiting for side selection ({})", 
                    if matches!(mfd, MfdState::LeftMfd) { "LEFT" } else { "RIGHT" })?;
            }
            AppState::SelectingOSB { mfd, side, .. } => {
                write!(self.stdout, "Selecting OSB on {} MFD, {} side", 
                    if matches!(mfd, MfdState::LeftMfd) { "LEFT" } else { "RIGHT" },
                    format!("{:?}", side).to_uppercase())?;
            }
            AppState::OSBPressed { mfd, osb_number } => {
                write!(self.stdout, "OSB {} pressed on {} MFD", 
                    osb_number,
                    if matches!(mfd, MfdState::LeftMfd) { "LEFT" } else { "RIGHT" })?;
            }
            AppState::InvalidSequence { .. } => {
                write!(self.stdout, "Invalid sequence")?;
            }
            AppState::BindingMode { waiting_for } => {
                write!(self.stdout, "Binding mode: Press button for {:?}", waiting_for)?;
            }
        }

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
                y: *rel_y,
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
        self.render_mfd(0, left_highlight, left_active_side, false)?;
        
        // Render right MFD
        self.render_mfd(48, right_highlight, right_active_side, true)?;
        
        Ok(())
    }
}

fn is_border_button(index: usize) -> bool {
    // Returns true if the button is on the outer edge
    index < 5 || // top row
    (5..10).contains(&index) || // right side
    (10..15).contains(&index) || // bottom row
    index >= 15 // left side
}

impl Drop for Ui {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = self.stdout.queue(cursor::Show);
        let _ = self.stdout.flush();
    }
} 