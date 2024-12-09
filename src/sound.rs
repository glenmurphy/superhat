use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

// Include the click sound file directly in the binary
const CLICK_LEFT_SOUND: &[u8] = include_bytes!("../assets/click_left.wav");
const CLICK_RIGHT_SOUND: &[u8] = include_bytes!("../assets/click_right.wav");

pub enum ClickSound {
    Left,
    Right,
}

fn play_sound(sound_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let (stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let cursor = Cursor::new(sound_data.to_vec());
    let source = Decoder::new(cursor)?;
    
    sink.append(source);
    sink.sleep_until_end();
    // Keep stream alive until sound finishes
    drop(stream);
    
    Ok(())
}

pub fn play_click(sound: ClickSound) {
    let sound_data = match sound {
        ClickSound::Left => CLICK_LEFT_SOUND,
        ClickSound::Right => CLICK_RIGHT_SOUND,
    };
    
    // Using spawn to avoid blocking the main thread
    std::thread::spawn(move || {
        if let Err(e) = play_sound(sound_data) {
            eprintln!("Failed to play sound: {}", e);
        }
    });
} 