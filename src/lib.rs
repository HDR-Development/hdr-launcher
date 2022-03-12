#![feature(proc_macro_hygiene)]

// use skyline::{hook, install_hook};
use skyline_web::{Webpage, WebSession};
use std::{thread, time};
use serde::{Serialize, Deserialize};

static HTML_TEXT: &str = include_str!("./web/index.html");
static JS_TEXT: &str = include_str!("./web/script.js");
static CSS_TEXT: &str = include_str!("./web/style.css");

#[derive(Serialize, Debug)]
struct Info {
    pub progress: f32,
    pub text: String,
    pub completed: bool
}

pub fn update_hdr(session: &WebSession){
    for i in 1..101 {
        let info = Info {
            progress: i as f32,
            text: format!("Updating... {}%", i),
            completed: i >= 99
        };

        if session.try_send_json(&info){
            println!("{}% of updating...", i);
        }

        thread::sleep(time::Duration::from_millis(50)); 
    }
}

pub fn verify_hdr(session: &WebSession){
    for i in 1..101 {
        let info = Info {
            progress: i as f32,
            text: format!("Verifying... {}%", i),
            completed: i >= 99
        };

        if session.try_send_json(&info){
            println!("{}% of verifying...", i);
        }

        thread::sleep(time::Duration::from_millis(20)); 
    }
}

#[skyline::main(name = "HDRLauncher")]
pub fn main() {
    let browser_thread = thread::spawn(|| {
        let session = Webpage::new()
            .htdocs_dir("contents")
            .file("index.html", &HTML_TEXT)
            .file("script.js", &JS_TEXT)
            .file("style.css", &CSS_TEXT)
            .background(skyline_web::Background::Default)
            .boot_display(skyline_web::BootDisplay::Default)
            .open_session(skyline_web::Visibility::InitiallyHidden).unwrap();
        
        session.show();

        loop {
            if let Some(msg) = session.try_recv() {
                println!("{}", msg);
    
                match msg.as_str() {
                    "start" => {
                        session.wait_for_exit();
                        break;
                    }
                    "verify_hdr" => verify_hdr(&session),
                    "update_hdr" => update_hdr(&session),
                    "version_select" => verify_hdr(&session),
                    "exit" => {
                        unsafe { skyline::nn::oe::ExitApplication() }
                    },
                    _ => { }
                };

                session.show();
            }
        }
    });

    // End thread so match can actually start
    browser_thread.join();
}
