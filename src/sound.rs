use rodio::{Decoder, OutputStream, Sink, OutputStreamHandle};
use std::io::Cursor;
use std::sync::Mutex;
use lazy_static::lazy_static;

// Include the click sound file directly in the binary
const CLICK_LEFT_SOUND: &[u8] = include_bytes!("../assets/click_left.wav");
const CLICK_RIGHT_SOUND: &[u8] = include_bytes!("../assets/click_right.wav");

pub enum ClickSound {
    Left,
    Right,
}

static mut AUDIO_INITIALIZED: bool = false;

// Keep the handle alive for the duration of the program
lazy_static! {
    static ref AUDIO: Mutex<Option<OutputStreamHandle>> = Mutex::new(None);
}

// Initialize the audio system and keep the OutputStream alive
pub fn init_audio() -> Option<OutputStream> {
    match OutputStream::try_default() {
        Ok((stream, handle)) => {
            // Store the handle for later use
            if let Ok(mut audio) = AUDIO.lock() {
                *audio = Some(handle);
            }
            Some(stream)
        }
        Err(e) => {
            eprintln!("Failed to initialize audio: {}", e);
            None
        }
    }
}

fn play_sound(sound_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let handle = AUDIO.lock()
        .map_err(|e| format!("Failed to lock audio: {}", e))?
        .as_ref()
        .ok_or("Audio not initialized")?
        .clone();
    
    let sink = Sink::try_new(&handle)?;
    let cursor = Cursor::new(sound_data.to_vec());
    let source = Decoder::new(cursor)?;
    
    sink.append(source);
    sink.sleep_until_end();
    
    Ok(())
}

pub fn play_click(sound: ClickSound) {
    // Initialize audio on first use
    unsafe {
        if !AUDIO_INITIALIZED {
            static mut STREAM: Option<OutputStream> = None;
            STREAM = init_audio();
            AUDIO_INITIALIZED = true;
        }
    }
    
    let sound_data = match sound {
        ClickSound::Left => CLICK_LEFT_SOUND,
        ClickSound::Right => CLICK_RIGHT_SOUND,
    };
    
    std::thread::spawn(move || {
        if let Err(e) = play_sound(sound_data) {
            eprintln!("Failed to play sound: {}", e);
        }
    });
} 