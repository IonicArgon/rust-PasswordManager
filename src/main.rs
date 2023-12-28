mod initialize;
mod errors;
use initialize::{Initialize, SettingsInitializer};

fn main() {
    let mut settings = SettingsInitializer::new(
        String::from("settings.json"),
        String::from(""),
        String::from(""),
    );

    settings.start_up();

    
}
