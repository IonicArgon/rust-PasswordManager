// question stuff
use requestty::Question;

// file stuff
use serde_json;
use std::io::prelude::*;
use std::path::Path;
use std::{fs::File, vec};

// password stuff
use chacha20::{
    cipher::{generic_array::GenericArray, KeyIvInit, StreamCipher},
    ChaCha20,
};
use rand::Rng;
use secrecy::{ExposeSecret, Secret};
use sha2::{Digest, Sha256};

// other stuff
use colored::Colorize;
use hex;

// my stuff
use crate::errors::AppError;

pub enum DBOperation {
    List,
    View,
    Create,
    Update,
    Delete,
    Exit,
}

pub struct DBHandler {
    path: String,
    json: serde_json::Value,
}

pub struct DBField {
    field_type: String,
    field_data: Vec<Secret<String>>,
}

//todo: - extract like half of these operations into separate traits
//todo: - also there's a lot of repeat code here, so maybe extract that into a
//todo:   reusable trait as well
pub trait ProcessDB {
    fn new(path: String) -> Self;
    fn start_up(&mut self) -> Result<(), AppError>;
    fn create_db(&self) -> Result<(), AppError>;
    fn load_db(&mut self) -> Result<(), AppError>;

    fn inquire_operation(&self) -> Result<DBOperation, AppError>;
    fn list_entries(&self) -> Result<(), AppError>;
    fn view_entry(&self, key: &String) -> Result<(), AppError>;
    fn create_entry(&mut self, key: &String) -> Result<(), AppError>;
    fn update_entry(&mut self, key: &String) -> Result<(), AppError>;
    fn delete_entry(&mut self) -> Result<(), AppError>;
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
                    Ok(_) => (),
                    Err(e) => println!("{}", e),
                }
                match self.load_db() {
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
            .choices(vec!["List", "View", "Create", "Update", "Delete", "Exit"])
            .build();

        let answer = requestty::prompt_one(q_operation).unwrap();
        let operation = answer.as_list_item().unwrap().text.as_str();

        match operation {
            "List" => Ok(DBOperation::List),
            "View" => Ok(DBOperation::View),
            "Create" => Ok(DBOperation::Create),
            "Update" => Ok(DBOperation::Update),
            "Delete" => Ok(DBOperation::Delete),
            "Exit" => Ok(DBOperation::Exit),
            _ => Err(AppError::new("Invalid operation.")),
        }
    }

    fn list_entries(&self) -> Result<(), AppError> {
        let entries = self.json["entries"].as_array().unwrap();

        if entries.len() == 0 {
            let no_entries = "No entries.".cyan();
            println!("{}", no_entries);
            return Ok(());
        }

        let entries_title = "Entries:".cyan();
        println!("{}", entries_title);
        for (i, entry) in entries.iter().enumerate() {
            let number = format!("{}.", i + 1).cyan();
            let name = entry["name"].as_str().unwrap();
            println!("{} {}", number, name);
        }
        println!();

        Ok(())
    }

    fn view_entry(&self, key: &String) -> Result<(), AppError> {
        let entries = self.json["entries"].as_array().unwrap();

        if entries.len() == 0 {
            let no_entries = "No entries.".cyan();
            println!("{}", no_entries);
            return Ok(());
        }

        let q_entry = Question::input("entry")
            .message("Entry name: ")
            .build();

        let answer = requestty::prompt_one(q_entry).unwrap();
        let entry_name = answer.as_string().unwrap();

        // find the entry
        let mut entry: Option<&serde_json::Value> = None;
        for e in entries {
            if e["name"].as_str().unwrap() == entry_name {
                entry = Some(e);
                break;
            }
        }

        if entry.is_none() {
            let entry_not_found = "Entry not found.".cyan();
            println!("{}", entry_not_found);
            return Ok(());
        }

        let entry = entry.unwrap();

        // print the entry
        let entry_title = format!("Entry: {}", entry["name"].as_str().unwrap()).cyan();
        println!("{}", entry_title);

        // we have to decrypt the fields before we can print them
        let fields = entry["fields"].as_array().unwrap();
        let mut decrypted_fields: Vec<DBField> = Vec::new();

        for field in fields {
            let field_type = field["type"].as_str().unwrap();

            let encrypted_field_data = field["data"].as_array().unwrap();
            let encrypted_field_nonce = field["nonce"].as_array().unwrap();

            let mut decrypted_field_data: Vec<Secret<String>> = Vec::new();

            for i in 0..encrypted_field_data.len() {
                let encrypted_field_data_str = encrypted_field_data[i].as_str().unwrap();
                let encrypted_field_nonce_str = encrypted_field_nonce[i].as_str().unwrap();

                let encrypted_field_data_bytes = hex::decode(encrypted_field_data_str).unwrap();
                let encrypted_field_nonce_bytes = hex::decode(encrypted_field_nonce_str).unwrap();

                let mut comb = Vec::new();
                comb.extend_from_slice(key.as_bytes());
                comb.extend_from_slice(entry_name.as_bytes());

                let mut hasher = Sha256::new();
                hasher.update(comb);
                let hash = hasher.finalize();

                let nonce = GenericArray::clone_from_slice(&encrypted_field_nonce_bytes);
                let mut cipher = ChaCha20::new(&hash.into(), &nonce.into());

                let mut decrypted_field_data_bytes = encrypted_field_data_bytes.clone();
                cipher.apply_keystream(&mut decrypted_field_data_bytes);

                let decrypted_field_data_str =
                    String::from_utf8(decrypted_field_data_bytes).unwrap();

                decrypted_field_data.push(Secret::new(decrypted_field_data_str));
            }

            decrypted_fields.push(DBField {
                field_type: String::from(field_type),
                field_data: decrypted_field_data,
            });
        }

        // now we can print the fields
        for (i, field) in decrypted_fields.iter().enumerate() {
            let number = format!("{}.", i + 1).cyan();
            let field_type = field.field_type.as_str();

            if field_type == "Username" || field_type == "Password" {
                let field_data = field.field_data[0].expose_secret();
                println!("{} {}: {}", number, field_type, field_data);
            } else if field_type == "Security Question" {
                let field_data_question = field.field_data[0].expose_secret();
                let field_data_answer = field.field_data[1].expose_secret();
                println!(
                    "{} {}: {}",
                    number, field_type, field_data_question
                );
                println!(
                    "{} {}: {}",
                    number, "Answer".cyan(), field_data_answer
                );
            } else {
                let field_data_name = field.field_data[0].expose_secret();
                let field_data_data = field.field_data[1].expose_secret();
                println!(
                    "{} {}: {}",
                    number, field_data_name, field_data_data
                );
            }
        }

        Ok(())
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
                .choices(vec!["Username", "Password", "Security Question", "Other"])
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
                "data": [],
                "nonce": [],
            });

            for j in 0..field_data.len() {
                let field_data_str = field_data[j].clone();

                let mut comb = Vec::new();
                comb.extend_from_slice(key.as_bytes());
                comb.extend_from_slice(name.as_bytes());

                let mut hasher = Sha256::new();
                hasher.update(comb);
                let hash = hasher.finalize();

                let mut nonce = [0u8; 12];
                let mut rng = rand::thread_rng();
                rng.fill(&mut nonce);
                let nonce = GenericArray::clone_from_slice(&nonce);

                let mut cipher = ChaCha20::new(&hash.into(), &nonce.into());
                let mut encrypted = field_data_str.expose_secret().clone().into_bytes();

                cipher.apply_keystream(&mut encrypted);

                // convert the encrypted bytes to a string of the literal hex
                let encrypted_str = hex::encode(encrypted);

                field["data"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::Value::String(encrypted_str));
                field["nonce"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::Value::String(hex::encode(nonce)));
            }

            entry["fields"].as_array_mut().unwrap().push(field);
        }

        json["entries"].as_array_mut().unwrap().push(entry);
        self.json = json.clone();

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

    fn update_entry(&mut self, key: &String) -> Result<(), AppError> {
        let entries = self.json["entries"].as_array().unwrap();

        if entries.len() == 0 {
            let no_entries = "No entries.".cyan();
            println!("{}", no_entries);
            return Ok(());
        }

        let q_entry = Question::input("entry")
            .message("Entry name: ")
            .build();

        let answer = requestty::prompt_one(q_entry).unwrap();
        let entry_name = answer.as_string().unwrap();

        // find the entry
        let mut entry: Option<&serde_json::Value> = None;
        for e in entries {
            if e["name"].as_str().unwrap() == entry_name {
                entry = Some(e);
                break;
            }
        }

        if entry.is_none() {
            let entry_not_found = "Entry not found.".cyan();
            println!("{}", entry_not_found);
            return Ok(());
        }

        let entry = entry.unwrap();

        // print the entry
        let entry_title = format!("Entry: {}", entry["name"].as_str().unwrap()).cyan();
        println!("{}", entry_title);

        // we have to decrypt the fields before we can print them
        let fields = entry["fields"].as_array().unwrap();
        let mut decrypted_fields: Vec<DBField> = Vec::new();

        for field in fields {
            let field_type = field["type"].as_str().unwrap();

            let encrypted_field_data = field["data"].as_array().unwrap();
            let encrypted_field_nonce = field["nonce"].as_array().unwrap();

            let mut decrypted_field_data: Vec<Secret<String>> = Vec::new();

            for i in 0..encrypted_field_data.len() {
                let encrypted_field_data_str = encrypted_field_data[i].as_str().unwrap();
                let encrypted_field_nonce_str = encrypted_field_nonce[i].as_str().unwrap();

                let encrypted_field_data_bytes = hex::decode(encrypted_field_data_str).unwrap();
                let encrypted_field_nonce_bytes = hex::decode(encrypted_field_nonce_str).unwrap();

                let mut comb = Vec::new();
                comb.extend_from_slice(key.as_bytes());
                comb.extend_from_slice(entry_name.as_bytes());

                let mut hasher = Sha256::new();
                hasher.update(comb);
                let hash = hasher.finalize();

                let nonce = GenericArray::clone_from_slice(&encrypted_field_nonce_bytes);
                let mut cipher = ChaCha20::new(&hash.into(), &nonce.into());

                let mut decrypted_field_data_bytes = encrypted_field_data_bytes.clone();

                cipher.apply_keystream(&mut decrypted_field_data_bytes);

                let decrypted_field_data_str =
                    String::from_utf8(decrypted_field_data_bytes).unwrap();

                decrypted_field_data.push(Secret::new(decrypted_field_data_str));
            }

            decrypted_fields.push(DBField {
                field_type: String::from(field_type),
                field_data: decrypted_field_data,
            });
        }

        // ask which field to update
        let q_field = Question::select("field")
            .message("Which field would you like to update?")
            .choices(
                decrypted_fields
                    .iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let number = format!("{}.", i + 1).cyan();
                        let field_type = field.field_type.as_str();
                        format!("{} {}", number, field_type)
                    })
                    .collect::<Vec<String>>(),
            )
            .build();

        let answer = requestty::prompt_one(q_field).unwrap();
        let field_index = answer.as_list_item().unwrap().index;

        // get the new field data
        let field_type = decrypted_fields[field_index].field_type.as_str();

        let mut field_data: Vec<Secret<String>> = Vec::new();

        if field_type == "Username" || field_type == "Password" {
            let q_field_data = Question::password("field_data")
                .message("Enter new data:")
                .mask('*')
                .build();

            let answer = requestty::prompt_one(q_field_data).unwrap();
            let field_data_str = Secret::new(String::from(answer.as_string().unwrap()));

            field_data.push(field_data_str.clone());
        } else if field_type == "Security Question" {
            // the first part of the field data is the question, the second part is the answer
            let q_field_data_question = Question::input("field_data_question")
                .message("Enter new question:")
                .build();

            let answer = requestty::prompt_one(q_field_data_question).unwrap();
            let field_data_question = Secret::new(String::from(answer.as_string().unwrap()));

            let q_field_data_answer = Question::password("field_data_answer")
                .message("Enter new answer:")
                .mask('*')
                .build();

            let answer = requestty::prompt_one(q_field_data_answer).unwrap();
            let field_data_answer = Secret::new(String::from(answer.as_string().unwrap()));

            field_data.push(field_data_question.clone());
            field_data.push(field_data_answer.clone());
        } else {
            // the "other" field contains the name of the custom field and the data
            let q_field_data_name = Question::input("field_data_name")
                .message("Enter new name:")
                .build();

            let answer = requestty::prompt_one(q_field_data_name).unwrap();
            let field_data_name = Secret::new(String::from(answer.as_string().unwrap()));

            let q_field_data_data = Question::password("field_data_data")
                .message("Enter new data:")
                .mask('*')
                .build();

            let answer = requestty::prompt_one(q_field_data_data).unwrap();
            let field_data_data = Secret::new(String::from(answer.as_string().unwrap()));

            field_data.push(field_data_name.clone());
            field_data.push(field_data_data.clone());
        }

        // assemble json, encrypt, and write to file
        let mut json = self.json.clone();
        
        let mut new_entry = serde_json::json!({
            "name": entry_name,
            "fields": []
        });

        for i in 0..fields.len() {
            let field = fields[i].clone();

            let mut new_field = serde_json::json!({
                "type": field["type"],
                "data": [],
                "nonce": [],
            });

            if i == field_index {
                for j in 0..field_data.len() {
                    let field_data_str = field_data[j].clone();

                    let mut comb = Vec::new();
                    comb.extend_from_slice(key.as_bytes());
                    comb.extend_from_slice(entry_name.as_bytes());

                    let mut hasher = Sha256::new();
                    hasher.update(comb);
                    let hash = hasher.finalize();

                    let mut nonce = [0u8; 12];
                    let mut rng = rand::thread_rng();
                    rng.fill(&mut nonce);
                    let nonce = GenericArray::clone_from_slice(&nonce);

                    let mut cipher = ChaCha20::new(&hash.into(), &nonce.into());
                    let mut encrypted = field_data_str.expose_secret().clone().into_bytes();

                    cipher.apply_keystream(&mut encrypted);

                    // convert the encrypted bytes to a string of the literal hex
                    let encrypted_str = hex::encode(encrypted);

                    new_field["data"]
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::Value::String(encrypted_str));
                    new_field["nonce"]
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::Value::String(hex::encode(nonce)));
                }
            } else {
                new_field["data"] = field["data"].clone();
                new_field["nonce"] = field["nonce"].clone();
            }

            new_entry["fields"].as_array_mut().unwrap().push(new_field);
        }

        // replace the old entry with the new entry
        let mut new_entries: Vec<serde_json::Value> = Vec::new();
        for e in entries {
            if e["name"].as_str().unwrap() == entry_name {
                new_entries.push(new_entry.clone());
            } else {
                new_entries.push(e.clone());
            }
        }

        json["entries"] = serde_json::Value::Array(new_entries);
        self.json = json.clone();

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

    fn delete_entry(&mut self) -> Result<(), AppError> {
        let entries = self.json["entries"].as_array().unwrap();

        if entries.len() == 0 {
            let no_entries = "No entries.".cyan();
            println!("{}", no_entries);
            return Ok(());
        }

        let q_entry = Question::input("entry")
            .message("Entry name: ")
            .build();

        let answer = requestty::prompt_one(q_entry).unwrap();
        let entry_name = answer.as_string().unwrap();

        // find the entry
        let mut entry: Option<&serde_json::Value> = None;
        for e in entries {
            if e["name"].as_str().unwrap() == entry_name {
                entry = Some(e);
                break;
            }
        }

        if entry.is_none() {
            let entry_not_found = "Entry not found.".cyan();
            println!("{}", entry_not_found);
            return Ok(());
        }

        let entry = entry.unwrap();

        // print the entry
        let entry_title = format!("Entry: {}", entry["name"].as_str().unwrap()).cyan();
        println!("{}", entry_title);

        // ask if they are sure they want to delete the entry
        let q_delete = Question::confirm("delete")
            .message("Are you sure you want to delete this entry?")
            .build();

        let answer = requestty::prompt_one(q_delete).unwrap();
        let delete = answer.as_bool().unwrap();

        if !delete {
            let delete_cancelled = "Delete cancelled.".cyan();
            println!("{}", delete_cancelled);
            return Ok(());
        }

        // delete the entry from the json
        let mut new_entries: Vec<serde_json::Value> = Vec::new();

        for e in entries {
            if e["name"].as_str().unwrap() != entry_name {
                new_entries.push(e.clone());
            }
        }

        let mut json = self.json.clone();
        json["entries"] = serde_json::Value::Array(new_entries);
        self.json = json.clone();

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
