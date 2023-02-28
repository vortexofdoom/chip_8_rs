use sdl2::{render::{Canvas, Texture}, video::Window, rect::Point, pixels::Color};

pub const COLOR_ON: [u8; 3] = [255, 255, 255];
pub const COLOR_OFF: [u8; 3] = [0, 0, 0];

#[derive(Debug)]
pub struct Display {
    changed: bool,
    //color_on: [u8; 3],
    //color_off: [u8; 3],
    hi_mode: bool,
    lo_res: [u64; 32],
    hi_res: [u128; 64],
}

impl Default for Display {
    fn default() -> Self {
        Self {
            changed: false,
            //color_on: [255, 255, 255],
            //color_off: [0, 0, 0], 
            hi_mode: false,
            lo_res: [0; 32],
            hi_res: [0; 64], 
        }
    }
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.hi_mode {
            true => for row in self.hi_res.iter() {
                writeln!(f, "{row:0128b}")?;
            }
            false => for row in self.lo_res.iter() {
                writeln!(f, "{row:064b}")?;
            }
        }
        Ok(())
    }
}

impl Display {
    pub fn draw(&mut self, x: u8, y: usize, sprite: Vec<u8>) -> bool {
        self.changed = true;
        let mut res = false;
        if self.hi_mode {
            for row in 0..sprite.len() {
                if y + row >= 64 {
                    break;
                }
                let sprite = (sprite[row] as u128) << (120 - x);
                if !res && self.hi_res[y + row] & sprite != 0 {
                    res = true;
                }
                self.hi_res[y + row] ^= sprite;
            }
        } else {
            for row in 0..sprite.len() {
                if y + row >= 32 {
                    break;
                }
                let sprite = (sprite[row] as u64) << (56 - x);
                if !res && self.lo_res[y + row] & sprite != 0 {
                    res = true;
                }
                self.lo_res[y + row] ^= sprite;
            }
        }
        res
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn render(&mut self, texture: &mut Texture, canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        let mut data = vec![];
        if self.hi_mode {
            for (i, row) in self.hi_res.iter().enumerate() {
                for col in (0..128).rev() {
                    if row >> col & 1 == 1 {
                        canvas.draw_point(Point::new(col, i as i32)).expect("failed to draw line");
                    } 
                }
            }
            texture.update(None, &data, 128 * 3).expect("couldn't update texture");
        } else {
            for row in self.lo_res.iter() {
                for col in (0..64).rev() {
                    if row >> col & 1 == 1 {
                        data.extend_from_slice(&COLOR_ON);
                    } else {
                        data.extend_from_slice(&COLOR_OFF);
                    }
                }
            }
            texture.update(None, &data, 64 * 3).expect("couldn't update texture");
        }
        // let mut data = vec![];
        // let pixel = |row, col| {
        //     (if self.hi_mode { self.hi_res[row] } else { self.lo_res[row] as u128 } >> col) & 1 == 1
        // };
        // for row in 0..rows {
        //     for col in (0..cols).rev() {
        //         if pixel(row, col) {
        //             data.extend_from_slice(&self.color_on);
        //         } else {
        //             data.extend_from_slice(&self.color_off);
        //         };
        //     }
        // }
        self.changed = false;
        canvas.copy(texture, None, None).unwrap();
        canvas.present();
    }

    pub fn clear(&mut self) {
        if self.hi_mode {
            self.hi_res.fill(0);
        } else {
            self.lo_res.fill(0);
        }
        self.changed = true;
    }

    pub(crate) fn scroll_down(&mut self, rows: usize) {
        if self.hi_mode {
            // move down all rows starting from the back
            for row in (rows..64).rev() {
                self.hi_res[row] = self.hi_res[row - rows];
            }
            // set the remainder to 0
            for row in 0..rows {
                self.hi_res[row] = 0;
            }
        } else {
            for row in (rows..32).rev() {
                self.hi_res[row] = self.hi_res[row - rows];
            }
            for row in 0..rows {
                self.hi_res[row] = 0;
            }
        }
    }

    pub(crate) fn scroll_right(&mut self) {
        if self.hi_mode {
            for row in self.hi_res.iter_mut() {
                *row >>= 4;
            }
        } else {
            for row in self.lo_res.iter_mut() {
                *row >>= 4;
            }
        }
    }

    pub(crate) fn scroll_left(&mut self) {
        if self.hi_mode {
            for row in self.hi_res.iter_mut() {
                *row <<= 4;
            }
        } else {
            for row in self.lo_res.iter_mut() {
                *row <<= 4;
            }
        }
    }
}