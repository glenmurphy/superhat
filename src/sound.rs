use windows::{
    Win32::Media::Audio::{PlaySoundA, SND_MEMORY, SND_ASYNC},
    core::PCSTR,
};

// Include the click sound file directly in the binary
const CLICK_LEFT_SOUND: &[u8] = include_bytes!("../assets/click_left.wav");
const CLICK_RIGHT_SOUND: &[u8] = include_bytes!("../assets/click_right.wav");

pub enum ClickSound {
    Left,
    Right,
}

pub fn play_click(sound: ClickSound) {
    let sound_data = match sound {
        ClickSound::Left => CLICK_LEFT_SOUND,
        ClickSound::Right => CLICK_RIGHT_SOUND,
    };
    
    // Safety: sound_data is valid for the duration of PlaySound
    unsafe {
        PlaySoundA(
            PCSTR(sound_data.as_ptr()),
            None,
            SND_MEMORY | SND_ASYNC,
        );
    }
}