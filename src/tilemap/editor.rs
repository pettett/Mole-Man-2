use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use imgui::TextureId;
use vulkano::image::StorageImage;

use crate::{imgui_vulkano_renderer::ImGuiRenderer, texture::Texture};

use super::{Orientation, TilemapSpriteConfig};

pub struct TilemapSpriteConfigEditor {
    target: Arc<Mutex<TilemapSpriteConfig>>,
    tex: TextureId,
    size: [u32; 2],
    selected_tile: (usize, usize),
}

impl TilemapSpriteConfigEditor {
    pub fn new(
        renderer: &mut ImGuiRenderer,
        target: Arc<Mutex<TilemapSpriteConfig>>,
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

        //let uv_x = 32.0 / x as f32;
        //let uv_y = 16.0 / y as f32;

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
                    p if self.target.lock().unwrap().orientations.contains_key(&p) => {
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

        let mut sprite_config = self.target.lock().unwrap();

        if sprite_config.orientations.contains_key(&self.selected_tile) {
            if ui.button("Delete") {
                sprite_config.orientations.remove(&self.selected_tile);
            } else {
                let mut o = *sprite_config.orientations.get(&self.selected_tile).unwrap();

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

                for off_y in -1..=1 {
                    ui.table_next_row_with_height(imgui::TableRowFlags::empty(), 50.0);

                    for off_x in -1..=1 {
                        ui.table_next_column();

                        if let Some(dir) = Orientation::orient(off_x, off_y) {
                            //we know this direction is valid
                            let condition = o.get_requirement_mut(dir).unwrap();

                            let (label, next_val) = match condition {
                                None => (format!("? {:?}", dir), Some(true)),
                                Some(true) => (format!("+ {:?}", dir), Some(false)),
                                Some(false) => (format!("- {:?}", dir), None),
                            };

                            if ui.button(label) {
                                *condition = next_val;
                                changed = true;
                            }
                        }
                    }
                }

                if changed {
                    println!("Changed!");
                    sprite_config.orientations.insert(self.selected_tile, o);
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

                let reqs = sprite_config.orientations[&self.selected_tile];

                // Draw the sprite preview -
                //  a 3 by 3 grid of sprites that show how the selected sprite's orientation
                //  will interact with it's surroundings
                for off_x in -1..=1 {
                    for off_y in -1..=1 {
                        //Only draw a tile if we have specifically requested it
                        if let Some((disp_x, disp_y)) =
                            if let Some(o) = Orientation::orient(off_x, off_y) {
                                //Test if this direction has been marked as solid

                                if let Some(Some(true)) = reqs.get_requirement(o) {
                                    //TODO: Allow placed tiles to also react to their surroundings
                                    Some((2, 2))
                                } else {
                                    None
                                }
                            } else {
                                //display the selected tile

                                Some(self.selected_tile)
                            }
                        {
                            let (uv_min, uv_max) = sprite_config.position_uv(disp_x, disp_y);

                            ui.get_window_draw_list()
                                .add_image(
                                    self.tex,
                                    [
                                        d_x + 50.0 * (off_x as f32 + 1.0),
                                        d_y + 50.0 * (off_y as f32 + 1.0),
                                    ],
                                    [
                                        d_x + 50.0 * (off_x as f32 + 2.0),
                                        d_y + 50.0 * (off_y as f32 + 2.0),
                                    ],
                                )
                                .uv_min(uv_min)
                                .uv_max(uv_max)
                                .build();
                        }
                    }
                }
            }
        } else {
            if ui.button("Add Entry") {
                sprite_config
                    .orientations
                    .insert(self.selected_tile, Default::default());
            }
        }
    }
}
