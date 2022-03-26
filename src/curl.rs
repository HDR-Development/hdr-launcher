use std::{io::{BufWriter, Write}, fs::File, sync::mpsc::Sender};

use skyline::libc::*;
use skyline_web::WebSession;

use crate::DownloadInfo;

#[skyline::hook(offset = 0x6aa8, inline)]
pub unsafe fn curl_log_hook(ctx: &skyline::hooks::InlineCtx) {
    let str_ptr;
    asm!("ldr {}, [x29, #0x18]", out(reg) str_ptr);
    println!("{}", skyline::from_c_str(str_ptr));
}

#[skyline::hook(offset = 0x27ac0, inline)]
pub unsafe fn libcurl_resolver_thread_stack_size_set(ctx: &mut skyline::hooks::InlineCtx) {
    *ctx.registers[1].x.as_mut() = 0x10_000;
}

#[skyline::hook(offset = 0x27af4, inline)]
pub unsafe fn libcurl_resolver_thread_stack_size_set2(ctx: &mut skyline::hooks::InlineCtx) {
    *ctx.registers[4].x.as_mut() = 0x10_000;
}

#[skyline::from_offset(0x7f0)]
pub unsafe extern "C" fn global_init_mem(
    init_args: u64,
    malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    free: unsafe extern "C" fn(*mut c_void),
    realloc: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    strdup: unsafe extern "C" fn(*const u8) -> *mut u8,
    calloc: unsafe extern "C" fn(usize, usize) -> *mut c_void
) -> curl_sys::CURLcode;

#[skyline::from_offset(0x16c00)]
pub unsafe extern "C" fn slist_append(slist: *mut curl_sys::curl_slist, header: *const u8) -> *mut curl_sys::curl_slist;

#[skyline::from_offset(0x960)]
pub unsafe extern "C" fn easy_init() -> *mut curl_sys::CURL;

#[skyline::from_offset(0xA00)]
pub unsafe extern "C" fn easy_setopt(curl: *mut curl_sys::CURL, option: curl_sys::CURLoption, ...) -> curl_sys::CURLcode;

#[skyline::from_offset(0xA90)]
pub unsafe extern "C" fn easy_perform(curl: *mut curl_sys::CURL) -> curl_sys::CURLcode;

#[skyline::from_offset(0xC70)]
pub unsafe extern "C" fn easy_cleanup(curl: *mut curl_sys::CURL) -> curl_sys::CURLcode;

#[skyline::from_offset(0x36f6d40)]
pub unsafe extern "C" fn curl_global_malloc(size: usize) -> *mut u8;

#[skyline::from_offset(0x36f6dc0)]
pub unsafe extern "C" fn curl_global_free(ptr: *mut u8);

#[skyline::from_offset(0x36f6e40)]
pub unsafe extern "C" fn curl_global_realloc(ptr: *mut u8, size: usize) -> *mut u8;

#[skyline::from_offset(0x36f6ec0)]
pub unsafe extern "C" fn curl_global_strdup(ptr: *const u8) -> *mut u8;

#[skyline::from_offset(0x36f6fa0)]
pub unsafe extern "C" fn curl_global_calloc(nmemb: usize, size: usize) -> *mut u8;

#[skyline::from_offset(0x21fd50)]
pub unsafe extern "C" fn curl_ssl_ctx_callback(arg1: u64, arg2: u64, arg3: u64) -> curl_sys::CURLcode;

unsafe extern "C" fn write_fn(data: *const u8, data_size: usize, data_count: usize, writer: &mut BufWriter<File>) -> usize {
    let true_size = data_size * data_count;
    let slice = std::slice::from_raw_parts(data, true_size);
    let _ = writer.write(slice);
    true_size
}

static mut START_TICK: usize = 0;
static mut SENDER: Option<*mut Sender<DownloadInfo>> = None;

unsafe extern "C" fn progress_func(session: &WebSession, dl_total: f64, dl_now: f64, ul_total: f64, ul_now: f64) -> usize {
    let current_tick = skyline::nn::os::GetSystemTick() as usize;
    let nanoseconds = ((current_tick - START_TICK) * 625) / 12;
    let seconds_f = (nanoseconds as f64) / (1000.0 * 1000.0 * 1000.0);
    let bps = dl_now * 8.0 / seconds_f;

    (**(SENDER.as_ref().unwrap())).send(crate::DownloadInfo::new(bps, dl_now, dl_total, "release archive"));
    // println!("{} / {} | {} mbps", dl_now, dl_total, bps / (1024.0 * 1024.0));

    // session.send_json(&crate::DownloadInfo::new(bps, dl_now, dl_total, "release archive"));
    0
}

macro_rules! curle {
    ($e:expr) => {{
        let result = $e;
        if result != ::curl_sys::CURLE_OK {
            Err(result)
        } else {
            Ok(())
        }
    }}
}

pub fn try_curl(
    url: &str,
    writer: &mut BufWriter<File>,
    session: &WebSession,
    mut sender: Sender<DownloadInfo>
) -> Result<(), u32> {
    unsafe {
        SENDER = Some(std::mem::transmute(&mut sender as *mut Sender<DownloadInfo>));
        // assert_eq!(global_init_mem(3, malloc, free, realloc, strdup, calloc), curl_sys::CURLE_OK);
        let ptr = [url, "\0"].concat();
        let curl = easy_init();
        let header = slist_append(std::ptr::null_mut(), "Accept: application/octet-stream\0".as_ptr());
        curle!(easy_setopt(curl, curl_sys::CURLOPT_URL, ptr.as_str().as_ptr()))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_HTTPHEADER, header))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_FOLLOWLOCATION, 1u64))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_WRITEDATA, writer))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_WRITEFUNCTION, write_fn as *const ()))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_NOPROGRESS, 0u64))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_PROGRESSDATA, session))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_PROGRESSFUNCTION, progress_func as *const ()))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_SSL_CTX_FUNCTION, curl_ssl_ctx_callback as *const ()))?;
        curle!(easy_setopt(curl, curl_sys::CURLOPT_USERAGENT, "HDR-Launcher\0".as_ptr()))?;
        START_TICK = skyline::nn::os::GetSystemTick() as usize;
        curle!(easy_perform(curl))?;
        curle!(easy_cleanup(curl))?;
        easy_cleanup(curl);
    }

    Ok(())
}

pub fn install() {
    skyline::install_hooks!(
        curl_log_hook,
        libcurl_resolver_thread_stack_size_set,
        libcurl_resolver_thread_stack_size_set2
    );
}