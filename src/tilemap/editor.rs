use std::{cell::RefCell, sync::Arc};

use imgui::TextureId;
use vulkano::image::StorageImage;

use crate::{imgui_vulkano_renderer::ImGuiRenderer, texture::Texture};

use super::{Orientation, TilemapSpriteConfig};

pub struct TilemapSpriteConfigEditor {
    target: Arc<RefCell<TilemapSpriteConfig>>,
    tex: TextureId,
    size: [u32; 2],
    selected_tile: (usize, usize),
}

impl TilemapSpriteConfigEditor {
    pub fn new(
        renderer: &mut ImGuiRenderer,
        target: Arc<RefCell<TilemapSpriteConfig>>,
        tex: Texture<StorageImage>,
    ) -> Self {
        let ui_tex = renderer.make_ui_texture(tex.clone());
        let id = renderer.textures().insert(ui_tex);

        Self {
            tex: id,
            target,
            size: tex.get_size(),
            selected_tile: (0, 0),
        }
    }

    pub fn run(&mut self, ui: &imgui::Ui) {
        let [x, y] = self.size;

        let uv_x = 32.0 / x as f32;
        let uv_y = 16.0 / y as f32;

        let [img_x, img_y] = [x as f32 * 4.0, y as f32 * 4.0];

        let [win_x, win_y] = ui.window_pos();

        let [s_x, s_y] = ui.cursor_pos();

        ui.get_window_draw_list()
            .add_image(
                self.tex,
                [win_x + s_x, win_y + s_y],
                [win_x + img_x + s_x, win_y + img_y + s_y],
            )
            .build();

        let tbl = ui
            .begin_table_with_sizing(
                "str_id",
                16,
                imgui::TableFlags::NO_PAD_INNER_X | imgui::TableFlags::NO_PAD_OUTER_X,
                [img_x, img_y],
                0.0,
            )
            .unwrap();

        for y in 0..8 {
            ui.table_next_row_with_height(imgui::TableRowFlags::empty(), 32.0);

            for x in 0..16 {
                ui.table_next_column();

                let l = match (x, y) {
                    p if p == self.selected_tile => String::from("X"),
                    p if self.target.borrow().orientations.contains_key(&p) => {
                        format!("[{},{}]", p.0, p.1)
                    }
                    p => format!("{},{}", p.0, p.1),
                };

                ui.text_colored([0.0, 0.0, 0.0, 1.0], l);
            }
        }

        tbl.end();

        // detect pressing on image

        ui.separator();

        let mut s = (self.selected_tile.0 as i32, self.selected_tile.1 as i32);

        imgui::Drag::new("X")
            .range(0, 15)
            .speed(1.0)
            .build(ui, &mut s.0);

        imgui::Drag::new("Y")
            .range(0, 15)
            .speed(1.0)
            .build(ui, &mut s.1);

        self.selected_tile.0 = 15.min(s.0 as usize);
        self.selected_tile.1 = 15.min(s.1 as usize);

        if self
            .target
            .borrow()
            .orientations
            .contains_key(&self.selected_tile)
        {
            if ui.button("Delete") {
                self.target
                    .borrow_mut()
                    .orientations
                    .remove(&self.selected_tile);
            } else {
                let mut o = *self
                    .target
                    .borrow()
                    .orientations
                    .get(&self.selected_tile)
                    .unwrap();

                let tbl = ui
                    .begin_table_with_sizing(
                        "t2",
                        3,
                        imgui::TableFlags::BORDERS,
                        [150.0, 150.0],
                        5.0,
                    )
                    .unwrap();

                let mut changed = false;

                for y in [Orientation::N, Orientation::NONE, Orientation::S] {
                    ui.table_next_row_with_height(imgui::TableRowFlags::empty(), 50.0);

                    for x in [Orientation::W, Orientation::NONE, Orientation::E] {
                        ui.table_next_column();

                        let dir = match (y, x) {
                            (x, Orientation::NONE) => x,
                            (Orientation::NONE, y) => y,
                            (Orientation::N, Orientation::E) => Orientation::NE,
                            (Orientation::N, Orientation::W) => Orientation::NW,
                            (Orientation::S, Orientation::E) => Orientation::SE,
                            (Orientation::S, Orientation::W) => Orientation::SW,
                            _ => panic!("impossible"),
                        };

                        if dir != Orientation::NONE {
                            //we know this direction is valid
                            let condition = o.get_requirement_mut(dir).unwrap();

                            if condition.is_none() {
                                //contains no value
                                if ui.button(format!("? {:?}", dir)) {
                                    // add

                                    *condition = Some(true);
                                    changed = true;
                                }
                            } else if condition.unwrap() {
                                //contains value true

                                if ui.button(format!("+ {:?}", dir)) {
                                    // remove

                                    *condition = Some(false);
                                    changed = true;
                                }
                            } else {
                                //contains value false

                                if ui.button(format!("- {:?}", dir)) {
                                    *condition = None;
                                    changed = true;
                                }
                            }
                        }
                    }
                }

                if changed {
                    println!("Changed!");
                    self.target
                        .borrow_mut()
                        .orientations
                        .insert(self.selected_tile, o);
                }

                tbl.end();

                ui.same_line();

                let [mut d_x, mut d_y] = ui.cursor_pos();

                d_x += win_x;
                d_y += win_y;

                //Draw a preview of this tile in space
                ui.get_window_draw_list()
                    .add_rect(
                        [d_x, d_y],
                        [d_x + 150.0, d_y + 150.0],
                        imgui::ImColor32::WHITE,
                    )
                    .build();
            }
        } else {
            if ui.button("Add Entry") {
                self.target
                    .borrow_mut()
                    .orientations
                    .insert(self.selected_tile, Default::default());
            }
        }
    }
}
