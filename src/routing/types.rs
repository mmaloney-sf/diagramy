// Types for arrow routing

/// A point in the routing grid (integral coordinates)
pub type Point = (i32, i32);

/// A path for an arrow, consisting of a sequence of points
#[derive(Debug, Clone)]
pub struct ArrowPath {
    pub points: Vec<Point>,
}

impl ArrowPath {
    pub fn new(points: Vec<Point>) -> Self {
        ArrowPath { points }
    }

    pub fn start(&self) -> Option<Point> {
        self.points.first().copied()
    }

    pub fn end(&self) -> Option<Point> {
        self.points.last().copied()
    }
}

/// Represents a crossing between two arrow paths
#[derive(Debug, Clone)]
pub struct ArrowPathCrossing {
    pub path1_index: usize,
    pub path2_index: usize,
    pub crossing_point: Point,
}

/// Bounding box for a child box (stored in fractional coordinates)
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min_frac: (f64, f64),
    pub max_frac: (f64, f64),
}

impl BoundingBox {
    /// Check if a discretized point is inside this bounding box
    /// point is in discretized coordinates (scaled by grid_resolution)
    /// This bounding box is in fractional coordinates
    pub fn contains(&self, point: Point, grid_resolution: i32) -> bool {
        let min_row = (self.min_frac.0 * grid_resolution as f64).round() as i32;
        let min_col = (self.min_frac.1 * grid_resolution as f64).round() as i32;
        let max_row = (self.max_frac.0 * grid_resolution as f64).round() as i32;
        let max_col = (self.max_frac.1 * grid_resolution as f64).round() as i32;

        point.0 >= min_row
            && point.0 <= max_row
            && point.1 >= min_col
            && point.1 <= max_col
    }

    /// Get the discretized min point
    pub fn min_discretized(&self, grid_resolution: i32) -> Point {
        (
            (self.min_frac.0 * grid_resolution as f64).round() as i32,
            (self.min_frac.1 * grid_resolution as f64).round() as i32,
        )
    }

    /// Get the discretized max point
    pub fn max_discretized(&self, grid_resolution: i32) -> Point {
        (
            (self.max_frac.0 * grid_resolution as f64).round() as i32,
            (self.max_frac.1 * grid_resolution as f64).round() as i32,
        )
    }
}

/// Direction of movement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// A* search node
#[derive(Debug, Clone)]
pub struct Node {
    pub position: Point,
    pub g_cost: f64, // Cost from start to this node
    pub h_cost: f64, // Heuristic cost from this node to end
    pub f_cost: f64, // Total cost (g + h)
    pub parent: Option<Point>,
}

fn dir_away_from_wall(position: Point, grid_height: i32, grid_width: i32) -> Option<Direction> {
    let (row, col) = position;

    // Check for corner positions (not allowed)
    if (row == 0 && col == 0) || (row == grid_height - 1 && col == grid_width - 1) {
        panic!("Corner positions are not allowed: ({}, {})", row, col);
    }

    // Determine direction based on wall position
    if row == 0 {
        // Top wall - direction should be Down (away from wall)
        Some(Direction::Down)
    } else if row == grid_height - 1 {
        // Bottom wall - direction should be Up (away from wall)
        Some(Direction::Up)
    } else if col == 0 {
        // Left wall - direction should be Right (away from wall)
        Some(Direction::Right)
    } else if col == grid_width - 1 {
        // Right wall - direction should be Left (away from wall)
        Some(Direction::Left)
    } else {
        // Not on a wall
        None
    }
}

pub fn relative_dir(from: Point, to: Point) -> Option<Direction> {
    let (from_row, from_col) = from;
    let (to_row, to_col) = to;

    let row_diff = to_row - from_row;
    let col_diff = to_col - from_col;

    // Check if points are adjacent (exactly 1 step away in one direction, 0 in the other)
    match (row_diff, col_diff) {
        (-1, 0) => Some(Direction::Up),
        (1, 0) => Some(Direction::Down),
        (0, -1) => Some(Direction::Left),
        (0, 1) => Some(Direction::Right),
        _ => None, // Not adjacent or diagonal
    }
}

impl Node {
    pub fn new(
        position: Point,
        g_cost: f64,
        h_cost: f64,
        parent: Option<Point>,
    ) -> Self {
        Node {
            position,
            g_cost,
            h_cost,
            f_cost: g_cost + h_cost,
            parent,
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}

impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}
