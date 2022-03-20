#![feature(proc_macro_hygiene)]
#![feature(allocator_api)]
#![feature(asm)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![feature(c_variadic)]
mod audio;
mod github;
mod looping_audio;
mod unzipper;
mod curl;

use github::GithubRelease;
use hound::WavReader;
use looping_audio::{LoopingAudio, AsyncCommand};
use semver::Version;
// use skyline::{hook, install_hook};
use skyline_web::{Webpage, WebSession};
use std::{thread::{self, JoinHandle}, time, sync::mpsc::{Sender, self}, alloc::{GlobalAlloc}, io::{Read, Write}, path::Path};
use serde::{Serialize, Deserialize};

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
struct Info {
    pub progress: f32,
    pub text: String,
    pub completed: bool
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
    thread::sleep(time::Duration::from_millis(1600));
    // if session.try_send_json(&EndCommand {
    //     contents: "begin_end".to_string()
    // })
    // {
    //     thread::sleep(time::Duration::from_millis(1000));
    // }

    signal.send(AsyncCommand::Quit);
    session.send_json(&EndCommand {
        contents: "exit".to_string()
    });

    session.wait_for_exit();
}


pub fn update_hdr(session: &WebSession, betas: &Vec<GithubRelease>) {

    let name = betas[0].assets[1].name.as_str();
    let release_url = betas[0].assets[1].api_url.as_str();
    let total_size = betas[0].assets[1].size;


    let session2 = session as *const WebSession as u64;
    let mut writer = std::io::BufWriter::with_capacity(
        0x40_0000,
        std::fs::File::create("sd:/download.zip").unwrap()
    );
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
    curl::try_curl("https://api.github.com/repos/HDR-Development/HDR-Releases/releases/assets/59031292", &mut writer, session, tx);
    // curl::try_curl("http://192.168.0.113:8080/HewDraw.Remix.v0.3.7-beta.SWITCH.zip", &mut writer, session);

    drop(writer);

    let mut zip = unzipper::get_zip_archive("sd:/download.zip").unwrap();

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

        let mut file_data = vec![];
        file.read_to_end(&mut file_data).unwrap();
        std::fs::write(Path::new("sd:/").join(file.name()), file_data).unwrap();
    }

    unsafe {
        skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 16);
    }
}

pub fn verify_hdr(session: &WebSession){
    for i in 1..101 {
        let mut info = Info {
            progress: i as f32,
            text: format!("Verifying... {}%", i),
            completed: i >= 99
        };

        let markdown = TEST_MARKDOWN.replace("\\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*", "");

        let parser = pulldown_cmark::Parser::new_ext(markdown.as_str(), pulldown_cmark::Options::all());

        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);

        info.text = html_output.replace("\"", "\\\"").replace("\n", "\\n");

        info.text = info.text.replacen("</a></p>\\n", "</a></p>\\n<hr style=\\\"width:66%\\\"><div id=\\\"changelogContents\\\">", 1);
        info.text += "</div>";

        if session.try_send_json(&info){
            println!("{}% of verifying...", i);
        }

        thread::sleep(time::Duration::from_millis(20)); 
    }
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

    let (nightly_plugins, nightly_romfs, betas) = std::thread::Builder::new()
        .stack_size(0x40_000)
        .spawn(|| {
            let plugins = github::get_all_releases_for_repository("HDR-Development", "HewDraw-Remix").unwrap();
            let romfs = github::get_all_releases_for_repository("HDR-Development", "romfs-release").unwrap();
            let betas = github::get_all_releases_for_repository("HDR-Development", "HDR-Releases").unwrap();
            (plugins, romfs, betas)
        }).unwrap().join().unwrap();

    let mut wav = WavReader::new(std::io::Cursor::new(BGM_WAV)).unwrap();
    let samples: Vec<i16> = wav.samples::<i16>().map(|x| x.unwrap()).collect();
    let audio = LoopingAudio::new(
        samples,
        151836 * 2,
        1776452 * 2,
        0.5,
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
        
        session.show();

        let signal = audio.start();

        loop {
            if let Some(msg) = session.try_recv() {
                match msg.as_str() {
                    "load" => {
                        session.send_json(&VersionInfo {
                            code: get_plugin_version().map(|x| x.to_string()).unwrap_or("???".to_string()),
                            romfs: get_romfs_version().map(|x| x.to_string()).unwrap_or("???".to_string())
                        });
                    }
                    "start" => {
                        end_session_and_launch(&session, signal);
                        break;
                    }
                    "verify_hdr" => verify_hdr(&session),
                    "update_hdr" => update_hdr(&session, &nightly_plugins),
                    "version_select" => verify_hdr(&session),
                    "exit" => {
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
