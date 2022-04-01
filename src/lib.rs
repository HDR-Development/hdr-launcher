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
use std::{thread::{self, JoinHandle}, time, sync::mpsc::{Sender, self}, alloc::{GlobalAlloc}, io::{Read, Write, BufRead}, path::{Path, PathBuf}};
use serde::{Serialize, Deserialize};
use util::*;
use std::collections::HashMap;

static HTML_TEXT: &str = include_str!("./web/index.html");
static JS_TEXT: &str = include_str!("./web/script.js");
static CSS_TEXT: &str = include_str!("./web/style.css");
static LOGO_PNG: &[u8] = include_bytes!("./web/logo.png");
static TEST_MARKDOWN: &str = include_str!("./web/test.md");
static START_WAV: &[u8] = include_bytes!("./web/start.wav");
static CURSOR_MOVE_WAV: &[u8] = include_bytes!("./web/cursor-move.wav");
static FAILURE_WAV: &[u8] = include_bytes!("./web/failure.wav");
static BGM_WAV: &[u8] = include_bytes!("./web/bgm.wav");

extern "C" {
    #[link_name = "_ZN2nn2oe23BeginBlockingHomeButtonEv"]
    fn block_home_button();

    #[link_name = "_ZN2nn2oe35BeginBlockingHomeButtonShortPressedEv"]
    fn block_home_button_short();

    #[link_name = "_ZN2nn2oe21EndBlockingHomeButtonEv"]
    fn allow_home_button();

    #[link_name = "_ZN2nn2oe33EndBlockingHomeButtonShortPressedEv"]
    fn allow_home_button_short();
}

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

fn download_file(url: &str, path: &str, session: &WebSession, file_name: String) -> std::io::Result<()> {
    // unsafe {
    //     block_home_button();
    //     block_home_button_short();
    // }
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
            let mut value: Option<DownloadInfo> = None;
            loop {
                match rx.try_recv() {
                    Ok(v) => value = Some(v),
                    Err(mpsc::TryRecvError::Empty) => break,
                    _ => return
                }
            }

            if let Some(mut value) = value {
                value.item_name = file_name.as_str();
                println!("{}", session.try_send_json(&value));
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

    // unsafe {
    //     allow_home_button();
    //     allow_home_button_short();
    // }

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
    if !Path::new("sd:/downloads/").exists() {
        std::fs::create_dir("sd:/downloads/");
    }

    // if the file exists but is empty, remove it
    if Path::new("sd:/downloads/hdr-update.zip").exists() && std::fs::metadata("sd:/downloads/hdr-update.zip").unwrap().len() < 100 {
        // todo: get hash of zip and verify the existing zip is valid, else delete it, rather than using file size (lol)
        println!("removing invalid hdr-update.zip, of size {}", std::fs::metadata("sd:/downloads/hdr-update.zip").unwrap().len());
        std::fs::remove_file("sd:/downloads/hdr-update.zip");
    }

    // check if the file exists, could exist due to extraction failure
    if !Path::new("sd:/downloads/hdr-update.zip").exists() {
        println!("we need to download the hdr update. is_nightly = {}", is_nightly);
        let url = if is_nightly {
            "https://github.com/HDR-Development/HDR-Nightlies/releases/latest/download/switch-package.zip"
            // "http://192.168.0.113:8000/package23.zip"
        } else {
            "https://github.com/HDR-Development/HDR-Releases/releases/latest/download/switch-package.zip"
        };

        download_file(url, "sd:/downloads/hdr-update.zip", session, "release archive".to_string()).unwrap();  
    } else {
        println!("dont need to download hdr update since there was a zip already on sd...");
    }

    if !Path::new("sd:/downloads/hdr-changelog.md").exists() {
        let url = if is_nightly {
            "https://github.com/HDR-Development/HDR-Nightlies/releases/latest/download/CHANGELOG.md"
            // "http://192.168.0.113:8000/package23.zip"
        } else {
            "https://github.com/HDR-Development/HDR-Releases/releases/latest/download/CHANGELOG.md"
        };

        download_file(url, "sd:/downloads/hdr-changelog.md", session, "release changelog".to_string()).unwrap();
    }

    let mut zip = unzipper::get_zip_archive("sd:/downloads/hdr-update.zip").unwrap();

    let count = zip.len();

    for file_no in 0..count {
        let mut file = zip.by_index(file_no).unwrap();
        if !file.is_file() {
            continue;
        }

        session.send_json(&ExtractInfo::new(
            file_no,
            count,
            file.name().trim_start_matches("ultimate/mods/")
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
    if Path::new("sd:/downloads/hdr-update.zip").exists() {
        println!("removing /downloads/hdr-update.zip, of size {}", std::fs::metadata("sd:/downloads/hdr-update.zip").unwrap().len());
        std::fs::remove_file("sd:/downloads/hdr-update.zip");
    }

    if Path::new("sd:/downloads/hdr-changelog.md").exists() {
        let text = std::fs::read_to_string("sd:/downloads/hdr-changelog.md").unwrap();
        let markdown = text.replace("\\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*", "");
    
        let parser = pulldown_cmark::Parser::new_ext(markdown.as_str(), pulldown_cmark::Options::all());
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);

        html_output = html_output.replace("\"", "\\\"").replace("\n", "\\n");
        html_output = html_output.replacen(",/a></p>\\n", "</a></p>\\n<hr style=\\\"width:66%\\\"><div id=\\\"changelogContents\\\">", 1);

        std::fs::remove_file("sd:/downloads/hdr-changelog.md");

        session.try_send_json(&commands::ChangeHtml::new("changelog", html_output.as_str()));
        session.try_send_json(&commands::ChangeMenu::new("text-view"));
    } else {
        session.try_send_json(&commands::ChangeMenu::new("main-menu"));
    }
    session.try_send_json(&commands::ChangeHtml::new("play-button", "<div><text>Restart&nbsp;&nbsp;</text></div>"));    
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

enum VerifyResult {
    Success(commands::VerifyInfo),
    MissingFile(PathBuf),
    IncorrectFile(PathBuf)
}

pub fn verify_hdr(session: &WebSession, is_nightly: bool) {
    println!("we need to download the hashes to check. is_nightly = {}", is_nightly);

    let version = match get_plugin_version() {
        Some(v) => {
            println!("version is : {}", v);
            v
        },
        None => {
            println!("could not determine current version!");
            let html_output = "<div id=\\\"changelogContents\\\">Could not determine current version! Cannot validate! </div>".to_string();
            show_verification_results(html_output.as_str(), session);
            return;
        }
    };

    let url = if is_nightly {
        "https://github.com/HDR-Development/HDR-Nightlies/releases/latest/download/content_hashes.txt"
    } else {
        "https://github.com/HDR-Development/HDR-Releases/releases/latest/download/content_hashes.txt"
    };

    if !Path::new("sd:/downloads/").exists() {
        std::fs::create_dir("sd:/downloads/");
    }

    download_file(url, "sd:/downloads/content_hashes.txt", session, "hash list".to_string()).unwrap();

    let file_data = std::fs::read_to_string("sd:/downloads/content_hashes.txt").unwrap();

    let (tx, rx) = mpsc::channel();

   
    let handle = std::thread::spawn(move || {
        let lines = file_data.lines();
        let line_count = lines.count();
        let mut file_hashes = Vec::<(String, String)>::new();
        
        let hdr_folders = vec![
            "sd:/ultimate/mods/hdr",
            "sd:/ultimate/mods/hdr-stages",
            "sd:/ultimate/mods/hdr-assets",
        ];

        let mut deleted_files: Vec<String> = vec![];

        /// file_path : (hash, is_present)
        let mut files_map = HashMap::new();
        let mut found_count = 0;
        
        // collect all of the allowed paths
        for (idx, line) in file_data.lines().into_iter().enumerate() {
            let mut split = line.split(":");
            let path = match split.next() {
                Some(x) => x,
                None => {
                    println!("Line {} was malformed!", line);
                    continue;
                }
            };
            let hash = match split.next() {
                Some(x) => x,
                None => {
                    println!("Line {} was malformed!", line);
                    continue;
                }
            };
    
            let path = format!("sd:{}", path);
            //println!("adding to map: {}", path.to_owned());
            files_map.insert(path.to_owned(), (hash.to_owned(), false));
        }

        // remove any undesired files during verification
        for dir in hdr_folders {
            let walk = walkdir::WalkDir::new(dir);
            for entry in walk {
                // handle any errors
                if entry.is_err() {
                    println!("error during walk: {:?}", entry.err().unwrap());
                    continue;
                }

                let entry = entry.unwrap();
                let entry_string = entry.path().as_os_str().to_str().unwrap();

                // skip directories
                if entry.file_type().is_dir() {
                    continue;
                }
                
                // check if this file path is present in the list of expected files
                //println!("checking for: {}", entry_string.to_string());
                let mut found = files_map.contains_key(&entry_string.to_string());
                
                if !found {
                    //println!("entry should be removed: {}", entry_string);
                    deleted_files.push(entry_string.to_string());
                } else {
                    found_count += 1;

                    let hash = files_map.get(&entry_string.to_string()).unwrap().0.clone();
                    let previously_found = files_map.get(&entry_string.to_string()).unwrap().1;

                    // update this file with true - it has been found already
                    files_map.insert(entry_string.to_string(), (hash.clone(), true));

                    // check its hash                    
                    let file_path = Path::new(entry_string);
                    let data = match std::fs::read(file_path) {
                        Ok(data) => data,
                        Err(e) => {
                            let _ = tx.send(VerifyResult::MissingFile(file_path.to_path_buf()));
                            continue;
                        }
                    };
                    let digest = md5::compute(data);
                    let digest = format!("{:x}", digest);
                    if digest != hash {
                        let _ = tx.send(VerifyResult::IncorrectFile(file_path.to_path_buf()));
                    } else {
                        let _ = tx.send(VerifyResult::Success(commands::VerifyInfo::new(found_count, line_count, entry_string.to_string().trim_start_matches("sd:/ultimate/mods/").to_string())));
                    }
                }
            }
        }

        // collect the rest of the expected (hashed) files which may not have already been encountered during the walkdir
        for (path, (hash, already_found)) in files_map {
            if already_found {
                continue;
            }
            found_count += 1;

            // check the hash                    
            let file_path = Path::new(&path);
            let data = match std::fs::read(file_path) {
                Ok(data) => data,
                Err(e) => {
                    let _ = tx.send(VerifyResult::MissingFile(file_path.to_path_buf()));
                    continue;
                }
            };
            let digest = md5::compute(data);
            let digest = format!("{:x}", digest);
            if digest != hash {
                let _ = tx.send(VerifyResult::IncorrectFile(file_path.to_path_buf()));
            } else {
                let _ = tx.send(VerifyResult::Success(commands::VerifyInfo::new(found_count, line_count, path.to_string().trim_start_matches("sd:/ultimate/mods/").to_string())));
            }
        }

        println!("deleting unwanted files");
        for file in deleted_files {
            println!("deleting file: {}", file);
            std::fs::remove_file(file);
        }

        println!("verify complete.");

    });


    let mut missing_files = vec![];
    let mut bad_files = vec![];
    loop {
        let mut value = None;
        let mut exit = false;
        loop {
            match rx.try_recv() {
                Ok(v) => {
                    if matches!(&v, VerifyResult::Success(_)) {
                        value = Some(v);
                        continue;
                    } else {
                        value = Some(v);
                        break;
                    }
                },
                Err(mpsc::TryRecvError::Empty) => break,
                _ => {
                    exit = true;
                    break;
                }
            }
        }

        if exit {
            break;
        }

        if let Some(value) = value {
            match value  {
                VerifyResult::Success(info) => session.send_json(&info),
                VerifyResult::MissingFile(path) => missing_files.push(path),
                VerifyResult::IncorrectFile(path) => bad_files.push(path),
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let mut html_output = String::new();

    if missing_files.is_empty() && bad_files.is_empty() {
        html_output = "<div id=\\\"changelogContents\\\">There were no issues found with the installation!</div>".to_string();
    } else {
        html_output = "<div id=\\\"changelogContents\\\">".to_string();
        if !missing_files.is_empty() {
            html_output += "Files not found:<br><ul>";
            for file in missing_files {
                html_output += format!("<li><b>{}</b></li>", file.display()).as_str();
            }
            html_output += "</ul><br><br>";
        }
        if !bad_files.is_empty() {
            html_output += "Corrupted files:<br><ul>";
            for file in bad_files {
                html_output += format!("<li><b>{}</b></li>", file.display()).as_str();
            }
            html_output += "</ul><br><br>"
        }
        html_output += "</div>";
    }

    show_verification_results(&html_output, session);
}

fn show_verification_results(html: &str, session: &WebSession) {
    session.try_send_json(&commands::ChangeHtml::new("changelog", html));
    session.try_send_json(&commands::ChangeMenu::new("text-view"));
    session.try_send_json(&commands::ChangeHtml::new("title", "HDR Launcher > Verification Results"));
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
    std::fs::read_to_string("sd:/ultimate/mods/hdr/ui/hdr_version.txt")
        .ok()
        .map(|x| Version::parse(
            x.as_str()
            .trim()
            .trim_start_matches("v")
        ).ok())
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
            .file("failure.wav", &FAILURE_WAV)
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
                        session.send_json(&commands::SetOptionStatus::new(
                            "nightlies",
                            config::is_enable_nightly_builds(&config)
                        ));
                        session.send_json(&commands::SetOptionStatus::new(
                            "skip_on_launch",
                            config::is_enable_skip_launcher(&config)
                        ));
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
                    x if x.starts_with("toggle:") => {
                        let option = x.trim_start_matches("toggle:");
                        if option == "nightlies" {
                            let is_enabled = config::is_enable_nightly_builds(&config);
                            config::enable_nightlies(&mut config, !is_enabled);
                            session.send_json(&commands::SetOptionStatus::new("nightlies", !is_enabled));
                        } else if option == "skip_on_launch" {
                            let is_enabled = config::is_enable_skip_launcher(&config);
                            config::enable_skip_launcher(&mut config, !is_enabled);
                            session.send_json(&commands::SetOptionStatus::new("skip_on_launch", !is_enabled));
                        }
                    }
                    _ => {}
                };

                session.show();
            }
        }
    });

    // End thread so match can actually start
    browser_thread.join();
}
