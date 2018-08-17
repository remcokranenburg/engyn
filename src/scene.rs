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

use cgmath::Matrix4;
use cgmath::SquareMatrix;
use glium::backend::Facade;
use math;
use serde_yaml;
use std::fs::File;
use std::io::Result;

use benchmark::Benchmark;
use light::Light;
use network_graph::Network;
use object::Object;
use resources::ResourceManager;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum SceneDrawable {
  Benchmark { path: String },
  Obj { path: String },
  Network { num_nodes: usize, num_links: usize },
  None,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
  pub children: Vec<SceneObject>,
  pub drawable: SceneDrawable,
  pub transform: [f32; 16],
}

impl SceneObject {
  pub fn as_object<F>(&self, context: &F, resource_manager: &ResourceManager) -> Object
      where F: Facade {
    let mut object = match &self.drawable {
      &SceneDrawable::Benchmark { ref path } => Benchmark::from_file(context, &path).as_object(),
      &SceneDrawable::Obj { ref path } => Object::from_file(context, resource_manager, &path),
      &SceneDrawable::Network { num_nodes, num_links } => {
        Network::new(context, num_nodes, num_links).as_object()
      },
      &SceneDrawable::None => Object {
        children: vec![],
        drawable: None,
        transform: Matrix4::identity(),
        size: 0.0,
      },
    };

    for child in &self.children {
      object.children.push(child.as_object(context, resource_manager));
    }

    object.transform = math::vec_to_matrix(&self.transform);

    object
  }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
  pub version: String,
  pub scene_objects: Vec<SceneObject>,
  pub lights: Vec<Light>,
}

impl Scene {
  pub fn new() -> Scene {
    Scene {
      version: "1.0".to_owned(),
      scene_objects: vec![
        SceneObject {
          children: vec![
            SceneObject {
              children: vec![],
              drawable: SceneDrawable::Obj { path: "cube.obj".to_owned() },
              transform: math::matrix_to_vec(&Matrix4::identity()),
            },
          ],
          drawable: SceneDrawable::None,
          transform: math::matrix_to_vec(&Matrix4::identity()),
        },
        SceneObject {
          children: vec![],
          drawable: SceneDrawable::Network { num_nodes: 10, num_links: 10 },
          transform: math::matrix_to_vec(&Matrix4::identity()),
        },
      ],
      lights: vec![
        Light { color: [1.0, 0.9, 0.9], position: [10.0, 10.0, 10.0] },
      ]
    }
  }

  pub fn from_yaml(filename: &str) -> Result<Scene> {
    let file = File::open(filename)?;
    let scene: Scene = serde_yaml::from_reader(&file).unwrap();
    Ok(scene)
  }

  pub fn to_yaml(&self, filename: &str) -> Result<()> {
    let file = File::create(filename)?;
    serde_yaml::to_writer(file, self).unwrap();
    Ok(())
  }

  pub fn as_object<F>(&self, context: &F, resource_manager: &ResourceManager) -> Object
      where F: Facade {
    let mut objects = vec![];

    for scene_object in &self.scene_objects {
      objects.push(scene_object.as_object(context, resource_manager));
    }

    Object {
      children: objects,
      drawable: None,
      transform: Matrix4::identity(),
      size: 0.0,
    }
  }
}
