// Types for arrow routing

/// A point in the routing grid (fractional coordinates)
pub type Point = (f64, f64);

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

/// Bounding box for a child box
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: Point,
    pub max: Point,
}

impl BoundingBox {
    pub fn contains(&self, point: Point) -> bool {
        point.0 >= self.min.0
            && point.0 <= self.max.0
            && point.1 >= self.min.1
            && point.1 <= self.max.1
    }
}

/// Direction of movement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    None, // Starting position
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::None => Direction::None,
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
    pub direction: Direction, // Direction we came from to reach this node
}

impl Node {
    pub fn new(
        position: Point,
        g_cost: f64,
        h_cost: f64,
        parent: Option<Point>,
        direction: Direction,
    ) -> Self {
        Node {
            position,
            g_cost,
            h_cost,
            f_cost: g_cost + h_cost,
            parent,
            direction,
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
