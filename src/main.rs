use anyhow::anyhow;
use anyhow::Error;
use rppal::gpio::{Gpio, Trigger};
use std::convert::TryFrom;
use std::fmt;
use std::fs;
use std::process::exit;
use std::time::Duration;

#[derive(Debug)]
enum ScreenState {
    On,
    Off,
}

pub const ON_VALUE: &str = "0\n";
pub const OFF_VALUE: &str = "1\n";
pub const STATE_FILE: &str = "/sys/class/backlight/rpi_backlight/bl_power";
const GPIO_PIN: u8 = 26;

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
    ScreenState::try_from(fs::read_to_string(STATE_FILE)?)
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

fn check_and_flip() {
    match check_screen_state() {
        Ok(state) => println!("Current state: {}", state),
        Err(e) => println!("Error: {}", e),
    };

    print!("Flipping state..");

    match flip_screen_state() {
        Ok(()) => println!("done!"),
        Err(e) => println!("\nError occured when setting state: [{}]", e),
    }
}

#[tokio::main]
async fn main() {
    // Initialize GPIO communications
    let gpio = Gpio::new().unwrap_or_else(|e| {
        println!("Error initializing gpio communication: [{}]", e);
        exit(-1)
    });

    // Create an input pin to use with our button
    let mut pin = gpio
        .get(GPIO_PIN)
        .unwrap_or_else(|e| {
            println!("Error initializing gpio pin: [{}]", e);
            exit(-1);
        })
        .into_input_pullup();

    // Define to reset the pin when the program ends
    pin.set_reset_on_drop(true);

    // Register callback to be executed when the button is pushed
    match pin.set_async_interrupt(Trigger::FallingEdge, |level| println!("Triggered!")) {
        Ok(()) => println!("Successfully configured pin {} as button input.", GPIO_PIN),
        Err(e) => println!("Error occurred when configuring pin {}: [{}]", GPIO_PIN, e),
    }

    // Do nothing, just need to keep the program alive to wait for the callback
    for _ in 1..4 {
        tokio::time::sleep(Duration::from_secs(10)).await;
        println!("Waking for periodic checking of stuff...");
    }
}
