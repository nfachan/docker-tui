use crate::{docker::Container, viewport::Viewport};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Margin,
    prelude::*,
    widgets::{Block, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::{cmp, ops::ControlFlow};

#[derive(Default)]
pub struct ContainerList {
    containers: Vec<Container>,
    viewport: Viewport,
}

impl ContainerList {
    fn block() -> Block<'static> {
        Block::bordered().title("Containers")
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> ControlFlow<()> {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char('q') | KeyCode::Esc, _) => {
                return ControlFlow::Break(());
            }
            (KeyCode::Char('k') | KeyCode::Up, _) => {
                self.viewport.move_selection_up_one_line();
            }
            (KeyCode::Char('j') | KeyCode::Down, _) => {
                self.viewport.move_selection_down_one_line();
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.viewport.scroll_up_n_lines(1);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_up_n_lines(cmp::max(1, self.viewport.height() / 2));
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_up_n_lines(cmp::max(1, self.viewport.height().saturating_sub(2)));
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.viewport.scroll_down_n_lines(1);
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_down_n_lines(cmp::max(1, self.viewport.height() / 2));
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_down_n_lines(cmp::max(1, self.viewport.height().saturating_sub(2)));
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    pub fn handle_resize(&mut self, height: u16) {
        let viewport_height = Self::block().inner(Rect::new(0, 0, 1, height)).height;
        self.viewport.change_viewport_height(viewport_height.into());
    }

    pub fn handle_containers(&mut self, containers: Vec<Container>) {
        self.containers = containers;
        self.viewport.change_num_containers(self.containers.len());
    }
}

impl Widget for &mut ContainerList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = ContainerList::block();

        if self.containers.is_empty() {
            // Handle special case of no containers.
            Paragraph::new("no containers")
                .block(block)
                .render(area, buf);
        } else {
            // We may not always get a resize event before some other event that causes a redraw.
            if self.viewport.height() != usize::from(block.inner(area).height) {
                self.handle_resize(area.height);
            }

            // Select the subset of list items we are going to render.
            let items = self.viewport.select_for_render(
                &self.containers[..],
                |container| ListItem::new(format!("  {} - {}", container.name, container.status)),
                |container| {
                    ListItem::new(format!("> {} - {}", container.name, container.status))
                        .add_modifier(Modifier::REVERSED)
                },
            );

            // Render the list.
            Widget::render(List::new(items).block(block), area, buf);

            // Possibly render a scrollbar.
            if self.viewport.top() != 0 || self.viewport.height() < self.containers.len() {
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(None)
                    .render(
                        area.inner(Margin {
                            vertical: 1,
                            horizontal: 0,
                        }),
                        buf,
                        &mut ScrollbarState::default()
                            .content_length(self.containers.len() + 1 - self.viewport.height())
                            .position(self.viewport.top()),
                    );
            }
        }
    }
}
