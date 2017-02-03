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

use glium::backend::glutin_backend::GlutinFacade;
use glium::index::IndexBuffer;
use glium::index::PrimitiveType;
use glium::vertex::VertexBuffer;

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
  pub indices: Option<IndexBuffer<u16>>,
  pub normals: VertexBuffer<Normal>,
  pub vertices: VertexBuffer<Vertex>,
  pub texcoords: VertexBuffer<Texcoord>,
}

impl Geometry {
  pub fn new_quad(window: &GlutinFacade, size: [f32; 2]) -> Geometry {
    let width_half = size[0] * 0.5;
    let height_half = size[1] * 0.5;

    Geometry {
      indices: Some(IndexBuffer::new(
          window,
          PrimitiveType::TriangleStrip,
          &[1, 2, 0, 3u16]).unwrap()),
      normals: VertexBuffer::new(window, &[
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) }]).unwrap(),
      vertices: VertexBuffer::new(window, &[
          Vertex { position: (-width_half, -height_half, 0.0) },
          Vertex { position: (-width_half,  height_half, 0.0) },
          Vertex { position: ( width_half,  height_half, 0.0) },
          Vertex { position: ( width_half, -height_half, 0.0) }]).unwrap(),
      texcoords: VertexBuffer::new(window, &[
          Texcoord { texcoord: (0.0, 0.0) },
          Texcoord { texcoord: (0.0, 1.0) },
          Texcoord { texcoord: (1.0, 1.0) },
          Texcoord { texcoord: (1.0, 0.0) }]).unwrap(),
    }
  }

  pub fn borrow_indices<'a>(&'a self) -> Result<&'a IndexBuffer<u16>, &str> {
    match self.indices {
      Some(ref x) => Ok(x),
      None => Err("Nope"),
    }
  }
}
