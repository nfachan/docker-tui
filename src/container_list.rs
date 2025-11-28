use crate::{docker::Container, viewport::Viewport};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Margin,
    prelude::*,
    widgets::{Block, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::{cmp, ops::ControlFlow};

enum Command {
    Quit,
    MoveSelectionUpOneLine,
    MoveSelectionDownOneLine,
    ScrollUpOneLine,
    ScrollUpHalfPage,
    ScrollUpFullPage,
    ScrollDownOneLine,
    ScrollDownHalfPage,
    ScrollDownFullPage,
}

#[derive(Default)]
pub struct ContainerList {
    containers: Vec<Container>,
    viewport: Viewport,
}

impl ContainerList {
    fn block() -> Block<'static> {
        Block::bordered().title("Containers")
    }

    fn handle_command(&mut self, command: Command) -> ControlFlow<()> {
        match command {
            Command::Quit => {
                return ControlFlow::Break(());
            }
            Command::MoveSelectionUpOneLine => {
                self.viewport.move_selection_up_one_line();
            }
            Command::MoveSelectionDownOneLine => {
                self.viewport.move_selection_down_one_line();
            }
            Command::ScrollUpOneLine => {
                self.viewport.scroll_up_n_lines(1);
            }
            Command::ScrollUpHalfPage => {
                self.viewport
                    .scroll_up_n_lines(cmp::max(1, self.viewport.height() / 2));
            }
            Command::ScrollUpFullPage => {
                self.viewport
                    .scroll_up_n_lines(cmp::max(1, self.viewport.height().saturating_sub(2)));
            }
            Command::ScrollDownOneLine => {
                self.viewport.scroll_down_n_lines(1);
            }
            Command::ScrollDownHalfPage => {
                self.viewport
                    .scroll_down_n_lines(cmp::max(1, self.viewport.height() / 2));
            }
            Command::ScrollDownFullPage => {
                self.viewport
                    .scroll_down_n_lines(cmp::max(1, self.viewport.height().saturating_sub(2)));
            }
        }
        ControlFlow::Continue(())
    }

    pub fn handle_key_event(&mut self, key_event: (KeyCode, KeyModifiers)) -> ControlFlow<()> {
        match key_event {
            (KeyCode::Char('q') | KeyCode::Esc, KeyModifiers::NONE) => {
                self.handle_command(Command::Quit)
            }
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                self.handle_command(Command::MoveSelectionUpOneLine)
            }
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                self.handle_command(Command::MoveSelectionDownOneLine)
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.handle_command(Command::ScrollUpOneLine)
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.handle_command(Command::ScrollUpHalfPage)
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.handle_command(Command::ScrollUpFullPage)
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.handle_command(Command::ScrollDownOneLine)
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.handle_command(Command::ScrollDownHalfPage)
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.handle_command(Command::ScrollDownFullPage)
            }
            _ => {
                print!("\x07"); // ASCII BEL to STDOUT
                ControlFlow::Continue(())
            }
        }
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
            if let Some(scrollbar_parameters) = self.viewport.scrollbar() {
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
                            .content_length(scrollbar_parameters.total_items)
                            .position(scrollbar_parameters.top_item),
                    );
            }
        }
    }
}
