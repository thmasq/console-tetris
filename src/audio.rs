use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

const MUSIC_DATA: &[u8] = include_bytes!("../assets/tetris.flac");

#[derive(Debug, Clone, PartialEq)]
pub enum AudioState {
    Playing,
    Paused,
}

#[derive(Debug)]
pub enum AudioCommand {
    SetVolume(f32),
    Stop,
    Resume,
}

pub struct AudioManager {
    command_sender: Sender<AudioCommand>,
    state: Arc<Mutex<AudioState>>,
    volume: Arc<Mutex<f32>>,
}

impl AudioManager {
    pub fn new() -> Self {
        let (command_sender, command_receiver) = channel();
        let state = Arc::new(Mutex::new(AudioState::Playing));
        let volume = Arc::new(Mutex::new(0.5));

        let state_clone = state.clone();
        let volume_clone = volume.clone();

        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
            let cursor = Cursor::new(MUSIC_DATA);
            let source = Decoder::new(cursor).unwrap();
            sink.append(source);

            sink.set_volume(*volume_clone.lock().unwrap());

            loop {
                if let Ok(command) = command_receiver.try_recv() {
                    match command {
                        AudioCommand::SetVolume(vol) => {
                            let clamped_vol = vol.clamp(0.0, 1.0);
                            *volume_clone.lock().unwrap() = clamped_vol;
                            sink.set_volume(clamped_vol);
                        }
                        AudioCommand::Stop => {
                            sink.pause();
                            *state_clone.lock().unwrap() = AudioState::Paused;
                        }
                        AudioCommand::Resume => {
                            sink.play();
                            *state_clone.lock().unwrap() = AudioState::Playing;
                        }
                    }
                }

                // Loop the audio if it's finished
                if sink.empty() {
                    let cursor = Cursor::new(include_bytes!("../assets/tetris.flac"));
                    let source = Decoder::new(cursor).unwrap();
                    sink.append(source);
                }

                thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        Self {
            command_sender,
            state,
            volume,
        }
    }

    pub fn set_volume(&self, volume: f32) {
        let clamped_vol = volume.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped_vol;
        let _ = self
            .command_sender
            .send(AudioCommand::SetVolume(clamped_vol));
    }

    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
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
    }

    pub fn resume(&self) {
        let _ = self.command_sender.send(AudioCommand::Resume);
    }

    pub fn toggle(&self) {
        let current_state = self.state.lock().unwrap().clone();
        match current_state {
            AudioState::Playing => self.stop(),
            AudioState::Paused => self.resume(),
        }
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop();
    }
}
