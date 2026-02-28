mod contour;
mod contour_combiner;
mod edge;
mod edge_coloring;
mod edge_selector;
mod glyph;
mod shape;
mod shape_distance_finder;
mod solvers;
mod types;
mod utils;

pub use glyph::*;
pub use ttf_parser;
pub use types::*;

#[cfg(feature = "atlas")]
mod atlas;
#[cfg(feature = "atlas")]
pub use atlas::*;

pub(crate) use utils::{Vec2Ext, bound_point, median};
