use skyline_config::*;

pub fn get_config() -> StorageHolder<SdCardStorage> {
    StorageHolder::new(SdCardStorage::new("ultimate/hdr"))
}

pub fn enable_skip_launcher(config: &mut StorageHolder<SdCardStorage>, enable: bool) {
    config.set_flag("skip_launcher", enable).expect("Unable to enable skipping the launcher!")
}

pub fn is_enable_skip_launcher(config: &StorageHolder<SdCardStorage>) -> bool {
    config.get_flag("skip_launcher")
}

pub fn enable_nightlies(config: &mut StorageHolder<SdCardStorage>, enable: bool) {
    config.set_flag("enable_nightly", enable).expect("Unable to enable nightlies!")
}

pub fn is_enable_nightly_builds(config: &StorageHolder<SdCardStorage>) -> bool {
    config.get_flag("enable_nightly")
}