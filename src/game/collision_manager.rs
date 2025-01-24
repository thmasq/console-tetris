use gemini_engine::{
    containers::{CollisionContainer, PixelContainer},
    core::{CanDraw, ColChar, Vec2D},
    primitives::Rect,
};

pub fn generate_borders() -> PixelContainer {
    let mut borders = PixelContainer::new();
    borders.draw(&Rect::new(
        // Left wall
        Vec2D::new(0, 0),
        Vec2D::new(1, 21),
        ColChar::SOLID,
    ));
    borders.draw(&Rect::new(
        // Right wall
        Vec2D::new(11, 0),
        Vec2D::new(1, 21),
        ColChar::SOLID,
    ));
    borders.draw(&Rect::new(
        // Floor
        Vec2D::new(1, 20),
        Vec2D::new(10, 1),
        ColChar::SOLID,
    ));

    borders
}

pub struct CollisionManager {
    pub game_boundaries: PixelContainer,
    pub stationary_blocks: PixelContainer,
}

impl CollisionManager {
    pub fn new() -> Self {
        Self {
            game_boundaries: generate_borders(),
            stationary_blocks: PixelContainer::new(),
        }
    }

    pub fn get(&self) -> CollisionContainer<'_> {
        let mut collision = CollisionContainer::new();
        collision.push(&self.game_boundaries);
        collision.push(&self.stationary_blocks);
        collision
    }

    pub fn draw<E: CanDraw>(&mut self, element: &E) {
        self.stationary_blocks.draw(element);
    }

    // Remove all filled lines and return the number of lines filled and removed
    pub fn clear_filled_lines(&mut self) -> i64 {
        let mut pixels = self.stationary_blocks.pixels.clone();
        if pixels.is_empty() {
            return 0;
        }

        let mut cleared_lines = 0;

        let mut min_y = pixels.iter().map(|p| p.pos.y).min().unwrap_or(0);
        let max_y = pixels.iter().map(|p| p.pos.y).max().unwrap_or(0);

        'row: for y in min_y..=max_y {
            let row_pixels: Vec<i64> = pixels
                .iter()
                .filter(|p| p.pos.y == y)
                .map(|p| p.pos.x)
                .collect();

            for x in 1..11 {
                if !row_pixels.contains(&x) {
                    continue 'row;
                }
            }

            cleared_lines += 1;
            pixels.retain(|p| p.pos.y != y);
        }

        let mut y = max_y + 1;
        loop {
            y -= 1;
            if y < min_y {
                break;
            }

            let is_row_empty: bool = pixels
                .iter()
                .filter(|p| p.pos.y == y)
                .map(|p| p.pos.x)
                .next()
                .is_none();

            if is_row_empty {
                pixels = pixels
                    .iter()
                    .map(|p| {
                        if p.pos.y < y {
                            let mut moved_p = *p;
                            moved_p.pos.y += 1;
                            moved_p
                        } else {
                            *p
                        }
                    })
                    .collect();

                y += 1;
                min_y += 1;
            }
        }

        self.stationary_blocks.pixels = pixels;

        cleared_lines
    }

    /// Add an element to the stationary blocks and clear all full lines
    ///
    /// Returns the number of cleared lines
    pub fn draw_and_clear_lines<E: CanDraw>(&mut self, block: &E) -> i64 {
        self.draw(block);
        self.clear_filled_lines()
    }
}

impl CanDraw for CollisionManager {
    fn draw_to(&self, canvas: &mut impl gemini_engine::core::Canvas) {
        self.stationary_blocks.draw_to(canvas);
        self.game_boundaries.draw_to(canvas);
    }
}
