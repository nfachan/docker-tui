use derive_getters::Getters;
use std::cmp;

#[derive(Clone, Debug, Default, Getters, Eq, PartialEq)]
pub struct Viewport {
    #[getter(skip)]
    num_containers: usize,
    #[getter(skip)]
    selection: usize,
    #[getter(skip)]
    top: usize,
    height: usize,
}

impl Viewport {
    fn is_valid(&self) -> bool {
        self.num_containers == 0 && self.selection == 0 && self.top == 0
            || self.num_containers > 0
                && self.selection < self.num_containers
                && (self.top + cmp::max(self.height, 1) <= self.num_containers || self.top == 0)
                && self.top <= self.selection
                && self.selection < self.top + cmp::max(self.height, 1)
    }

    fn validate(&self) {
        if self.num_containers == 0 {
            assert_eq!(self.selection, 0);
            assert_eq!(self.top, 0);
        } else {
            assert!(self.selection < self.num_containers);
            assert!(self.top + cmp::max(self.height, 1) <= self.num_containers || self.top == 0);
            assert!(self.top <= self.selection);
            assert!(self.selection < self.top + cmp::max(self.height, 1));
        }
        assert!(self.is_valid());
    }

    pub fn move_selection_up_one_line(&mut self) {
        self.selection = self.selection.saturating_sub(1);
        if self.top > self.selection {
            self.top -= 1;
        }
        self.validate();
    }

    pub fn move_selection_down_one_line(&mut self) {
        self.selection = cmp::min(self.num_containers.saturating_sub(1), self.selection + 1);
        if self.height == 0 {
            self.top = self.selection;
        } else if self.selection >= self.top + self.height {
            self.top += 1;
        }
        self.validate();
    }

    pub fn scroll_up_n_lines(&mut self, n: usize) {
        // First, try to scroll the viewport up by as much as possible.
        let to_scroll = cmp::min(n, self.top);
        self.top -= to_scroll;
        self.selection = cmp::min(self.selection, self.top + self.height.saturating_sub(1));

        // Second, move the viewport up if we still need to scroll more.
        self.selection = self.selection.saturating_sub(n - to_scroll);

        self.validate();
    }

    pub fn scroll_down_n_lines(&mut self, n: usize) {
        // First, try to scroll the viewport down by as much as possible without it overhanging.
        let to_scroll = cmp::min(
            n,
            self.num_containers
                .saturating_sub(self.top + cmp::max(self.height, 1)),
        );
        self.top += to_scroll;
        self.selection = cmp::max(self.selection, self.top);

        // Second, move the viewport down if we still need to scroll more.
        self.selection += cmp::min(
            n - to_scroll,
            self.num_containers.saturating_sub(1) - self.selection,
        );

        self.validate();
    }

    pub fn change_viewport_height(&mut self, height: usize) {
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

    pub fn change_num_containers(&mut self, num_containers: usize) {
        self.num_containers = num_containers;

        if num_containers == 0 {
            self.selection = 0;
            self.top = 0;
        } else {
            if self.selection >= num_containers {
                self.selection = num_containers - 1;
            }
            if self.height == 0 {
                self.top = self.selection;
            } else {
                self.top = self
                    .top
                    .saturating_sub((self.top + self.height).saturating_sub(num_containers));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    macro_rules! viewport {
        ($num_containers:expr, $selection:expr, $top:expr, $height:expr) => {
            Viewport {
                num_containers: $num_containers,
                selection: $selection,
                top: $top,
                height: $height,
            }
        };
    }

    #[rstest]
    #[case(viewport!(0, 0, 0, 0), viewport!(0, 0, 0, 0))]
    #[case(viewport!(0, 0, 0, 1), viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 0), viewport!(1, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), viewport!(1, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 0), viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 1), viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 2), viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), viewport!(2, 0, 0, 3))]
    #[case(viewport!(2, 1, 1, 0), viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 1), viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 1, 0, 2), viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 1, 0, 3), viewport!(2, 0, 0, 3))]
    #[case(viewport!(3, 0, 0, 0), viewport!(3, 0, 0, 0))]
    #[case(viewport!(3, 0, 0, 1), viewport!(3, 0, 0, 1))]
    #[case(viewport!(3, 0, 0, 2), viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 0, 0, 3), viewport!(3, 0, 0, 3))]
    #[case(viewport!(3, 0, 0, 4), viewport!(3, 0, 0, 4))]
    #[case(viewport!(3, 1, 1, 0), viewport!(3, 0, 0, 0))]
    #[case(viewport!(3, 1, 1, 1), viewport!(3, 0, 0, 1))]
    #[case(viewport!(3, 1, 0, 2), viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 1, 1, 2), viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 1, 0, 3), viewport!(3, 0, 0, 3))]
    #[case(viewport!(3, 1, 0, 4), viewport!(3, 0, 0, 4))]
    #[case(viewport!(3, 2, 2, 0), viewport!(3, 1, 1, 0))]
    #[case(viewport!(3, 2, 2, 1), viewport!(3, 1, 1, 1))]
    #[case(viewport!(3, 2, 1, 2), viewport!(3, 1, 1, 2))]
    #[case(viewport!(3, 2, 0, 3), viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 2, 0, 4), viewport!(3, 1, 0, 4))]
    #[case(viewport!(4, 0, 0, 0), viewport!(4, 0, 0, 0))]
    #[case(viewport!(4, 0, 0, 1), viewport!(4, 0, 0, 1))]
    #[case(viewport!(4, 0, 0, 2), viewport!(4, 0, 0, 2))]
    #[case(viewport!(4, 0, 0, 3), viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 0, 0, 4), viewport!(4, 0, 0, 4))]
    #[case(viewport!(4, 0, 0, 5), viewport!(4, 0, 0, 5))]
    #[case(viewport!(4, 1, 1, 0), viewport!(4, 0, 0, 0))]
    #[case(viewport!(4, 1, 1, 1), viewport!(4, 0, 0, 1))]
    #[case(viewport!(4, 1, 0, 2), viewport!(4, 0, 0, 2))]
    #[case(viewport!(4, 1, 1, 2), viewport!(4, 0, 0, 2))]
    #[case(viewport!(4, 1, 0, 3), viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 1, 1, 3), viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 1, 0, 4), viewport!(4, 0, 0, 4))]
    #[case(viewport!(4, 1, 0, 5), viewport!(4, 0, 0, 5))]
    #[case(viewport!(4, 2, 2, 0), viewport!(4, 1, 1, 0))]
    #[case(viewport!(4, 2, 2, 1), viewport!(4, 1, 1, 1))]
    #[case(viewport!(4, 2, 1, 2), viewport!(4, 1, 1, 2))]
    #[case(viewport!(4, 2, 2, 2), viewport!(4, 1, 1, 2))]
    #[case(viewport!(4, 2, 0, 3), viewport!(4, 1, 0, 3))]
    #[case(viewport!(4, 2, 1, 3), viewport!(4, 1, 1, 3))]
    #[case(viewport!(4, 2, 0, 4), viewport!(4, 1, 0, 4))]
    #[case(viewport!(4, 2, 0, 5), viewport!(4, 1, 0, 5))]
    #[case(viewport!(4, 3, 3, 0), viewport!(4, 2, 2, 0))]
    #[case(viewport!(4, 3, 3, 1), viewport!(4, 2, 2, 1))]
    #[case(viewport!(4, 3, 2, 2), viewport!(4, 2, 2, 2))]
    #[case(viewport!(4, 3, 1, 3), viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 3, 0, 4), viewport!(4, 2, 0, 4))]
    #[case(viewport!(4, 3, 0, 5), viewport!(4, 2, 0, 5))]
    fn move_selection_up_one_line(#[case] mut before: Viewport, #[case] after: Viewport) {
        before.move_selection_up_one_line();
        assert_eq!(before, after);
    }

    #[rstest]
    #[case(viewport!(0, 0, 0, 0), viewport!(0, 0, 0, 0))]
    #[case(viewport!(0, 0, 0, 1), viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 0), viewport!(1, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), viewport!(1, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 0), viewport!(2, 1, 1, 0))]
    #[case(viewport!(2, 0, 0, 1), viewport!(2, 1, 1, 1))]
    #[case(viewport!(2, 0, 0, 2), viewport!(2, 1, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), viewport!(2, 1, 0, 3))]
    #[case(viewport!(2, 1, 1, 0), viewport!(2, 1, 1, 0))]
    #[case(viewport!(2, 1, 1, 1), viewport!(2, 1, 1, 1))]
    #[case(viewport!(2, 1, 0, 2), viewport!(2, 1, 0, 2))]
    #[case(viewport!(2, 1, 0, 3), viewport!(2, 1, 0, 3))]
    #[case(viewport!(3, 0, 0, 0), viewport!(3, 1, 1, 0))]
    #[case(viewport!(3, 0, 0, 1), viewport!(3, 1, 1, 1))]
    #[case(viewport!(3, 0, 0, 2), viewport!(3, 1, 0, 2))]
    #[case(viewport!(3, 0, 0, 3), viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 0, 0, 4), viewport!(3, 1, 0, 4))]
    #[case(viewport!(3, 1, 1, 0), viewport!(3, 2, 2, 0))]
    #[case(viewport!(3, 1, 1, 1), viewport!(3, 2, 2, 1))]
    #[case(viewport!(3, 1, 0, 2), viewport!(3, 2, 1, 2))]
    #[case(viewport!(3, 1, 1, 2), viewport!(3, 2, 1, 2))]
    #[case(viewport!(3, 1, 0, 3), viewport!(3, 2, 0, 3))]
    #[case(viewport!(3, 1, 0, 4), viewport!(3, 2, 0, 4))]
    #[case(viewport!(3, 2, 2, 0), viewport!(3, 2, 2, 0))]
    #[case(viewport!(3, 2, 2, 1), viewport!(3, 2, 2, 1))]
    #[case(viewport!(3, 2, 1, 2), viewport!(3, 2, 1, 2))]
    #[case(viewport!(3, 2, 0, 3), viewport!(3, 2, 0, 3))]
    #[case(viewport!(3, 2, 0, 4), viewport!(3, 2, 0, 4))]
    #[case(viewport!(4, 0, 0, 0), viewport!(4, 1, 1, 0))]
    #[case(viewport!(4, 0, 0, 1), viewport!(4, 1, 1, 1))]
    #[case(viewport!(4, 0, 0, 2), viewport!(4, 1, 0, 2))]
    #[case(viewport!(4, 0, 0, 3), viewport!(4, 1, 0, 3))]
    #[case(viewport!(4, 0, 0, 4), viewport!(4, 1, 0, 4))]
    #[case(viewport!(4, 0, 0, 5), viewport!(4, 1, 0, 5))]
    #[case(viewport!(4, 1, 1, 0), viewport!(4, 2, 2, 0))]
    #[case(viewport!(4, 1, 1, 1), viewport!(4, 2, 2, 1))]
    #[case(viewport!(4, 1, 0, 2), viewport!(4, 2, 1, 2))]
    #[case(viewport!(4, 1, 1, 2), viewport!(4, 2, 1, 2))]
    #[case(viewport!(4, 1, 0, 3), viewport!(4, 2, 0, 3))]
    #[case(viewport!(4, 1, 1, 3), viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 1, 0, 4), viewport!(4, 2, 0, 4))]
    #[case(viewport!(4, 1, 0, 5), viewport!(4, 2, 0, 5))]
    #[case(viewport!(4, 2, 2, 0), viewport!(4, 3, 3, 0))]
    #[case(viewport!(4, 2, 2, 1), viewport!(4, 3, 3, 1))]
    #[case(viewport!(4, 2, 1, 2), viewport!(4, 3, 2, 2))]
    #[case(viewport!(4, 2, 2, 2), viewport!(4, 3, 2, 2))]
    #[case(viewport!(4, 2, 0, 3), viewport!(4, 3, 1, 3))]
    #[case(viewport!(4, 2, 1, 3), viewport!(4, 3, 1, 3))]
    #[case(viewport!(4, 2, 0, 4), viewport!(4, 3, 0, 4))]
    #[case(viewport!(4, 2, 0, 5), viewport!(4, 3, 0, 5))]
    #[case(viewport!(4, 3, 3, 0), viewport!(4, 3, 3, 0))]
    #[case(viewport!(4, 3, 3, 1), viewport!(4, 3, 3, 1))]
    #[case(viewport!(4, 3, 2, 2), viewport!(4, 3, 2, 2))]
    #[case(viewport!(4, 3, 1, 3), viewport!(4, 3, 1, 3))]
    #[case(viewport!(4, 3, 0, 4), viewport!(4, 3, 0, 4))]
    #[case(viewport!(4, 3, 0, 5), viewport!(4, 3, 0, 5))]
    fn move_selection_down_one_line(#[case] mut before: Viewport, #[case] after: Viewport) {
        before.move_selection_down_one_line();
        assert_eq!(before, after);
    }

    #[rstest]
    #[case(viewport!(0, 0, 0, 0), viewport!(0, 0, 0, 0))]
    #[case(viewport!(0, 0, 0, 1), viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 0), viewport!(1, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), viewport!(1, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 0), viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 1), viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 2), viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), viewport!(2, 0, 0, 3))]
    #[case(viewport!(2, 1, 1, 0), viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 1), viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 1, 0, 2), viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 1, 0, 3), viewport!(2, 0, 0, 3))]
    #[case(viewport!(3, 0, 0, 0), viewport!(3, 0, 0, 0))]
    #[case(viewport!(3, 0, 0, 1), viewport!(3, 0, 0, 1))]
    #[case(viewport!(3, 0, 0, 2), viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 0, 0, 3), viewport!(3, 0, 0, 3))]
    #[case(viewport!(3, 0, 0, 4), viewport!(3, 0, 0, 4))]
    #[case(viewport!(3, 1, 1, 0), viewport!(3, 0, 0, 0))]
    #[case(viewport!(3, 1, 1, 1), viewport!(3, 0, 0, 1))]
    #[case(viewport!(3, 1, 0, 2), viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 1, 1, 2), viewport!(3, 1, 0, 2))]
    #[case(viewport!(3, 1, 0, 3), viewport!(3, 0, 0, 3))]
    #[case(viewport!(3, 1, 0, 4), viewport!(3, 0, 0, 4))]
    #[case(viewport!(3, 2, 2, 0), viewport!(3, 1, 1, 0))]
    #[case(viewport!(3, 2, 2, 1), viewport!(3, 1, 1, 1))]
    #[case(viewport!(3, 2, 1, 2), viewport!(3, 1, 0, 2))]
    #[case(viewport!(3, 2, 0, 3), viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 2, 0, 4), viewport!(3, 1, 0, 4))]
    #[case(viewport!(4, 0, 0, 0), viewport!(4, 0, 0, 0))]
    #[case(viewport!(4, 0, 0, 1), viewport!(4, 0, 0, 1))]
    #[case(viewport!(4, 0, 0, 2), viewport!(4, 0, 0, 2))]
    #[case(viewport!(4, 0, 0, 3), viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 0, 0, 4), viewport!(4, 0, 0, 4))]
    #[case(viewport!(4, 0, 0, 5), viewport!(4, 0, 0, 5))]
    #[case(viewport!(4, 1, 1, 0), viewport!(4, 0, 0, 0))]
    #[case(viewport!(4, 1, 1, 1), viewport!(4, 0, 0, 1))]
    #[case(viewport!(4, 1, 0, 2), viewport!(4, 0, 0, 2))]
    #[case(viewport!(4, 1, 1, 2), viewport!(4, 1, 0, 2))]
    #[case(viewport!(4, 1, 0, 3), viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 1, 1, 3), viewport!(4, 1, 0, 3))]
    #[case(viewport!(4, 1, 0, 4), viewport!(4, 0, 0, 4))]
    #[case(viewport!(4, 1, 0, 5), viewport!(4, 0, 0, 5))]
    #[case(viewport!(4, 2, 2, 0), viewport!(4, 1, 1, 0))]
    #[case(viewport!(4, 2, 2, 1), viewport!(4, 1, 1, 1))]
    #[case(viewport!(4, 2, 1, 2), viewport!(4, 1, 0, 2))]
    #[case(viewport!(4, 2, 2, 2), viewport!(4, 2, 1, 2))]
    #[case(viewport!(4, 2, 0, 3), viewport!(4, 1, 0, 3))]
    #[case(viewport!(4, 2, 1, 3), viewport!(4, 2, 0, 3))]
    #[case(viewport!(4, 2, 0, 4), viewport!(4, 1, 0, 4))]
    #[case(viewport!(4, 2, 0, 5), viewport!(4, 1, 0, 5))]
    #[case(viewport!(4, 3, 3, 0), viewport!(4, 2, 2, 0))]
    #[case(viewport!(4, 3, 3, 1), viewport!(4, 2, 2, 1))]
    #[case(viewport!(4, 3, 2, 2), viewport!(4, 2, 1, 2))]
    #[case(viewport!(4, 3, 1, 3), viewport!(4, 2, 0, 3))]
    #[case(viewport!(4, 3, 0, 4), viewport!(4, 2, 0, 4))]
    #[case(viewport!(4, 3, 0, 5), viewport!(4, 2, 0, 5))]
    fn scroll_up_one_line(#[case] mut before: Viewport, #[case] after: Viewport) {
        before.scroll_up_n_lines(1);
        assert_eq!(before, after);
    }

    #[test]
    fn scroll_up_n_lines() {
        for num_containers in 0..10 {
            for selection in 0..10 {
                for top in 0..10 {
                    for height in 0..10 {
                        let initial_viewport = viewport!(num_containers, selection, top, height);
                        if !initial_viewport.is_valid() {
                            continue;
                        }
                        let mut viewport = initial_viewport.clone();
                        for n in 1..10 {
                            viewport.scroll_up_n_lines(1);
                            let mut scrolled_viewport = initial_viewport.clone();
                            scrolled_viewport.scroll_up_n_lines(n);
                            assert_eq!(viewport, scrolled_viewport);
                        }
                    }
                }
            }
        }
    }

    #[rstest]
    #[case(viewport!(0, 0, 0, 0), viewport!(0, 0, 0, 0))]
    #[case(viewport!(0, 0, 0, 1), viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 0), viewport!(1, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), viewport!(1, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 0), viewport!(2, 1, 1, 0))]
    #[case(viewport!(2, 0, 0, 1), viewport!(2, 1, 1, 1))]
    #[case(viewport!(2, 0, 0, 2), viewport!(2, 1, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), viewport!(2, 1, 0, 3))]
    #[case(viewport!(2, 1, 1, 0), viewport!(2, 1, 1, 0))]
    #[case(viewport!(2, 1, 1, 1), viewport!(2, 1, 1, 1))]
    #[case(viewport!(2, 1, 0, 2), viewport!(2, 1, 0, 2))]
    #[case(viewport!(2, 1, 0, 3), viewport!(2, 1, 0, 3))]
    #[case(viewport!(3, 0, 0, 0), viewport!(3, 1, 1, 0))]
    #[case(viewport!(3, 0, 0, 1), viewport!(3, 1, 1, 1))]
    #[case(viewport!(3, 0, 0, 2), viewport!(3, 1, 1, 2))]
    #[case(viewport!(3, 0, 0, 3), viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 0, 0, 4), viewport!(3, 1, 0, 4))]
    #[case(viewport!(3, 1, 1, 0), viewport!(3, 2, 2, 0))]
    #[case(viewport!(3, 1, 1, 1), viewport!(3, 2, 2, 1))]
    #[case(viewport!(3, 1, 0, 2), viewport!(3, 1, 1, 2))]
    #[case(viewport!(3, 1, 1, 2), viewport!(3, 2, 1, 2))]
    #[case(viewport!(3, 1, 0, 3), viewport!(3, 2, 0, 3))]
    #[case(viewport!(3, 1, 0, 4), viewport!(3, 2, 0, 4))]
    #[case(viewport!(3, 2, 2, 0), viewport!(3, 2, 2, 0))]
    #[case(viewport!(3, 2, 2, 1), viewport!(3, 2, 2, 1))]
    #[case(viewport!(3, 2, 1, 2), viewport!(3, 2, 1, 2))]
    #[case(viewport!(3, 2, 0, 3), viewport!(3, 2, 0, 3))]
    #[case(viewport!(3, 2, 0, 4), viewport!(3, 2, 0, 4))]
    #[case(viewport!(4, 0, 0, 0), viewport!(4, 1, 1, 0))]
    #[case(viewport!(4, 0, 0, 1), viewport!(4, 1, 1, 1))]
    #[case(viewport!(4, 0, 0, 2), viewport!(4, 1, 1, 2))]
    #[case(viewport!(4, 0, 0, 3), viewport!(4, 1, 1, 3))]
    #[case(viewport!(4, 0, 0, 4), viewport!(4, 1, 0, 4))]
    #[case(viewport!(4, 0, 0, 5), viewport!(4, 1, 0, 5))]
    #[case(viewport!(4, 1, 1, 0), viewport!(4, 2, 2, 0))]
    #[case(viewport!(4, 1, 1, 1), viewport!(4, 2, 2, 1))]
    #[case(viewport!(4, 1, 0, 2), viewport!(4, 1, 1, 2))]
    #[case(viewport!(4, 1, 1, 2), viewport!(4, 2, 2, 2))]
    #[case(viewport!(4, 1, 0, 3), viewport!(4, 1, 1, 3))]
    #[case(viewport!(4, 1, 1, 3), viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 1, 0, 4), viewport!(4, 2, 0, 4))]
    #[case(viewport!(4, 1, 0, 5), viewport!(4, 2, 0, 5))]
    #[case(viewport!(4, 2, 2, 0), viewport!(4, 3, 3, 0))]
    #[case(viewport!(4, 2, 2, 1), viewport!(4, 3, 3, 1))]
    #[case(viewport!(4, 2, 1, 2), viewport!(4, 2, 2, 2))]
    #[case(viewport!(4, 2, 2, 2), viewport!(4, 3, 2, 2))]
    #[case(viewport!(4, 2, 0, 3), viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 2, 1, 3), viewport!(4, 3, 1, 3))]
    #[case(viewport!(4, 2, 0, 4), viewport!(4, 3, 0, 4))]
    #[case(viewport!(4, 2, 0, 5), viewport!(4, 3, 0, 5))]
    #[case(viewport!(4, 3, 3, 0), viewport!(4, 3, 3, 0))]
    #[case(viewport!(4, 3, 3, 1), viewport!(4, 3, 3, 1))]
    #[case(viewport!(4, 3, 2, 2), viewport!(4, 3, 2, 2))]
    #[case(viewport!(4, 3, 1, 3), viewport!(4, 3, 1, 3))]
    #[case(viewport!(4, 3, 0, 4), viewport!(4, 3, 0, 4))]
    #[case(viewport!(4, 3, 0, 5), viewport!(4, 3, 0, 5))]
    fn scroll_down_one_line(#[case] mut before: Viewport, #[case] after: Viewport) {
        before.scroll_down_n_lines(1);
        assert_eq!(before, after);
    }

    #[test]
    fn scroll_down_n_lines() {
        for num_containers in 0..10 {
            for selection in 0..10 {
                for top in 0..10 {
                    for height in 0..10 {
                        let initial_viewport = viewport!(num_containers, selection, top, height);
                        if !initial_viewport.is_valid() {
                            continue;
                        }
                        let mut viewport = initial_viewport.clone();
                        for n in 1..10 {
                            viewport.scroll_down_n_lines(1);
                            let mut scrolled_viewport = initial_viewport.clone();
                            scrolled_viewport.scroll_down_n_lines(n);
                            assert_eq!(viewport, scrolled_viewport);
                        }
                    }
                }
            }
        }
    }

    #[rstest]
    // Height zero to height zero.
    #[case(viewport!(0, 0, 0, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 0), 0, viewport!(1, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 0), 0, viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 0), 0, viewport!(2, 1, 1, 0))]
    // Height zero to height one.
    #[case(viewport!(0, 0, 0, 0), 1, viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 0), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 0), 1, viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 1, 1, 0), 1, viewport!(2, 1, 1, 1))]
    // Height zero to height two.
    #[case(viewport!(0, 0, 0, 0), 2, viewport!(0, 0, 0, 2))]
    #[case(viewport!(1, 0, 0, 0), 2, viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 0), 2, viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 1, 1, 0), 2, viewport!(2, 1, 0, 2))]
    #[case(viewport!(3, 0, 0, 0), 2, viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 1, 1, 0), 2, viewport!(3, 1, 1, 2))]
    #[case(viewport!(3, 2, 2, 0), 2, viewport!(3, 2, 1, 2))]
    // Height zero to height three.
    #[case(viewport!(0, 0, 0, 0), 3, viewport!(0, 0, 0, 3))]
    #[case(viewport!(1, 0, 0, 0), 3, viewport!(1, 0, 0, 3))]
    #[case(viewport!(2, 0, 0, 0), 3, viewport!(2, 0, 0, 3))]
    #[case(viewport!(2, 1, 1, 0), 3, viewport!(2, 1, 0, 3))]
    #[case(viewport!(3, 0, 0, 0), 3, viewport!(3, 0, 0, 3))]
    #[case(viewport!(3, 1, 1, 0), 3, viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 2, 2, 0), 3, viewport!(3, 2, 0, 3))]
    #[case(viewport!(4, 0, 0, 0), 3, viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 1, 1, 0), 3, viewport!(4, 1, 1, 3))]
    #[case(viewport!(4, 2, 2, 0), 3, viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 3, 3, 0), 3, viewport!(4, 3, 1, 3))]
    // Height one to height zero.
    #[case(viewport!(0, 0, 0, 1), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), 0, viewport!(1, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 1), 0, viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 1), 0, viewport!(2, 1, 1, 0))]
    // Height one to height one.
    #[case(viewport!(0, 0, 0, 1), 1, viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 1), 1, viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 1, 1, 1), 1, viewport!(2, 1, 1, 1))]
    // Height one to height two.
    #[case(viewport!(0, 0, 0, 1), 2, viewport!(0, 0, 0, 2))]
    #[case(viewport!(1, 0, 0, 1), 2, viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 1), 2, viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 1, 1, 1), 2, viewport!(2, 1, 0, 2))]
    #[case(viewport!(3, 0, 0, 1), 2, viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 1, 1, 1), 2, viewport!(3, 1, 1, 2))]
    #[case(viewport!(3, 2, 2, 1), 2, viewport!(3, 2, 1, 2))]
    // Height one to height three.
    #[case(viewport!(0, 0, 0, 1), 3, viewport!(0, 0, 0, 3))]
    #[case(viewport!(1, 0, 0, 1), 3, viewport!(1, 0, 0, 3))]
    #[case(viewport!(2, 0, 0, 1), 3, viewport!(2, 0, 0, 3))]
    #[case(viewport!(2, 1, 1, 1), 3, viewport!(2, 1, 0, 3))]
    #[case(viewport!(3, 0, 0, 1), 3, viewport!(3, 0, 0, 3))]
    #[case(viewport!(3, 1, 1, 1), 3, viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 2, 2, 1), 3, viewport!(3, 2, 0, 3))]
    #[case(viewport!(4, 0, 0, 1), 3, viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 1, 1, 1), 3, viewport!(4, 1, 1, 3))]
    #[case(viewport!(4, 2, 2, 1), 3, viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 3, 3, 1), 3, viewport!(4, 3, 1, 3))]
    // Height two to height zero.
    #[case(viewport!(0, 0, 0, 2), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 2), 0, viewport!(1, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 2), 0, viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 1, 0, 2), 0, viewport!(2, 1, 1, 0))]
    // Height two to height one.
    #[case(viewport!(0, 0, 0, 2), 1, viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 2), 1, viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 1, 0, 2), 1, viewport!(2, 1, 1, 1))]
    // Height two to height two.
    #[case(viewport!(0, 0, 0, 2), 2, viewport!(0, 0, 0, 2))]
    #[case(viewport!(1, 0, 0, 2), 2, viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 2), 2, viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 1, 0, 2), 2, viewport!(2, 1, 0, 2))]
    #[case(viewport!(3, 0, 0, 2), 2, viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 1, 1, 2), 2, viewport!(3, 1, 1, 2))]
    #[case(viewport!(3, 2, 1, 2), 2, viewport!(3, 2, 1, 2))]
    // Height two to height three.
    #[case(viewport!(0, 0, 0, 2), 3, viewport!(0, 0, 0, 3))]
    #[case(viewport!(1, 0, 0, 2), 3, viewport!(1, 0, 0, 3))]
    #[case(viewport!(2, 0, 0, 2), 3, viewport!(2, 0, 0, 3))]
    #[case(viewport!(2, 1, 0, 2), 3, viewport!(2, 1, 0, 3))]
    #[case(viewport!(3, 0, 0, 2), 3, viewport!(3, 0, 0, 3))]
    #[case(viewport!(3, 1, 0, 2), 3, viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 1, 1, 2), 3, viewport!(3, 1, 0, 3))]
    #[case(viewport!(3, 2, 1, 2), 3, viewport!(3, 2, 0, 3))]
    #[case(viewport!(4, 0, 0, 2), 3, viewport!(4, 0, 0, 3))]
    #[case(viewport!(4, 1, 0, 2), 3, viewport!(4, 1, 0, 3))]
    #[case(viewport!(4, 1, 1, 2), 3, viewport!(4, 1, 1, 3))]
    #[case(viewport!(4, 2, 1, 2), 3, viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 2, 2, 2), 3, viewport!(4, 2, 1, 3))]
    #[case(viewport!(4, 3, 2, 2), 3, viewport!(4, 3, 1, 3))]
    // Height three to height zero.
    #[case(viewport!(0, 0, 0, 3), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 3), 0, viewport!(1, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 3), 0, viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 1, 0, 3), 0, viewport!(2, 1, 1, 0))]
    #[case(viewport!(3, 0, 0, 3), 0, viewport!(3, 0, 0, 0))]
    #[case(viewport!(3, 1, 0, 3), 0, viewport!(3, 1, 1, 0))]
    #[case(viewport!(3, 2, 0, 3), 0, viewport!(3, 2, 2, 0))]
    // Height three to height one.
    #[case(viewport!(0, 0, 0, 3), 1, viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 3), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 3), 1, viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 1, 0, 3), 1, viewport!(2, 1, 1, 1))]
    #[case(viewport!(3, 0, 0, 3), 1, viewport!(3, 0, 0, 1))]
    #[case(viewport!(3, 1, 0, 3), 1, viewport!(3, 1, 1, 1))]
    #[case(viewport!(3, 2, 0, 3), 1, viewport!(3, 2, 2, 1))]
    // Height three to height two.
    #[case(viewport!(0, 0, 0, 3), 2, viewport!(0, 0, 0, 2))]
    #[case(viewport!(1, 0, 0, 3), 2, viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), 2, viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 1, 0, 3), 2, viewport!(2, 1, 0, 2))]
    #[case(viewport!(3, 0, 0, 3), 2, viewport!(3, 0, 0, 2))]
    #[case(viewport!(3, 1, 0, 3), 2, viewport!(3, 1, 0, 2))]
    #[case(viewport!(3, 2, 0, 3), 2, viewport!(3, 2, 1, 2))]
    #[case(viewport!(4, 0, 0, 3), 2, viewport!(4, 0, 0, 2))]
    #[case(viewport!(4, 1, 0, 3), 2, viewport!(4, 1, 0, 2))]
    #[case(viewport!(4, 1, 1, 3), 2, viewport!(4, 1, 1, 2))]
    #[case(viewport!(4, 2, 0, 3), 2, viewport!(4, 2, 1, 2))]
    #[case(viewport!(4, 2, 1, 3), 2, viewport!(4, 2, 1, 2))]
    #[case(viewport!(4, 3, 1, 3), 2, viewport!(4, 3, 2, 2))]
    fn change_viewport_height(
        #[case] mut before: Viewport,
        #[case] resize_to: usize,
        #[case] after: Viewport,
    ) {
        before.change_viewport_height(resize_to);
        assert_eq!(before, after);
    }

    #[rstest]
    // Zero containers to zero containers.
    #[case(viewport!(0, 0, 0, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(0, 0, 0, 1), 0, viewport!(0, 0, 0, 1))]
    #[case(viewport!(0, 0, 0, 2), 0, viewport!(0, 0, 0, 2))]
    // Zero containers to one container.
    #[case(viewport!(0, 0, 0, 0), 1, viewport!(1, 0, 0, 0))]
    #[case(viewport!(0, 0, 0, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(0, 0, 0, 2), 1, viewport!(1, 0, 0, 2))]
    // Zero containers to two containers.
    #[case(viewport!(0, 0, 0, 0), 2, viewport!(2, 0, 0, 0))]
    #[case(viewport!(0, 0, 0, 1), 2, viewport!(2, 0, 0, 1))]
    #[case(viewport!(0, 0, 0, 2), 2, viewport!(2, 0, 0, 2))]
    // One container to zero containers.
    #[case(viewport!(1, 0, 0, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), 0, viewport!(0, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), 0, viewport!(0, 0, 0, 2))]
    // One container to one container.
    #[case(viewport!(1, 0, 0, 0), 1, viewport!(1, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), 1, viewport!(1, 0, 0, 2))]
    // One container to two containers.
    #[case(viewport!(1, 0, 0, 0), 2, viewport!(2, 0, 0, 0))]
    #[case(viewport!(1, 0, 0, 1), 2, viewport!(2, 0, 0, 1))]
    #[case(viewport!(1, 0, 0, 2), 2, viewport!(2, 0, 0, 2))]
    // Two containers to zero containers.
    #[case(viewport!(2, 0, 0, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 1), 0, viewport!(0, 0, 0, 1))]
    #[case(viewport!(2, 1, 1, 1), 0, viewport!(0, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 2), 0, viewport!(0, 0, 0, 2))]
    #[case(viewport!(2, 1, 0, 2), 0, viewport!(0, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), 0, viewport!(0, 0, 0, 3))]
    #[case(viewport!(2, 1, 0, 3), 0, viewport!(0, 0, 0, 3))]
    // Two containers to one container.
    #[case(viewport!(2, 0, 0, 0), 1, viewport!(1, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 0), 1, viewport!(1, 0, 0, 0))]
    #[case(viewport!(2, 0, 0, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(2, 1, 1, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(2, 0, 0, 2), 1, viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 1, 0, 2), 1, viewport!(1, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), 1, viewport!(1, 0, 0, 3))]
    #[case(viewport!(2, 1, 0, 3), 1, viewport!(1, 0, 0, 3))]
    #[case(viewport!(2, 2, 0, 3), 1, viewport!(1, 0, 0, 3))]
    // Two containers to two containers.
    #[case(viewport!(2, 0, 0, 0), 2, viewport!(2, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 0), 2, viewport!(2, 1, 1, 0))]
    #[case(viewport!(2, 0, 0, 1), 2, viewport!(2, 0, 0, 1))]
    #[case(viewport!(2, 1, 1, 1), 2, viewport!(2, 1, 1, 1))]
    #[case(viewport!(2, 0, 0, 2), 2, viewport!(2, 0, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), 2, viewport!(2, 0, 0, 3))]
    // Two containers to three containers.
    #[case(viewport!(2, 0, 0, 0), 3, viewport!(3, 0, 0, 0))]
    #[case(viewport!(2, 1, 1, 0), 3, viewport!(3, 1, 1, 0))]
    #[case(viewport!(2, 0, 0, 1), 3, viewport!(3, 0, 0, 1))]
    #[case(viewport!(2, 1, 1, 1), 3, viewport!(3, 1, 1, 1))]
    #[case(viewport!(2, 0, 0, 2), 3, viewport!(3, 0, 0, 2))]
    #[case(viewport!(2, 1, 0, 2), 3, viewport!(3, 1, 0, 2))]
    #[case(viewport!(2, 0, 0, 3), 3, viewport!(3, 0, 0, 3))]
    #[case(viewport!(2, 1, 0, 3), 3, viewport!(3, 1, 0, 3))]
    // Three containers to zero containers.
    #[case(viewport!(3, 0, 0, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(3, 1, 1, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(3, 2, 2, 0), 0, viewport!(0, 0, 0, 0))]
    #[case(viewport!(3, 0, 0, 1), 0, viewport!(0, 0, 0, 1))]
    #[case(viewport!(3, 1, 1, 1), 0, viewport!(0, 0, 0, 1))]
    #[case(viewport!(3, 2, 2, 1), 0, viewport!(0, 0, 0, 1))]
    #[case(viewport!(3, 0, 0, 2), 0, viewport!(0, 0, 0, 2))]
    #[case(viewport!(3, 1, 0, 2), 0, viewport!(0, 0, 0, 2))]
    #[case(viewport!(3, 1, 1, 2), 0, viewport!(0, 0, 0, 2))]
    #[case(viewport!(3, 2, 1, 2), 0, viewport!(0, 0, 0, 2))]
    #[case(viewport!(3, 0, 0, 3), 0, viewport!(0, 0, 0, 3))]
    #[case(viewport!(3, 1, 0, 3), 0, viewport!(0, 0, 0, 3))]
    #[case(viewport!(3, 2, 0, 3), 0, viewport!(0, 0, 0, 3))]
    // Three containers to one container.
    #[case(viewport!(3, 0, 0, 0), 1, viewport!(1, 0, 0, 0))]
    #[case(viewport!(3, 1, 1, 0), 1, viewport!(1, 0, 0, 0))]
    #[case(viewport!(3, 2, 2, 0), 1, viewport!(1, 0, 0, 0))]
    #[case(viewport!(3, 0, 0, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(3, 1, 1, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(3, 2, 2, 1), 1, viewport!(1, 0, 0, 1))]
    #[case(viewport!(3, 0, 0, 2), 1, viewport!(1, 0, 0, 2))]
    #[case(viewport!(3, 1, 0, 2), 1, viewport!(1, 0, 0, 2))]
    #[case(viewport!(3, 1, 1, 2), 1, viewport!(1, 0, 0, 2))]
    #[case(viewport!(3, 2, 1, 2), 1, viewport!(1, 0, 0, 2))]
    #[case(viewport!(3, 0, 0, 3), 1, viewport!(1, 0, 0, 3))]
    #[case(viewport!(3, 1, 0, 3), 1, viewport!(1, 0, 0, 3))]
    #[case(viewport!(3, 2, 0, 3), 1, viewport!(1, 0, 0, 3))]
    // Three containers to two containers.
    #[case(viewport!(3, 0, 0, 0), 2, viewport!(2, 0, 0, 0))]
    #[case(viewport!(3, 1, 1, 0), 2, viewport!(2, 1, 1, 0))]
    #[case(viewport!(3, 2, 2, 0), 2, viewport!(2, 1, 1, 0))]
    #[case(viewport!(3, 0, 0, 1), 2, viewport!(2, 0, 0, 1))]
    #[case(viewport!(3, 1, 1, 1), 2, viewport!(2, 1, 1, 1))]
    #[case(viewport!(3, 2, 2, 1), 2, viewport!(2, 1, 1, 1))]
    #[case(viewport!(3, 0, 0, 2), 2, viewport!(2, 0, 0, 2))]
    #[case(viewport!(3, 1, 0, 2), 2, viewport!(2, 1, 0, 2))]
    #[case(viewport!(3, 1, 1, 2), 2, viewport!(2, 1, 0, 2))]
    #[case(viewport!(3, 2, 1, 2), 2, viewport!(2, 1, 0, 2))]
    #[case(viewport!(3, 0, 0, 3), 2, viewport!(2, 0, 0, 3))]
    #[case(viewport!(3, 1, 0, 3), 2, viewport!(2, 1, 0, 3))]
    #[case(viewport!(3, 2, 0, 3), 2, viewport!(2, 1, 0, 3))]
    fn change_num_containers(
        #[case] mut before: Viewport,
        #[case] num_containers: usize,
        #[case] after: Viewport,
    ) {
        before.change_num_containers(num_containers);
        assert_eq!(before, after);
    }
}
