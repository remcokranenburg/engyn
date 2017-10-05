// Copyright (c) 2017 Remco Kranenburg
//
// GNU GENERAL PUBLIC LICENSE
//    Version 3, 29 June 2007
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::cell::RefCell;
use std::rc::Rc;

pub struct Quality {
  pub level: Rc<RefCell<f32>>,
  pub weight_resolution: Rc<RefCell<f32>>,
  pub weight_msaa: Rc<RefCell<f32>>,
}

impl Quality {
  pub fn new() -> Quality {
    Quality {
      level: Rc::new(RefCell::new(1.0)),
      weight_resolution: Rc::new(RefCell::new(0.5)),
      weight_msaa: Rc::new(RefCell::new(0.5)),
    }
  }
}
