use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ExtractInfo<'a> {
    pub tag: &'static str,
    pub file_number: usize,
    pub file_count: usize,
    pub file_name: &'a str,
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
pub struct VerifyInfo {
    pub tag: &'static str,
    pub file_number: usize,
    pub file_count: usize,
    pub file_name: String
}

impl VerifyInfo {
    pub fn new(
        file_number: usize,
        file_count: usize,
        file_name: String
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
pub struct SetOptionStatus<'a> {
    pub tag: &'static str,
    pub option: &'a str,
    pub status: bool,
}

impl<'a> SetOptionStatus<'a> {
    pub fn new(
        option: &'a str,
        status: bool
    ) -> Self
    {
        Self {
            tag: "set-option",
            option,
            status
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
pub struct End {
    pub tag: &'static str
}

impl End {
    pub fn new() -> Self {
        Self {
            tag: "end-launcher"
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Start {
    pub tag: &'static str
}

impl Start {
    pub fn new() -> Self {
        Self {
            tag: "start-game"
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Restart {
    pub tag: &'static str
}

impl Restart {
    pub fn new() -> Self {
        Self {
            tag: "restart-game"
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ChangeMenu<'a> {
    pub tag: &'static str,
    pub going_to: &'a str
}

impl<'a> ChangeMenu<'a> {
    pub fn new(going_to: &'a str) -> Self {
        Self {
            tag: "change-menu",
            going_to
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ChangeHtml<'a, 'b> {
    pub tag: &'static str,
    pub id: &'a str,
    pub text: &'b str
}

impl<'a, 'b> ChangeHtml<'a, 'b> {
    pub fn new(id: &'a str, text: &'b str) -> Self {
        Self {
            tag: "change-html",
            id,
            text
        }
    }
}