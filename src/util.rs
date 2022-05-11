// use github::GithubRelease;
use hound::WavReader;
use crate::looping_audio::{LoopingAudio, AsyncCommand};
use semver::Version;
// use skyline::{hook, install_hook};
use skyline_web::{Webpage, WebSession};
use std::{thread::{self, JoinHandle}, time, sync::mpsc::{Sender, self}, alloc::{GlobalAlloc}, io::{Read, Write, BufRead, Error, ErrorKind}, path::{Path, PathBuf}};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::*;
use online::sync::check;


#[derive(Serialize, Debug)]
pub struct VersionInfo {
    pub code: String,
    pub romfs: String
}

pub fn is_online() -> bool {
    check(None).is_ok()
}

pub fn show_results(html: &str, session: &WebSession) {
    session.try_send_json(&commands::ChangeMenu::new("text-view"));
    session.try_send_json(&commands::ChangeHtml::new("changelog", html));
    session.try_send_json(&commands::ChangeHtml::new("title", "HDR Launcher > Results"));
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

pub fn count_file_lines(file_name: &str) -> i32 {
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

pub fn download_from_latest(is_nightly: bool, artifact_name: &str, created_file_name: &str, session: Option<&WebSession>) -> std::io::Result<()> {

    println!("we need to download the hdr update. is_nightly = {}", is_nightly);
    let mut url = String::new();
    if is_nightly {
        url = format!("https://github.com/HDR-Development/HDR-Nightlies/releases/latest/download/{}", artifact_name);
    } else {
        url = format!("https://github.com/HDR-Development/HDR-Releases/releases/latest/download/{}", artifact_name);
    }

    // if you remove this print statement, download will panic. so dont do that.
    println!("downloading from version: {}", url);
    let url_str = url.as_str();
    println!("downloading from version as str: {}", url_str);

    download_file(url_str, format!("sd:/downloads/{}", created_file_name).as_str(), session, artifact_name.to_string())  
    
}

pub fn download_from_version(is_nightly: bool, artifact_name: &str, created_file_name: &str, version: Version, session: Option<&WebSession>) -> std::io::Result<()> {

    let mut url = String::new();
    if is_nightly {
        url = format!("https://github.com/HDR-Development/HDR-Nightlies/releases/download/v{}/{}", version.to_string().trim_end_matches("-nightly"), artifact_name);
    } else {
        url = format!("https://github.com/HDR-Development/HDR-Releases/releases/download/v{}/{}", version.to_string().trim_end_matches("-beta"), artifact_name);
    }

    // if you remove this print statement, download will panic. so dont do that.
    println!("downloading from version: {}", url);
    let url_str = url.as_str();
    println!("downloading from version as str: {}", url_str);

    download_file(url_str, format!("sd:/downloads/{}", created_file_name).as_str(), session, artifact_name.to_string())
}

pub fn get_latest_version(session: Option<&WebSession>, is_nightly: bool) -> Result<Version, Error> {
    match download_from_latest(is_nightly, "hdr_version.txt", "hdr_version.txt", session) {
        Ok(i) => println!("latest version info downloaded!"),
        Err(e) => {
            println!("error while downloading latest version file! Either the latest upload is broken, or you do not have interenet access? {:?}", e);
            return Err(e);
        }
    }

    let latest_str = match std::fs::read_to_string(Path::new("sd:/downloads/hdr_version.txt")) {
        Ok(i) => i,
        Err(e) => {
            println!("error while reading version string: {:?}", e);
            return Err(e);
        }
    };
    println!("latest version str is: {}", latest_str);
    match Version::parse(latest_str.trim_start_matches("v")) {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::new(ErrorKind::Other, e))
    }

}


pub fn download_file(url: &str, path: &str, session: Option<&WebSession>, file_name: String) -> std::io::Result<()> {
    // unsafe {
    //     block_home_button();
    //     block_home_button_short();
    // }
    
    if !is_online() {
        return Err(Error::new(ErrorKind::Other, format!("We do not have an internet connection! Cannot download!")));
    }

    let session2 = if let Some(session) = session.as_ref() {
        *session as *const WebSession as u64
    } else {
        0
    };

    println!("making updater thread");

    let (tx, rx) = mpsc::channel();
    let ui_updater = std::thread::spawn(move || {
        let session = if session2 == 0 {
            None
        } else {
            unsafe { Some(&*(session2 as *const WebSession)) }
        };
        loop {
            let mut value: Option<DownloadInfo> = None;
            loop {
                match rx.try_recv() {
                    Ok(v) => value = Some(v),
                    Err(mpsc::TryRecvError::Empty) => break,
                    _ => return
                }
            }

            if let Some(session) = session.as_ref() {
                if let Some(mut value) = value {
                    value.item_name = file_name.as_str();
                    //println!("{}", session.try_send_json(&value));
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    println!("changing thread priority!");

    unsafe {
        skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 2);
    }

    println!("checking original path: {}", path);
    // delete the original file if this file already exists on sd
    if Path::new(path).exists() {
        println!("removing original path: {}", path);
        std::fs::remove_file(path);
    }

    println!("creating file");
    let mut writer = std::io::BufWriter::with_capacity(
        0x40_0000,
        std::fs::File::create(path)?
    );

    println!("trying curl with url: {}", url);

    let result = if let Some(session) = session {
        println!("trying a real curl");
        crate::curl::try_curl(url, &mut writer, session, tx)
    } else {
        println!("dropping the other thread");
        std::mem::drop(tx);
        println!("performing maidenless curl");
        crate::curl::try_curl_maidenless(url, &mut writer)
    };

    println!("flushing writer");
    writer.flush();
    println!("dropping writer");
    std::mem::drop(writer);

    println!("checking if the result is valid");
    match result {
        Ok(_) => {
            if confirm_file_is_valid(path) {
                println!("download is successful");
            } else {
                println!("file was not valid!");
                return Err(Error::new(ErrorKind::Other, format!("Error while trying to download! Not found")));
            }
        },
        Err(e) => {
            println!("error during download");
            return Err(Error::new(ErrorKind::Other, format!("Error while trying to download! code: {}", e)));
        }
    };

    println!("resetting priority of thread");
    unsafe {
        skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 16);
    }

    println!("download complete.");    

    ui_updater.join();
    println!("Ok(())");
    // unsafe {
    //     allow_home_button();
    //     allow_home_button_short();
    // }

    
    Ok(())
}

pub fn should_version_swap(config: &StorageHolder<SdCardStorage>) -> bool {
    let current_version = get_plugin_version();
    if let Some(current_version) = current_version {
        println!("current version is: {}", current_version);
        let compare_to = if config::is_enable_nightly_builds(config) {
            "nightly"
        } else {
            "beta"
        };

        current_version.pre.as_str() != compare_to
    } else {
        println!("Could not get current version!");
        false
    }
}

pub fn confirm_file_is_valid<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    println!("Checking metadata");
    if std::fs::metadata(path).map(|x| x.len()).unwrap_or(0) == 0 {
        println!("{} is 0", path.display());
        return false;
    }

    if let Ok(string) = std::fs::read_to_string(path) {
        println!("file is valid? {}", string != "Not Found");
        string != "Not Found"
    } else {
        println!("file is valid");
        true
    }
}

/// tries to open the main arcropolis configuration ui
pub fn try_open_arcropolis() {
    let api_version = arcropolis_api::get_api_version();
    println!("opening arcrop menu...");
    if api_version.major >= 1 && api_version.minor >= 7 {
        arcropolis_api::show_main_menu();
    } else {
        println!("Error: We cannot open arcrop because arcrop is out of date!");
        skyline_web::DialogOk::ok("Error: Cannot open arcropolis menu because your arcropolis is out of date! You may want to update in the launcher.");
    }
}

pub fn try_open_mod_manager() {
    let api_version = arcropolis_api::get_api_version();
    println!("opening arcrop mod manager...");
    if api_version.major >= 1 && api_version.minor >= 7 {
        arcropolis_api::show_mod_manager();
    } else {
        println!("Error: We cannot open arcrop because arcrop is out of date!");
        skyline_web::DialogOk::ok("Error: Cannot open arcropolis mod manager because your arcropolis is out of date! You may want to update in the launcher.");
    }
}
