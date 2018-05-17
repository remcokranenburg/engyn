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

use bincode;
use bincode::Infinite;
use std::fs::File;
use std::io::Result;
use std::io::Read;
use std::io::Write;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DemoEntry {
  pub head_left: [[f32; 4]; 4],
  pub head_right: [[f32; 4]; 4],
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Demo {
  pub entries: Vec<DemoEntry>,
}

impl Demo {
  pub fn new() -> Demo {
    Demo { entries: Vec::new() }
  }

  pub fn from_bincode(filename: &str) -> Result<Demo> {
    let mut bytes = Vec::new();
    let mut file = File::open(filename)?;

    file.read_to_end(&mut bytes)?;

    let demo: Demo = bincode::deserialize(&bytes).unwrap();
    Ok(demo)
  }

  pub fn to_bincode(&self, filename: &str) -> Result<()> {
    let mut file = File::create(filename)?;
    let bytes: Vec<u8> = bincode::serialize(self, Infinite).unwrap();
    file.write_all(&bytes)
  }
}
