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
use glium::framebuffer::ColorAttachment;
use glium::framebuffer::DepthAttachment;
use glium::framebuffer::ValidationError;
use glium::texture::DepthFormat;
use glium::texture::DepthTexture2d;
use glium::texture::DepthTexture2dMultisample;
use glium::texture::MipmapsOption;
use glium::texture::Texture2d;
use glium::texture::Texture2dMultisample;
use webvr::VRLayer;

use geometry::Geometry;
use geometry::Texcoord;

pub struct AdaptiveCanvas {
  pub rectangle: Geometry,
  pub viewports: [Rect; 2],

  color_buffer: Texture2d,
  color_buffers_msaa: Vec<Texture2dMultisample>,
  depth_buffer: DepthTexture2d,
  depth_buffers_msaa: Vec<DepthTexture2dMultisample>,
  layers: Vec<VRLayer>,
  max_width: u32,
  max_height: u32,
  max_msaa_level: usize,
  current_msaa_level: usize,
}

impl<'a> AdaptiveCanvas {
  pub fn new(display: &Facade, max_width: u32, max_height: u32, max_msaa_level: usize) -> AdaptiveCanvas {
    let max_half_width = max_width / 2;


    let mut color_buffers_msaa = Vec::new();
    let mut depth_buffers_msaa = Vec::new();
    let mut layers = Vec::new();

    let color_buffer = Texture2d::empty(display, max_width, max_height).unwrap();
    let depth_buffer = DepthTexture2d::empty(display, max_width, max_height).unwrap();
    layers.push(VRLayer { texture_id: color_buffer.get_id(), .. Default::default() });

    for i in 1..max_msaa_level + 1 {
      color_buffers_msaa.push(Texture2dMultisample::empty(
          display,
          max_width,
          max_height,
          2u32.pow(i as u32)).unwrap());

      depth_buffers_msaa.push(DepthTexture2dMultisample::empty(
          display,
          max_width,
          max_height,
          2u32.pow(i as u32)).unwrap());

      layers.push(VRLayer { texture_id: color_buffers_msaa[i as usize - 1].get_id(), ..Default::default() });
    }

    println!("number of msaa color buffers: {}", color_buffers_msaa.len());

    AdaptiveCanvas {
      layers: layers,
      rectangle: Geometry::new_quad(display, [2.0, 2.0], true),
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
      color_buffer: color_buffer,
      color_buffers_msaa: color_buffers_msaa,
      depth_buffer: depth_buffer,
      depth_buffers_msaa: depth_buffers_msaa,
      max_width: max_width,
      max_height: max_height,
      max_msaa_level: max_msaa_level,
      current_msaa_level: max_msaa_level,
    }
  }

  pub fn set_resolution_scale(&mut self, scale: f32) {
    let width = scale * self.max_width as f32;
    let height = scale * self.max_height as f32;

    if width as u32 <= self.max_width && height as u32 <= self.max_height {
      self.set_resolution(width, height);
    } else {
      println!("Can't set resolution {}x{}: too high", width, height);
    }
  }

  fn set_resolution(&mut self, width: f32, height: f32) {
    let bounded_width = f32::max(width, 320.0);
    let bounded_height = f32::max(height, 240.0);
    let fraction_width = bounded_width / self.max_width as f32;
    let fraction_height = bounded_height / self.max_height as f32;

    let fraction_half_width = fraction_width * 0.5;
    let half_width = (width * 0.5) as u32;

    for ref mut layer in &mut self.layers {
      layer.left_bounds = [
          0.0,
          1.0 - fraction_height,
          fraction_half_width,
          fraction_height];

      layer.right_bounds = [
          fraction_half_width,
          1.0 - fraction_height,
          fraction_half_width,
          fraction_height];
    }

    self.rectangle.texcoords.write(&[
        Texcoord { texcoord: (0.0, 0.0) },
        Texcoord { texcoord: (0.0, fraction_height) },
        Texcoord { texcoord: (fraction_width, fraction_height) },
        Texcoord { texcoord: (fraction_width, 0.0) }]);

    self.viewports[0].width = half_width;
    self.viewports[0].height = height as u32;

    self.viewports[1].left = half_width;
    self.viewports[1].width = half_width;
    self.viewports[1].height = height as u32;
  }

  pub fn set_msaa_scale(&mut self, scale: f32) {
    let level = (scale * (self.max_msaa_level as f32)) as usize;

    if level <= self.max_msaa_level {
      self.set_msaa_level(level);
    } else {
      println!("Can't set MSAA level {}: {} is maximum", level, self.max_msaa_level);
    }
  }

  fn set_msaa_level(&mut self, msaa_level: usize) {
    if msaa_level < self.color_buffers_msaa.len() + 1 {
      self.current_msaa_level = msaa_level;
    }
  }

  pub fn get_framebuffer(&self, display: &Facade)
      -> Result<SimpleFrameBuffer, ValidationError> {
    SimpleFrameBuffer::with_depth_buffer(
        display,
        self.get_color_attachment(),
        self.get_depth_attachment())
  }

  fn get_color_attachment(&self) -> ColorAttachment {
    if self.current_msaa_level == 0 {
      self.color_buffer.to_color_attachment()
    } else {
      // TODO resolve msaa to non-msaa first
      self.color_buffers_msaa[self.current_msaa_level - 1].to_color_attachment()
    }
  }

  fn get_depth_attachment(&self) -> DepthAttachment {
    if self.current_msaa_level == 0 {
      self.depth_buffer.to_depth_attachment()
    } else {
      // TODO resolve msaa to non-msaa first
      self.depth_buffers_msaa[self.current_msaa_level].to_depth_attachment()
    }
  }

  pub fn get_layer(&self) -> &VRLayer {
    &self.layers[self.current_msaa_level]
  }
}
