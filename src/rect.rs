#[derive(Debug, Clone, Copy)]
pub struct Rect {
    /// Absolute position in the diagram coordinate space
    pub pos : (f64, f64),
    /// Absolute size (width, height) in the diagram coordinate space
    pub size : (f64, f64),
}

impl Rect {
    /// Create a new Rect
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect {
            pos: (x, y),
            size: (width, height),
        }
    }

    /// Get x coordinate
    pub fn x(&self) -> f64 {
        self.pos.0
    }

    /// Get y coordinate
    pub fn y(&self) -> f64 {
        self.pos.1
    }

    /// Get width
    pub fn width(&self) -> f64 {
        self.size.0
    }

    /// Get height
    pub fn height(&self) -> f64 {
        self.size.1
    }

    /// Get right edge x coordinate
    pub fn right(&self) -> f64 {
        self.pos.0 + self.size.0
    }

    /// Get bottom edge y coordinate
    pub fn bottom(&self) -> f64 {
        self.pos.1 + self.size.1
    }

    /// Scale the rectangle by a factor of s, centered at the center of the box
    ///
    /// # Arguments
    /// * `s` - The scaling factor (e.g., 2.0 doubles the size, 0.5 halves it)
    ///
    /// # Returns
    /// A new Rect that is scaled by the factor s, with the same center point
    pub fn scale_at_center(&self, s: f64) -> Rect {
        // Calculate current center
        let center_x = self.pos.0 + self.size.0 / 2.0;
        let center_y = self.pos.1 + self.size.1 / 2.0;

        // Calculate new size
        let new_width = self.size.0 * s;
        let new_height = self.size.1 * s;

        // Calculate new position to maintain the center
        let new_x = center_x - new_width / 2.0;
        let new_y = center_y - new_height / 2.0;

        Rect::new(new_x, new_y, new_width, new_height)
    }

    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }

    pub fn margin(&self, margin: f64) -> Rect {
        // Calculate new position to maintain the center
        let new_x = self.x() + margin;
        let new_y = self.y() + margin;
        let new_width = self.width() - 2.0 * margin;
        let new_height = self.height() - 2.0 * margin;
        Rect::new(new_x, new_y, new_width, new_height)
    }
}
