// password stuff
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use requestty::Question;

// my stuff
use crate::errors::AppError;

// ------------------ //

pub trait ProcessPassword {
    fn new (raw_password: String) -> Self;
    fn verify_password(&mut self, password_hash: String) -> Result<(), AppError>;
}

pub struct PasswordHandler {
    raw_password: String,
}

impl ProcessPassword for PasswordHandler {
    fn new (raw_password: String) -> Self {
        PasswordHandler {
            raw_password,
        }
    }

    fn verify_password(&mut self, password_hash: String) -> Result<(), AppError> {
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
            self.raw_password = password.to_string();
            Ok(())
        } else {
            Err(AppError::new("Password incorrect!"))
        }
    }
}
