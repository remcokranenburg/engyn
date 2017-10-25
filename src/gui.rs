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
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use std::f32;

use adaptive_canvas::AdaptiveCanvas;

widget_ids! {
  pub struct Ids {
    container,
    title_text,
    help_text,
    resume_button,
    quality_text,
    resolution_slider,
    msaa_slider,
    quit_button,
  }
}

#[derive(Clone, Copy)]
pub enum Action {
  Quit,
  Resume,
  None,
}

pub struct GuiElement {
  pub action: Action,
  pub weight: Rc<RefCell<f32>>,
}

pub struct Gui<'a> {
  pub is_visible: bool,
  pub widgets: Vec<GuiElement>,
  pub selected_widget: usize,

  canvas: AdaptiveCanvas,
  display: &'a Display,
  ids: Ids,
  image_map: Map<Texture2d>,
  renderer: Renderer,
  ui: Ui,
  widget_order: HashMap<String, usize>,
}

impl<'a> Gui<'a> {
  pub fn new(display: &'a Display, weight_resolution: Rc<RefCell<f32>>,
      weight_msaa: Rc<RefCell<f32>>) -> Gui<'a> {
    // TODO: put this in a 'system integration' module
    let executable_string = env::args().nth(0).unwrap();
    let executable_path = Path::new(&executable_string).parent().unwrap();
    let project_path = executable_path.parent().unwrap().parent().unwrap();

    let theme = Theme {
      name: "Engyn Default Theme".to_string(),
      padding: Padding { x: Range::new(25.0, 25.0), y: Range::new(25.0, 25.0) },
      x_position: Position::Relative(Relative::Align(Align::Start), None),
      y_position: Position::Relative(Relative::Direction(Direction::Backwards, 25.0), None),
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

    let mut ui = UiBuilder::new([768.0, 960.0]).theme(theme).build();
    ui.fonts.insert_from_file(project_path.join("data").join("Cantarell-Regular.ttf")).unwrap();

    Gui {
      is_visible: false,
      selected_widget: 0,
      widgets: vec![
        GuiElement { action: Action::Resume, weight: Rc::new(RefCell::new(0.0)) },
        GuiElement { action: Action::None, weight: weight_resolution },
        GuiElement { action: Action::None, weight: weight_msaa },
        GuiElement { action: Action::Quit, weight: Rc::new(RefCell::new(0.0)) },
      ],

      canvas: AdaptiveCanvas::new(display, 768, 960, 0),
      display: display,
      ids: Ids::new(ui.widget_id_generator()),
      image_map: Map::<Texture2d>::new(),
      renderer: Renderer::new(display).unwrap(),
      ui: ui,
      widget_order: [
        ("Resume".to_owned(), 0),
        ("Resolution".to_owned(), 1),
        ("MSAA".to_owned(), 2),
        ("Quit".to_owned(), 3),
      ].iter().cloned().collect(),
    }
  }

  pub fn prepare(&mut self, quality_level: f32) -> Action {
    if !self.is_visible { return Action::None }

    let mut action = Action::None;
    let button_default_style = ButtonStyle::default();
    let button_focussed_style = ButtonStyle { color: Some(color::BLUE), ..ButtonStyle::default() };
    let slider_default_color = color::LIGHT_CHARCOAL;
    let slider_focussed_color = color::BLUE;

    {
      let ui = &mut self.ui.set_widgets();

      Canvas::new()
          .scroll_kids()
          .set(self.ids.container, ui);

      // "Hello World!" in the middle of the screen.
      Text::new("Welcome to Engyn")
          .parent(self.ids.container)
          .mid_top_of(self.ids.container)
          .font_size(200)
          .set(self.ids.title_text, ui);

      Text::new("Press Escape to bring up this menu and use arrow keys to navigate.")
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 25.0)
          .wrap_by_word()
          .set(self.ids.help_text, ui);

      Text::new(&format!("Quality: {}", quality_level))
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 25.0)
          .set(self.ids.quality_text, ui);

      let resume_index = *self.widget_order.get("Resume").unwrap();

      if Button::new()
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 25.0)
          .with_style(if self.selected_widget == resume_index {
              button_focussed_style
            } else {
              button_default_style
            })
          .label("Resume [Escape]")
          .set(self.ids.resume_button, ui)
          .was_clicked() {
        self.selected_widget = resume_index;
        action = self.widgets[resume_index].action;
      }

      let resolution_index = *self.widget_order.get("Resolution").unwrap();
      let resolution_weight_ref = Rc::clone(&self.widgets[resolution_index].weight);
      let resolution_weight = *resolution_weight_ref.borrow();

      if let Some(weight) = Slider::new(resolution_weight, 0.0, 1.0)
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 25.0)
          .color(if self.selected_widget == resolution_index {
              slider_focussed_color
            } else {
              slider_default_color
            })
          .label(&format!("Resolution weight: {}", resolution_weight))
          .small_font(ui)
          .set(self.ids.resolution_slider, ui) {
        *resolution_weight_ref.borrow_mut() = weight;
        self.selected_widget = resolution_index;
      }

      let msaa_index = *self.widget_order.get("MSAA").unwrap();
      let msaa_weight_ref = Rc::clone(&self.widgets[msaa_index].weight);
      let msaa_weight = *msaa_weight_ref.borrow();

      if let Some(weight) = Slider::new(msaa_weight, 0.0, 1.0)
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 25.0)
          .color(if self.selected_widget == msaa_index {
              slider_focussed_color
            } else {
              slider_default_color
            })
          .label(&format!("Anti-aliasing weight: {}", msaa_weight))
          .small_font(ui)
          .set(self.ids.msaa_slider, ui) {
        *msaa_weight_ref.borrow_mut() = weight;
        self.selected_widget = msaa_index;
      }

      let quit_index = *self.widget_order.get("Quit").unwrap();

      if Button::new()
          .parent(self.ids.container)
          .padded_w_of(self.ids.container, 25.0)
          .with_style(if self.selected_widget == quit_index {
              button_focussed_style
            } else {
              button_default_style
            })
          .label("Quit [Q]")
          .set(self.ids.quit_button, ui)
          .was_clicked() {
        self.selected_widget = quit_index;
        action = Action::Quit;
      }
    }

    // Render the UI and draw it to a texture
    let primitives = self.ui.draw();
    self.renderer.fill(self.display, primitives, &self.image_map);

    let mut framebuffer = self.canvas.get_framebuffer(self.display).unwrap();

    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    {
      self.renderer.draw(self.display, &mut framebuffer, &self.image_map).unwrap();
    }

    action
  }

  pub fn draw<S>(&mut self, target: &mut S, viewport: Rect) where S: Surface {
    if !self.is_visible { return; }

    let left = viewport.left == 0;

    let src_rect = Rect {
      left: 0,
      bottom: 0,
      width: self.canvas.viewports[0].width * 2,
      height: self.canvas.viewports[0].height,
    };

    let gui_width = viewport.width as f64 * 0.4;
    let gui_height = viewport.height as f64 * 0.4;

    let blit_target = if left {
      BlitTarget {
        left: viewport.left + viewport.width / 2 - (gui_width * 0.4) as u32 + viewport.width / 10,
        bottom: viewport.bottom + viewport.height / 2 - (gui_height * 0.4) as u32 - viewport.height / 12,
        width: gui_width as i32,
        height: gui_height as i32,
      }
    } else {
      BlitTarget {
        left: viewport.left + viewport.width / 2 - (gui_width * 0.4) as u32 - viewport.width / 10,
        bottom: viewport.bottom + viewport.height / 2 - (gui_height * 0.4) as u32 - viewport.height / 12,
        width: gui_width as i32,
        height: gui_height as i32,
      }
    };

    let framebuffer = self.canvas.get_framebuffer(self.display).unwrap();
    framebuffer.blit_color(&src_rect, target, &blit_target, MagnifySamplerFilter::Linear);
  }

  pub fn handle_event(&mut self, event: Input) {
    self.ui.handle_event(event);
  }

  pub fn select_previous(&mut self) {
    if self.selected_widget == 0 {
      self.selected_widget = self.widgets.len() - 1;
    } else {
      self.selected_widget -= 1;
    }
  }

  pub fn select_next(&mut self) {
    if self.selected_widget == self.widgets.len() - 1 {
      self.selected_widget = 0;
    } else {
      self.selected_widget += 1;
    }
  }

  pub fn decrease_slider(&mut self) {
    let weight = Rc::clone(&self.widgets[self.selected_widget].weight);
    let original_weight = *weight.borrow();
    *weight.borrow_mut() = f32::max(original_weight - 0.01, 0.0);
  }

  pub fn increase_slider(&mut self) {
    let weight = Rc::clone(&self.widgets[self.selected_widget].weight);
    let original_weight = *weight.borrow();
    *weight.borrow_mut() = f32::min(original_weight + 0.01, 1.0);
  }

  pub fn activate(&mut self) -> Action {
    self.widgets[self.selected_widget].action
  }
}
