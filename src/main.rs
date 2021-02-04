use anyhow::anyhow;
use anyhow::Error;
use rppal::gpio::{Gpio, InputPin, Trigger};
use std::convert::TryFrom;
use std::fs;
use std::process::exit;
use std::str::FromStr;
use std::time::Instant;
use std::{env, fmt};
use tokio::signal::unix::{signal, SignalKind};

#[derive(Debug)]
enum ScreenState {
    On,
    Off,
}

const ON_VALUE: &str = "0\n";
const OFF_VALUE: &str = "1\n";
const STATE_FILE: &str = "/sys/class/backlight/rpi_backlight/bl_power";
const TRIGGER_DELAY_MS: u128 = 1000;

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

fn cleanup(mut pin: InputPin, signal: &str, pin_number: u8) {
    println!("Got [{}], cleaning up and shutting down.", signal);
    match pin.clear_async_interrupt() {
        Ok(()) => println!("Successfully cleared interrupt on pin [{}]", pin_number),
        Err(e) => println!("Error clearing interrupt on pin [{}]: [{}]", pin_number, e),
    }
}

#[tokio::main]
async fn main() {
    // Create signal handlers
    let mut sighup = signal(SignalKind::hangup()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();

    // Parse pin number from command line
    let args: Vec<String> = env::args().collect();
    let pin_number = match args.get(1) {
        Some(pin_string) => u8::from_str(pin_string).unwrap_or_else(|e| {
            println!("Unable to parse pin number from string: [{}]", e);
            exit(-1);
        }),
        None => {
            println!("No port specified!\nUsage: button <port>");
            exit(-1);
        }
    };

    // Initialize GPIO communications
    let gpio = Gpio::new().unwrap_or_else(|e| {
        println!("Error initializing gpio communication: [{}]", e);
        exit(-1)
    });

    // Create an input pin to use with our button
    let mut pin = gpio
        .get(pin_number)
        .unwrap_or_else(|e| {
            println!("Error initializing gpio pin: [{}]", e);
            exit(-1);
        })
        .into_input_pullup();

    let mut last_triggered = Instant::now();

    // Register callback to be executed when the button is pushed
    match pin.set_async_interrupt(Trigger::FallingEdge, move |_level| {
        if last_triggered.elapsed().as_millis() > TRIGGER_DELAY_MS {
            println!("Recognized button press, flipping screen state.");
            last_triggered = Instant::now();
            check_and_flip();
        }
    }) {
        Ok(()) => println!(
            "Successfully configured pin {} as button input.",
            pin_number
        ),
        Err(e) => println!(
            "Error occurred when configuring pin {}: [{}]",
            pin_number, e
        ),
    }

    // Do nothing, just need to keep the program alive to wait for the callback or until
    // we are interrupted
    tokio::select! {
    _ = sigint.recv() => cleanup(pin, "SIGINT", pin_number),
    _ = sighup.recv() => cleanup(pin, "SIGHUP", pin_number),
    _ = sigterm.recv() => cleanup(pin, "SIGTERM", pin_number),
    }
}
