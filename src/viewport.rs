use std::cmp;

#[derive(Default)]
pub struct Viewport {
    num_containers: usize,
    pub selection: usize,
    top: usize,
    pub height: usize,
}

impl Viewport {
    fn validate(&self) {
        if self.num_containers == 0 {
            assert_eq!(self.selection, 0);
            assert_eq!(self.top, 0);
        } else if self.height == 0 {
            assert!(self.selection < self.num_containers);
            assert_eq!(self.top, self.selection);
        } else {
            assert!(self.selection < self.num_containers);
            assert!(self.top <= self.selection);
            assert!(self.selection < self.top + self.height);
            assert!(self.top + self.height <= self.num_containers || self.top == 0);
        }
    }

    pub fn handle_up(&mut self) {
        self.selection = self.selection.saturating_sub(1);
        if self.top > self.selection {
            self.top -= 1;
        }
        self.validate();
    }

    pub fn handle_down(&mut self) {
        self.selection = cmp::min(self.num_containers.saturating_sub(1), self.selection + 1);
        if self.height == 0 {
            self.top = self.selection;
        } else if self.selection >= self.top + self.height {
            self.top += 1;
        }
        self.validate();
    }

    pub fn handle_resize(&mut self, height: usize) {
        let old_height = self.height;
        self.height = height;
        if self.num_containers > 0 {
            if self.height < old_height {
                if self.height == 0 {
                    self.top = self.selection;
                } else {
                    self.top += self.selection.saturating_sub(self.top + self.height - 1);
                }
            } else if self.height > old_height {
                self.top = self
                    .top
                    .saturating_sub((self.top + self.height).saturating_sub(self.num_containers));
            }
        }
        self.validate();
    }

    pub fn handle_num_containers(&mut self, num_containers: usize) {
        self.num_containers = num_containers;

        if num_containers == 0 {
            self.selection = 0;
            self.top = 0;
        } else {
            if self.selection >= num_containers {
                self.selection = num_containers - 1;
            }
            self.top = self
                .top
                .saturating_sub((self.top + self.height).saturating_sub(num_containers));
        }
        self.validate();
    }

    pub fn select_for_render<C, LI, F, G>(
        &self,
        containers: &[C],
        f: F,
        g: G,
    ) -> impl Iterator<Item = LI>
    where
        F: Fn(&C) -> LI,
        G: Fn(&C) -> LI,
    {
        assert_eq!(containers.len(), self.num_containers);
        let empty_rows = (self.top + self.height).saturating_sub(self.num_containers);
        let container_rows = self.height - empty_rows;
        let selection_offset_in_viewport = self.selection - self.top;
        containers[self.top..self.top + container_rows]
            .iter()
            .enumerate()
            .map(move |(offset, container)| {
                if offset == selection_offset_in_viewport {
                    g(container)
                } else {
                    f(container)
                }
            })
    }
}
