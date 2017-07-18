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

use conrod::backend::glium::Renderer;
use conrod::color;
use conrod::event::Input;
use conrod::image::Map;
use conrod::Labelable;
use conrod::position::Align;
use conrod::position::Direction;
use conrod::position::Padding;
use conrod::position::Position;
use conrod::position::range::Range;
use conrod::position::Relative;
use conrod::Positionable;
use conrod::Sizeable;
use conrod::Theme;
use conrod::theme::StyleMap;
use conrod::Ui;
use conrod::UiBuilder;
use conrod::Widget;
use conrod::widget::Button;
use conrod::widget::Canvas;
use conrod::widget::Slider;
use conrod::widget::Text;
use glium::backend::glutin_backend::GlutinFacade;
use glium::BlitTarget;
use glium::Rect;
use glium::Surface;
use glium::texture::Texture2d;
use glium::uniforms::MagnifySamplerFilter;
use std::time::Duration;

use adaptive_canvas::AdaptiveCanvas;

widget_ids! {
  pub struct Ids {
    container,
    title_text,
    help_text,
    resume_button,
    resolution_slider,
    quit_button,
  }
}

pub enum Action {
  ChangeResolution(u32),
  Quit,
  None,
}

pub struct Gui<'a> {
  pub is_visible: bool,

  canvas: AdaptiveCanvas,
  context: &'a GlutinFacade,
  ids: Ids,
  image_map: Map<Texture2d>,
  renderer: Renderer,
  ui: Ui,

  resolution_scale: u32,
}

impl<'a> Gui<'a> {
  pub fn new(context: &'a GlutinFacade, width: f64, height: f64) -> Gui {
    let theme = Theme {
      name: "Engyn Default Theme".to_string(),
      padding: Padding { x: Range::new(50.0, 50.0), y: Range::new(50.0, 50.0) },
      x_position: Position::Relative(Relative::Align(Align::Start), None),
      y_position: Position::Relative(Relative::Direction(Direction::Backwards, 50.0), None),
      background_color: color::DARK_CHARCOAL,
      shape_color: color::LIGHT_CHARCOAL,
      border_color: color::BLACK,
      border_width: 0.0,
      label_color: color::WHITE,
      font_id: None,
      font_size_large: 200,
      font_size_medium: 75,
      font_size_small: 50,
      widget_styling: StyleMap::default(),
      mouse_drag_threshold: 0.0,
      double_click_threshold: Duration::from_millis(500),
    };

    let mut ui = UiBuilder::new([width, height]).theme(theme).build();
    ui.fonts.insert_from_file("data/Cantarell-Regular.ttf").unwrap();

    Gui {
      is_visible: false,

      canvas: AdaptiveCanvas::new(context, width as u32, height as u32),
      context: context,
      ids: Ids::new(ui.widget_id_generator()),
      image_map: Map::<Texture2d>::new(),
      renderer: Renderer::new(context).unwrap(),
      ui: ui,
      resolution_scale: 10,
    }
  }

  pub fn draw<S>(&mut self, target: &mut S, viewport: Rect) -> Action where S: Surface {
    let mut action = Action::None;

    if !self.is_visible { return action; }

    {
      let ui = &mut self.ui.set_widgets();

      Canvas::new()
          .scroll_kids()
          .set(self.ids.container, ui);

      // "Hello World!" in the middle of the screen.
      Text::new("Welcome to Engyn")
          .parent(self.ids.container)
          .mid_top_of(self.ids.container)
          .font_size(150)
          .set(self.ids.title_text, ui);

      Text::new("Press Escape to bring up this menu and use arrow keys to navigate.")
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 50.0)
          .wrap_by_word()
          .set(self.ids.help_text, ui);

      if Button::new()
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 50.0)
          .label("Resume [Escape]")
          .set(self.ids.resume_button, ui)
          .was_clicked() {
        self.is_visible = false;
      }

      if let Some(scale) = Slider::new(self.resolution_scale as f64, 1.0, 20.0)
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 50.0)
          .label(&format!("Resolution: {}", self.resolution_scale))
          .small_font(ui)
          .set(self.ids.resolution_slider, ui) {
        self.resolution_scale = scale as u32;
        action = Action::ChangeResolution(scale as u32);
      }

      if Button::new()
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 50.0)
          .label("Quit [Q]")
          .set(self.ids.quit_button, ui)
          .was_clicked() {
        action = Action::Quit;
      }
    }

    // Render the `Ui` and then display it on the screen.
    let primitives = self.ui.draw();
    self.renderer.fill(self.context, primitives, &self.image_map);

    let mut framebuffer = self.canvas.get_framebuffer(self.context).unwrap();

    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    {
      self.renderer.draw(self.context, &mut framebuffer, &self.image_map).unwrap();
    }

    let src_rect = Rect {
      left: 0,
      bottom: 0,
      width: self.canvas.viewports[0].width * 2,
      height: self.canvas.viewports[0].height,
    };

    let blit_target = BlitTarget {
      left: viewport.left + (viewport.width as f64 * 0.25) as u32,
      bottom: viewport.bottom + (viewport.height as f64 * 0.25) as u32,
      width: (viewport.width as f64 * 0.5) as i32,
      height: (viewport.height as f64 * 0.5) as i32,
    };

    framebuffer.blit_color(&src_rect, target, &blit_target, MagnifySamplerFilter::Linear);

    action
  }

  pub fn handle_event(&mut self, event: Input) {
    self.ui.handle_event(event);
  }
}
