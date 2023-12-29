// question stuff
use requestty::Question;

// file stuff
use serde_json;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// password stuff
//todo: add some kind of symmetric encryption

// my stuff
use crate::errors::AppError;

pub struct DBHandler {
    path: String,
    json: serde_json::Value,
}

pub enum DBOperation {
    Create,
    Read,
    Update,
    Delete,
    Exit,
}

pub trait ProcessDB {
    fn new(path: String) -> Self;
    fn start_up(&mut self) -> Result<(), AppError>;
    fn create_db(&self) -> Result<(), AppError>;
    fn load_db(&mut self) -> Result<(), AppError>;

    fn inquire_operation(&self) -> Result<DBOperation, AppError>;
    //fn create_entry(&mut self) -> Result<(), AppError>;
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
                "Create",
                "Read",
                "Update",
                "Delete",
                "Exit",
            ])
            .build();

        let answer = requestty::prompt_one(q_operation).unwrap();
        let operation = answer.as_list_item().unwrap().text.as_str();

        match operation {
            "Create" => Ok(DBOperation::Create),
            "Read" => Ok(DBOperation::Read),
            "Update" => Ok(DBOperation::Update),
            "Delete" => Ok(DBOperation::Delete),
            "Exit" => Ok(DBOperation::Exit),
            _ => Err(AppError::new("Invalid operation.")),
        }
    }
}