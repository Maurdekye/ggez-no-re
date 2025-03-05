pub fn checker_spiral_pos(i: u32) -> (i32, i32) {
    if i == 0 {
        return (0, 0);
    }
    
    let i = i as i32;
    let layer = (((1.0 + 2.0 * (i as f64)).sqrt() - 1.0) / 2.0).ceil() as i32;
    let last_layer = layer - 1;
    let subindex = i - (2 * last_layer * last_layer + 2 * last_layer + 1);
    let wall_width = 2 * layer + 1;
    let wall_elements = layer;
    let side = subindex / wall_elements;
    let side_index = subindex % wall_elements;
    match side {
        0 => (wall_width / 2 - side_index * 2, layer),
        1 => (-layer, wall_width / 2 - side_index * 2),
        2 => (side_index * 2 - wall_width / 2, -layer),
        3 | _ => (layer, side_index * 2 - wall_width / 2),
    }
}

pub fn checker_spiral() -> impl Iterator<Item = (i32, i32)> {
    (0..).map(checker_spiral_pos)
}