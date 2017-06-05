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
use conrod::position::Align;
use conrod::position::Direction;
use conrod::position::Padding;
use conrod::position::Position;
use conrod::position::Relative;
use conrod::Positionable;
use conrod::Theme;
use conrod::theme::StyleMap;
use conrod::Ui;
use conrod::UiBuilder;
use conrod::Widget;
use conrod::widget::Text;
use glium::backend::glutin_backend::GlutinFacade;
use glium::Frame;
use glium::texture::Texture2d;
use std::time::Duration;

widget_ids! {
  pub struct Ids {
    title_text,
  }
}

pub struct Gui<'a> {
  context: &'a GlutinFacade,
  ids: Ids,
  image_map: Map<Texture2d>,
  renderer: Renderer,
  ui: Ui,
}

impl<'a> Gui<'a> {
  pub fn new(context: &'a GlutinFacade, width: f64, height: f64) -> Gui {
    let theme = Theme {
      name: "Engyn Default Theme".to_string(),
      padding: Padding::none(),
      x_position: Position::Relative(Relative::Align(Align::Start), None),
      y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
      background_color: color::DARK_CHARCOAL,
      shape_color: color::LIGHT_CHARCOAL,
      border_color: color::BLACK,
      border_width: 0.0,
      label_color: color::WHITE,
      font_id: None,
      font_size_large: 26,
      font_size_medium: 18,
      font_size_small: 12,
      widget_styling: StyleMap::default(),
      mouse_drag_threshold: 0.0,
      double_click_threshold: Duration::from_millis(500),
    };

    let mut ui = UiBuilder::new([width, height]).theme(theme).build();
    ui.fonts.insert_from_file("data/Cantarell-Regular.ttf").unwrap();

    Gui {
      context: context,
      ids: Ids::new(ui.widget_id_generator()),
      image_map: Map::<Texture2d>::new(),
      renderer: Renderer::new(context).unwrap(),
      ui: ui,
    }
  }

  pub fn draw(&mut self, target: &mut Frame) {
    {
      let ui = &mut self.ui.set_widgets();

      // "Hello World!" in the middle of the screen.
      Text::new("Hello, world!")
        .middle_of(ui.window)
        .color(color::WHITE)
        .font_size(32)
        .set(self.ids.title_text, ui);
    }

    // Render the `Ui` and then display it on the screen.
    let primitives = self.ui.draw();
    self.renderer.fill(self.context, primitives, &self.image_map);
    self.renderer.draw(self.context, target, &self.image_map).unwrap();
  }

  pub fn handle_event(&mut self, event: Input) {
    self.ui.handle_event(event);
  }
}
