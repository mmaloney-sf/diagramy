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
    pub fn route(&mut self, start: Point, end: Point) -> Option<ArrowPath> {
        let path = self.find_path(start, end);

        // Generate debug SVG after routing (whether it succeeded or failed)
        self.generate_routing_debug_svg(start, end, self.routed_paths.len(), path.as_ref());

        if let Some(ref p) = path {
            self.routed_paths.push(p.clone());
        }

        path
    }

    /// A* pathfinding algorithm
    fn find_path(&self, start: Point, end: Point) -> Option<ArrowPath> {
        let mut open_set = BinaryHeap::new();
        // Store (parent_cell, direction_to_reach_this_cell)
        let mut came_from: HashMap<(i32, i32), ((i32, i32), Direction)> = HashMap::new();
        let mut g_score: HashMap<(i32, i32), f64> = HashMap::new();

        // Discretize start and end points to grid cells
        let start_cell = self.discretize(start);
        let end_cell = self.discretize(end);

        eprintln!("\n=== A* Pathfinding Debug ===");
        eprintln!("Start: {:?} (cell: {:?})", start, start_cell);
        eprintln!("End: {:?} (cell: {:?})", end, end_cell);
        eprintln!("Grid: {}x{}", self.grid_width, self.grid_height);
        eprintln!("Bounding boxes: {}", self.obstacle_boxes.len());

        // Initialize start node
        let h = self.heuristic(start_cell, end_cell);
        open_set.push(Node::new(
            self.cell_to_point(start_cell),
            0.0,
            h,
            None,
            Direction::None,
        ));
        g_score.insert(start_cell, 0.0);

        let mut iteration = 0;
        while let Some(current) = open_set.pop() {
            let current_cell = self.discretize(current.position);
            iteration += 1;

            eprintln!("\n--- Iteration {} ---", iteration);
            eprintln!(
                "Current position: {:?} (cell: {:?})",
                current.position, current_cell
            );
            eprintln!("Current direction: {:?}", current.direction);
            eprintln!(
                "Current g_cost: {:.2}, h_cost: {:.2}, f_cost: {:.2}",
                current.g_cost, current.h_cost, current.f_cost
            );

            // Check if we reached the goal
            if current_cell == end_cell {
                eprintln!("\n*** GOAL REACHED! ***");
                return Some(self.reconstruct_path(&came_from, current_cell, start, end));
            }

            // Explore neighbors
            eprintln!("Exploring neighbors:");
            let neighbors = self.get_neighbors(current_cell);
            for (neighbor_cell, direction) in neighbors {
                let neighbor_point = self.cell_to_point(neighbor_cell);
                eprintln!(
                    "  Neighbor: {:?} (cell: {:?}), direction: {:?}",
                    neighbor_point, neighbor_cell, direction
                );

                // Special constraint: if current position has a 0.0 coordinate and this is the first move,
                // we can only move in the direction away from that boundary
                if current.direction == Direction::None {
                    // This is the first move from start
                    let is_on_row_boundary = (current.position.0 - 0.0).abs() < 1e-6
                        || (current.position.0 - self.grid_height as f64).abs() < 1e-6;
                    let is_on_col_boundary = (current.position.1 - 0.0).abs() < 1e-6
                        || (current.position.1 - self.grid_width as f64).abs() < 1e-6;

                    if is_on_row_boundary || is_on_col_boundary {
                        // We're on a boundary, must move perpendicular to it
                        let allowed = if is_on_row_boundary && !is_on_col_boundary {
                            // On top or bottom boundary, can only move up/down
                            direction == Direction::Up || direction == Direction::Down
                        } else if is_on_col_boundary && !is_on_row_boundary {
                            // On left or right boundary, can only move left/right
                            direction == Direction::Left || direction == Direction::Right
                        } else {
                            // On a corner (both boundaries), this shouldn't happen but allow any move
                            true
                        };

                        if !allowed {
                            eprintln!("    -> SKIPPED: First move from boundary must be perpendicular to boundary");
                            continue;
                        }
                    }
                }

                // Skip if neighbor is out of bounds
                if !self.is_in_bounds(neighbor_point) {
                    eprintln!("    -> SKIPPED: Out of bounds");
                    continue;
                }

                // Skip if neighbor is inside a bounding box
                if let Some(bbox) = self.find_containing_bounding_box(neighbor_point) {
                    eprintln!("    -> SKIPPED: Inside bounding box");
                    eprintln!(
                        "       Bounding box: min={:?}, max={:?}",
                        bbox.min, bbox.max
                    );
                    continue;
                }

                // Check if this would create consecutive turns in the same direction
                // Get the direction we used to reach current cell
                let prev_direction = current.direction;

                // If we're turning, check if we turned in the previous step too
                if prev_direction != Direction::None && prev_direction != direction {
                    // We're changing direction (turning)
                    // Check if the previous move was also a turn
                    if let Some(&(_grandparent_cell, grandparent_dir)) =
                        came_from.get(&current_cell)
                    {
                        // If grandparent_dir != prev_direction, then the previous move was a turn
                        // We can't turn twice in a row
                        if grandparent_dir != Direction::None && grandparent_dir != prev_direction {
                            // Previous move was a turn, and now we're turning again - not allowed
                            eprintln!("    -> SKIPPED: Consecutive turns (grandparent: {:?}, prev: {:?}, current: {:?})",
                                     grandparent_dir, prev_direction, direction);
                            continue;
                        }
                    }
                }

                // Calculate movement cost
                let move_cost = self.calculate_move_cost(current.direction, direction);
                let tentative_g = g_score.get(&current_cell).unwrap_or(&f64::INFINITY) + move_cost;
                let current_best_g = *g_score.get(&neighbor_cell).unwrap_or(&f64::INFINITY);

                eprintln!(
                    "    Move cost: {:.2}, tentative_g: {:.2}, current_best_g: {:.2}",
                    move_cost, tentative_g, current_best_g
                );

                if tentative_g < current_best_g {
                    // This path to neighbor is better
                    came_from.insert(neighbor_cell, (current_cell, direction));
                    g_score.insert(neighbor_cell, tentative_g);

                    let h = self.heuristic(neighbor_cell, end_cell);
                    let f = tentative_g + h;
                    eprintln!(
                        "    -> ADDED to open set: g={:.2}, h={:.2}, f={:.2}",
                        tentative_g, h, f
                    );
                    open_set.push(Node::new(
                        neighbor_point,
                        tentative_g,
                        h,
                        Some(current.position),
                        direction,
                    ));
                } else {
                    eprintln!("    -> SKIPPED: Not better than existing path");
                }
            }
        }

        // No path found
        eprintln!("\n*** NO PATH FOUND after {} iterations ***", iteration);
        eprintln!("Open set is empty - all reachable nodes have been explored");
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
        (
            cell.0 as f64 * GRID_RESOLUTION,
            cell.1 as f64 * GRID_RESOLUTION,
        )
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

    /// Check if a point is within the grid bounds
    fn is_in_bounds(&self, point: Point) -> bool {
        point.0 >= 0.0
            && point.0 <= self.grid_height as f64
            && point.1 >= 0.0
            && point.1 <= self.grid_width as f64
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
        came_from: &HashMap<(i32, i32), ((i32, i32), Direction)>,
        mut current: (i32, i32),
        start: Point,
        end: Point,
    ) -> ArrowPath {
        eprintln!("\n=== Reconstructing Path ===");
        let mut path = vec![end];
        let mut directions = Vec::new();
        eprintln!("End point: {:?}", end);

        let mut step = 0;
        while let Some(&(parent, direction)) = came_from.get(&current) {
            step += 1;
            let point = self.cell_to_point(current);
            eprintln!(
                "Step {}: {:?} (cell: {:?}) <- direction: {:?}",
                step, point, current, direction
            );
            path.push(point);
            directions.push(direction);
            current = parent;
        }

        path.push(start);
        eprintln!("Start point: {:?}", start);
        path.reverse();
        directions.reverse();

        eprintln!("\n=== Path Explanation ===");
        eprintln!("Starting at: {:?}", start);
        for (i, point) in path.iter().enumerate().skip(1) {
            if i - 1 < directions.len() {
                let dir = directions[i - 1];
                let prev_point = path[i - 1];
                eprintln!(
                    "Step {}: From {:?} move {:?} to {:?}",
                    i, prev_point, dir, point
                );
            }
        }
        eprintln!("Arrived at: {:?}", end);

        eprintln!("\nFinal path ({} points):", path.len());
        for (i, point) in path.iter().enumerate() {
            eprintln!("  {}: {:?}", i, point);
        }

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
}
