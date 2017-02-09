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
use cgmath::Vector3;

pub fn vec_to_matrix(m: &[f32; 16]) -> Matrix4<f32> {
  Matrix4::new(
      m[0], m[1], m[2], m[3],
      m[4], m[5], m[6], m[7],
      m[8], m[9], m[10], m[11],
      m[12], m[13], m[14], m[15])
}

pub fn matrix_to_uniform(m: Matrix4<f32>) -> [[f32; 4]; 4] {
  *m.as_ref()
}

pub fn vec_to_translation(t: &[f32; 3]) -> Matrix4<f32> {
    Matrix4::from_translation(Vector3::new(t[0], t[1], t[2]))
}
