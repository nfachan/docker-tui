use crate::{
    docker::{self, Container},
    input_state_machine::{Builder, InputResult, InputStateMachine},
    viewport::Viewport,
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Margin, Position, Rect},
    style::{Style, Stylize as _},
    text::Span,
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget as _,
        Widget,
    },
};
use std::{
    cmp, iter,
    time::{Duration, UNIX_EPOCH},
};
use timeago::Formatter;

pub enum MessageOut {
    Exit,
    Render,
    ToDocker(docker::MessageIn),
}

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
    style: Style,
    block_style: Style,
    line_style: Style,
    selected_line_style: Style,
    fields: Vec<ContainerField>,
    field_layouts: Vec<FieldLayout>,
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
            style: Style::default().light_blue(),
            block_style: Style::default().red(),
            line_style: Style::default(),
            selected_line_style: Style::default().reversed(),
            fields: vec![
                ContainerField::Id,
                ContainerField::Image,
                ContainerField::Command,
                ContainerField::Created,
                ContainerField::Status,
                ContainerField::Names,
            ],
            field_layouts: Vec::default(),
        }
    }
}

impl ContainerList {
    fn block(&self) -> Block<'static> {
        Block::bordered()
            .title("Containers")
            .style(self.style.patch(self.block_style))
    }

    fn handle_command(&mut self, command: Command) -> Option<MessageOut> {
        match command {
            Command::Quit => {
                return Some(MessageOut::Exit);
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
        Some(MessageOut::Render)
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> Option<MessageOut> {
        if event.kind == KeyEventKind::Press {
            match self
                .input_state_machine
                .input((event.code, event.modifiers))
            {
                InputResult::Done(command) => {
                    return self.handle_command(command);
                }
                InputResult::NeedMore => {}
                InputResult::Invalid => {
                    print!("\x07"); // ASCII BEL to STDOUT
                }
            }
        }
        None
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<MessageOut> {
        match event.kind {
            MouseEventKind::ScrollDown => {
                self.viewport.scroll_down_n_lines(3);
            }
            MouseEventKind::ScrollUp => {
                self.viewport.scroll_up_n_lines(3);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let position = Position::new(event.column, event.row);
                let inner_area = self.block().inner(self.area);
                if inner_area.contains(position) {
                    self.viewport
                        .handle_click(usize::from(position.y - inner_area.top()));
                }
            }
            _ => {
                return None;
            }
        }
        Some(MessageOut::Render)
    }

    fn handle_resize(&mut self, area: Rect) -> Option<MessageOut> {
        let block = self.block();
        let inner_area = block.inner(area);
        self.viewport
            .change_viewport_height(inner_area.height.into());
        self.area = area;

        // We need at least one column of screen space for one container field, plus one column of
        // spacing in between. If we have fewer screen columns, we drop container fields.
        let width = usize::from(inner_area.width);
        let num_fields = cmp::min(self.fields.len(), width.div_ceil(2));
        self.field_layouts = if num_fields == 0 {
            vec![]
        } else {
            Layout::horizontal(Itertools::intersperse(
                iter::repeat_n(
                    Constraint::Ratio(
                        ((width - num_fields + 1) / num_fields).try_into().unwrap(),
                        width.try_into().unwrap(),
                    ),
                    num_fields,
                ),
                Constraint::Length(1),
            ))
            .flex(Flex::Legacy)
            .split(inner_area)
            .iter()
            .step_by(2)
            .map(|area| FieldLayout {
                x: area.x,
                width: area.width,
            })
            .collect()
        };

        Some(MessageOut::Render)
    }

    pub fn handle_input_event(&mut self, event: Event) -> Option<MessageOut> {
        match event {
            Event::Key(event) => self.handle_key_event(event),
            Event::Mouse(event) => self.handle_mouse_event(event),
            Event::Resize(columns, rows) => self.handle_resize(Rect::new(0, 0, columns, rows)),
            Event::FocusGained | Event::FocusLost | Event::Paste(_) => None,
        }
    }

    pub fn handle_docker_response(&mut self, response: docker::MessageOut) -> Option<MessageOut> {
        let docker::MessageOut::GetContainers(containers) = response;
        self.containers = containers;
        self.viewport.change_num_containers(self.containers.len());
        Some(MessageOut::Render)
    }
}

#[derive(Clone, Copy)]
enum ContainerField {
    Id,
    Image,
    Command,
    Created,
    Status,
    Names,
}

impl ContainerField {
    fn format<'a>(self, container: &'a Container) -> Span<'a> {
        match self {
            Self::Id => container.id.as_deref().unwrap_or("N/A").into(),
            Self::Image => container.image.as_deref().unwrap_or("N/A").into(),
            Self::Command => container
                .command
                .as_deref()
                .map(|command| Span::from(format!("{command:?}")))
                .unwrap_or(Span::from("N/A")),
            Self::Created => match &container.created {
                None => Span::from("N/A"),
                Some(when) => {
                    let formatter = Formatter::new();
                    let when = UNIX_EPOCH + Duration::from_secs(*when as u64);
                    formatter.convert(when.elapsed().unwrap()).into()
                }
            },
            Self::Status => container.status.as_deref().unwrap_or("N/A").into(),
            Self::Names => match &container.names {
                None => "[]".into(),
                Some(names) => match &names
                    .iter()
                    .map(|name| name.trim_start_matches('/'))
                    .collect::<Vec<_>>()[..]
                {
                    [] => "[]".into(),
                    [name] => (*name).into(),
                    names => names.iter().join(", ").into(),
                },
            },
        }
    }
}

struct FieldLayout {
    x: u16,
    width: u16,
}

impl Widget for &mut ContainerList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // We may not always get a resize event before some other event that causes a redraw.
        if area != self.area {
            self.handle_resize(area);
        }

        let block = self.block();
        if self.containers.is_empty() {
            // Handle special case of no containers.
            Paragraph::new("no containers")
                .block(block)
                .render(area, buf);
        } else {
            let inner_area = block.inner(area);

            // Render the block.
            block.render(area, buf);

            // Render the items.
            for (row_index, (container_index, selected)) in
                self.viewport.select_for_render().enumerate()
            {
                let row_area = Rect::new(
                    inner_area.x,
                    inner_area
                        .y
                        .saturating_add(row_index.try_into().unwrap_or(u16::MAX)),
                    inner_area.width,
                    1,
                );
                let mut row_style = self.style.patch(self.line_style);
                if selected {
                    row_style = row_style.patch(self.selected_line_style)
                }
                buf.set_style(row_area, row_style);
                for (column, field) in self.field_layouts.iter().zip(self.fields.iter()) {
                    field.format(&self.containers[container_index]).render(
                        Rect::new(column.x, row_area.y, column.width, row_area.height),
                        buf,
                    );
                }
            }

            // Possibly render a scrollbar.
            if let Some(scrollbar_parameters) = self.viewport.scrollbar() {
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .style(self.style.patch(self.block_style))
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
