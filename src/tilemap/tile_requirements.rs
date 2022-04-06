use serde::{Deserialize, Serialize};

use super::Orientation;

#[derive(Default, Clone, Copy, Serialize, Deserialize)]
pub struct TileRequirements {
    n: Option<bool>,
    s: Option<bool>,
    e: Option<bool>,
    w: Option<bool>,

    ne: Option<bool>,
    nw: Option<bool>,
    se: Option<bool>,
    sw: Option<bool>,
}

impl TileRequirements {
    /// Get the tile requirement that corresponds to this direction - mutable
    pub fn get_requirement_mut(&mut self, o: Orientation) -> Result<&mut Option<bool>, ()> {
        match o {
            Orientation::N => Ok(&mut self.n),
            Orientation::S => Ok(&mut self.s),
            Orientation::E => Ok(&mut self.e),
            Orientation::W => Ok(&mut self.w),

            Orientation::NE => Ok(&mut self.ne),
            Orientation::NW => Ok(&mut self.nw),
            Orientation::SE => Ok(&mut self.se),
            Orientation::SW => Ok(&mut self.sw),

            _ => Err(()),
        }
    }
    /// Get the tile requirement that corresponds to this direction
    pub fn get_requirement(&self, o: Orientation) -> Option<&Option<bool>> {
        match o {
            Orientation::N => Some(&self.n),
            Orientation::S => Some(&self.s),
            Orientation::E => Some(&self.e),
            Orientation::W => Some(&self.w),

            Orientation::NE => Some(&self.ne),
            Orientation::NW => Some(&self.nw),
            Orientation::SE => Some(&self.se),
            Orientation::SW => Some(&self.sw),

            _ => None,
        }
    }
}
