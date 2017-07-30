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

use glium::Rect;
use glium::GlObject;
use glium::backend::Facade;
use glium::framebuffer::DepthRenderBuffer;
use glium::framebuffer::SimpleFrameBuffer;
use glium::framebuffer::ToColorAttachment;
use glium::framebuffer::ToDepthAttachment;
use glium::framebuffer::ValidationError;
use glium::texture::DepthFormat;
use glium::texture::Texture2d;
use webvr::VRLayer;

use geometry::Geometry;
use geometry::Texcoord;

pub struct AdaptiveCanvas {
  pub layer: VRLayer,
  pub rectangle: Geometry,
  pub texture: Texture2d,
  pub viewports: [Rect; 2],

  depth_buffer: DepthRenderBuffer,
  max_width: u32,
  max_height: u32,
  step_width: u32,
  step_height: u32,
}

impl<'a> AdaptiveCanvas {
  pub fn new(display: &Facade, max_width: u32, max_height: u32) -> AdaptiveCanvas {
    let max_half_width = max_width / 2;
    let texture = Texture2d::empty(
        display,
        max_width,
        max_height).unwrap();

    let depth_buffer = DepthRenderBuffer::new(
        display,
        DepthFormat::I24,
        max_width,
        max_height).unwrap();

    AdaptiveCanvas {
      layer: VRLayer { texture_id: texture.get_id(), ..Default::default() },
      rectangle: Geometry::new_quad(display, [2.0, 2.0], true),
      texture: texture,
      viewports: [
          Rect {
            left: 0,
            bottom: 0,
            width: max_half_width,
            height: max_height,
          },
          Rect {
            left: max_half_width,
            bottom: 0,
            width: max_half_width,
            height: max_height,
          }],
      depth_buffer: depth_buffer,
      max_width: max_width,
      max_height: max_height,
      step_width: (max_width as f32 * 0.05) as u32,
      step_height: (max_height as f32 * 0.05) as u32,
    }
  }

  pub fn decrease_resolution(&mut self) {
    let width = self.viewports[0].width * 2 - self.step_width;
    let height = self.viewports[0].height - self.step_height;

    if width >= self.step_width && height >= self.step_height {
      println!("Decreasing resolution.");
      self.set_resolution(width, height);
    } else {
      println!("Already at lowest resolution.");
    }
  }

  pub fn increase_resolution(&mut self) {
    let width = self.viewports[0].width * 2 + self.step_width;
    let height = self.viewports[0].height + self.step_height;

    if width <= self.max_width && height <= self.max_height {
      println!("Increasing resolution.");
      self.set_resolution(width, height);
    } else {
      println!("Already at highest resolution.");
    }
  }

  pub fn set_resolution_scale(&mut self, scale: u32) {
    let width = scale * self.step_width;
    let height = scale * self.step_height;

    if width <= self.max_width && height <= self.max_height {
      println!("Setting resolution {}x{}.", width, height);
      self.set_resolution(width, height);
    } else {
      println!("Can't set resolution {}x{}: too high", width, height);
    }
  }

  pub fn set_resolution(&mut self, width: u32, height: u32) {
    let fraction_width = width as f32 / self.texture.get_width() as f32;
    let fraction_height = height as f32 / self.texture.get_height().unwrap() as f32;

    let fraction_half_width = fraction_width * 0.5;
    let half_width = (width as f32 * 0.5) as u32;

    self.layer.left_bounds = [
        0.0,
        1.0 - fraction_height,
        fraction_half_width,
        fraction_height];

    self.layer.right_bounds = [
        fraction_half_width,
        1.0 - fraction_height,
        fraction_half_width,
        fraction_height];

    self.rectangle.texcoords.write(&[
        Texcoord { texcoord: (0.0, 0.0) },
        Texcoord { texcoord: (0.0, fraction_height) },
        Texcoord { texcoord: (fraction_width, fraction_height) },
        Texcoord { texcoord: (fraction_width, 0.0) }]);

    self.viewports[0].width = half_width;
    self.viewports[0].height = height;

    self.viewports[1].left = half_width;
    self.viewports[1].width = half_width;
    self.viewports[1].height = height;
  }

  pub fn get_framebuffer(&self, display: &Facade)
      -> Result<SimpleFrameBuffer, ValidationError> {
    SimpleFrameBuffer::with_depth_buffer(
        display,
        self.texture.to_color_attachment(),
        self.depth_buffer.to_depth_attachment())
  }
}
