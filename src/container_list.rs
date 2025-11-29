use crate::{
    docker::Container,
    input_state_machine::{InputStateMachine, InputStateMachineBuilder, InputStateMachineResult},
    viewport::Viewport,
};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Margin,
    prelude::*,
    widgets::{Block, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::{cmp, ops::ControlFlow};

#[derive(Copy, Clone)]
enum Command {
    Quit,
    MoveSelectionUpOneLine,
    MoveSelectionDownOneLine,
    MoveSelectionToFirstLine,
    MoveSelectionToLastLine,
    ScrollUpOneLine,
    ScrollUpHalfPage,
    ScrollUpFullPage,
    ScrollDownOneLine,
    ScrollDownHalfPage,
    ScrollDownFullPage,
    ScrollSelectionToTop,
    ScrollSelectionToMiddle,
    ScrollSelectionToBottom,
}

pub struct ContainerList {
    containers: Vec<Container>,
    viewport: Viewport,
    input_state_machine: InputStateMachine<(KeyCode, KeyModifiers), Command>,
}

impl Default for ContainerList {
    fn default() -> Self {
        let input_state_machine = InputStateMachineBuilder::default()
            .binding((KeyCode::Char('q'), KeyModifiers::NONE), Command::Quit)
            .binding((KeyCode::Esc, KeyModifiers::NONE), Command::Quit)
            .binding(
                (KeyCode::Char('k'), KeyModifiers::NONE),
                Command::MoveSelectionUpOneLine,
            )
            .binding(
                (KeyCode::Up, KeyModifiers::NONE),
                Command::MoveSelectionUpOneLine,
            )
            .binding(
                (KeyCode::Char('j'), KeyModifiers::NONE),
                Command::MoveSelectionDownOneLine,
            )
            .binding(
                (KeyCode::Down, KeyModifiers::NONE),
                Command::MoveSelectionDownOneLine,
            )
            .multikey_binding(
                [
                    (KeyCode::Char('g'), KeyModifiers::NONE),
                    (KeyCode::Char('g'), KeyModifiers::NONE),
                ],
                Command::MoveSelectionToFirstLine,
            )
            .binding(
                (KeyCode::Char('G'), KeyModifiers::SHIFT),
                Command::MoveSelectionToLastLine,
            )
            .binding(
                (KeyCode::Char('y'), KeyModifiers::CONTROL),
                Command::ScrollUpOneLine,
            )
            .binding(
                (KeyCode::Char('u'), KeyModifiers::CONTROL),
                Command::ScrollUpHalfPage,
            )
            .binding(
                (KeyCode::Char('b'), KeyModifiers::CONTROL),
                Command::ScrollUpFullPage,
            )
            .binding(
                (KeyCode::Char('e'), KeyModifiers::CONTROL),
                Command::ScrollDownOneLine,
            )
            .binding(
                (KeyCode::Char('d'), KeyModifiers::CONTROL),
                Command::ScrollDownHalfPage,
            )
            .binding(
                (KeyCode::Char('f'), KeyModifiers::CONTROL),
                Command::ScrollDownFullPage,
            )
            .multikey_binding(
                [
                    (KeyCode::Char('z'), KeyModifiers::NONE),
                    (KeyCode::Char('t'), KeyModifiers::NONE),
                ],
                Command::ScrollSelectionToTop,
            )
            .multikey_binding(
                [
                    (KeyCode::Char('z'), KeyModifiers::NONE),
                    (KeyCode::Char('m'), KeyModifiers::NONE),
                ],
                Command::ScrollSelectionToMiddle,
            )
            .multikey_binding(
                [
                    (KeyCode::Char('z'), KeyModifiers::NONE),
                    (KeyCode::Char('b'), KeyModifiers::NONE),
                ],
                Command::ScrollSelectionToBottom,
            )
            .build();
        Self {
            containers: Vec::default(),
            viewport: Viewport::default(),
            input_state_machine,
        }
    }
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
            Command::MoveSelectionToFirstLine => {
                self.viewport.move_selection_to_first_line();
            }
            Command::MoveSelectionToLastLine => {
                self.viewport.move_selection_to_last_line();
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
            Command::ScrollSelectionToTop => {
                self.viewport.scroll_selection_to_top();
            }
            Command::ScrollSelectionToMiddle => {
                self.viewport.scroll_selection_to_middle();
            }
            Command::ScrollSelectionToBottom => {
                self.viewport.scroll_selection_to_bottom();
            }
        }
        ControlFlow::Continue(())
    }

    pub fn handle_key_event(&mut self, key_event: (KeyCode, KeyModifiers)) -> ControlFlow<()> {
        match self.input_state_machine.input(key_event) {
            InputStateMachineResult::Done(command) => self.handle_command(command),
            InputStateMachineResult::NeedMore => ControlFlow::Continue(()),
            InputStateMachineResult::Invalid => {
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
