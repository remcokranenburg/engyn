// Copyright (c) 2018 Remco Kranenburg
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

use glium::texture::SrgbTexture2d;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Material {
  pub albedo_map: Rc<RefCell<SrgbTexture2d>>,
  pub ambient_color: [f32; 3],
  pub diffuse_color: [f32; 3],
  pub specular_color: [f32; 3],
  pub shininess: f32,
  pub metalness: f32,
  pub reflectivity: f32,
}
