use std::{thread::JoinHandle, sync::mpsc::{Receiver, Sender}};

use skyline::nn;

use crate::audio::{AudioOutBuffer, set_audio_out_buffer_info};

use super::audio;

macro_rules! align {
    ($x:expr, $a:expr) => {
        if $x % $a == 0 {
            $x
        } else {
            $x + ($a - ($x % $a))

        } 
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AsyncCommand {
    ChangeVolumeOverTime {
        new_volume: f32,
        time: f32
    },
    Quit
}

unsafe impl Send for AsyncCommand {}
unsafe impl Sync for AsyncCommand {}

pub struct LoopingAudio {
    audio: &'static mut audio::AudioOut,
    audio_complete_event: *mut nn::os::SystemEventType,
    samples: Vec<i16>,
    loop_start: usize,
    loop_end: usize,
    volume: f32,
    buffer_count: usize,
    seconds_buffered: f32,

    playing_current_sample: usize,
    volume_dec: Vec<f32>
}

unsafe impl Sync for LoopingAudio {}
unsafe impl Send for LoopingAudio {}

impl LoopingAudio {
    pub fn new(
        samples: Vec<i16>,
        loop_start: usize,
        loop_end: usize,
        volume: f32,
        buffer_count: usize,
        seconds_buffered: f32,
    ) -> Self
    {
        let (audio, audio_complete_event) = audio::default_audio_out(48000, 2).expect("Failed to initialize audio driver!");
        Self {
            audio,
            audio_complete_event,
            samples,
            loop_start,
            loop_end,
            volume,
            buffer_count,
            seconds_buffered,

            playing_current_sample: 0,
            volume_dec: vec![]
        }
    }

    pub fn start(mut self) -> Sender<AsyncCommand> {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = std::thread::spawn(move || {
            unsafe {
                nn::os::ChangeThreadPriority(nn::os::GetCurrentThread(), 2);
            }
            for _ in 0..self.buffer_count {
                self.create_buffer();
            }

            let mut is_ending = false;

            loop {
                if let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        AsyncCommand::Quit => is_ending = true,
                        AsyncCommand::ChangeVolumeOverTime { new_volume, time } => {
                            let num_decreases = (time / self.seconds_buffered * self.buffer_count as f32).ceil() as usize;
                            let dec_amt = (self.volume - new_volume) / num_decreases as f32;
                            self.volume_dec = vec![dec_amt; num_decreases];
                        }
                    }
                }
                self.audio.start();
                std::thread::sleep(std::time::Duration::from_millis(20));
                if !is_ending || !self.volume_dec.is_empty() {
                    self.recycle_buffers();
                }
                if self.audio.state() == audio::State::Stopped && is_ending {
                    break;
                }
            }
            self.release();
        });
        tx
    }

    fn calc_samples_per_buffer(&self) -> usize {
        ((self.seconds_buffered * 48000.0) as usize * (self.audio.channel_count() as usize)) / self.buffer_count
    }

    fn get_current_sample(&mut self) -> i16 {
        let sample = (self.samples[self.playing_current_sample] as f32) * self.volume;
        self.playing_current_sample += 1;
        sample as i16
    }

    unsafe fn fill_buffer(&mut self, buffer: *mut AudioOutBuffer) {
        let buffer_ptr = audio::get_data_ptr(buffer) as *mut i16;
        let buffer_size = audio::get_buffer_size(buffer);        
        let samples_per_buffer = self.calc_samples_per_buffer();
        for sample in 0..samples_per_buffer {
            if self.playing_current_sample > self.loop_end {
                self.playing_current_sample = self.loop_start;
            }
            
            *buffer_ptr.add((sample / 2) * (self.audio.channel_count() as usize) + (sample % 2)) = self.get_current_sample();
        }
        set_audio_out_buffer_info(buffer, buffer_ptr as *mut u8, buffer_size, samples_per_buffer * 2);
        self.audio.append(buffer);
    }

    fn create_buffer(&mut self) {
        let samples_per_buffer = self.calc_samples_per_buffer();
        let data_size = samples_per_buffer * 4;
        let buffer_size = align!(data_size, audio::BUFFER_ALIGNMENT);
        unsafe {
            let buffer_ptr = skyline::libc::memalign(audio::BUFFER_ALIGNMENT, buffer_size) as *mut u8;
            let buffer = Box::leak(Box::new(AudioOutBuffer([0; 0x10])));
            set_audio_out_buffer_info(buffer, buffer_ptr, buffer_size, data_size);
            self.fill_buffer(buffer);
        }
    }

    fn recycle_buffers(&mut self) {
        while let Some(buffer) = self.audio.released_buffer() {
            unsafe {
                if let Some(amt) = self.volume_dec.pop() {
                    self.volume -= amt;
                }
                self.fill_buffer(buffer);
            }
        }
    }

    fn release(&mut self) {
        while let Some(buffer) = self.audio.released_buffer() {
            unsafe {
                let buffer_ptr = audio::get_data_ptr(buffer);
                skyline::libc::free(buffer_ptr as _);
                drop(Box::from_raw(buffer));
            }
        }

        unsafe {
            drop(Box::from_raw(self.audio_complete_event));
            audio::close_audio_out(self.audio);
            drop(Box::from_raw(self.audio));
        }
    }
}