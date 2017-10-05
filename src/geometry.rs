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

use glium::backend::Facade;
use glium::index::IndexBuffer;
use glium::index::PrimitiveType;
use glium::vertex::VertexBuffer;
use std::path::Path;
use tobj;

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
  pub indices: Option<IndexBuffer<u32>>,
  pub normals: VertexBuffer<Normal>,
  pub vertices: VertexBuffer<Vertex>,
  pub texcoords: VertexBuffer<Texcoord>,
}

impl Geometry {
  pub fn from_obj(context: &Facade, filename: &str) -> Geometry {
    let (models, _) = tobj::load_obj(&Path::new(filename)).unwrap();

    assert!(models.len() > 0);

    let mesh = &models[0].mesh;

    let indices = if mesh.indices.len() > 0 {
      Some(IndexBuffer::new(context, PrimitiveType::TrianglesList, &mesh.indices).unwrap())
    } else {
      None
    };

    let mut normals = VertexBuffer::empty(context, mesh.normals.len()).unwrap();
    {
      let mut mapped = normals.map();
      for i in 0..mesh.normals.len() / 3 {
        mapped[i] = Normal { normal: (
          mesh.normals[i * 3 + 0],
          mesh.normals[i * 3 + 1],
          mesh.normals[i * 3 + 2],
        )};
      }
    }

    let mut vertices = VertexBuffer::empty(context, mesh.positions.len()).unwrap();
    {
      let mut mapped = vertices.map();
      for i in 0..mesh.positions.len() / 3 {
        mapped[i] = Vertex { position: (
          mesh.positions[i * 3 + 0],
          mesh.positions[i * 3 + 1],
          mesh.positions[i * 3 + 2],
        )};
      }
    }

    let mut texcoords = VertexBuffer::empty(context, mesh.texcoords.len()).unwrap();
    {
      let mut mapped = texcoords.map();
      for i in 0..mesh.texcoords.len() / 2 {
        mapped[i] = Texcoord { texcoord: (
          mesh.texcoords[i * 2 + 0],
          mesh.texcoords[i * 2 + 1],
        )};
      }
    }

    Geometry {
      indices: indices,
      normals: normals,
      vertices: vertices,
      texcoords: texcoords,
    }
  }

  pub fn new_quad(context: &Facade, size: [f32; 2], dynamic_texcoords: bool) -> Geometry {
    let width_half = size[0] * 0.5;
    let height_half = size[1] * 0.5;

    Geometry {
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
