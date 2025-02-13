use ggez::glam::Vec2;

pub type Line = Vec<Vec2>;

pub struct LineSectionsIter<'a> {
    line: &'a Line,
    current_segment_length: f32,
    current_segment_unconsumed_length: f32,
    off_length: f32,
    on_length: f32,
    vert_index: usize,
    initial_offset: Option<f32>,
}

impl LineSectionsIter<'_> {
    fn advance_segment(&mut self) -> Option<()> {
        (self.vert_index < self.line.len() - 2).then(|| {
            self.vert_index += 1;
            self.current_segment_length =
                (self.line[self.vert_index + 1] - self.line[self.vert_index]).length();
            self.current_segment_unconsumed_length = self.current_segment_length;
        })
    }

    fn current_vertex(&self) -> Vec2 {
        let alpha = self.current_segment_length / self.current_segment_unconsumed_length;
        (self.line[self.vert_index] * alpha) + (self.line[self.vert_index + 1] * (1.0 - alpha))
    }
}

impl Iterator for LineSectionsIter<'_> {
    type Item = Line;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = Vec::new();

        let mut remaining_length = self.initial_offset.take().unwrap_or(self.off_length);
        while remaining_length >= self.current_segment_length {
            remaining_length -= self.current_segment_length;
            self.advance_segment()?;
        }
        self.current_segment_length -= remaining_length;
        line.push(self.current_vertex());

        let mut remaining_length = self.on_length;
        while remaining_length >= self.current_segment_length {
            remaining_length -= self.current_segment_length;
            line.push(self.line[self.vert_index + 1]);
            if self.advance_segment().is_none() {
                return Some(line);
            }
        }
        self.current_segment_length -= remaining_length;
        line.push(self.current_vertex());

        Some(line)
    }
}

fn _subsections(
    line: &Line,
    off_length: f32,
    on_length: f32,
    initial_offset: Option<f32>,
) -> LineSectionsIter<'_> {
    let current_segment_length = (line[1] - line[0]).length();
    LineSectionsIter {
        line,
        current_segment_length,
        current_segment_unconsumed_length: current_segment_length,
        off_length,
        on_length,
        vert_index: 0,
        initial_offset,
    }
}

pub trait LineExt {
    #[allow(unused)]
    fn subsections(&self, off_length: f32, on_length: f32) -> LineSectionsIter<'_>;
    fn offset_subsections(
        &self,
        off_length: f32,
        on_length: f32,
        offset: f32,
    ) -> LineSectionsIter<'_>;
}

impl LineExt for Line {
    fn offset_subsections(
        &self,
        off_length: f32,
        on_length: f32,
        offset: f32,
    ) -> LineSectionsIter<'_> {
        _subsections(self, off_length, on_length, Some(offset))
    }

    fn subsections(&self, off_length: f32, on_length: f32) -> LineSectionsIter<'_> {
        _subsections(self, off_length, on_length, None)
    }
}

#[cfg(test)]
mod test {
    use ggez::glam::vec2;

    use super::LineExt;

    #[test]
    fn test_offset_sections() {
        let line = vec![vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(2.0, 0.0)];
        let subsections: Vec<_> = line.offset_subsections(0.25, 0.25, 0.0).collect();
        dbg!(subsections);
        let subsections: Vec<_> = line.offset_subsections(0.25, 0.25, 0.1).collect();
        dbg!(subsections);
    }
}
