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

use glium::backend::Facade;
use glium::index::IndexBuffer;
use glium::index::PrimitiveType;
use glium::vertex::VertexBuffer;
use std::f32;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: (f32, f32, f32)
}

implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
pub struct Texcoord {
    pub texcoord: (f32, f32)
}

implement_vertex!(Texcoord, texcoord);

#[derive(Copy, Clone)]
pub struct Normal {
    pub normal: (f32, f32, f32)
}

implement_vertex!(Normal, normal);

pub struct Geometry {
  pub bounding_box: ([f32; 3], [f32; 3]),
  pub indices: Option<IndexBuffer<u32>>,
  pub normals: VertexBuffer<Normal>,
  pub vertices: VertexBuffer<Vertex>,
  pub texcoords: VertexBuffer<Texcoord>,
}

impl Geometry {
  pub fn new_quad(context: &Facade, size: [f32; 2], dynamic_texcoords: bool) -> Geometry {
    let width_half = size[0] * 0.5;
    let height_half = size[1] * 0.5;

    Geometry {
      bounding_box: (
        [-width_half, -height_half, 0.0],
        [width_half, height_half, 0.0],
      ),
      indices: Some(IndexBuffer::new(
          context,
          PrimitiveType::TriangleStrip,
          &[1, 2, 0, 3]).unwrap()),
      normals: VertexBuffer::new(context, &[
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) }]).unwrap(),
      vertices: VertexBuffer::new(context, &[
          Vertex { position: (-width_half, -height_half, 0.0) },
          Vertex { position: (-width_half,  height_half, 0.0) },
          Vertex { position: ( width_half,  height_half, 0.0) },
          Vertex { position: ( width_half, -height_half, 0.0) }]).unwrap(),
      texcoords:
          if dynamic_texcoords {
            VertexBuffer::dynamic(context, &[
                Texcoord { texcoord: (0.0, 0.0) },
                Texcoord { texcoord: (0.0, 1.0) },
                Texcoord { texcoord: (1.0, 1.0) },
                Texcoord { texcoord: (1.0, 0.0) }]).unwrap()
          } else {
            VertexBuffer::new(context, &[
                Texcoord { texcoord: (0.0, 0.0) },
                Texcoord { texcoord: (0.0, 1.0) },
                Texcoord { texcoord: (1.0, 1.0) },
                Texcoord { texcoord: (1.0, 0.0) }]).unwrap()
          },
    }
  }

  pub fn new_triangle(context: &Facade, size: [f32; 2]) -> Geometry {
    let width_half = size[0] * 0.5;
    let height_half = size[1] * 0.5;

    Geometry {
      bounding_box: (
        [-width_half, -height_half, 0.0],
        [width_half, height_half, 0.0],
      ),
      indices: None,
      normals: VertexBuffer::new(context, &[
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) }]).unwrap(),
      vertices: VertexBuffer::new(context, &[
          Vertex { position: (-width_half, -height_half, 0.0) },
          Vertex { position: ( width_half, -height_half, 0.0) },
          Vertex { position: (        0.0,  height_half, 0.0) }]).unwrap(),
      texcoords: VertexBuffer::new(context, &[
          Texcoord { texcoord: (0.0, 0.0) },
          Texcoord { texcoord: (1.0, 0.0) },
          Texcoord { texcoord: (0.5, 1.0) },
        ]).unwrap(),
    }
  }
}
