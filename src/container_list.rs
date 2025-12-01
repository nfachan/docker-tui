use crate::{
    docker::Container,
    input_state_machine::{Builder, InputResult, InputStateMachine},
    viewport::Viewport,
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Position, Rect},
    style::{Modifier, Stylize as _},
    text::Span,
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget as _,
        Widget,
    },
};
use std::ops::ControlFlow;

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
    area: Rect,
}

impl Default for ContainerList {
    fn default() -> Self {
        let input_state_machine = Builder::default()
            .binding([(KeyCode::Char('q'), KeyModifiers::NONE)], Command::Quit)
            .unwrap()
            .binding([(KeyCode::Esc, KeyModifiers::NONE)], Command::Quit)
            .unwrap()
            .binding(
                [(KeyCode::Char('k'), KeyModifiers::NONE)],
                Command::MoveSelectionUpOneLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Up, KeyModifiers::NONE)],
                Command::MoveSelectionUpOneLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('j'), KeyModifiers::NONE)],
                Command::MoveSelectionDownOneLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Down, KeyModifiers::NONE)],
                Command::MoveSelectionDownOneLine,
            )
            .unwrap()
            .binding(
                [
                    (KeyCode::Char('g'), KeyModifiers::NONE),
                    (KeyCode::Char('g'), KeyModifiers::NONE),
                ],
                Command::MoveSelectionToFirstLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Home, KeyModifiers::NONE)],
                Command::MoveSelectionToFirstLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('G'), KeyModifiers::SHIFT)],
                Command::MoveSelectionToLastLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::End, KeyModifiers::NONE)],
                Command::MoveSelectionToLastLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('y'), KeyModifiers::CONTROL)],
                Command::ScrollUpOneLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('u'), KeyModifiers::CONTROL)],
                Command::ScrollUpHalfPage,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('b'), KeyModifiers::CONTROL)],
                Command::ScrollUpFullPage,
            )
            .unwrap()
            .binding(
                [(KeyCode::PageUp, KeyModifiers::NONE)],
                Command::ScrollUpFullPage,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('e'), KeyModifiers::CONTROL)],
                Command::ScrollDownOneLine,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('d'), KeyModifiers::CONTROL)],
                Command::ScrollDownHalfPage,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('f'), KeyModifiers::CONTROL)],
                Command::ScrollDownFullPage,
            )
            .unwrap()
            .binding(
                [(KeyCode::PageDown, KeyModifiers::NONE)],
                Command::ScrollDownFullPage,
            )
            .unwrap()
            .binding(
                [
                    (KeyCode::Char('z'), KeyModifiers::NONE),
                    (KeyCode::Char('t'), KeyModifiers::NONE),
                ],
                Command::ScrollSelectionToTop,
            )
            .unwrap()
            .binding(
                [
                    (KeyCode::Char('z'), KeyModifiers::NONE),
                    (KeyCode::Char('m'), KeyModifiers::NONE),
                ],
                Command::ScrollSelectionToMiddle,
            )
            .unwrap()
            .binding(
                [
                    (KeyCode::Char('z'), KeyModifiers::NONE),
                    (KeyCode::Char('b'), KeyModifiers::NONE),
                ],
                Command::ScrollSelectionToBottom,
            )
            .unwrap()
            .build();
        Self {
            containers: Vec::default(),
            viewport: Viewport::default(),
            input_state_machine,
            area: Rect::ZERO,
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
            Command::ScrollUpHalfPage => self.viewport.scroll_up_n_lines(self.viewport.half_page()),
            Command::ScrollUpFullPage => {
                self.viewport.scroll_up_n_lines(self.viewport.full_page());
            }
            Command::ScrollDownOneLine => {
                self.viewport.scroll_down_n_lines(1);
            }
            Command::ScrollDownHalfPage => {
                self.viewport.scroll_down_n_lines(self.viewport.half_page());
            }
            Command::ScrollDownFullPage => {
                self.viewport.scroll_down_n_lines(self.viewport.full_page());
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

    pub fn handle_key_event(&mut self, event: KeyEvent) -> ControlFlow<()> {
        match event.kind {
            KeyEventKind::Press => {
                match self
                    .input_state_machine
                    .input((event.code, event.modifiers))
                {
                    InputResult::Done(command) => self.handle_command(command),
                    InputResult::NeedMore => ControlFlow::Continue(()),
                    InputResult::Invalid => {
                        print!("\x07"); // ASCII BEL to STDOUT
                        ControlFlow::Continue(())
                    }
                }
            }
            _ => ControlFlow::Continue(()),
        }
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::ScrollDown => {
                self.viewport.scroll_down_n_lines(3);
            }
            MouseEventKind::ScrollUp => {
                self.viewport.scroll_up_n_lines(3);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let position = Position::new(event.column, event.row);
                let inner_area = Self::block().inner(self.area);
                if inner_area.contains(position) {
                    self.viewport
                        .handle_click(usize::from(position.y - inner_area.top()));
                }
            }
            _ => {}
        }
    }

    pub fn handle_resize(&mut self, area: Rect) {
        let viewport_height = Self::block().inner(area).height;
        self.viewport.change_viewport_height(viewport_height.into());
        self.area = area;
    }

    pub fn handle_containers(&mut self, containers: Vec<Container>) {
        self.containers = containers;
        self.viewport.change_num_containers(self.containers.len());
    }
}

fn format_container(container: &Container, is_selected: bool) -> Span<'static> {
    let name = match &container.names {
        None => "[]".to_string(),
        Some(names) => match &names
            .iter()
            .map(|name| name.trim_start_matches('/'))
            .collect::<Vec<_>>()[..]
        {
            [] => "[]".to_string(),
            [name] => name.to_string(),
            names => names.iter().fold("".to_string(), |accum, name| {
                if accum.is_empty() {
                    name.to_string()
                } else {
                    format!("{accum}, {name}")
                }
            }),
        },
    };
    let status = container.status.as_deref().unwrap_or("N/A");
    let result = Span::from(format!("{name} - {status}"));
    if is_selected {
        result.add_modifier(Modifier::REVERSED)
    } else {
        result
    }
}

impl Widget for &mut ContainerList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // We may not always get a resize event before some other event that causes a redraw.
        if area != self.area {
            self.handle_resize(area);
        }

        let block = ContainerList::block();
        if self.containers.is_empty() {
            // Handle special case of no containers.
            Paragraph::new("no containers")
                .block(block)
                .render(area, buf);
        } else {
            let inner_area = block.inner(area);

            // Select the items to render.
            let items = self
                .viewport
                .select_for_render()
                .map(|(i, selected)| format_container(&self.containers[i], selected));

            // Render the block.
            block.render(area, buf);

            // Render the items.
            for (i, item) in items.enumerate() {
                item.render(
                    Rect::new(
                        inner_area.x,
                        inner_area
                            .y
                            .saturating_add(i.try_into().unwrap_or(u16::MAX)),
                        inner_area.width,
                        1,
                    ),
                    buf,
                );
            }

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
