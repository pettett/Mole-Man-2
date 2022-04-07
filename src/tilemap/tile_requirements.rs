use serde::{Deserialize, Serialize};

use super::Orientation;

#[derive(Default, Clone, Copy, Serialize, Deserialize)]
/// 0   `n: Option<bool>`
///
/// 1   `s: Option<bool>`
///
/// 2   `e: Option<bool>`
///
/// 3   `w: Option<bool>`
///
/// 4   `ne: Option<bool>`
///
/// 5   `nw: Option<bool>`
///
/// 6   `se: Option<bool>`
///
/// 7   `sw: Option<bool>`
pub struct TileRequirements {
    pub dirs: [Option<bool>; 8],
}

impl From<Orientation> for TileRequirements {
    fn from(o: Orientation) -> Self {
        Self {
            dirs: [
                Some(o.contains(Orientation::N)),
                Some(o.contains(Orientation::S)),
                Some(o.contains(Orientation::E)),
                Some(o.contains(Orientation::W)),
                Some(o.contains(Orientation::NE)),
                Some(o.contains(Orientation::NW)),
                Some(o.contains(Orientation::SE)),
                Some(o.contains(Orientation::SW)),
            ],
        }
    }
}

impl TileRequirements {
    pub fn n_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[0]
    }
    pub fn s_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[1]
    }
    pub fn e_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[2]
    }
    pub fn w_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[3]
    }
    pub fn ne_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[4]
    }
    pub fn nw_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[5]
    }
    pub fn se_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[6]
    }
    pub fn sw_mut(&mut self) -> &mut Option<bool> {
        &mut self.dirs[7]
    }

    pub fn n(&self) -> &Option<bool> {
        &self.dirs[0]
    }
    pub fn s(&self) -> &Option<bool> {
        &self.dirs[1]
    }
    pub fn e(&self) -> &Option<bool> {
        &self.dirs[2]
    }
    pub fn w(&self) -> &Option<bool> {
        &self.dirs[3]
    }
    pub fn ne(&self) -> &Option<bool> {
        &self.dirs[4]
    }
    pub fn nw(&self) -> &Option<bool> {
        &self.dirs[5]
    }
    pub fn se(&self) -> &Option<bool> {
        &self.dirs[6]
    }
    pub fn sw(&self) -> &Option<bool> {
        &self.dirs[7]
    }
    /// Get the tile requirement that corresponds to this direction - mutable
    pub fn get_requirement_mut(&mut self, o: Orientation) -> Result<&mut Option<bool>, ()> {
        match o {
            Orientation::N => Ok(self.n_mut()),
            Orientation::S => Ok(self.s_mut()),
            Orientation::E => Ok(self.e_mut()),
            Orientation::W => Ok(self.w_mut()),

            Orientation::NE => Ok(self.ne_mut()),
            Orientation::NW => Ok(self.nw_mut()),
            Orientation::SE => Ok(self.se_mut()),
            Orientation::SW => Ok(self.sw_mut()),

            _ => Err(()),
        }
    }
    /// Get the tile requirement that corresponds to this direction
    pub fn get_requirement(&self, o: Orientation) -> Option<&Option<bool>> {
        match o {
            Orientation::N => Some(self.n()),
            Orientation::S => Some(self.s()),
            Orientation::E => Some(self.e()),
            Orientation::W => Some(self.w()),

            Orientation::NE => Some(self.ne()),
            Orientation::NW => Some(self.nw()),
            Orientation::SE => Some(self.se()),
            Orientation::SW => Some(self.sw()),

            _ => None,
        }
    }
}
