// Arrow routing algorithms using A* pathfinding

pub mod debug;
pub mod types;

// Re-export types for convenience
pub use types::{ArrowPath, ArrowPathCrossing, BoundingBox, Direction, Node, Point};

use std::collections::{BinaryHeap, HashMap};

/// Arrow router using A* pathfinding
pub struct ArrowRouter {
    grid_width: u64,
    grid_height: u64,
    grid_resolution: i32,
    obstacle_boxes: Vec<BoundingBox>,
    routed_paths: Vec<ArrowPath>,
    debug_dir: Option<String>,
    box_name: Option<String>,
}

impl ArrowRouter {
    pub fn new(grid_width: f64, grid_height: f64, bounding_boxes: Vec<BoundingBox>) -> Self {
        ArrowRouter {
            grid_width: grid_width as u64,
            grid_height: grid_height as u64,
            grid_resolution: 10, // 10 routable grid squares per original grid square
            obstacle_boxes: bounding_boxes,
            routed_paths: Vec::new(),
            debug_dir: None,
            box_name: None,
        }
    }

    pub fn set_debug_dir(&mut self, dir: &str, box_name: &str) {
        self.debug_dir = Some(dir.to_string());
        self.box_name = Some(box_name.to_string());
    }

    /// Route an arrow from start to end using A* pathfinding
    /// Takes fractional coordinates and discretizes them immediately
    pub fn route(&mut self, start: (f64, f64), end: (f64, f64)) -> Option<ArrowPath> {
        // Discretize to integral coordinates
        let start_point = self.discretize(start);
        let end_point = self.discretize(end);

        let path = self.find_path(start_point, end_point);

        // Generate debug SVG after routing (whether it succeeded or failed)
        self.generate_routing_debug_svg(start_point, end_point, self.routed_paths.len(), path.as_ref());

        if let Some(ref p) = path {
            self.routed_paths.push(p.clone());
        }

        path
    }

    /// Discretize a continuous point to integral grid coordinates
    fn discretize(&self, point: (f64, f64)) -> Point {
        let row = (point.0 * self.grid_resolution as f64).round() as i32;
        let col = (point.1 * self.grid_resolution as f64).round() as i32;
        (row, col)
    }

    /// A* pathfinding algorithm
    fn find_path(&self, start: Point, end: Point) -> Option<ArrowPath> {
        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<Point, Point> = HashMap::new();
        let mut g_score: HashMap<Point, f64> = HashMap::new();

        // Initialize start node
        let h = self.heuristic(start, end);
        open_set.push(Node::new(
            start,
            0.0,
            h,
            None,
        ));
        g_score.insert(start, 0.0);

        while let Some(current) = open_set.pop() {
            let current_point = current.position;

            // Check if we reached the goal
            if current_point == end {
                return Some(self.reconstruct_path(&came_from, current_point, start, end));
            }

            // Explore neighbors
            let neighbors = self.get_neighbors(current_point);
            for (neighbor_point, _direction) in neighbors {
                // Skip if neighbor is out of bounds
                if !self.is_in_bounds(neighbor_point) {
                    continue;
                }

                // Skip if neighbor is inside a bounding box
                if let Some(_bbox) = self.find_containing_bounding_box(neighbor_point) {
                    continue;
                }

                // Calculate movement cost (uniform cost)
                let move_cost = 1.0;
                let tentative_g = g_score.get(&current_point).unwrap_or(&f64::INFINITY) + move_cost;
                let current_best_g = *g_score.get(&neighbor_point).unwrap_or(&f64::INFINITY);

                if tentative_g < current_best_g {
                    // This path to neighbor is better
                    came_from.insert(neighbor_point, current_point);
                    g_score.insert(neighbor_point, tentative_g);

                    let h = self.heuristic(neighbor_point, end);
                    open_set.push(Node::new(
                        neighbor_point,
                        tentative_g,
                        h,
                        Some(current.position),
                    ));
                }
            }
        }

        // No path found
        None
    }

    /// Manhattan distance heuristic
    fn heuristic(&self, from: Point, to: Point) -> f64 {
        ((from.0 - to.0).abs() + (from.1 - to.1).abs()) as f64
    }

    /// Get neighboring points (4-connected grid)
    fn get_neighbors(&self, point: Point) -> Vec<(Point, Direction)> {
        vec![
            ((point.0 - 1, point.1), Direction::Up),
            ((point.0 + 1, point.1), Direction::Down),
            ((point.0, point.1 - 1), Direction::Left),
            ((point.0, point.1 + 1), Direction::Right),
        ]
    }

    /// Check if a point is within the grid bounds
    fn is_in_bounds(&self, point: Point) -> bool {
        point.0 >= 0
            && point.0 <= (self.grid_height as i32) * self.grid_resolution
            && point.1 >= 0
            && point.1 <= (self.grid_width as i32) * self.grid_resolution
    }

    /// Check if a point is inside any bounding box
    fn is_inside_bounding_box(&self, point: Point) -> bool {
        self.obstacle_boxes.iter().any(|bbox| bbox.contains(point))
    }

    /// Find the bounding box that contains a point, if any
    fn find_containing_bounding_box(&self, point: Point) -> Option<&BoundingBox> {
        self.obstacle_boxes.iter().find(|bbox| bbox.contains(point))
    }

    /// Reconstruct the path from the came_from map
    fn reconstruct_path(
        &self,
        came_from: &HashMap<Point, Point>,
        mut current: Point,
        start: Point,
        end: Point,
    ) -> ArrowPath {
        let mut path = vec![end];

        while let Some(&parent) = came_from.get(&current) {
            path.push(current);
            current = parent;
        }

        path.push(start);
        path.reverse();

        ArrowPath::new(path)
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

                if let Some(intersection) =
                    self.line_intersection(p1_start, p1_end, p2_start, p2_end)
                {
                    return Some(intersection);
                }
            }
        }
        None
    }

    /// Find intersection point of two line segments
    fn line_intersection(&self, p1: Point, p2: Point, p3: Point, p4: Point) -> Option<Point> {
        let x1 = p1.1 as f64;
        let y1 = p1.0 as f64;
        let x2 = p2.1 as f64;
        let y2 = p2.0 as f64;
        let x3 = p3.1 as f64;
        let y3 = p3.0 as f64;
        let x4 = p4.1 as f64;
        let y4 = p4.0 as f64;

        let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
        if denom.abs() < 1e-10 {
            return None; // Lines are parallel
        }

        let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
        let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / denom;

        if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
            let x = x1 + t * (x2 - x1);
            let y = y1 + t * (y2 - y1);
            Some((y.round() as i32, x.round() as i32)) // Return as (row, col)
        } else {
            None
        }
    }

    pub fn get_routed_paths(&self) -> &[ArrowPath] {
        &self.routed_paths
    }
}
