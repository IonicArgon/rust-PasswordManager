mod errors;
mod handle_pass;
mod initialize;
use handle_pass::{PasswordHandler, ProcessPassword};
use initialize::{Initialize, SettingsInitializer};

fn main() {
    let mut settings = SettingsInitializer::new(String::from("settings.json"));

    let start_up_res = settings.start_up();
    match start_up_res {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }

    let mut password_handler = PasswordHandler::new();
    let password_res = password_handler.verify_password(
        settings.get_password_hash(),
        settings.get_key_salt(),
    );
    match password_res {
        Ok(_) => {
            println!("Password verified!");
        },
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }
}
