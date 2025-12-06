use crate::{
    docker::{self, Container},
    input_state_machine::{Builder, InputResult, InputStateMachine},
    timer,
    viewport::{ScrollbarParameters, Viewport},
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Margin, Position, Rect},
    style::{Style, Stylize as _},
    text::Span,
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget as _,
        Widget,
    },
};
use std::{
    cmp,
    io::{self, Write as _},
    iter,
    time::{Duration, UNIX_EPOCH},
};
use timeago::Formatter;

pub enum MessageOut {
    Exit,
    Render,
    ToDocker(docker::MessageIn),
    ToTimer(timer::MessageIn),
}

#[derive(Clone, Copy)]
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
    ToggleBlock,
    ToggleScrollbar,
    ToggleAllContainers,
}

#[derive(Clone, Copy)]
enum HaveScrollbar {
    IfNecesssary,
    Never,
    Always,
}

impl HaveScrollbar {
    fn next(self) -> Self {
        match self {
            Self::IfNecesssary => Self::Never,
            Self::Never => Self::Always,
            Self::Always => Self::IfNecesssary,
        }
    }
}

pub struct ContainerList {
    containers: Vec<Container>,
    viewport: Viewport,
    next_timer_id: timer::Id,
    refresh_timer: timer::Id,
    input_timer: Option<timer::Id>,
    input_state_machine: InputStateMachine<(KeyCode, KeyModifiers), Command>,
    click_map: Option<Rect>,
    style: Style,
    block_style: Style,
    line_style: Style,
    selected_line_style: Style,
    fields: Vec<ContainerField>,
    have_block: bool,
    have_scrollbar: HaveScrollbar,
    all_containers: bool,
}

impl ContainerList {
    const MULTIKEY_INPUT_TIMEOUT: Duration = Duration::from_secs(3);
    const REFRESH_INTERVAL: Duration = Duration::from_secs(10);

    pub fn new() -> (Self, Vec<MessageOut>) {
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
            .binding(
                [(KeyCode::Char('b'), KeyModifiers::NONE)],
                Command::ToggleBlock,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('S'), KeyModifiers::SHIFT)],
                Command::ToggleScrollbar,
            )
            .unwrap()
            .binding(
                [(KeyCode::Char('a'), KeyModifiers::NONE)],
                Command::ToggleAllContainers,
            )
            .unwrap()
            .build();

        let mut next_timer_id = timer::Id::default();
        let refresh_timer = next_timer_id;
        next_timer_id += 1;
        let all_containers = true;

        (
            Self {
                containers: Vec::default(),
                viewport: Viewport::default(),
                next_timer_id,
                refresh_timer,
                input_timer: None,
                input_state_machine,
                click_map: None,
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
                have_block: true,
                have_scrollbar: HaveScrollbar::IfNecesssary,
                all_containers,
            },
            vec![
                MessageOut::ToDocker(docker::MessageIn::GetContainers {
                    all: all_containers,
                }),
                MessageOut::ToTimer(timer::MessageIn::Start(
                    refresh_timer,
                    Self::REFRESH_INTERVAL,
                )),
                MessageOut::Render,
            ],
        )
    }

    fn bell() {
        print!("\x07");
        let _ = io::stdout().flush();
    }

    fn handle_command(&mut self, command: Command) -> Vec<MessageOut> {
        match command {
            Command::Quit => {
                return vec![MessageOut::Exit];
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
            Command::ToggleBlock => {
                self.have_block = !self.have_block;
            }
            Command::ToggleScrollbar => {
                self.have_scrollbar = self.have_scrollbar.next();
            }
            Command::ToggleAllContainers => {
                self.all_containers = !self.all_containers;
                return vec![MessageOut::ToDocker(docker::MessageIn::GetContainers {
                    all: self.all_containers,
                })];
            }
        }
        vec![MessageOut::Render]
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> Vec<MessageOut> {
        let mut messages_out = vec![];
        if event.kind == KeyEventKind::Press {
            if let Some(id) = self.input_timer.take() {
                messages_out.push(MessageOut::ToTimer(timer::MessageIn::Cancel(id)));
            }
            match self
                .input_state_machine
                .input((event.code, event.modifiers))
            {
                InputResult::Done(command) => {
                    return self.handle_command(command);
                }
                InputResult::NeedMore => {
                    let input_timer = self.next_timer_id;
                    self.next_timer_id += 1;
                    self.input_timer = Some(input_timer);
                    messages_out.push(MessageOut::ToTimer(timer::MessageIn::Start(
                        input_timer,
                        Self::MULTIKEY_INPUT_TIMEOUT,
                    )));
                }
                InputResult::Invalid => {
                    Self::bell();
                }
            }
        }
        messages_out
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) -> Vec<MessageOut> {
        match event.kind {
            MouseEventKind::ScrollDown => {
                self.viewport.scroll_down_n_lines(3);
            }
            MouseEventKind::ScrollUp => {
                self.viewport.scroll_up_n_lines(3);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let position = Position::new(event.column, event.row);
                if let Some(area) = &self.click_map
                    && area.contains(position)
                {
                    self.viewport
                        .handle_click(usize::from(position.y - area.top()));
                }
            }
            _ => {
                return vec![];
            }
        }
        vec![MessageOut::Render]
    }

    pub fn handle_input_event(&mut self, event: Event) -> Vec<MessageOut> {
        match event {
            Event::Key(event) => self.handle_key_event(event),
            Event::Mouse(event) => self.handle_mouse_event(event),
            Event::Resize(_, _) => vec![MessageOut::Render],
            Event::FocusGained | Event::FocusLost | Event::Paste(_) => vec![],
        }
    }

    pub fn handle_docker_response(&mut self, response: docker::MessageOut) -> Vec<MessageOut> {
        let docker::MessageOut::GetContainers(containers) = response;
        self.containers = containers;
        self.viewport.change_num_containers(self.containers.len());
        vec![MessageOut::Render]
    }

    pub fn handle_timer_event(
        &mut self,
        timer::MessageOut::TimerExpired(id): timer::MessageOut,
    ) -> Vec<MessageOut> {
        if let Some(input_id) = &self.input_timer
            && *input_id == id
        {
            Self::bell();
            self.input_state_machine.reset();
            self.input_timer = None;
            vec![]
        } else if self.refresh_timer == id {
            self.refresh_timer = self.next_timer_id;
            self.next_timer_id += 1;
            vec![
                MessageOut::ToDocker(docker::MessageIn::GetContainers {
                    all: self.all_containers,
                }),
                MessageOut::ToTimer(timer::MessageIn::Start(
                    self.refresh_timer,
                    Self::REFRESH_INTERVAL,
                )),
            ]
        } else {
            vec![]
        }
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
        let block = Block::bordered()
            .title(if self.all_containers {
                "All Containers"
            } else {
                "Running Containers"
            })
            .title_alignment(Alignment::Center)
            .style(self.style.patch(self.block_style));

        if self.containers.is_empty() {
            // Handle special case of no containers.
            let mut paragraph = Paragraph::new("[no containers to show]")
                .alignment(Alignment::Center)
                .style(self.style);
            if self.have_block {
                paragraph = paragraph.block(block);
            }
            paragraph.render(area, buf);
            self.click_map = None;
            return;
        }

        let list_height = if self.have_block {
            block.inner(area).height
        } else {
            area.height
        };
        self.viewport.change_viewport_height(list_height.into());
        let mut scrollbar_parameters = match self.have_scrollbar {
            HaveScrollbar::Never => None,
            HaveScrollbar::IfNecesssary => self.viewport.scrollbar(),
            HaveScrollbar::Always => self.viewport.scrollbar().or(Some(ScrollbarParameters {
                total_items: 1,
                top_item: 1,
            })),
        };
        let list_area;
        let scrollbar_area;
        if self.have_block {
            list_area = block.inner(area);
            scrollbar_area = area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            });
        } else if scrollbar_parameters.is_some() && area.width >= 2 {
            list_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width - 1,
                height: area.height,
            };
            scrollbar_area = Rect {
                x: area.x.saturating_add(area.width - 1),
                y: area.y,
                width: 1,
                height: area.height,
            };
        } else {
            scrollbar_parameters = None;
            list_area = area;
            scrollbar_area = Rect::ZERO;
        }
        self.click_map = Some(list_area);
        assert_eq!(list_height, list_area.height);

        // We need at least one column of screen space for one container field, plus one column of
        // spacing in between. If we have fewer screen columns, we drop container fields.
        let list_width = usize::from(list_area.width);
        let num_fields = cmp::min(self.fields.len(), list_width.div_ceil(2));
        let field_layouts = if num_fields == 0 {
            vec![]
        } else {
            Layout::horizontal(Itertools::intersperse(
                iter::repeat_n(
                    Constraint::Ratio(
                        ((list_width - num_fields + 1) / num_fields)
                            .try_into()
                            .unwrap(),
                        list_width.try_into().unwrap(),
                    ),
                    num_fields,
                ),
                Constraint::Length(1),
            ))
            .flex(Flex::Legacy)
            .split(list_area)
            .iter()
            .step_by(2)
            .map(|area| FieldLayout {
                x: area.x,
                width: area.width,
            })
            .collect()
        };

        // Possibly render the block.
        if self.have_block {
            block.render(area, buf);
        }

        // Render the items.
        for (row_index, (container_index, selected)) in
            self.viewport.select_for_render().enumerate()
        {
            let row_area = Rect::new(
                list_area.x,
                list_area
                    .y
                    .saturating_add(row_index.try_into().unwrap_or(u16::MAX)),
                list_area.width,
                1,
            );
            let mut row_style = self.style.patch(self.line_style);
            if selected {
                row_style = row_style.patch(self.selected_line_style)
            }
            buf.set_style(row_area, row_style);
            for (column, field) in field_layouts.iter().zip(self.fields.iter()) {
                field.format(&self.containers[container_index]).render(
                    Rect::new(column.x, row_area.y, column.width, row_area.height),
                    buf,
                );
            }
        }

        // Possibly render a scrollbar.
        if let Some(scrollbar_parameters) = scrollbar_parameters {
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(self.style.patch(self.block_style))
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(None)
                .render(
                    scrollbar_area,
                    buf,
                    &mut ScrollbarState::default()
                        .content_length(scrollbar_parameters.total_items)
                        .position(scrollbar_parameters.top_item),
                );
        }
    }
}
