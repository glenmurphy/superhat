use winky::Key;

// These are the default keys for the OSBs in BMS 4.37
pub static MFD_KEYS: &[&[Key]] = &[
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

pub fn press_osb(osb_number: u8) {
    let key_combo = MFD_KEYS[osb_number as usize - 1];
    for key in key_combo.iter() {
        winky::press(*key);
    }
}

pub fn release_osb(osb_number: u8) {
    let key_combo = MFD_KEYS[osb_number as usize - 1];
    for key in key_combo.iter().rev() {
        winky::release(*key);
    }
} 