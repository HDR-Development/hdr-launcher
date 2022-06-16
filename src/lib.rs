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
use arcropolis_api::{self, ApiVersion, get_api_version};
use looping_audio::{LoopingAudio, AsyncCommand};
use semver::Version;
use skyline::nn::hid::NpadHandheldState;
use skyline_config::{StorageHolder, SdCardStorage};
// use skyline::{hook, install_hook};
use skyline_web::{Webpage, WebSession};
use std::{thread::{self, JoinHandle}, time, sync::mpsc::{Sender, self}, alloc::{GlobalAlloc}, io::{Read, Write, BufRead, Error, ErrorKind}, path::{Path, PathBuf}};
use serde::{Serialize, Deserialize};
use util::*;
use std::collections::HashMap;
use util::show_results;

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

pub fn is_update_available(session: Option<&WebSession>, is_nightly: bool) -> bool {
    if !Path::new("sd:/ultimate/hdr/downloads/").exists() {
        std::fs::create_dir_all("sd:/ultimate/hdr/downloads/");
    }
    
    let mut current = match util::get_plugin_version() {
        Some(v) => {
            println!("version is : {}", v);
            v
        },
        None => {
            println!("could not determine current version!");
            // let html_output = "<div id=\\\"changelogContents\\\">Could not determine current version! We will perform a full install! </div>";
            // show_results(html_output, session);
            Version::new(0,0,0)
        }
    };

    let latest = match util::get_latest_version(session, is_nightly) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not determine latest version due to error: {:?}", e);
            return false;
        }
    };

    // compare versions
    if latest < current && latest.pre == current.pre {
        println!("Somehow your current version ({}) is newer than the latest on the github releases ({}). This should not be possible.", current, latest);
        false
    } else if current == latest {
        println!("You are already on the latest! Current install: {}, Latest: {}", current, latest);
        false
    } else {
        println!("updates are available!");
        true
    }
}


/// switches the hdr installation between nightly and beta
pub fn switch_install_type(session: &WebSession, config: &StorageHolder<SdCardStorage>) {
    let going_to_nightly = config::is_enable_nightly_builds(&config);
    println!("beginning switch! going_to_nightly: {}", going_to_nightly);

    let target_type = match going_to_nightly {
        true => "nightly",
        false => "beta"
    };

    if !Path::new("sd:/ultimate/hdr/downloads/").exists() {
        std::fs::create_dir_all("sd:/ultimate/hdr/downloads/");
    }
    let mut current = match util::get_plugin_version() {
        Some(v) => {
            println!("version is : {}", v);
            v
        },
        None => {
            println!("could not determine current version!");
            let html_output = "<div id=\\\"changelogContents\\\">Could not determine current version! You will need to fix your installation! </div>";
            show_results(html_output, session);
            Version::new(0,0,0)
        }
    };

    if current == Version::new(0, 0, 0) {
        return
    }

    // delete the old update zip
    if Path::new("sd:/ultimate/hdr/downloads/hdr-update.zip").exists() {
        println!("removing /downloads/hdr-update.zip, of size {}", std::fs::metadata("sd:/ultimate/hdr/downloads/hdr-update.zip").unwrap().len());
        std::fs::remove_file("sd:/ultimate/hdr/downloads/hdr-update.zip");
    }

    let zip_name = match going_to_nightly {
        true => "to-nightly.zip",
        false => "to-beta.zip"
    };

    // try and see if we can get the zip for switching
    let can_quick_switch = match util::download_from_version(!going_to_nightly, zip_name, "hdr-update.zip", current.clone(), Some(session)) {
        Ok(i) => {
            println!("we can switch easily!");
            true
        },
        Err(e) => {
            println!("we cannot switch easily! Error: {:?}", e);
            false
        }
    };

    // if there is no switching zip available for the current version, then we will just have to perform a full install
    if !can_quick_switch {
        match util::download_from_latest(going_to_nightly, "switch-package.zip", "hdr-update.zip", Some(session)) {
            Ok(i) => println!("latest full download complete."),
            Err(e) => {
                println!("could not get full download! Error: {:?}", e);
                let html_output = format!(
                    "<div id=\\\"changelogContents\\\">We could not switch easily, and we could not get the latest full install download either! Version: {}, Update failed. Error: {:?} </div>", current, e);
                show_results(html_output.as_str(), session);
                return;
            }
        }
    }

    // unzip the update, and then delete the zip
    unzip_update(&session);

    if verify_hdr(session, &config).is_ok() {
        let html_output = format!(
            "<div id=\\\"changelogContents\\\">Switching to {} was successful!", target_type);
        show_results(html_output.as_str(), session);
    
        session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Update&nbsp;&nbsp;</text>"));
    }

    session.try_send_json(&commands::ChangeHtml::new("play-button", "<div><text>Restart&nbsp;&nbsp;</text></div>"));
    set_nightly_toggle_button(config, session);
}

fn set_nightly_toggle_button(config: &StorageHolder<SdCardStorage>, session: &WebSession) {
    let is_nightly = config::is_enable_nightly_builds(&config);
    if is_nightly {
        session.try_send_json(&commands::ChangeHtml::new("switch-button", "<div class='button-background'><h2>&nbsp;&nbsp;Switch to Beta&nbsp;&nbsp;</h2></div>"));
    } else {
        session.try_send_json(&commands::ChangeHtml::new("switch-button", "<div class='button-background'><h2>&nbsp;&nbsp;Switch to Nightly&nbsp;&nbsp;</h2></div>"));
    }
}

/// unzips and then deletes the update zip
pub fn unzip_update(session: &WebSession) {
    session.send_json(&commands::ChangeHtml::new("progressText", "Parsing package metadata...<br>Do not exit the menu"));

    let mut zip = match unzipper::get_zip_archive("sd:/ultimate/hdr/downloads/hdr-update.zip") {
        Ok(zip) => zip,
        Err(_) => {
            session.send_json(&commands::ChangeHtml::new("progressText", "Failed to parse package metadata..."));
            return;
        }
    };

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
    if Path::new("sd:/ultimate/hdr/downloads/hdr-update.zip").exists() {
        println!("removing /downloads/hdr-update.zip, of size {}", std::fs::metadata("sd:/ultimate/hdr/downloads/hdr-update.zip").unwrap().len());
        std::fs::remove_file("sd:/ultimate/hdr/downloads/hdr-update.zip");
    }
}


/// updates the hdr installation
pub fn update_hdr(session: &WebSession, config: &StorageHolder<SdCardStorage>, needs_full_redownload: bool) {

    if !util::is_online() {
        let html_output = "<div id=\\\"changelogContents\\\"> We cannot update, as you are not currently connected to the internet. </div>";
        show_results(html_output, session);
        return;
    }

    let is_nightly = config::is_enable_nightly_builds(&config);
    println!("beginning update! is_nightly: {}, full redownload: {}", is_nightly, needs_full_redownload);
    if !Path::new("sd:/ultimate/hdr/downloads/").exists() {
        std::fs::create_dir_all("sd:/ultimate/hdr/downloads/");
    }
    let mut final_changelog = String::new();
    
    let mut current = match util::get_plugin_version() {
        Some(v) => {
            println!("version is : {}", v);
            v
        },
        None => {
            println!("could not determine current version!");
            // let html_output = "<div id=\\\"changelogContents\\\">Could not determine current version! We will perform a full install! </div>";
            // show_results(html_output, session);
            Version::new(0,0,0)
        }
    };

    // if we know we need to do a full redownload anyway, then mark our version as invalid
    if needs_full_redownload {
        current = Version::new(0,0,0);
    }

    let latest = match util::get_latest_version(Some(session), is_nightly) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not determine latest version due to error: {:?}", e);
            let html_output = format!("<div id=\\\"changelogContents\\\">Could not determine latest version! Please connect to the internet to update. Error: {:?} </div>", e);
            show_results(html_output.as_str(), session);
            return;
        }
    };

    // compare versions
    if latest < current && latest.pre == current.pre {
        println!("Somehow your current version ({}) is newer than the latest on the github releases ({}). This should not be possible.", current, latest);
        let html_output = format!("<div id=\\\"changelogContents\\\">Somehow your current version ({}) is newer than the latest on the github releases ({}). This should not be possible. We cannot update in this state. </div>", current, latest);
        show_results(html_output.as_str(), session);
        return;
    } else if current == latest {
        println!("You are already on the latest! Current install: {}, Latest: {}", current, latest);
        let html_output = format!("<div id=\\\"changelogContents\\\">You are already on the latest! Current install: {}, Latest: {}. No Update is necessary. </div>", current, latest);
        show_results(html_output.as_str(), session);
        return;
    } else {
        println!("we need to update. Current: {}, Latest: {}", current, latest);
    }

    // walk the chain forever potentially (we have breaking conditions below)
    while (current != latest) {
        
        // delete the old update zip
        if Path::new("sd:/ultimate/hdr/downloads/hdr-update.zip").exists() {
            println!("removing /downloads/hdr-update.zip, of size {}", std::fs::metadata("sd:/ultimate/hdr/downloads/hdr-update.zip").unwrap().len());
            std::fs::remove_file("sd:/ultimate/hdr/downloads/hdr-update.zip");
        }


        // try and see if we can get the upgrade.zip file to "walk the chain" of updates.
        let can_upgrade = match util::download_from_version(is_nightly, "upgrade.zip", "hdr-update.zip", current.clone(), Some(session)) {
            Ok(i) => {
                println!("we can walk the chain!");
                true
            },
            Err(e) => {
                println!("error while checking for upgrade.zip: {:?}", e);
                false
            }
        };
        
        // if there is no upgrade package available for the current version, then we will just have to perform a full
        if !can_upgrade {
            match util::download_from_latest(is_nightly, "switch-package.zip", "hdr-update.zip", Some(session)) {
                Ok(i) => println!("latest full download complete."),
                Err(e) => {
                    println!("could not get full download! Error: {:?}", e);
                    let html_output = format!(
                        "<div id=\\\"changelogContents\\\">Could not get full install download! Current version: {}, Latest: {}. Update failed. Error: {:?} </div>", current, latest, e);
                    show_results(html_output.as_str(), session);
                    return;
                }
            }
        }

        // unzip the update, and then delete the zip
        unzip_update(&session);

        // get the newly installed version
        let new_version = match util::get_plugin_version() {
            Some(v) => {
                println!("version is : {}", v);
                v
            },
            None => {
                println!("could not determine current version!");
                let html_output = "<div id=\\\"changelogContents\\\">Could not determine new version! Please try to update again. </div>";
                show_results(html_output, session);
                return;
            }
        };

        // get the current changelog for the version we just installed, and append it
        final_changelog = format!("{}\n{}", final_changelog, match util::download_from_version(is_nightly, "CHANGELOG.md", "hdr-changelog.md", new_version.clone(), Some(session)) {
            Ok(i) => {
                // append the eventual changelog text
                if Path::new("sd:/ultimate/hdr/downloads/hdr-changelog.md").exists() {
                    match std::fs::read_to_string("sd:/ultimate/hdr/downloads/hdr-changelog.md") {
                        Ok(s) => s,
                        Err(e) => format!("Error while getting changelog for new version: {}", new_version)

                    }
                } else {
                    format!("Changelog for {} was downloaded but could not be found? This should be impossible. Please report this.", new_version)
                }
            },
            Err(e) => {
                format!("Changelog for {} could not be downloaded. Please report this.", new_version)
            }
        });

        if new_version == latest {
            break;
        }

        current = new_version;
    }


    // display the built up changelog
    let markdown = final_changelog.replace("\\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*", "");

    let parser = pulldown_cmark::Parser::new_ext(markdown.as_str(), pulldown_cmark::Options::all());
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    html_output = html_output.replace("\"", "\\\"").replace("\n", "\\n");
    html_output = html_output.replacen(",/a></p>\\n", "</a></p>\\n<hr style=\\\"width:66%\\\"><div id=\\\"changelogContents\\\">", 1);

    std::fs::remove_file("sd:/ultimate/hdr/downloads/hdr-changelog.md");

    if verify_hdr(session, &config).is_ok() {
        session.try_send_json(&commands::ChangeHtml::new("changelog", html_output.as_str()));
        session.try_send_json(&commands::ChangeMenu::new("text-view"));
    
        session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Update&nbsp;&nbsp;</text>"));
    }

    
    session.try_send_json(&commands::ChangeHtml::new("play-button", "<div><text>Restart&nbsp;&nbsp;</text></div>"));
}



enum VerifyResult {
    Success(commands::VerifyInfo),
    MissingFile(PathBuf),
    DisabledPlugin(PathBuf),
    IncorrectFile(PathBuf)
}

/// 
/// this function will verify hdr to make sure that everything is ok
/// 
/// returns:
/// String - a warning/info output string, which may be worth displaying (if you are about to display a different UI than the one this generates)
/// Error - potentially an Error
/// 
pub fn verify_hdr(session: &WebSession, config: &StorageHolder<SdCardStorage>) -> Result<String, std::io::Error> {

    if !util::is_online() {
        let html_output = "<div id=\\\"changelogContents\\\"> We cannot verify, as you are not currently connected to the internet. </div>";
        show_results(html_output, session);
        return Err(Error::new(ErrorKind::Other, "The system is not connected to the internet."));
    }

    let is_nightly = config::is_enable_nightly_builds(&config);
    println!("we need to download the hashes to check. is_nightly = {}", is_nightly);
    let mut return_string = String::new();

    let version = match util::get_plugin_version() {
        Some(v) => {
            println!("version is : {}", v);
            v
        },
        None => {
            println!("could not determine current version!");
            let html_output = "<div id=\\\"changelogContents\\\">Could not determine current version! Cannot validate! </div>";
            show_results(html_output, session);
            return Err(Error::new(ErrorKind::Other, "Could not determine version"));
        }
    };
    
    if !Path::new("sd:/ultimate/hdr/downloads/").exists() {
        std::fs::create_dir_all("sd:/ultimate/hdr/downloads/");
    }

    match util::download_from_version(is_nightly, "content_hashes.txt", "content_hashes.txt", version, Some(session)) {
        Ok(_) => {},
        Err(e) => {
            println!("error: {:?}", e);
            show_results("<b>Failed to download HDR hash file in order to validate the installation. This is likely an internet connection problem.</b>", session);
            session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Fix HDR&nbsp;&nbsp;</text>"));
            return Err(Error::new(ErrorKind::Other, "Could not download the expected hash file"));
        }
    }

    println!("downloaded version");
    
    let file_data = std::fs::read_to_string("sd:/ultimate/hdr/downloads/content_hashes.txt").unwrap();

    
    println!("read to string");

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

        
        // if these files are present, we will always move them into disabled_plugins
        let always_disable_plugins = vec![
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libsmashline_hook_development.nro",
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libhdr.nro",
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libnn_hid_hook",
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libparam_hook.nro",
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libtraining_modpack.nro",
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libHDR-Launcher.nro",
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libnn_hid_hook.nro",
            "sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/libacmd_hook.nro",
        ];

        let mut disabled_plugins = vec![];
        for file in always_disable_plugins {
            if Path::new(file).exists() {
                println!("disabling plugin: {}", file);
                if !Path::new("sd:/atmosphere/contents/01006a800016e000/romfs/skyline/disabled_plugins").exists() {
                    std::fs::create_dir_all("sd:/atmosphere/contents/01006a800016e000/romfs/skyline/disabled_plugins");
                }
                std::fs::copy(file, format!("sd:/atmosphere/contents/01006a800016e000/romfs/skyline/disabled_plugins/{}",
                    file.trim_start_matches("sd:/atmosphere/contents/01006a800016e000/romfs/skyline/plugins/")));
                std::fs::remove_file(file);
                disabled_plugins.push(file.clone());
                let _ = tx.send(VerifyResult::DisabledPlugin(Path::new(file).to_path_buf()));
            }
        }
        
        let mut ignore_files = vec!["changelog.toml", "libarcropolis.nro", "hdr-launcher.nro"];
        
        // if the user wants to ignore the music file during verify (so they can set up custom music),
        // then add it to the list of ignored files
        if config::is_enable_ignore_music(&config::get_config()) {
            println!("ignoring music files!");
            ignore_files.push("bgm_property.bin");
            ignore_files.push("ui_bgm_db.prc");
            ignore_files.push("ui_series_db.prc");
            ignore_files.push("msg_bgm+us_en.msbt");
            ignore_files.push("msg_title+us_en.mbst");
            ignore_files.push("bgm_property.bin");
        }

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

        if files_map.len() == 0 {
            return;
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
                            if ignore_files.contains(&file_path.file_name().unwrap().to_str().unwrap()) {
                                let _ = tx.send(VerifyResult::Success(commands::VerifyInfo::new(found_count, line_count, format!("{}", file_path.display()))));
                            } else {
                                let _ = tx.send(VerifyResult::MissingFile(file_path.to_path_buf()));
                            }
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
                    if ignore_files.contains(&file_path.file_name().unwrap().to_str().unwrap()) {
                        let _ = tx.send(VerifyResult::Success(commands::VerifyInfo::new(found_count, line_count, format!("{}", file_path.display()))));
                        println!("ignoring missing file:{}", file_path.file_name().unwrap().to_str().unwrap());
                    } else {
                        let _ = tx.send(VerifyResult::MissingFile(file_path.to_path_buf()));
                        println!("could not find:{}", file_path.file_name().unwrap().to_str().unwrap());
                    }
                    continue;
                }
            };
            let digest = md5::compute(data);
            let digest = format!("{:x}", digest);
            if digest != hash && !ignore_files.contains(&file_path.file_name().unwrap().to_str().unwrap()) {
                let _ = tx.send(VerifyResult::IncorrectFile(file_path.to_path_buf()));
            } else {
                let _ = tx.send(VerifyResult::Success(commands::VerifyInfo::new(found_count, line_count, path.to_string().trim_start_matches("sd:/ultimate/mods/").to_string())));
            }
        }

        println!("deleting unwanted hdr files");
        for file in deleted_files {
            println!("deleting file: {}", file);
            std::fs::remove_file(file);
        }

        println!("verify complete.");

    });


    let mut missing_files = vec![];
    let mut bad_files = vec![];
    let mut disabled_plugins = vec![];
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
                VerifyResult::DisabledPlugin(path) => disabled_plugins.push(path),
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let mut html_output = String::new();

    // build the output info about what's enabled or not

    // check for development.nro real quick
    let mut has_dev_nro = Path::new("sd:/atmosphere/contents/01006a800016e000/romfs/smashline/development.nro").exists();
    let mut general_warnings = "".to_string();

    let api_version = arcropolis_api::get_api_version();

    if api_version.major >= 1 && api_version.minor >= 7 {
        // check if hdr is enabled
        let mut hdr_enabled = arcropolis_api::is_mod_enabled(arcropolis_api::hash40("sd:/ultimate/mods/hdr").as_u64());
        let mut hdr_assets_enabled = arcropolis_api::is_mod_enabled(arcropolis_api::hash40("sd:/ultimate/mods/hdr-assets").as_u64());
        let mut hdr_stages_enabled = arcropolis_api::is_mod_enabled(arcropolis_api::hash40("sd:/ultimate/mods/hdr-stages").as_u64());
        let mut hdr_dev_enabled = arcropolis_api::is_mod_enabled(arcropolis_api::hash40("sd:/ultimate/mods/hdr-dev").as_u64());

        if !hdr_enabled {
            general_warnings += "<br>The main hdr mod folder is not enabled in Arcropolis config! Please enable this in the options menu or the mod manager.<br>";
        }
        if !hdr_assets_enabled {
            general_warnings += "<br>The hdr-assets mod folder is not enabled in Arcropolis config! Please enable this in the options menu or the mod manager.<br>";
        }
        if !hdr_stages_enabled {
            general_warnings += "<br>The hdr-stages mod folder is not enabled in Arcropolis config! Please enable this in the options menu or the mod manager.<br>";
        }
        if hdr_dev_enabled {
            general_warnings += "<br>hdr-dev is currently enabled! Be aware that this is not currently an official build.<br>";
        }
    }

    if has_dev_nro {
        general_warnings += "<br>There is also a development.nro on this installation which may be a mistake! Proceed at your own peril.<br>";
    }
    
    let result = if missing_files.is_empty() && bad_files.is_empty() {
        html_output = "<div id=\\\"changelogContents\\\">There were no major problems found with the installation!".to_string();
        if !disabled_plugins.is_empty() {
            html_output += "The following plugins were automatically moved to disabled_plugins:<br><ul>";
            for file in disabled_plugins {
                html_output += format!("<li><b>{}</b></li>", file.display()).as_str();
            }
            html_output += "</ul><br><br>";
        }
        html_output += format!("{}", general_warnings).as_str();
        html_output += "</div>";
        Ok(return_string.clone())
    } else {
        // our verification failed
        session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Fix HDR&nbsp;&nbsp;</text></div>"));
        //let _ = std::fs::write("sd:/ultimate/mods/hdr/ui/hdr_version.txt", "v0.0.0-invalid");
        html_output = "<div id=\\\"changelogContents\\\">".to_string();
        if !disabled_plugins.is_empty() {
            html_output += "The following plugins were automatically moved to disabled_plugins:<br><ul>";
            for file in disabled_plugins {
                html_output += format!("<li><b>{}</b></li>", file.display()).as_str();
            }
            html_output += "</ul><br><br>";
        }
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
        html_output += format!("{}", general_warnings).as_str();
        html_output += "</div>";
        Err(Error::new(ErrorKind::Other, return_string.clone()))
    };

    show_results(&html_output, session);
    result
}



fn check_if_show_launcher(config: &StorageHolder<SdCardStorage>) -> bool {
    if !config::is_enable_skip_launcher(&config) {
        return true;
    }

    let plugin_version = util::get_plugin_version();
    if plugin_version.is_none() {
        skyline_web::DialogOk::ok("HDR is not installed. You will be directed to the HDR launcher.");
        return true;
    }

    let plugin_version = plugin_version.unwrap();

    if util::is_online() {
        let latest_version = util::get_latest_version(None, config::is_enable_nightly_builds(&config));
        if latest_version.is_err() {
            skyline_web::DialogOk::ok("Unable to get the latest version of HDR. You will be directed to the HDR launcher.");
            return true;
        }
        
        let latest_version = latest_version.unwrap();
        if latest_version != plugin_version {
            skyline_web::DialogOk::ok("There is an update for HDR. You will be directed to the HDR launcher.");
            return true;
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(1500));
    if ninput::any::is_down(ninput::Buttons::X) {
        return true;
    }

    false
}

extern "C" {
    fn arcrop_show_mod_manager();
}

#[skyline::main(name = "HDRLauncher")]
pub fn main() {
    ninput::init();
    curl::install();

    let mut config = config::get_config();
    if !check_if_show_launcher(&config) {
        return;
    }
    // let handle = start_audio_thread();

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

    // open our web session
    open_session();
}


pub fn open_session() {
    let mut config = config::get_config();

    let mut wav = WavReader::new(std::io::Cursor::new(BGM_WAV)).unwrap();
    let samples: Vec<i16> = wav.samples::<i16>().map(|x| x.unwrap()).collect();
    let audio = LoopingAudio::new(
        samples,
        151836 * 2,
        1776452 * 2,
        0.22,
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
                        let plugin_version = util::get_plugin_version().map(|x| x.to_string()).unwrap_or("???".to_string());
                        let romfs_version = get_romfs_version().map(|x| x.to_string()).unwrap_or("???".to_string());
                        session.send_json(&VersionInfo {
                            code: plugin_version.clone(),
                            romfs: romfs_version.clone()
                        });

                        if is_update_available(None, config::is_enable_nightly_builds(&config)) {
                            println!("updates are available!");
                            session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Update(!)&nbsp;&nbsp;</text></div>"));
                        }

                        if !Path::new("sd:/ultimate/mods/hdr").exists() {
                            session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Install&nbsp;&nbsp;</text></div>"));
                            session.send_json(&commands::ChangeHtml::new("play-button", "<div><text>Vanilla&nbsp;&nbsp;</text></div>"));
                            session.send_json(&commands::ChangeMenu::new("main-menu"));
                        } else if plugin_version == "???" || plugin_version == "0.0.0-invalid" {
                            session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Fix HDR&nbsp;&nbsp;</text>"));
                        } else if util::should_version_swap(&config) {
                            session.send_json(&commands::ChangeHtml::new("update-button", "<div><text>Install&nbsp;&nbsp;</text>"));
                        }
                        session.send_json(&commands::SetOptionStatus::new(
                            "nightlies",
                            config::is_enable_nightly_builds(&config)
                        ));

                        // set the correct text for the nightly/beta toggle button
                        set_nightly_toggle_button(&config, &session);

                        session.send_json(&commands::SetOptionStatus::new(
                            "skip_on_launch",
                            config::is_enable_skip_launcher(&config)
                        ));
                        session.send_json(&commands::SetOptionStatus::new(
                            "enable_ignore_music",
                            config::is_enable_ignore_music(&config)
                        ));
                    }
                    "start" => {
                        println!("starting hdr...");
                        end_session_and_launch(&session, signal);
                        break;
                    }
                    "open_arcropolis" => {
                        end_session_and_launch(&session, signal);
                        util::try_open_arcropolis();
                        unsafe { skyline::nn::oe::RequestToRelaunchApplication(); }
                    }
                    "restart" => {
                        restart(&session, signal);
                        break;
                    }
                    "verify_hdr" => {
                        let _ = verify_hdr(&session, &config);
                    },
                    "update_hdr" => update_hdr(&session, &config, false),
                    "reinstall_hdr" => {

                        // update (install) hdr
                        update_hdr(&session, &config, true);
                        
                        // close the session
                        end_session_and_launch(&session, signal);

                        // pop up some user instructions
                        skyline_web::DialogOk::ok("Arcropolis' mod manager will now open. Please enable HDR's components in the Mod Manager menu to complete installation.");

                        // open the arcropolis ui
                        util::try_open_mod_manager();
                        unsafe { skyline::nn::oe::RequestToRelaunchApplication(); }
                    }
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
                            session.send_json(&commands::ChangeHtml::new("progressText", "Parsing package metadata...<br>Do not exit the menu"));
                            session.try_send_json(&commands::ChangeMenu::new("progress"));
                            let is_enabled = config::is_enable_nightly_builds(&config);
                            config::enable_nightlies(&mut config, !is_enabled);
                            session.send_json(&commands::SetOptionStatus::new("nightlies", !is_enabled));
                            println!("enable nightlies is now: {}", config::is_enable_nightly_builds(&config));
                            if util::should_version_swap(&config) {
                                println!("swapping!");
                                switch_install_type(&session, &config);
                            } else {
                                println!("not swapping!");
                            }
                        } else if option == "skip_on_launch" {
                            let is_enabled = config::is_enable_skip_launcher(&config);
                            config::enable_skip_launcher(&mut config, !is_enabled);
                            session.send_json(&commands::SetOptionStatus::new("skip_on_launch", !is_enabled));
                        }
                        else if option == "enable_ignore_music" {
                            let is_enabled = config::is_enable_ignore_music(&config);
                            config::set_ignore_music(&mut config, !is_enabled);
                            session.send_json(&commands::SetOptionStatus::new("enable_ignore_music", !is_enabled));
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

    // clear any remaining downloaded files
    std::fs::remove_dir_all("sd:/ultimate/hdr/downloads/");
}
