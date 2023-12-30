use colored::Colorize;
use std::process::Command;
use chrono::prelude::*;

pub trait HandleLogo {
    fn print_logo(&self);
}

pub struct LogoHandler {}

impl HandleLogo for LogoHandler {
    fn print_logo(&self) {
        let logo = r#"
          $$$$$$$\                               $$\      $$\                     
          $$  __$$\                              $$$\    $$$ |                    
$$$$$$\   $$ |  $$ |$$$$$$\   $$$$$$$\  $$$$$$$\ $$$$\  $$$$ | $$$$$$\  $$$$$$$\  
$$  __$$\ $$$$$$$  |\____$$\ $$  _____|$$  _____|$$\$$\$$ $$ | \____$$\ $$  __$$\ 
$$ |  \__|$$  ____/ $$$$$$$ |\$$$$$$\  \$$$$$$\  $$ \$$$  $$ | $$$$$$$ |$$ |  $$ |
$$ |      $$ |     $$  __$$ | \____$$\  \____$$\ $$ |\$  /$$ |$$  __$$ |$$ |  $$ |
$$ |      $$ |     \$$$$$$$ |$$$$$$$  |$$$$$$$  |$$ | \_/ $$ |\$$$$$$$ |$$ |  $$ |
\__|      \__|      \_______|\_______/ \_______/ \__|     \__| \_______|\__|  \__|
"#;

        let _ = Command::new("clear").status();

        for line in logo.lines() {
            println!("{}", line.green());
        }

        let now = Local::now();
        let year = now.year().to_string();

        let postline1 = "A barely Rusty, ok password manager.".green();
        let postline2 = format!("Version: {}", env!("CARGO_PKG_VERSION")).green();
        let postline3 = format!("© IonicArgon {}", year).yellow();

        println!();
        println!("{}", postline1);
        println!("{}", postline2);
        println!("{}", postline3);
        println!();
    }
}
