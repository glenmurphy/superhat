use openxr as xr;
use lazy_static::lazy_static;
use std::sync::Mutex;

pub struct HeadTracker {
    instance: xr::Instance,
    system: xr::SystemId,
}

impl HeadTracker {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Load OpenXR
        #[cfg(feature = "static")]
        let entry = xr::Entry::linked();
        #[cfg(not(feature = "static"))]
        let entry = unsafe { xr::Entry::load()? };
        
        // Create instance without any extensions
        let instance = entry.create_instance(
            &xr::ApplicationInfo {
                application_name: "MFD Controller",
                application_version: 1,
                engine_name: "None",
                engine_version: 0,
                api_version: xr::Version::new(1, 0, 0),
            },
            &xr::ExtensionSet::default(),
            &[],
        )?;

        let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;

        Ok(Self {
            instance,
            system,
        })
    }

    pub fn is_head_left(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Get current system properties
        let props = self.instance.system_properties(self.system)?;
        
        // For now, just print out what we can get from the system
        println!("System properties: {:?}", props);
        println!("Tracking properties: {:?}", props.tracking_properties);
        
        // Default to left until we figure out the correct API calls
        Ok(true)
    }
}

lazy_static! {
    pub static ref HEAD_TRACKER: Mutex<HeadTracker> = Mutex::new(
        HeadTracker::new().expect("Failed to create HeadTracker")
    );
}

pub fn is_head_left() -> bool {
    if let Ok(config_lock) = crate::config::CONFIG.lock() {
        if let Some(config) = config_lock.as_ref() {
            if !config.use_openxr {
                return true;
            }
        }
    }

    if let Ok(tracker) = HEAD_TRACKER.lock() {
        match tracker.is_head_left() {
            Ok(is_left) => is_left,
            Err(e) => {
                eprintln!("Head tracking error: {}", e);
                true // Default to left on error
            }
        }
    } else {
        eprintln!("Failed to lock head tracker");
        true // Default to left if we can't get the lock
    }
} 