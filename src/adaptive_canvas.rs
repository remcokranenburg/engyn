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

use glium::BlitTarget;
use glium::Rect;
use glium::GlObject;
use glium::Surface;
use glium::backend::Facade;
use glium::framebuffer::SimpleFrameBuffer;
use glium::framebuffer::ToColorAttachment;
use glium::framebuffer::ToDepthAttachment;
use glium::framebuffer::ColorAttachment;
use glium::framebuffer::DepthAttachment;
use glium::framebuffer::ValidationError;
use glium::texture::DepthTexture2d;
use glium::texture::DepthTexture2dMultisample;
use glium::texture::Texture2d;
use glium::texture::Texture2dMultisample;
use glium::uniforms::MagnifySamplerFilter;
use webvr::VRLayer;

use geometry::Geometry;
use geometry::Texcoord;

pub struct AdaptiveCanvas {
  pub rectangle: Geometry,
  pub viewports: [Rect; 2],
  pub viewport: Rect,

  color_buffer: Texture2d,
  color_buffers_msaa: Vec<Texture2dMultisample>,
  depth_buffer: DepthTexture2d,
  depth_buffers_msaa: Vec<DepthTexture2dMultisample>,
  layer: VRLayer,
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

    let color_buffer = Texture2d::empty(display, max_width, max_height).unwrap();
    let depth_buffer = DepthTexture2d::empty(display, max_width, max_height).unwrap();

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
    }

    AdaptiveCanvas {
      layer: VRLayer { texture_id: color_buffer.get_id(), .. Default::default() },
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
      viewport: Rect { left: 0, bottom: 0, width: max_width, height: max_height },
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
    let bounded_half_width = (bounded_width * 0.5) as u32;

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

    self.viewports[0].width = bounded_half_width as u32;
    self.viewports[0].height = bounded_height as u32;

    self.viewports[1].left = bounded_half_width as u32;
    self.viewports[1].width = bounded_half_width as u32;
    self.viewports[1].height = bounded_height as u32;

    self.viewport.width = bounded_width as u32;
    self.viewport.height = bounded_height as u32;
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
      if self.current_msaa_level != msaa_level {
        self.current_msaa_level = msaa_level;
      }
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
      self.color_buffers_msaa[self.current_msaa_level - 1].to_color_attachment()
    }
  }

  fn get_depth_attachment(&self) -> DepthAttachment {
    if self.current_msaa_level == 0 {
      self.depth_buffer.to_depth_attachment()
    } else {
      self.depth_buffers_msaa[self.current_msaa_level - 1].to_depth_attachment()
    }
  }

  pub fn resolve(&self, display: &Facade) {
    // if we're doing MSAA, resolve it to non-MSAA
    if self.current_msaa_level > 0 {
      let framebuffer = SimpleFrameBuffer::with_depth_buffer(
          display,
          self.color_buffer.to_color_attachment(),
          self.depth_buffer.to_depth_attachment()).unwrap();
      let msaa_color_buffer = &self.color_buffers_msaa[self.current_msaa_level - 1];
      let msaa_color_attachment = msaa_color_buffer.to_color_attachment();
      let msaa_depth_buffer = &self.depth_buffers_msaa[self.current_msaa_level - 1];
      let msaa_depth_attachment = msaa_depth_buffer.to_depth_attachment();
      let msaa_framebuffer = SimpleFrameBuffer::with_depth_buffer(
          display,
          msaa_color_attachment,
          msaa_depth_attachment).unwrap();
      let rect = Rect {
        left: 0,
        bottom: 0,
        width: self.viewport.width,
        height: self.viewport.height,
      };
      let blit_target = BlitTarget {
        left: 0,
        bottom: 0,
        width: rect.width as i32,
        height: rect.height as i32,
      };
      msaa_framebuffer.blit_color(&rect, &framebuffer, &blit_target, MagnifySamplerFilter::Nearest);
    }
  }

  pub fn get_resolved_framebuffer(&self, display: &Facade)
      -> Result<SimpleFrameBuffer, ValidationError> {
    SimpleFrameBuffer::with_depth_buffer(
        display,
        self.color_buffer.to_color_attachment(),
        self.depth_buffer.to_depth_attachment())
  }

  pub fn get_resolved_layer(&self) -> &VRLayer {
    &self.layer
  }
}
