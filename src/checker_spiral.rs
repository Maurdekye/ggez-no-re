pub struct CheckerSpiral(u32);

impl CheckerSpiral {
    pub fn new() -> Self {
        CheckerSpiral(0)
    }
}

impl Iterator for CheckerSpiral {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        let CheckerSpiral(i) = self;
        let j = *i;
        *i += 1;
        let i = j as i32;
        if i == 0 {
            return Some((0, 0));
        }
        
        let layer = (((1.0 + 2.0 * (i as f64)).sqrt() - 1.0) / 2.0).ceil() as i32;
        let last_layer = layer - 1;
        let subindex = i - (2 * last_layer * last_layer + 2 * last_layer + 1);
        let wall_width = 2 * layer + 1;
        let wall_elements = layer;
        let side = subindex / wall_elements;
        let side_index = subindex % wall_elements;
        let pos = match side {
            0 => (wall_width / 2 - side_index * 2, layer),
            1 => (-layer, wall_width / 2 - side_index * 2),
            2 => (side_index * 2 - wall_width / 2, -layer),
            3 | _ => (layer, side_index * 2 - wall_width / 2),
        };
        Some(pos)
    }
}