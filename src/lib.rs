#![feature(proc_macro_hygiene)]
#![feature(allocator_api)]
#![feature(asm)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![feature(c_variadic)]
mod audio;
mod commands;
mod config;
// mod github;
mod looping_audio;
mod unzipper;
mod curl;
mod util;

// use github::GithubRelease;
use hound::WavReader;
use looping_audio::{LoopingAudio, AsyncCommand};
use semver::Version;
// use skyline::{hook, install_hook};
use skyline_web::{Webpage, WebSession};
use std::{thread::{self, JoinHandle}, time, sync::mpsc::{Sender, self}, alloc::{GlobalAlloc}, io::{Read, Write, BufRead}, path::Path};
use serde::{Serialize, Deserialize};
use util::*;

static HTML_TEXT: &str = include_str!("./web/index.html");
static JS_TEXT: &str = include_str!("./web/script.js");
static CSS_TEXT: &str = include_str!("./web/style.css");
static LOGO_PNG: &[u8] = include_bytes!("./web/logo.png");
static TEST_MARKDOWN: &str = include_str!("./web/test.md");
static START_WAV: &[u8] = include_bytes!("./web/start.wav");
static CURSOR_MOVE_WAV: &[u8] = include_bytes!("./web/cursor-move.wav");
static BGM_WAV: &[u8] = include_bytes!("./web/bgm.wav");

#[global_allocator]
static SMASH_ALLOCATOR: skyline::unix_alloc::UnixAllocator = skyline::unix_alloc::UnixAllocator;

extern "C" {
    #[link_name = "_ZN2nn2os15WaitSystemEventEPNS0_15SystemEventTypeE"]
    fn wait_sys_event(event: *mut skyline::nn::os::SystemEventType);
}

#[derive(Serialize, Debug)]
struct Info<'a> {
    pub tag: &'static str,
    pub progress: f32,
    pub text: &'a str,
    pub completed: bool
}

impl<'a> Info<'a> {
    pub fn new(
        progress: f32,
        text: &'a str,
        completed: bool
    ) -> Self
    {
        Self {
            tag: "verify-install",
            progress,
            text,
            completed
        }
    }
}

#[derive(Serialize, Debug)]
struct ExtractInfo<'a> {
    pub tag: &'static str,
    pub file_number: usize,
    pub file_count: usize,
    pub file_name: &'a str
}

impl<'a> ExtractInfo<'a> {
    pub fn new(
        file_number: usize,
        file_count: usize,
        file_name: &'a str
    ) -> Self
    {
        Self {
            tag: "extract-update",
            file_number,
            file_count,
            file_name
        }
    }
}

#[derive(Serialize, Debug)]
struct VerifyInfo<'a> {
    pub tag: &'static str,
    pub file_number: usize,
    pub file_count: usize,
    pub file_name: &'a str
}

impl<'a> VerifyInfo<'a> {
    pub fn new(
        file_number: usize,
        file_count: usize,
        file_name: &'a str
    ) -> Self
    {
        Self {
            tag: "verify-install",
            file_number,
            file_count,
            file_name
        }
    }
}

#[derive(Serialize, Debug)]
pub struct DownloadInfo<'a> {
    pub tag: &'static str,
    pub bps: f64,
    pub bytes_downloaded: f64,
    pub total_bytes: f64,
    pub item_name: &'a str
}

unsafe impl<'a> Send for DownloadInfo<'a> {}
unsafe impl<'a> Sync for DownloadInfo<'a> {}

impl<'a> DownloadInfo<'a> {
    pub fn new(
        bps: f64,
        bytes_downloaded: f64,
        total_bytes: f64,
        item_name: &'a str
    ) -> Self {
        Self {
            tag: "download-update",
            bps,
            bytes_downloaded,
            total_bytes,
            item_name
        }
    }
}

#[derive(Serialize, Debug)]
struct EndCommand {
    pub contents: String
}

pub fn end_session_and_launch(session: &WebSession, signal: Sender<AsyncCommand>) {
    signal.send(AsyncCommand::ChangeVolumeOverTime { new_volume: 0.0, time: 1.6 });
    
    std::thread::sleep(std::time::Duration::from_millis(1500));

    signal.send(AsyncCommand::Quit);
    session.send_json(&commands::Start::new());
    session.wait_for_exit();
}

fn download_file(url: &str, path: &str, session: &WebSession) -> std::io::Result<()> {
    println!("downloading from: {}", url);
    let mut writer = std::io::BufWriter::with_capacity(
        0x40_0000,
        std::fs::File::create(path)?
    );

    let session2 = session as *const WebSession as u64;

    let (tx, rx) = mpsc::channel();
    let ui_updater = std::thread::spawn(move || {
        let session = unsafe { &*(session2 as *const WebSession) };
        loop {
            let mut value = None;
            loop {
                match rx.try_recv() {
                    Ok(v) => value = Some(v),
                    Err(mpsc::TryRecvError::Empty) => break,
                    _ => return
                }
            }

            if let Some(value) = value {
                session.send_json(&value);
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    unsafe {
        skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 2);
    }

    println!("trying curl with url: {}", url);
    curl::try_curl(url, &mut writer, session, tx).unwrap();

    // unsafe {
    //     skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 16);
    // }

    println!("download complete.");    

    ui_updater.join();

    Ok(())
}

pub fn restart(session: &WebSession, signal: Sender<AsyncCommand>) {
    signal.send(AsyncCommand::ChangeVolumeOverTime { new_volume: 0.0, time: 1.6 });
    
    std::thread::sleep(std::time::Duration::from_millis(1500));

    signal.send(AsyncCommand::Quit);
    session.send_json(&commands::Restart::new());
    session.wait_for_exit();
    unsafe {
        skyline::nn::oe::RequestToRelaunchApplication();
    }
}

pub fn update_hdr(session: &WebSession, is_nightly: bool) {
    println!("beginning update!");

    // if the file exists but is empty, remove it
    if Path::new("sd:/hdr-update.zip").exists() && std::fs::metadata("sd:/hdr-update.zip").unwrap().len() < 1000 {
        // todo: get hash of zip and verify the existing zip is valid, else delete it, rather than using file size (lol)
        println!("removing invalid hdr-update.zip, of size {}", std::fs::metadata("sd:/hdr-update.zip").unwrap().len());
        std::fs::remove_file("sd:/hdr-update.zip");
    }

    // check if the file exists, could exist due to extraction failure
    if !Path::new("sd:/hdr-update.zip").exists() {
        println!("we need to download the hdr update. is_nightly = {}", is_nightly);
        let url = if is_nightly {
            "https://github.com/HDR-Development/HDR-Nightlies/releases/latest/download/switch-package.zip"
            // "http://192.168.0.113:8000/package23.zip"
        } else {
            "https://github.com/HDR-Development/HDR-Releases/releases/latest/download/switch-package.zip"
        };

        download_file(url, "sd:/hdr-update.zip", session).unwrap();
        
    }else {
        println!("dont need to download hdr update since there was a zip already on sd...");
    }

    let mut zip = unzipper::get_zip_archive("sd:/hdr-update.zip").unwrap();

    let count = zip.len();

    for file_no in 0..count {
        let mut file = zip.by_index(file_no).unwrap();
        if !file.is_file() {
            continue;
        }

        session.send_json(&ExtractInfo::new(
            file_no,
            count,
            file.name()
        ));

        let path = Path::new("sd:/").join(file.name());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent);
        }

        let mut file_data = vec![];
        file.read_to_end(&mut file_data).unwrap();
        std::fs::write(path, file_data).unwrap();
    }

    // delete the update zip
    if Path::new("sd:/hdr-update.zip").exists() {
        println!("removing hdr-update.zip, of size {}", std::fs::metadata("sd:/hdr-update.zip").unwrap().len());
        std::fs::remove_file("sd:/hdr-update.zip");
    }

    session.send_json(&commands::ChangeHtml::new("play-button", "<div><text>Restart&nbsp;&nbsp;</text></div>"));
    session.send_json(&commands::ChangeMenu::new("main-menu"));
}

fn count_file_lines(file_name: &str) -> i32 {
    let file_handle = match std::fs::File::open(file_name) {
        Ok(hashes) => hashes,
        Err(e) => {
            println!("line error: {:?}", e);
            panic!("could not read file: {}", file_name);
        }
    };
    let lines_iter_initial = std::io::BufReader::new(file_handle).lines();
    let mut line_total = 0;
    for line in lines_iter_initial {
        line_total += 1;
    }
    line_total
}


pub fn verify_hdr(session: &WebSession, is_nightly: bool){
    println!("we need to download the hashes to check. is_nightly = {}", is_nightly);
    let url = if is_nightly {
        "https://github.com/HDR-Development/HDR-Nightlies/releases/latest/download/content_hashes.txt"
    } else {
        "https://github.com/HDR-Development/HDR-Releases/releases/latest/download/content_hashes.txt"
    };

    if !Path::new("sd:/downloads/").exists() {
        std::fs::create_dir("sd:/downloads/");
    }

    download_file(url, "sd:/downloads/content_hashes.txt", session);

    println!("hash download completed. opening file.");

    
    let line_total = count_file_lines("sd:/downloads/content_hashes.txt");
    println!("counted lines: {}", line_total);
    
    let hash_file = match std::fs::File::open("sd:/downloads/content_hashes.txt") {
        Ok(hashes) => hashes,
        Err(e) => {
            println!("line error: {:?}", e);
            return;
        }
    };


    println!("opened file handle.");
    
    let lines_iter = std::io::BufReader::new(hash_file).lines();
    let mut lines: Vec<Vec<String>> = Vec::new();
    let mut i = 0;
    
    for line in lines_iter {
        
        println!("handling: {}", i);

        let line_vec = match line {
            Ok(ref the_line) => {
                println!("line: {}", the_line);
                if the_line.trim() == "" { continue; }
                let mut split = the_line.split(":");
                let path_name = split.next().expect("malformed hash file! No path name???").to_owned();
                let hash = split.next().expect("malformed hash file! No hash value???").to_owned();
                vec![path_name, hash]
            },
            Err(e) => {
                println!("line error!");
                return;
            }
        };
        let line = line.unwrap();
        println!("line: {}", line);

        if line_vec.len() == 0 {
            println!("skipping line: {}!", line);
        }

        let file_name = "sd:".to_owned() + line_vec.get(0).unwrap();
        let file_name = file_name.as_str();
        let hash_value = line_vec.get(1).unwrap();
        

        let mut info = VerifyInfo::new(i, line_total, file_name);

        let file_to_hash = match std::fs::read(file_name) {
            Ok(i) => i,
            Err(e) => {
                println!("error while reading file {}:\n{:?}", file_name, e);
                return;
            }
        };
        let digest = md5::compute(file_to_hash);
        println!("computed md5: {:x}", digest);

        if !(format!("{:x}", digest).as_str() == hash_value) {
            println!("could not verify file {}\nfile's md5: {}\nexpected value: {}",
                file_name,    
                format!("{:x}", digest),
                hash_value
            );
            return;

        }

        let markdown = TEST_MARKDOWN.replace("\\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*", "");

        let parser = pulldown_cmark::Parser::new_ext(markdown.as_str(), pulldown_cmark::Options::all());

        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);
/*
        info.text = html_output.replace("\"", "\\\"").replace("\n", "\\n");

        info.text = info.text.replacen("</a></p>\\n", "</a></p>\\n<hr style=\\\"width:66%\\\"><div id=\\\"changelogContents\\\">", 1);
        info.text += "</div>";
*/
        if session.try_send_json(&info){
            println!("verifying {}", file_name);
        }

        i += 1;        
    }
    session.send_json(&Info::new(1.0, "completed", true));
    println!("verify complete!");
}

#[derive(Serialize, Debug)]
pub struct VersionInfo {
    code: String,
    romfs: String
}

pub fn get_romfs_version() -> Option<Version> {
    std::fs::read_to_string("sd:/ultimate/mods/hdr-assets/ui/romfs_version.txt")
        .ok()
        .map(|x| dbg!(Version::parse(x.as_str().trim().trim_start_matches("v"))).ok())
        .flatten()
}

pub fn get_plugin_version() -> Option<Version> {
    std::fs::read_to_string("sd:/ultimate/mods/hdr/ui/plugin_version.txt")
        .ok()
        .map(|x| Version::parse(x.as_str().trim().trim_start_matches("v")).ok())
        .flatten()
}

#[skyline::main(name = "HDRLauncher")]
pub fn main() {
    let mut config = config::get_config();
    if config::is_enable_skip_launcher(&mut config) {
        return;
    }
    // let handle = start_audio_thread();
    curl::install();

    std::panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>"
            }
        };

        let err_msg = format!("thread has panicked at '{}', {}", msg, location);
        skyline::error::show_error(
            69,
            "Skyline plugin as panicked! Please open the details and send a screenshot to the developer, then close the game.\n",
            err_msg.as_str()
        );
    }));

    let mut wav = WavReader::new(std::io::Cursor::new(BGM_WAV)).unwrap();
    let samples: Vec<i16> = wav.samples::<i16>().map(|x| x.unwrap()).collect();
    let audio = LoopingAudio::new(
        samples,
        151836 * 2,
        1776452 * 2,
        0.5,
        0.0,
        30,
        3.0
    );

    unsafe {
        extern "C" {
            #[link_name = "_ZN2nn2oe24SetExpectedVolumeBalanceEff"]
            fn set_volume_balance(system: f32, applet: f32);
        }

        set_volume_balance(1.0, 1.0);
    }

    let browser_thread = thread::spawn(move || {
        let session = Webpage::new()
            .htdocs_dir("contents")
            .file("index.html", &HTML_TEXT)
            .file("script.js", &JS_TEXT)
            .file("style.css", &CSS_TEXT)
            .file("logo.png", &LOGO_PNG)
            .file("cursor-move.wav", &CURSOR_MOVE_WAV)
            .file("start.wav", &START_WAV)
            .background(skyline_web::Background::Default)
            .boot_display(skyline_web::BootDisplay::Black)
            .open_session(skyline_web::Visibility::InitiallyHidden).unwrap();
        
        

        let signal = audio.start();

        loop {
            if let Some(msg) = session.try_recv() {
                match msg.as_str() {
                    "load" => {
                        let plugin_version = get_plugin_version().map(|x| x.to_string()).unwrap_or("???".to_string());
                        let romfs_version = get_romfs_version().map(|x| x.to_string()).unwrap_or("???".to_string());
                        session.send_json(&VersionInfo {
                            code: plugin_version.clone(),
                            romfs: romfs_version.clone()
                        });
                        if !Path::new("sd:/ultimate/mods/hdr").exists() {
                            session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Install&nbsp;&nbsp;</text></div>"));
                            session.send_json(&commands::ChangeHtml::new("play-button", "<div><text>Vanilla&nbsp;&nbsp;</text></div>"));
                            session.send_json(&commands::ChangeMenu::new("main-menu"));
                        }
                    }
                    "start" => {
                        println!("starting hdr...");
                        end_session_and_launch(&session, signal);
                        break;
                    }
                    "restart" => {
                        restart(&session, signal);
                        break;
                    }
                    "verify_hdr" => verify_hdr(&session, config::is_enable_nightly_builds(&config)),
                    "update_hdr" => update_hdr(&session, config::is_enable_nightly_builds(&config)),
                    "version_select" => verify_hdr(&session,config::is_enable_nightly_builds(&config)),
                    "exit" => {
                        println!("exiting!");
                        unsafe { skyline::nn::oe::ExitApplication() }
                    },
                    x if x.starts_with("log:") => {
                        println!("{}", x.trim_start_matches("log:"));
                    },
                    _ => {}
                };

                session.show();
            }
        }
    });

    // End thread so match can actually start
    browser_thread.join();
}
