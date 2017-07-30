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
use conrod::Colorable;
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
use conrod::widget::button::Style as ButtonStyle;
use conrod::widget::Canvas;
use conrod::widget::Slider;
use conrod::widget::Text;
use glium::Display;
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
  Resume,
  None,
}

pub struct Gui<'a> {
  pub is_visible: bool,

  canvas: AdaptiveCanvas,
  display: &'a Display,
  ids: Ids,
  image_map: Map<Texture2d>,
  renderer: Renderer,
  ui: Ui,
  selected_widget: u32,
  resolution_scale: u32,
}

impl<'a> Gui<'a> {
  pub fn new(display: &'a Display, width: f64, height: f64) -> Gui {
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

    let mut ui = UiBuilder::new([width * 2.0, height]).theme(theme).build();
    ui.fonts.insert_from_file("data/Cantarell-Regular.ttf").unwrap();

    Gui {
      is_visible: false,

      canvas: AdaptiveCanvas::new(display, width as u32, height as u32),
      display: display,
      ids: Ids::new(ui.widget_id_generator()),
      image_map: Map::<Texture2d>::new(),
      renderer: Renderer::new(display).unwrap(),
      ui: ui,
      selected_widget: 0,
      resolution_scale: 10,
    }
  }

  pub fn draw<S>(&mut self, target: &mut S, viewport: Rect) -> Action where S: Surface {
    let mut action = Action::None;
    let button_default_style = ButtonStyle::default();
    let button_focussed_style = ButtonStyle { color: Some(color::BLUE), ..ButtonStyle::default() };
    let slider_default_color = color::LIGHT_CHARCOAL;
    let slider_focussed_color = color::BLUE;

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
          .with_style(if self.selected_widget == 0 {
              button_focussed_style
            } else {
              button_default_style
            })
          .label("Resume [Escape]")
          .set(self.ids.resume_button, ui)
          .was_clicked() {
        self.selected_widget = 0;
        action = Action::Resume;
      }

      if let Some(scale) = Slider::new(self.resolution_scale as f64, 1.0, 20.0)
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 50.0)
          .color(if self.selected_widget == 1 {
              slider_focussed_color
            } else {
              slider_default_color
            })
          .label(&format!("Resolution: {}", self.resolution_scale))
          .small_font(ui)
          .set(self.ids.resolution_slider, ui) {
        self.resolution_scale = scale as u32;
        self.selected_widget = 1;
        action = Action::ChangeResolution(scale as u32);
      }

      if Button::new()
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 50.0)
          .with_style(if self.selected_widget == 2 {
              button_focussed_style
            } else {
              button_default_style
            })
          .label("Quit [Q]")
          .set(self.ids.quit_button, ui)
          .was_clicked() {
        self.selected_widget = 2;
        action = Action::Quit;
      }
    }

    // Render the `Ui` and then display it on the screen.
    let primitives = self.ui.draw();
    self.renderer.fill(self.display, primitives, &self.image_map);

    let mut framebuffer = self.canvas.get_framebuffer(self.display).unwrap();

    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    {
      self.renderer.draw(self.display, &mut framebuffer, &self.image_map).unwrap();
    }

    let src_rect = Rect {
      left: 0,
      bottom: 0,
      width: self.canvas.viewports[0].width * 2,
      height: self.canvas.viewports[0].height,
    };

    let blit_target = BlitTarget {
      left: viewport.left + viewport.width / 4,
      bottom: viewport.bottom + viewport.height / 4,
      width: (viewport.width / 2) as i32,
      height: (viewport.height / 2) as i32,
    };

    framebuffer.blit_color(&src_rect, target, &blit_target, MagnifySamplerFilter::Linear);

    action
  }

  pub fn handle_event(&mut self, event: Input) {
    self.ui.handle_event(event);
  }
}
