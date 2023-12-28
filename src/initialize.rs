// file stuff
use serde_json;
use std::{fs::File, fmt::Debug};
use std::io::prelude::*;
use std::path::Path;

// password stuff
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use rand::Rng;
use requestty::Question;

// my stuff
use crate::errors::AppError;

// ------------------ //

pub trait Initialize {
    fn new(path: String, password_hash: String, hash_salt: String) -> Self;
    fn start_up(&mut self);
    fn create_settings(&self) -> Result<(), AppError>;
    fn load_settings(&mut self);
}

pub struct SettingsInitializer {
    path: String,
    password_hash: String,
    hash_salt: String,
}

impl Initialize for SettingsInitializer {
    fn new(path: String, password_hash: String, hash_salt: String) -> Self {
        SettingsInitializer {
            path,
            password_hash,
            hash_salt,
        }
    }

    fn start_up(&mut self) {
        // check if the file exists
        let path = Path::new(&self.path);
        if !path.exists() {
            println!("Settings file does not exist. Creating new settings file...");
            loop {
                match self.create_settings() {
                    Ok(_) => break,
                    Err(e) => println!("{}", e),
                }
            }
        } else {
            println!("Settings file exists. Loading settings...");
            self.load_settings();
        }
    }

    fn create_settings(&self) -> Result<(), AppError> {
        // create the new json file
        let mut file = match File::create(&self.path) {
            Err(why) => panic!("couldn't create settings file: {}", why),
            Ok(file) => file,
        };

        // set up argon2
        let argon2 = Argon2::default();

        // generate the salt
        let rng = rand::thread_rng();
        let salt_string = rng
            .sample_iter(rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect::<String>();
        let salt = SaltString::from_b64(salt_string.as_str()).unwrap();

        // get the password
        let q_pass = Question::password("password")
            .message("Enter a new password: ")
            .mask('*')
            .build();

        let q_pass_confirm = Question::password("password_confirm")
            .message("Confirm your password: ")
            .mask('*')
            .build();

        let answers = requestty::prompt(vec![q_pass, q_pass_confirm]).unwrap();

        let password = answers
            .get("password")
            .unwrap()
            .as_string()
            .unwrap() as &str;

        let password_confirm = answers
            .get("password_confirm")
            .unwrap()
            .as_string()
            .unwrap() as &str;

        // check if the passwords match
        if password != password_confirm {
            return Err(AppError::new("Passwords do not match."));
        } else {
            // hash the password
            let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();

            // serialize the password hash and salt to json
            let json = serde_json::json!({
                "password_hash": password_hash.to_string(),
                "hash_salt": salt_string
            });

            // write the json to the file
            match file.write_all(json.to_string().as_bytes()) {
                Err(why) => panic!("couldn't write to settings file: {}", why),
                Ok(_) => println!("Successfully wrote to settings file."),
            }
        }

        Ok(())
    }

    fn load_settings(&mut self) {
        // open the file
        let mut file = match File::open(&self.path) {
            Err(why) => panic!("couldn't open settings file: {}", why),
            Ok(file) => file,
        };

        // read the file
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Err(why) => panic!("couldn't read settings file: {}", why),
            Ok(_) => println!("Successfully read settings file."),
        }

        // parse the json
        let v: serde_json::Value = serde_json::from_str(&contents).unwrap();

        // get the password hash and salt
        self.password_hash = v["password_hash"].to_string();
        self.hash_salt = v["hash_salt"].to_string();
    }
}

impl Debug for SettingsInitializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsInitializer")
            .field("path", &self.path)
            .field("password_hash", &self.password_hash)
            .field("hash_salt", &self.hash_salt)
            .finish()
    }
}
