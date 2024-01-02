mod errors;
mod handle_logo;
mod handle_operations;
mod handle_pass;
mod initialize;

use handle_logo::{HandleLogo, LogoHandler};
use handle_operations::{DBHandler, DBOperation, ProcessDB};
use handle_pass::{PasswordHandler, ProcessPassword};
use initialize::{Initialize, SettingsInitializer};

use secrecy::ExposeSecret;
use std::process::Command;

fn main() {
    // get current directory
    let current_dir = std::env::current_dir().unwrap();
    let current_dir = current_dir.to_str().unwrap();

    let mut settings =
        SettingsInitializer::new(String::from(format!("{}/settings.json", current_dir)));

    let start_up_res = settings.start_up();
    match start_up_res {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }

    let mut password_handler = PasswordHandler::new();
    let password_res =
        password_handler.verify_password(settings.get_password_hash(), settings.get_key_salt());
    match password_res {
        Ok(_) => {
            println!("Password verified!");
        }
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }

    // create the db handler
    let mut db_handler = DBHandler::new(String::from(format!("{}/db.json", current_dir)));
    let db_res = db_handler.start_up();
    match db_res {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }

    // print the title
    let logo_handler = LogoHandler {};
    logo_handler.print_logo();

    // main loop
    loop {
        let operation = db_handler.inquire_operation();
        match operation {
            Ok(DBOperation::List) => {
                let _ = db_handler.list_entries();
            }
            Ok(DBOperation::View) => {
                let _ = db_handler.view_entry(password_handler.get_decrypt_key().expose_secret());
            }
            Ok(DBOperation::Create) => {
                let _ = db_handler.create_entry(password_handler.get_decrypt_key().expose_secret());
            }
            Ok(DBOperation::Update) => {
                let _ = db_handler.update_entry(password_handler.get_decrypt_key().expose_secret());
            }
            Ok(DBOperation::Delete) => {
                let _ = db_handler.delete_entry();
            }
            Ok(DBOperation::Exit) => {
                println!("Exit");
                let _ = Command::new("clear").status();
                break;
            }
            Err(e) => {
                println!("{}", e);
                std::process::exit(1);
            }
        }
    }
}
