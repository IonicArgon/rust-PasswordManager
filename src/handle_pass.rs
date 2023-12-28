// password stuff
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher,
};
use requestty::Question;

// my stuff
use crate::errors::AppError;

// ------------------ //

pub trait ProcessPassword {
    fn new() -> Self;
    fn verify_password(
        &mut self,
        password_hash: String,
        derived_key_salt: String,
    ) -> Result<(), AppError>;
    fn get_decrypt_key(&self) -> String;
}

pub struct PasswordHandler {
    decrypt_key: String,
}

impl ProcessPassword for PasswordHandler {
    fn new() -> Self {
        PasswordHandler {
            decrypt_key: String::from(""),
        }
    }

    fn verify_password(
        &mut self,
        password_hash: String,
        derived_key_salt: String,
    ) -> Result<(), AppError> {
        let q_pass = Question::password("password")
            .message("Enter your password")
            .mask('*')
            .build();

        let answer = requestty::prompt_one(q_pass).unwrap();
        let password = answer.as_string().unwrap();

        let argon2 = Argon2::default();
        let password_hash = PasswordHash::new(&password_hash).unwrap();

        if argon2
            .verify_password(password.as_bytes(), &password_hash)
            .is_ok()
        {
            // set the decrypt key
            let salt = SaltString::from_b64(derived_key_salt.as_str()).unwrap();
            self.decrypt_key = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
            Ok(())
        } else {
            Err(AppError::new("Password incorrect!"))
        }
    }

    fn get_decrypt_key(&self) -> String {
        self.decrypt_key.clone()
    }
}
