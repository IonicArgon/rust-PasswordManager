// question stuff
use requestty::Question;

// file stuff
use serde_json;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// password stuff
use secrecy::{ExposeSecret, Secret};
use chacha20::{
    cipher::{KeyIvInit, StreamCipher, generic_array::GenericArray},
    ChaCha20
};
use sha2::{Digest, Sha256};

// other stuff
use hex;

// my stuff
use crate::errors::AppError;

pub enum DBOperation {
    List,
    Search,
    Create,
    Update,
    Delete,
    Exit,
}

pub struct DBHandler {
    path: String,
    json: serde_json::Value,
}

pub trait ProcessDB {
    fn new(path: String) -> Self;
    fn start_up(&mut self) -> Result<(), AppError>;
    fn create_db(&self) -> Result<(), AppError>;
    fn load_db(&mut self) -> Result<(), AppError>;

    fn inquire_operation(&self) -> Result<DBOperation, AppError>;
    fn create_entry(&mut self, key: &String) -> Result<(), AppError>;
    //fn read_entry(&self) -> Result<(), AppError>;
    //fn update_entry(&mut self) -> Result<(), AppError>;
    //fn delete_entry(&mut self) -> Result<(), AppError>;
}

impl ProcessDB for DBHandler {
    fn new(path: String) -> Self {
        DBHandler {
            path,
            json: serde_json::Value::Null,
        }
    }

    fn start_up(&mut self) -> Result<(), AppError> {
        // check if the file exists
        let path = Path::new(&self.path);

        if !path.exists() {
            println!("Database file does not exist. Creating new database file...");
            loop {
                match self.create_db() {
                    Ok(_) => break,
                    Err(e) => println!("{}", e),
                }
            }
        } else {
            loop {
                match self.load_db() {
                    Ok(_) => break,
                    Err(e) => println!("{}", e),
                }
            }
        }

        Ok(())
    }

    fn create_db(&self) -> Result<(), AppError> {
        // create the new json file
        let mut file = match File::create(&self.path) {
            Err(why) => panic!("Couldn't create database file: {}", why),
            Ok(file) => file,
        };

        // create the json object
        let json = serde_json::json!({
            "entries": []
        });

        // write the json object to the file
        match file.write_all(json.to_string().as_bytes()) {
            Err(why) => panic!("Couldn't write to database file: {}", why),
            Ok(_) => (),
        }

        Ok(())
    }

    fn load_db(&mut self) -> Result<(), AppError> {
        // open the file
        let mut file = match File::open(&self.path) {
            Err(why) => panic!("Couldn't open database file: {}", why),
            Ok(file) => file,
        };

        // read the file
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Err(why) => panic!("Couldn't read database file: {}", why),
            Ok(_) => (),
        }

        // parse the json
        let json: serde_json::Value = match serde_json::from_str(&contents) {
            Err(why) => panic!("Couldn't parse database file: {}", why),
            Ok(json) => json,
        };

        // set the json
        self.json = json;

        Ok(())
    }

    fn inquire_operation(&self) -> Result<DBOperation, AppError> {
        let q_operation = Question::select("operation")
            .message("What operation would you like to perform?")
            .choices(vec![
                "List",
                "Search",
                "Create",
                "Update",
                "Delete",
                "Exit",
            ])
            .build();

        let answer = requestty::prompt_one(q_operation).unwrap();
        let operation = answer.as_list_item().unwrap().text.as_str();

        match operation {
            "List" => Ok(DBOperation::List),
            "Search" => Ok(DBOperation::Search),
            "Create" => Ok(DBOperation::Create),
            "Update" => Ok(DBOperation::Update),
            "Delete" => Ok(DBOperation::Delete),
            "Exit" => Ok(DBOperation::Exit),
            _ => Err(AppError::new("Invalid operation.")),
        }
    }

    fn create_entry(&mut self, key: &String) -> Result<(), AppError> {
        // get name of whatever website or service this entry is for
        let q_name = Question::input("name")
            .message("What is the name of this entry?")
            .build();

        let answer = requestty::prompt_one(q_name).unwrap();    
        let name = answer.as_string().unwrap();

        // ask for number of fields for this entry
        let q_num_fields = Question::int("num_fields")
            .message("How many fields would you like to add to this entry?")
            .build();

        let answer = requestty::prompt_one(q_num_fields).unwrap();
        let num_fields = answer.as_int().unwrap();

        // get the fields
        let mut field_types: Vec<String> = Vec::new();
        let mut field_data: Vec<Vec<Secret<String>>> = Vec::new();

        for i in 0..num_fields {
            // get the type of the field
            let q_field_type = Question::select(format!("field_type_{}", i))
                .message(format!("What type is field {}?", i + 1))
                .choices(vec![
                    "Username",
                    "Password",
                    "Security Question",
                    "Other",
                ])
                .build();

            let answer = requestty::prompt_one(q_field_type).unwrap();
            let field_type = answer.as_list_item().unwrap().text.as_str();

            // if field is username or password, field data is just one secret string
            if field_type == "Username" || field_type == "Password" {
                let q_field_data = Question::password(format!("field_data_{}", i))
                    .message(format!("Enter data for field {}:", i + 1))
                    .mask('*')
                    .build();

                let answer = requestty::prompt_one(q_field_data).unwrap();
                let field_data_str = Secret::new(String::from(answer.as_string().unwrap()));

                field_data.push(vec![field_data_str.clone()]);
            } else if field_type == "Security Question" {
                // the first part of the field data is the question, the second part is the answer
                let q_field_data_question = Question::input(format!("field_data_question_{}", i))
                    .message(format!("Enter question for field {}:", i + 1))
                    .build();

                let answer = requestty::prompt_one(q_field_data_question).unwrap();
                let field_data_question = Secret::new(String::from(answer.as_string().unwrap()));

                let q_field_data_answer = Question::password(format!("field_data_answer_{}", i))
                    .message(format!("Enter answer for field {}:", i + 1))
                    .mask('*')
                    .build();

                let answer = requestty::prompt_one(q_field_data_answer).unwrap();
                let field_data_answer = Secret::new(String::from(answer.as_string().unwrap()));

                field_data.push(vec![field_data_question.clone(), field_data_answer.clone()]);
            } else {
                // the "other" field contains the name of the custom field and the data
                let q_field_data_name = Question::input(format!("field_data_name_{}", i))
                    .message(format!("Enter name for field {}:", i + 1))
                    .build();

                let answer = requestty::prompt_one(q_field_data_name).unwrap();
                let field_data_name = Secret::new(String::from(answer.as_string().unwrap()));

                let q_field_data_data = Question::password(format!("field_data_data_{}", i))
                    .message(format!("Enter data for field {}:", i + 1))
                    .mask('*')
                    .build();

                let answer = requestty::prompt_one(q_field_data_data).unwrap();
                let field_data_data = Secret::new(String::from(answer.as_string().unwrap()));

                field_data.push(vec![field_data_name.clone(), field_data_data.clone()]);
            }

            field_types.push(String::from(field_type));
        }

        // assemble json, encrypt, and write to file
        let mut json = self.json.clone();

        let mut entry = serde_json::json!({
            "name": name,
            "fields": []
        });

        for i in 0..num_fields {
            let field_type = field_types[i as usize].clone();
            let field_data = field_data[i as usize].clone();

            let mut field = serde_json::json!({
                "type": field_type,
                "data": []
            });

            for j in 0..field_data.len() {
                let field_data_str = field_data[j].clone();

                let mut comb = Vec::new();
                comb.extend_from_slice(key.as_bytes());
                comb.extend_from_slice(name.as_bytes());

                let mut hasher = Sha256::new();
                hasher.update(comb);
                let hash = hasher.finalize();
                let hash_slice = &hash[..12];
                let nonce = GenericArray::clone_from_slice(hash_slice);

                let mut cipher = ChaCha20::new(&hash.into(), &nonce.into());
                let mut encrypted = field_data_str.expose_secret().clone().into_bytes();

                cipher.apply_keystream(&mut encrypted);

                // convert the encrypted bytes to a string of the literal hex
                let encrypted_str = hex::encode(encrypted);

                field["data"].as_array_mut().unwrap().push(serde_json::Value::String(encrypted_str));
            }

            entry["fields"].as_array_mut().unwrap().push(field);
        }

        json["entries"].as_array_mut().unwrap().push(entry);

        // write the json to the file
        //? file is guaranteed to exist at this point
        let mut file = match File::create(&self.path) {
            Err(why) => panic!("Couldn't create database file: {}", why),
            Ok(file) => file,
        };

        match file.write_all(json.to_string().as_bytes()) {
            Err(why) => panic!("Couldn't write to database file: {}", why),
            Ok(_) => (),
        }

        Ok(())
    }
}