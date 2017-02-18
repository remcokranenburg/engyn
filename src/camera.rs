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

use cgmath::Matrix4;
use cgmath::Rad;
use cgmath::SquareMatrix;
use cgmath::Vector3;

pub struct FpsCamera {
  pub forward: bool,
  pub backward: bool,
  pub left: bool,
  pub right: bool,
  pub pitch: Rad<f32>,
  pub yaw: Rad<f32>,
  position: Vector3<f32>,
}

impl FpsCamera {
  pub fn new(position: Vector3<f32>) -> FpsCamera {
    FpsCamera {
      forward: false,
      backward: false,
      left: false,
      right: false,
      pitch: Rad(0.0),
      yaw: Rad(0.0),
      position: position,
    }
  }

  pub fn get_view(&mut self, time_delta_ms: f32) -> Matrix4<f32> {
    let translation = {
      let x = if self.left == self.right { 0.0 } else if self.left { -1.0 } else { 1.0 };
      let y = 0.0;
      let z = if self.forward == self.backward { 0.0 } else if self.forward { -1.0 } else { 1.0 };
      Vector3::new(x, y, z) * time_delta_ms
    };


    let mut m = Matrix4::<f32>::identity();
    m = m * Matrix4::from_translation(self.position);
    m = m * Matrix4::from_angle_y(self.yaw);
    m = m * Matrix4::from_translation(translation);
    m = m * Matrix4::from_angle_x(self.pitch);

    // add global translation
    self.position = Vector3::new(m.w.x, m.w.y, m.w.z);

    m.invert().unwrap()
  }
}
