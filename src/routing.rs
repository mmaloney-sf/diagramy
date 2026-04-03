pub mod debug;
pub mod types;

pub use types::{ArrowPath, ArrowPathCrossing, BoundingBox, Direction, Node, Point, relative_dir};

use std::collections::{BinaryHeap, HashMap};

pub struct ArrowRouter {
    grid_width: f64,
    grid_height: f64,
    grid_resolution: i32,
    bounding_boxes: Vec<BoundingBox>,
    debug_dir: Option<String>,
    box_name: Option<String>,
}

impl ArrowRouter {
    pub fn new(grid_width: f64, grid_height: f64, bounding_boxes: Vec<BoundingBox>) -> Self {
        ArrowRouter {
            grid_width,
            grid_height,
            grid_resolution: 10,
            bounding_boxes,
            debug_dir: None,
            box_name: None,
        }
    }

    pub fn set_debug_dir(&mut self, dir: &str, box_name: &str) {
        self.debug_dir = Some(dir.to_string());
        self.box_name = Some(box_name.to_string());
    }

    pub fn grid_resolution(&self) -> i32 {
        self.grid_resolution
    }

    pub fn route(&mut self, _start: (f64, f64), _end: (f64, f64), _excluded_box_indices: &[usize]) -> Option<ArrowPath> {
        // TODO: Implement arrow routing
        None
    }
}
