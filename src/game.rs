use std::{
    io::stdout,
    time::{Duration, Instant},
};

use console_input::keypress::exit_raw_mode;
use crossterm::{
    cursor::MoveTo,
    event::{Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{Clear, ClearType},
};
use gemini_engine::{
    ascii::{Sprite, Text},
    core::{ColChar, Modifier, Vec2D},
    gameloop::MainLoopRoot,
    view::View,
};

mod alerts;
mod block_manager;
mod collision_manager;
mod pause;
use crate::audio::AudioManager;
use alerts::AlertDisplay;
use block_manager::BlockManager;
use collision_manager::CollisionManager;
use pause::pause;

use self::alerts::generate_alert_for_filled_lines;

pub struct Game {
    view: View,
    alert_display: AlertDisplay,
    block_manager: BlockManager,
    collision_manager: CollisionManager,
    score: i64,
    t: usize,
    // Constants
    controls_help_text: String,
    audio_manager: AudioManager,
    last_volume_adjust: Instant,
}

impl Game {
    pub fn new(
        block_place_cooldown: u32,
        piece_preview_count: usize,
        controls_help_text: &str,
    ) -> Self {
        let audio_manager = AudioManager::new();
        Self {
            view: View::new(50, 21, ColChar::EMPTY),
            alert_display: AlertDisplay::new(Vec2D::new(12, 7)),
            block_manager: BlockManager::new(block_place_cooldown, piece_preview_count),
            collision_manager: CollisionManager::new(),
            score: 0,
            t: 0,
            // Constants
            controls_help_text: controls_help_text.to_string(),
            audio_manager,
            last_volume_adjust: Instant::now(),
        }
    }
}

impl MainLoopRoot for Game {
    type InputDataType = Event;

    fn get_fps(&self) -> f32 {
        60.0
    }

    fn frame(&mut self, input_data: Option<Self::InputDataType>) {
        self.t += 1;
        let mut block_speed = 12;

        // Generate a collision with the current walls and placed blocks
        let collision = self.collision_manager.get();

        // Handle Inputs
        if let Some(Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        })) = input_data
        {
            let now = Instant::now();

            match code {
                // Pause
                KeyCode::Esc => {
                    self.view.clear();
                    self.view.display_render().expect("Failed to clear screen");
                    self.audio_manager.toggle();
                    pause();
                    self.audio_manager.toggle();
                }

                // Shift left
                KeyCode::Left => {
                    self.block_manager
                        .try_move_block(&collision, Vec2D::new(-1, 0));
                }

                // Shift right
                KeyCode::Right => {
                    self.block_manager
                        .try_move_block(&collision, Vec2D::new(1, 0));
                }

                // Rotate anti-clockwise
                KeyCode::Char('z') => {
                    self.block_manager.try_rotate_block(&collision, false);
                }

                // Rotate clockwise
                KeyCode::Up | KeyCode::Char('x') => {
                    self.block_manager.try_rotate_block(&collision, true);
                }

                // Soft drop
                KeyCode::Down => block_speed = 2,

                // Hard drop
                KeyCode::Char(' ') => {
                    self.score +=
                        self.block_manager.ghost_block.pos.y - self.block_manager.block.pos.y;
                    self.block_manager.block = self.block_manager.ghost_block.clone();
                    self.t = block_speed - 1;
                    self.block_manager.placing_cooldown = 1;
                }

                KeyCode::Char('c') => self.block_manager.hold(),

                KeyCode::Char('+') | KeyCode::Char('=') => {
                    if now.duration_since(self.last_volume_adjust) > Duration::from_millis(100) {
                        self.audio_manager.increase_volume(0.1);
                        self.last_volume_adjust = now;
                    }
                }
                KeyCode::Char('-') => {
                    if now.duration_since(self.last_volume_adjust) > Duration::from_millis(100) {
                        self.audio_manager.decrease_volume(0.1);
                        self.last_volume_adjust = now;
                    }
                }

                _ => (),
            }
        }

        // Place the ghost block directly beneath the active block
        self.block_manager.generate_ghost_block(&collision);

        // If the active block is on the floor...
        if collision.will_overlap_element(&self.block_manager.block, Vec2D::new(0, 1)) {
            // If the block's way down is blocked...
            self.block_manager.placing_cooldown -= 1;
            if self.block_manager.placing_cooldown == 0 {
                let pre_clear_blocks = self.collision_manager.stationary_blocks.clone();

                // If the current block is at the very top of the board...
                if self.block_manager.reset() {
                    println!("Game over!\r");
                    exit_raw_mode();
                }

                let cleared_lines = self
                    .collision_manager
                    .draw_and_clear_lines(&self.block_manager.block);

                // Display an appropriate alert
                self.alert_display.priorised_alerts_with_score(
                    &[
                        self.block_manager
                            .check_for_t_spin(&pre_clear_blocks, cleared_lines),
                        generate_alert_for_filled_lines(cleared_lines),
                    ],
                    &mut self.score,
                );

                self.block_manager.generate_new_block();
            }
        } else if self.t % block_speed == 0 {
            // move down and increase score for soft drop
            self.block_manager
                .try_move_block(&collision, Vec2D::new(0, 1));
            if block_speed == 2 {
                self.score += 1;
            }
        }
    }

    fn render_frame(&mut self) {
        self.view.clear();

        // Blit the walls and stationary blocks
        self.view.draw_double_width(&self.collision_manager);

        self.view.draw_double_width(&self.block_manager.ghost_block);
        self.view.draw_double_width(&self.block_manager.block);

        // Next piece display
        self.view
            .draw(&Text::new(Vec2D::new(29, 9), "Next:", Modifier::None));
        self.view
            .draw_double_width(&self.block_manager.next_piece_display());

        // Held piece display
        if let Some(held_piece) = self.block_manager.held_piece_display() {
            self.view
                .draw(&Text::new(Vec2D::new(29, 1), "Hold", Modifier::None));
            self.view.draw_double_width(&held_piece);
        } else {
            self.view.draw(&Sprite::new(
                Vec2D::new(26, 0),
                &self.controls_help_text,
                Modifier::None,
            ));
        }

        // Score display
        self.view.draw(&Text::new(
            Vec2D::new(26, 7),
            &format!("Score: {}", self.score),
            Modifier::None,
        ));

        // Alerts display
        self.view.draw(&self.alert_display);
        self.alert_display.frame();

        execute!(stdout(), MoveTo(0, 0)).unwrap();
        execute!(stdout(), Clear(ClearType::FromCursorDown)).unwrap();
        self.view
            .display_render()
            .expect("Failed to print render to screen");
    }

    fn sleep_and_get_input_data(
        &self,
        fps: f32,
        elapsed: std::time::Duration,
    ) -> (bool, Option<Self::InputDataType>) {
        let frame_duration = std::time::Duration::from_secs_f32(1.0 / fps);
        let sleep_duration = frame_duration
            .checked_sub(elapsed)
            .unwrap_or(std::time::Duration::ZERO);

        if sleep_duration > std::time::Duration::ZERO {
            std::thread::sleep(sleep_duration);
        }

        if let Ok(event) = crossterm::event::poll(std::time::Duration::from_millis(0)) {
            if event {
                if let Ok(input_event) = crossterm::event::read() {
                    return (false, Some(input_event));
                }
            }
        }

        (false, None)
    }
}
