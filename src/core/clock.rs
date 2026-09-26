#[cfg(target_os = "windows")]
pub fn local_hour() -> u32 {
    unsafe { windows::Win32::System::SystemInformation::GetLocalTime().wHour as u32 }
}

#[cfg(not(target_os = "windows"))]
pub fn local_hour() -> u32 {
    12
}

pub fn greeting(hour: u32, rtl: bool) -> &'static str {
    match hour {
        4..=11 if rtl => "صباح الخير",
        4..=11 => "Good morning",
        12..=16 if rtl => "مساء الخير",
        12..=16 => "Good afternoon",
        17..=22 if rtl => "مساء الخير",
        17..=22 => "Good evening",
        _ if rtl => "سهرة سعيدة",
        _ => "Still up",
    }
}

pub fn machine_name() -> String {
    for key in ["USERNAME", "COMPUTERNAME", "USER", "HOSTNAME"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return v;
            }
        }
    }
    "Profile 1".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greetings_cover_the_whole_clock() {
        for h in 0..24 {
            assert!(!greeting(h, true).is_empty());
            assert!(!greeting(h, false).is_empty());
        }
    }

    #[test]
    fn machine_name_is_never_empty() {
        assert!(!machine_name().is_empty());
    }
}
