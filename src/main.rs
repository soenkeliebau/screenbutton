use anyhow::Error;
use anyhow::anyhow;
use std::convert::TryFrom;
use std::fmt;
use std::fs;

#[derive(Debug)]
enum ScreenState {
    On,
    Off,
}

pub const ON_VALUE: &str = "0\n";
pub const OFF_VALUE: &str = "1\n";
pub const STATE_FILE: &str = "/sys/class/backlight/rpi_backlight/bl_power";
impl fmt::Display for ScreenState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<String> for ScreenState {
    type Error = anyhow::Error;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        match v.as_str() {
            ON_VALUE => Ok(ScreenState::On),
            OFF_VALUE => Ok(ScreenState::Off),
            _ => Err(anyhow!("Got unknown state representation!")),
        }
    }
}

impl ScreenState {
    pub fn value(&self) -> &str {
        match self {
            ScreenState::On => ON_VALUE,
            ScreenState::Off => OFF_VALUE,
        }
    }
}

fn check_screen_state() -> Result<ScreenState, Error> {
    ScreenState::try_from(fs::read_to_string(
        STATE_FILE,
    )?)
}

fn flip_screen_state() -> Result<(), anyhow::Error> {
    match check_screen_state() {
        Ok(ScreenState::On) => screen_off(),
        Ok(ScreenState::Off) => screen_on(),
        Err(e) => Err(e),
    }
}

fn screen_on() -> Result<(), anyhow::Error> {
    fs::write(STATE_FILE, ScreenState::On.value())?;
    Ok(())
}

fn screen_off() -> Result<(), anyhow::Error> {
    fs::write(STATE_FILE, ScreenState::Off.value())?;
    Ok(())
}

fn main() {

    for _ in 0..4 {
        match check_screen_state() {
            Ok(state) => println!("Current state: {}", state),
            Err(e) => println!("Error: {}", e),
        };
        println!("Flipping state..");
        flip_screen_state().unwrap_or_else(|e| println!("Error occured when setting state: [{}]", e));
    }
}
