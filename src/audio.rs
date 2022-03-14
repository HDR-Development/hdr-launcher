#[repr(C)]
#[derive(Copy, Clone)]
pub struct AudioOutInfo([u8; 0x100]);

impl Default for AudioOutInfo {
    fn default() -> Self {
        Self([0; 0x100])
    }
}

impl AudioOutInfo {
    pub fn as_str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(&self.0)
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum SampleFormat {
    Invalid,
    PCM_Int8,
    PCM_Int16,
    PCM_Int24,
    PCM_Int32,
    PCM_Float,
    PCM_Adpcm
}

impl SampleFormat {
    pub fn byte_size(&self) -> usize {
        match self {
            Self::Invalid => 0,
            Self::PCM_Int8 => 1,
            Self::PCM_Int16 => 2,
            Self::PCM_Int24 => 3,
            _ => 4,
        }
    }
}


#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum State {
    Started,
    Stopped
}

#[repr(C)]
pub struct AudioOut([u8; 0x600]); // idk the actual size so imma just use this

use std::ops::Range;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct AudioOutBuffer(pub [u64; 0x10]);

#[repr(C)]
pub struct AudioOutParameter {
    sample_rate: u32,
    channel_count: u16,
    _x6: u16
}

extern "C" {
    #[link_name = "_ZN2nn5audio13CloseAudioOutEPNS0_8AudioOutE"]
    pub(crate) fn close_audio_out(out: *mut AudioOut);

    #[link_name = "_ZN2nn5audio13ListAudioOutsEPNS0_12AudioOutInfoEi"]
    pub(crate) fn list_audio_outs(audio_outs: *mut AudioOutInfo, count: i32) -> i32;

    #[link_name = "_ZN2nn5audio19OpenDefaultAudioOutEPNS0_8AudioOutERKNS0_17AudioOutParameterE"]
    pub(crate) fn open_default_audio_out(audio_out: *mut AudioOut, parameter: &AudioOutParameter) -> u32;

    #[link_name = "_ZN2nn5audio19OpenDefaultAudioOutEPNS0_8AudioOutEPNS_2os11SystemEventERKNS0_17AudioOutParameterE"]
    pub(crate) fn open_default_audio_out_with_event(audio_out: *mut AudioOut, event: *mut skyline::nn::os::SystemEventType, parameter: &AudioOutParameter) -> u32;

    #[link_name = "_ZN2nn5audio27InitializeAudioOutParameterEPNS0_17AudioOutParameterE"]
    pub(crate) fn init_param(audio_out_param: *mut AudioOutParameter);

    #[link_name = "_ZN2nn5audio23GetAudioOutChannelCountEPKNS0_8AudioOutE"]
    pub(crate) fn get_channel_count(audio_out: *const AudioOut) -> i32;

    #[link_name = "_ZN2nn5audio15GetAudioOutNameEPKNS0_8AudioOutE"]
    pub(crate) fn get_name(audio_out: *const AudioOut) -> *const u8;

    #[link_name = "_ZN2nn5audio28GetAudioOutPlayedSampleCountEPKNS0_8AudioOutE"]
    pub(crate) fn get_played_sample_count(audio_out: *const AudioOut) -> u64;

    #[link_name = "_ZN2nn5audio23GetAudioOutSampleFormatEPKNS0_8AudioOutE"]
    pub(crate) fn get_sample_format(audio_out: *const AudioOut) -> SampleFormat;

    #[link_name = "_ZN2nn5audio21GetAudioOutSampleRateEPKNS0_8AudioOutE"]   
    pub(crate) fn get_sample_rate(audio_out: *const AudioOut) -> i32;

    #[link_name = "_ZN2nn5audio16GetAudioOutStateEPKNS0_8AudioOutE"]
    pub(crate) fn get_state(audio_out: *const AudioOut) -> State;

    #[link_name = "_ZN2nn5audio17GetAudioOutVolumeEPKNS0_8AudioOutE"]
    pub(crate) fn get_volume(audio_out: *const AudioOut) -> f32;

    #[link_name = "_ZN2nn5audio21SetAudioOutBufferInfoEPNS0_14AudioOutBufferEPvmm"]
    pub(crate) fn set_audio_out_buffer_info(audio_out: *mut AudioOutBuffer, memory: *mut u8, buf_size: usize, data_size: usize);

    #[link_name = "_ZN2nn5audio20AppendAudioOutBufferEPNS0_8AudioOutEPNS0_14AudioOutBufferE"]
    pub(crate) fn append_buffer(audio_out: *mut AudioOut, buffer: *mut AudioOutBuffer);

    #[link_name = "_ZN2nn5audio13StartAudioOutEPNS0_8AudioOutE"]
    pub(crate) fn start(audio_out: *mut AudioOut) -> i32;

    #[link_name = "_ZN2nn5audio25GetReleasedAudioOutBufferEPNS0_8AudioOutE"]
    pub(crate) fn get_released_buffer(audio_out: *mut AudioOut) -> *mut AudioOutBuffer;

    #[link_name = "_ZN2nn5audio28GetAudioOutBufferDataPointerEPKNS0_14AudioOutBufferE"]
    pub(crate) fn get_data_ptr(buffer: *const AudioOutBuffer) -> *mut u8;

    #[link_name = "_ZN2nn5audio27GetAudioOutBufferBufferSizeEPKNS0_14AudioOutBufferE"]
    pub(crate) fn get_buffer_size(buffer: *const AudioOutBuffer) -> usize;

    #[link_name = "_ZN2nn5audio17SetAudioOutVolumeEPNS0_8AudioOutEf"]
    pub(crate) fn set_volume(buffer: *mut AudioOut, volume: f32);
}

impl AudioOut {
    pub fn channel_count(&self) -> i32 {
        unsafe {
            get_channel_count(self)
        }
    }

    pub fn name(&self) -> String {
        unsafe {
            skyline::from_c_str(get_name(self))
        }
    }

    pub fn played_sample_count(&self) -> u64 {
        unsafe {
            get_played_sample_count(self)
        }
    }

    pub fn sample_format(&self) -> SampleFormat {
        unsafe {
            get_sample_format(self)
        }
    }

    pub fn sample_rate(&self) -> i32 {
        unsafe {
            get_sample_rate(self)
        }
    }
    pub fn state(&self) -> State {
        unsafe {
            get_state(self)
        }
    }
    pub fn volume(&self) -> f32 {
        unsafe {
            get_volume(self)
        }
    }
    pub fn set_volume(&mut self, volume: f32) {
        unsafe {
            set_volume(self, volume)
        }
    }
    pub fn append(&mut self, buffer: *mut AudioOutBuffer) {
        unsafe {
            append_buffer(self, buffer)
        }
    }
    pub fn start(&mut self) -> bool {
        unsafe {
            start(self) == 0
        }
    }
    pub fn released_buffer(&mut self) -> Option<*mut AudioOutBuffer> {
        unsafe {
            let buffer = get_released_buffer(self);
            if buffer.is_null() {
                None
            } else {
                Some(buffer)
            }
        }
    }
}

pub fn get_audio_outs(max: i32) -> Vec<AudioOutInfo> {
    let mut vec = vec![AudioOutInfo::default(); max as usize];
    let actual_count = unsafe {
        list_audio_outs(vec.as_mut_ptr(),max)
    };
    vec.truncate(actual_count as usize);
    vec
}

pub fn default_audio_out(sample_rate: u32, channel_count: u16) -> Option<(&'static mut AudioOut, *mut skyline::nn::os::SystemEventType)> {
    let mut out = Box::leak(Box::new(AudioOut([0; 0x600])));
    let mut parameter = AudioOutParameter {
        sample_rate,
        channel_count,
        _x6: 0
    };
    unsafe {
        let mut event = Box::new([0u8; 0x60]);
        let mut event = Box::leak(event) as *mut u8 as *mut skyline::nn::os::SystemEventType;
        init_param(&mut parameter);
        parameter.sample_rate = sample_rate;
        parameter.channel_count = channel_count;
        if open_default_audio_out_with_event(out, event, &parameter) != 0 {
            None
        } else {
            Some((out, event))
        }
    }
}

pub fn generate_square_wave_int16(buffer: *mut u8, channel_count: i32, sample_rate: i32, sample_count: i32, amplitude: i32) {
    static mut TOTAL_SAMPLE_COUNT: [i32; 6] = [0; 6];
    const FREQUENCIES: [i32; 6] = [415, 698, 554, 104, 349, 377];

    let mut buffer = buffer as *mut i16;
    for ch in 0..channel_count {
        let wavelength = sample_rate / FREQUENCIES[ch as usize];

        for sample in 0..sample_count {
            unsafe {
                let value = if TOTAL_SAMPLE_COUNT[ch as usize] < wavelength / 2 {
                    amplitude as i16
                } else {
                    -amplitude as i16
                };

                *buffer.add((sample * channel_count + ch) as usize) = value;
                TOTAL_SAMPLE_COUNT[ch as usize] += 1;
                if TOTAL_SAMPLE_COUNT[ch as usize] == wavelength {
                    TOTAL_SAMPLE_COUNT[ch as usize] = 0;
                }
            }
        }
    }
}

pub const BUFFER_ALIGNMENT: usize = 4 * 1024;

macro_rules! align {
    ($x:expr, $a:expr) => {
        if $x % $a == 0 {
            $x
        } else {
            $x + ($a - ($x % $a))

        } 
    }
}

pub fn make_buffer16(audio_out: &mut AudioOut, samples: &[i16], buffer_count: usize, volume: f32) {
    let data_size = samples.len() * 2 * (audio_out.channel_count() as usize);
    let buffer_size = align!(data_size, BUFFER_ALIGNMENT);
    let amplitude = ((i16::MAX / 16) as f32 * volume) as i16;
    unsafe {
        for x in 0..buffer_count {
            let buffer_mem = skyline::libc::memalign(BUFFER_ALIGNMENT, buffer_size) as *mut i16;
            for (idx, sample) in samples.iter().enumerate() {
                // if idx % 20 < 9 {
                //     continue;
                // }
                for ch in 0..audio_out.channel_count() {
                    *buffer_mem.add(idx * (audio_out.channel_count() as usize)) = *sample;
                }
            }
            let mut buf = AudioOutBuffer([0; 0x10]);
            let mut buf = Box::leak(Box::new(buf));
            println!("buffer {}: {:?}, {:#x}, {:#x}", x, buffer_mem, buffer_size, data_size);
            set_audio_out_buffer_info(buf, buffer_mem as _, buffer_size, data_size);
            audio_out.append(buf);
        }
    }
}

pub fn make_buffer16_with_loop(audio_out: &mut AudioOut, samples: &[i16], buffer_count: usize, loop_range: Range<usize>, volume: f32) {
    make_buffer16(audio_out, &samples[0..loop_range.end], 1, volume);
    make_buffer16(audio_out, &samples[loop_range], buffer_count - 1, volume);
}

pub fn recycle_buffers(audio_out: &mut AudioOut, samples: &[i16]) {
    let channel_count = audio_out.channel_count() as usize;
    let new_data_size = samples.len() * 2 * channel_count;
    while let Some(buffer) = audio_out.released_buffer() {
        unsafe {
            let ptr = get_data_ptr(buffer) as *mut i16;
            let size = align!(get_buffer_size(buffer), BUFFER_ALIGNMENT);
            dbg!(size);
            dbg!(new_data_size);
            dbg!(ptr);
            // assert!(new_data_size <= size);

            for (idx, sample) in samples.iter().enumerate() {
                for ch in 0..audio_out.channel_count() {
                    *ptr.add(idx * (audio_out.channel_count() as usize)) = *sample;
                }
            }
            set_audio_out_buffer_info(buffer, ptr as _, size, new_data_size);
            audio_out.append(buffer);
        }

    }
}

pub fn make_buffers(audio_out: &mut AudioOut, buffer_count: usize) -> Vec<(Box<AudioOutBuffer>, *mut u8)> {
    let framerate = 20;
    let frame_sample_count = audio_out.sample_rate() / framerate;
    let data_size = audio_out.sample_format().byte_size() * (frame_sample_count * audio_out.channel_count()) as usize;
    let buffer_size = align!(data_size, BUFFER_ALIGNMENT);
    let amplitude = i16::MAX / 16;
    let mut buffers = vec![AudioOutBuffer([0; 0x10]); buffer_count];
    unsafe {
        buffers.into_iter().map(|buf| {
            let mut buf = Box::new(buf);
            let buffer_mem = skyline::libc::memalign(BUFFER_ALIGNMENT, buffer_size) as *mut u8;
            generate_square_wave_int16(buffer_mem, audio_out.channel_count(), audio_out.sample_rate(), frame_sample_count, amplitude as i32);
            set_audio_out_buffer_info(&mut *buf, buffer_mem, buffer_size, data_size);
            audio_out.append(&mut *buf);
            (buf, buffer_mem)
        }).collect()
    }
}