mod errors;
mod handle_pass;
mod initialize;
use handle_pass::{PasswordHandler, ProcessPassword};
use initialize::{Initialize, SettingsInitializer};

fn main() {
    let mut settings = SettingsInitializer::new(
        String::from("settings.json"),
        String::from(""),
        String::from(""),
    );

    let start_up_res = settings.start_up();

    match start_up_res {
        Ok(made_new_file) => {
            if !made_new_file {
                let mut password_handler = PasswordHandler::new(String::from(""));
                let check_pass_res =
                    password_handler.verify_password(settings.get_password_hash());

                match check_pass_res {
                    Ok(_) => println!("Password correct!"),
                    Err(e) => {
                        println!("{}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }
}
