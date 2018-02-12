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
use glium::Program;
use glium::texture::RawImage2d;
use glium::texture::SrgbTexture2d;
use image;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::rc::Rc;

pub enum Resource {
  Program(Rc<RefCell<Program>>),
  SrgbTexture2d(Rc<RefCell<SrgbTexture2d>>),
}

pub struct ResourceManager<'a> {
  context: &'a Facade,
  resources: RefCell<HashMap<String, Resource>>,
}

impl<'a> ResourceManager<'a> {
  pub fn new(context: &Facade) -> ResourceManager {
    ResourceManager {
      resources: RefCell::new(HashMap::new()),
      context: context,
    }
  }

  /**
   * Retrieves a program from the ResourceManager
   */

  pub fn get_program(&self, path: &str, compile: &Fn() -> Program)
      -> Result<Rc<RefCell<Program>>, &str> {
    println!("get_program: {}", path);
    if self.resources.borrow().contains_key(path) {
      match self.resources.borrow().get(path) {
        Some(&Resource::Program(ref p)) => Ok(Rc::clone(p)),
        Some(_) => Err("Not a program"),
        None => panic!(),
      }
    } else {
      self.resources.borrow_mut().insert(path.to_string(), Resource::Program(Rc::new(RefCell::new(compile()))));
      match self.resources.borrow().get(path) {
        Some(&Resource::Program(ref p)) => Ok(Rc::clone(p)),
        _ => panic!()
      }
    }
  }

  /**
   * Retrieves a texture from the ResourceManager.
   */

  pub fn get_texture(&self, path: &str) -> Result<Rc<RefCell<SrgbTexture2d>>, &str> {
    println!("get_texture: {}", path);
    if self.resources.borrow().contains_key(path) {
      match self.resources.borrow().get(path) {
        Some(&Resource::SrgbTexture2d(ref t)) => Ok(Rc::clone(t)),
        Some(_) => Err("Not a texture"),
        None => panic!(),
      }
    } else {
      let texture = self.load_texture(Path::new(path));

      match texture {
        Ok(t) => {
          self.resources.borrow_mut().insert(
              path.to_string(),
              Resource::SrgbTexture2d(Rc::new(RefCell::new(t))));
          match self.resources.borrow().get(path) {
            Some(&Resource::SrgbTexture2d(ref tref)) => Ok(Rc::clone(tref)),
            _ => panic!()
          }
        },
        _ => {
          eprintln!("Could not load texture: {}", path);
          Ok(Rc::new(RefCell::new(SrgbTexture2d::empty(self.context, 1, 1).unwrap())))
        }
      }
    }
  }

  fn load_texture(&self, name: &Path) -> Result<SrgbTexture2d, Box<Error>> {
    let image = image::open(name)?.to_rgba();
    let image_dimensions = image.dimensions();
    let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let texture = SrgbTexture2d::new(self.context, image)?;

    Ok(texture)
  }
}
