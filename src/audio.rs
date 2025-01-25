use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;

const MUSIC_DATA: &[u8] = include_bytes!("../assets/tetris.flac");

#[derive(Debug)]
pub enum AudioCommand {
    SetVolume(f32),
    Stop,
    Resume,
}

pub struct AudioManager {
    command_sender: Sender<AudioCommand>,
    volume: Arc<AtomicI32>,
    playing: Arc<AtomicBool>,
}

impl AudioManager {
    pub fn new() -> Self {
        let (command_sender, command_receiver) = channel();
        let volume = Arc::new(AtomicI32::new(50));
        let playing = Arc::new(AtomicBool::new(true));
        let volume_clone = volume.clone();
        let playing_clone = playing.clone();

        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();

            let cursor = Cursor::new(MUSIC_DATA);
            let source = Decoder::new(cursor).unwrap();
            sink.append(source);
            sink.set_volume(volume_clone.load(Ordering::Relaxed) as f32 / 100.0);

            loop {
                if let Ok(command) = command_receiver.try_recv() {
                    match command {
                        AudioCommand::SetVolume(vol) => {
                            let vol_int = (vol * 100.0).clamp(0.0, 100.0) as i32;
                            volume_clone.store(vol_int, Ordering::Relaxed);
                            sink.set_volume(vol_int as f32 / 100.0);
                        }
                        AudioCommand::Stop => {
                            sink.stop();
                            playing_clone.store(false, Ordering::Relaxed);
                        }
                        AudioCommand::Resume => {
                            sink.play();
                            playing_clone.store(true, Ordering::Relaxed);
                        }
                    }
                }

                if sink.empty() {
                    let cursor = Cursor::new(MUSIC_DATA);
                    let source = Decoder::new(cursor).unwrap();
                    sink.append(source);
                }

                thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        Self {
            command_sender,
            volume,
            playing,
        }
    }

    pub fn set_volume(&self, volume: f32) {
        let vol_int = (volume * 100.0).clamp(0.0, 100.0) as i32;
        self.volume.store(vol_int, Ordering::Relaxed);
        let volume_float = vol_int as f32 / 100.0;
        let _ = self
            .command_sender
            .send(AudioCommand::SetVolume(volume_float));
    }

    pub fn get_volume(&self) -> f32 {
        self.volume.load(Ordering::Relaxed) as f32 / 100.0
    }

    pub fn increase_volume(&self, amount: f32) {
        let current = self.get_volume();
        self.set_volume(current + amount);
    }

    pub fn decrease_volume(&self, amount: f32) {
        let current = self.get_volume();
        self.set_volume(current - amount);
    }

    pub fn stop(&self) {
        let _ = self.command_sender.send(AudioCommand::Stop);
        self.playing.store(false, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        let _ = self.command_sender.send(AudioCommand::Resume);
        self.playing.store(true, Ordering::Relaxed);
    }

    pub fn toggle(&self) {
        if self.playing.load(Ordering::Relaxed) {
            self.stop();
        } else {
            self.resume();
        }
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop();
    }
}
