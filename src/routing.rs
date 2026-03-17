// Arrow routing algorithms using A* pathfinding

use crate::ast::CoordsFrac;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;

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
        point.0 >= self.min.0 && point.0 <= self.max.0 &&
        point.1 >= self.min.1 && point.1 <= self.max.1
    }
}

/// Direction of movement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
    None, // Starting position
}

impl Direction {
    fn opposite(&self) -> Direction {
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
struct Node {
    position: Point,
    g_cost: f64, // Cost from start to this node
    h_cost: f64, // Heuristic cost from this node to end
    f_cost: f64, // Total cost (g + h)
    parent: Option<Point>,
    direction: Direction, // Direction we came from to reach this node
}

impl Node {
    fn new(position: Point, g_cost: f64, h_cost: f64, parent: Option<Point>, direction: Direction) -> Self {
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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
    }
}

/// Arrow router using A* pathfinding
pub struct ArrowRouter {
    grid_width: f64,
    grid_height: f64,
    bounding_boxes: Vec<BoundingBox>,
    routed_paths: Vec<ArrowPath>,
    crossings: Vec<ArrowPathCrossing>,
}

impl ArrowRouter {
    pub fn new(grid_width: f64, grid_height: f64, bounding_boxes: Vec<BoundingBox>) -> Self {
        ArrowRouter {
            grid_width,
            grid_height,
            bounding_boxes,
            routed_paths: Vec::new(),
            crossings: Vec::new(),
        }
    }

    /// Route an arrow from start to end using A* pathfinding
    pub fn route(&mut self, start: Point, end: Point) -> Option<ArrowPath> {
        let path = self.find_path(start, end)?;

        // Check for crossings with existing paths
        self.detect_crossings(&path);

        self.routed_paths.push(path.clone());
        Some(path)
    }

    /// A* pathfinding algorithm
    fn find_path(&self, start: Point, end: Point) -> Option<ArrowPath> {
        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<(i32, i32), ((i32, i32), Direction)> = HashMap::new();
        let mut g_score: HashMap<(i32, i32), f64> = HashMap::new();

        // Discretize start and end points to grid cells
        let start_cell = self.discretize(start);
        let end_cell = self.discretize(end);

        // Initialize start node
        let h = self.heuristic(start_cell, end_cell);
        open_set.push(Node::new(self.cell_to_point(start_cell), 0.0, h, None, Direction::None));
        g_score.insert(start_cell, 0.0);

        while let Some(current) = open_set.pop() {
            let current_cell = self.discretize(current.position);

            // Check if we reached the goal
            if current_cell == end_cell {
                return Some(self.reconstruct_path(&came_from, current_cell, start, end));
            }

            // Explore neighbors
            for (neighbor_cell, direction) in self.get_neighbors(current_cell) {
                // Skip if neighbor is inside a bounding box
                if self.is_inside_bounding_box(self.cell_to_point(neighbor_cell)) {
                    continue;
                }

                // Calculate movement cost
                let move_cost = self.calculate_move_cost(current.direction, direction);
                let tentative_g = g_score.get(&current_cell).unwrap_or(&f64::INFINITY) + move_cost;

                if tentative_g < *g_score.get(&neighbor_cell).unwrap_or(&f64::INFINITY) {
                    // This path to neighbor is better
                    came_from.insert(neighbor_cell, (current_cell, direction));
                    g_score.insert(neighbor_cell, tentative_g);

                    let h = self.heuristic(neighbor_cell, end_cell);
                    let neighbor_point = self.cell_to_point(neighbor_cell);
                    open_set.push(Node::new(neighbor_point, tentative_g, h, Some(current.position), direction));
                }
            }
        }

        // No path found
        None
    }

    /// Discretize a continuous point to a grid cell
    fn discretize(&self, point: Point) -> (i32, i32) {
        const GRID_RESOLUTION: f64 = 0.1; // 10 cells per unit
        let row = (point.0 / GRID_RESOLUTION).round() as i32;
        let col = (point.1 / GRID_RESOLUTION).round() as i32;
        (row, col)
    }

    /// Convert a grid cell back to a continuous point
    fn cell_to_point(&self, cell: (i32, i32)) -> Point {
        const GRID_RESOLUTION: f64 = 0.1;
        (cell.0 as f64 * GRID_RESOLUTION, cell.1 as f64 * GRID_RESOLUTION)
    }

    /// Manhattan distance heuristic
    fn heuristic(&self, from: (i32, i32), to: (i32, i32)) -> f64 {
        ((from.0 - to.0).abs() + (from.1 - to.1).abs()) as f64
    }

    /// Get neighboring cells (4-connected grid)
    fn get_neighbors(&self, cell: (i32, i32)) -> Vec<((i32, i32), Direction)> {
        vec![
            ((cell.0 - 1, cell.1), Direction::Up),
            ((cell.0 + 1, cell.1), Direction::Down),
            ((cell.0, cell.1 - 1), Direction::Left),
            ((cell.0, cell.1 + 1), Direction::Right),
        ]
    }

    /// Calculate movement cost with penalty for direction changes
    fn calculate_move_cost(&self, from_dir: Direction, to_dir: Direction) -> f64 {
        const BASE_COST: f64 = 1.0;
        const TURN_PENALTY: f64 = 2.0;

        if from_dir == Direction::None || from_dir == to_dir {
            BASE_COST
        } else if from_dir == to_dir.opposite() {
            // 180-degree turn (very bad)
            BASE_COST + TURN_PENALTY * 2.0
        } else {
            // 90-degree turn
            BASE_COST + TURN_PENALTY
        }
    }

    /// Check if a point is inside any bounding box
    fn is_inside_bounding_box(&self, point: Point) -> bool {
        self.bounding_boxes.iter().any(|bbox| bbox.contains(point))
    }

    /// Reconstruct the path from the came_from map
    fn reconstruct_path(
        &self,
        came_from: &HashMap<(i32, i32), ((i32, i32), Direction)>,
        mut current: (i32, i32),
        start: Point,
        end: Point,
    ) -> ArrowPath {
        let mut path = vec![end];

        while let Some(&(parent, _)) = came_from.get(&current) {
            path.push(self.cell_to_point(current));
            current = parent;
        }

        path.push(start);
        path.reverse();

        ArrowPath::new(path)
    }

    /// Detect crossings between the new path and existing paths
    fn detect_crossings(&mut self, new_path: &ArrowPath) {
        let new_path_index = self.routed_paths.len();

        for (existing_index, existing_path) in self.routed_paths.iter().enumerate() {
            if let Some(crossing_point) = self.find_crossing(new_path, existing_path) {
                self.crossings.push(ArrowPathCrossing {
                    path1_index: existing_index,
                    path2_index: new_path_index,
                    crossing_point,
                });
            }
        }
    }

    /// Find if two paths cross
    fn find_crossing(&self, path1: &ArrowPath, path2: &ArrowPath) -> Option<Point> {
        // Check each segment of path1 against each segment of path2
        for i in 0..path1.points.len().saturating_sub(1) {
            let p1_start = path1.points[i];
            let p1_end = path1.points[i + 1];

            for j in 0..path2.points.len().saturating_sub(1) {
                let p2_start = path2.points[j];
                let p2_end = path2.points[j + 1];

                if let Some(intersection) = self.line_intersection(p1_start, p1_end, p2_start, p2_end) {
                    return Some(intersection);
                }
            }
        }
        None
    }

    /// Find intersection point of two line segments
    fn line_intersection(&self, p1: Point, p2: Point, p3: Point, p4: Point) -> Option<Point> {
        let x1 = p1.1;
        let y1 = p1.0;
        let x2 = p2.1;
        let y2 = p2.0;
        let x3 = p3.1;
        let y3 = p3.0;
        let x4 = p4.1;
        let y4 = p4.0;

        let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
        if denom.abs() < 1e-10 {
            return None; // Lines are parallel
        }

        let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
        let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / denom;

        if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
            let x = x1 + t * (x2 - x1);
            let y = y1 + t * (y2 - y1);
            Some((y, x)) // Return as (row, col)
        } else {
            None
        }
    }

    pub fn get_routed_paths(&self) -> &[ArrowPath] {
        &self.routed_paths
    }

    pub fn get_crossings(&self) -> &[ArrowPathCrossing] {
        &self.crossings
    }
}
