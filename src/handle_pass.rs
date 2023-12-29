// password stuff
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher,
};
use requestty::Question;
use secrecy::{ExposeSecret, Secret};

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
    fn get_decrypt_key(&self) -> Secret<String>;
}

pub struct PasswordHandler {
    decrypt_key: Secret<String>,
}

impl ProcessPassword for PasswordHandler {
    fn new() -> Self {
        PasswordHandler {
            decrypt_key: Secret::new(String::from("")),
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
        let password = Secret::new(String::from(answer.as_string().unwrap()));

        let argon2 = Argon2::default();
        let password_hash = PasswordHash::new(&password_hash).unwrap();

        if argon2
            .verify_password(password.expose_secret().as_bytes(), &password_hash)
            .is_ok()
        {
            // set the decrypt key
            let salt = SaltString::from_b64(derived_key_salt.as_str()).unwrap();
            let decryption_key = Secret::new(
                argon2
                    .hash_password(password.expose_secret().as_bytes(), &salt)
                    .unwrap()
                    .to_string(),
            );

            // make a secret, then zeroize our decryption key and password
            self.decrypt_key = decryption_key.clone();

            Ok(())
        } else {
            Err(AppError::new("Password incorrect!"))
        }
    }

    fn get_decrypt_key(&self) -> Secret<String> {
        self.decrypt_key.clone()
    }
}
