use crate::error::Result;
use crate::pack::{build_project, BuildReport};
use crate::project::Project;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::{self, Stdout};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Input,
    Project,
    BuildResult,
}

#[derive(Debug, Clone)]
enum Pending {
    None,
    New {
        step: NewStep,
        name: String,
        namespace: String,
        path: String,
    },
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewStep {
    Name,
    Namespace,
    Path,
}

pub struct App {
    screen: Screen,
    should_quit: bool,
    status: String,
    input: String,
    pending: Pending,
    menu: ListState,
    project: Option<Project>,
    last_report: Option<BuildReport>,
    cwd: PathBuf,
}

impl App {
    pub fn new() -> Self {
        let mut menu = ListState::default();
        menu.select(Some(0));
        Self {
            screen: Screen::Home,
            should_quit: false,
            status: "PackCreator — Craft-Engine 材质包 TUI".into(),
            input: String::new(),
            pending: Pending::None,
            menu,
            project: None,
            last_report: None,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let result = self.event_loop(&mut terminal);
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        result
    }

    fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.ui(f))?;
            if !event::poll(std::time::Duration::from_millis(200))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            self.handle_key(key.code)?;
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) -> Result<()> {
        if self.screen == Screen::Input {
            match code {
                KeyCode::Esc => {
                    self.pending = Pending::None;
                    self.input.clear();
                    self.screen = Screen::Home;
                    self.menu.select(Some(0));
                }
                KeyCode::Enter => self.submit_input()?,
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c) => self.input.push(c),
                _ => {}
            }
            return Ok(());
        }

        match self.screen {
            Screen::Home => match code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Down | KeyCode::Char('j') => self.menu_delta(1, 4),
                KeyCode::Up | KeyCode::Char('k') => self.menu_delta(-1, 4),
                KeyCode::Enter => self.home_select()?,
                _ => {}
            },
            Screen::Project => match code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.screen = Screen::Home;
                    self.menu.select(Some(0));
                }
                KeyCode::Down | KeyCode::Char('j') => self.menu_delta(1, 5),
                KeyCode::Up | KeyCode::Char('k') => self.menu_delta(-1, 5),
                KeyCode::Enter => self.project_select()?,
                _ => {}
            },
            Screen::BuildResult => {
                if matches!(code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
                    self.screen = Screen::Project;
                    self.menu.select(Some(0));
                }
            }
            Screen::Input => {}
        }
        Ok(())
    }

    fn menu_delta(&mut self, delta: isize, len: usize) {
        let i = self.menu.selected().unwrap_or(0) as isize;
        let n = len as isize;
        let next = (i + delta).rem_euclid(n) as usize;
        self.menu.select(Some(next));
    }

    fn home_select(&mut self) -> Result<()> {
        match self.menu.selected().unwrap_or(0) {
            0 => {
                self.pending = Pending::New {
                    step: NewStep::Name,
                    name: String::new(),
                    namespace: String::new(),
                    path: String::new(),
                };
                self.input.clear();
                self.screen = Screen::Input;
            }
            1 => {
                self.pending = Pending::Open;
                self.input.clear();
                self.screen = Screen::Input;
            }
            2 => match Project::open(&self.cwd) {
                Ok(p) => {
                    self.status = format!("已打开 {}", p.root.display());
                    self.project = Some(p);
                    self.screen = Screen::Project;
                    self.menu.select(Some(0));
                }
                Err(e) => self.status = format!("当前目录不是 Project: {e}"),
            },
            3 => self.should_quit = true,
            _ => {}
        }
        Ok(())
    }

    fn submit_input(&mut self) -> Result<()> {
        let value = self.input.trim().to_string();
        match &mut self.pending {
            Pending::New {
                step,
                name,
                namespace,
                path,
            } => match *step {
                NewStep::Name => {
                    if value.is_empty() {
                        self.status = "名称不能为空".into();
                        return Ok(());
                    }
                    *name = value;
                    *step = NewStep::Namespace;
                    self.input.clear();
                }
                NewStep::Namespace => {
                    if value.is_empty() {
                        self.status = "namespace 不能为空".into();
                        return Ok(());
                    }
                    *namespace = value;
                    *step = NewStep::Path;
                    self.input = name.clone();
                }
                NewStep::Path => {
                    if value.is_empty() {
                        self.status = "路径不能为空".into();
                        return Ok(());
                    }
                    *path = value;
                    let name = name.clone();
                    let namespace = namespace.clone();
                    let path = path.clone();
                    let root = self.cwd.join(&path);
                    match Project::create(&root, &name, &namespace) {
                        Ok(p) => {
                            self.status = format!("已创建 {}", p.root.display());
                            self.project = Some(p);
                            self.pending = Pending::None;
                            self.input.clear();
                            self.screen = Screen::Project;
                            self.menu.select(Some(0));
                        }
                        Err(e) => {
                            self.status = format!("创建失败: {e}");
                            self.pending = Pending::None;
                            self.input.clear();
                            self.screen = Screen::Home;
                        }
                    }
                }
            },
            Pending::Open => {
                let root = self.cwd.join(&value);
                match Project::open(&root) {
                    Ok(p) => {
                        self.status = format!("已打开 {}", p.root.display());
                        self.project = Some(p);
                        self.pending = Pending::None;
                        self.input.clear();
                        self.screen = Screen::Project;
                        self.menu.select(Some(0));
                    }
                    Err(e) => {
                        self.status = format!("打开失败: {e}");
                        self.pending = Pending::None;
                        self.input.clear();
                        self.screen = Screen::Home;
                    }
                }
            }
            Pending::None => {
                self.screen = Screen::Home;
            }
        }
        Ok(())
    }

    fn project_select(&mut self) -> Result<()> {
        let Some(project) = self.project.clone() else {
            return Ok(());
        };
        match self.menu.selected().unwrap_or(0) {
            0 => match build_project(&project) {
                Ok(report) => {
                    self.status = format!(
                        "构建成功 ZIP={}",
                        report.resource_pack_zip.display()
                    );
                    self.last_report = Some(report);
                    self.screen = Screen::BuildResult;
                }
                Err(e) => self.status = format!("构建失败: {e}"),
            },
            1 => {
                self.status = format!(
                    "mappings.mode={:?} font_start={} cmd_start={} offset={}",
                    project.build.mappings.mode,
                    project.build.mappings.font.codepoint_starting_value,
                    project.build.mappings.custom_model_data.starting_value,
                    project.build.mappings.font.offset_characters
                );
            }
            2 => {
                let idx = crate::config::scan_configuration(&Project::configuration_dir(
                    &project.root,
                ))?;
                let keys: Vec<_> = idx.sections.keys().cloned().collect();
                self.status = if keys.is_empty() {
                    "未扫描到已知配置节".into()
                } else {
                    format!("配置节: {}", keys.join(", "))
                };
            }
            3 => {
                let f = &project.build.pack.features;
                self.status = format!(
                    "images={} emoji={} lang={} sounds={} equipment={} paintings={} items={} blocks={} furniture={} templates={}",
                    f.images, f.emoji, f.lang, f.sounds, f.equipment, f.paintings, f.items, f.blocks, f.furniture, f.templates
                );
            }
            4 => {
                self.project = None;
                self.screen = Screen::Home;
                self.menu.select(Some(0));
            }
            _ => {}
        }
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(f.area());

        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                " PackCreator ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  Craft-Engine Resource Pack Authoring"),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        match self.screen {
            Screen::Home => self.draw_home(f, chunks[1]),
            Screen::Input => self.draw_input(f, chunks[1]),
            Screen::Project => self.draw_project(f, chunks[1]),
            Screen::BuildResult => self.draw_build(f, chunks[1]),
        }

        let status = Paragraph::new(self.status.as_str())
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("状态"));
        f.render_widget(status, chunks[2]);
    }

    fn draw_home(&mut self, f: &mut Frame, area: Rect) {
        let items = [
            "新建 Project (build.pk + src/main)",
            "打开 Project",
            "打开当前工作目录",
            "退出",
        ];
        let list = List::new(items.iter().map(|s| ListItem::new(*s)).collect::<Vec<_>>())
            .block(Block::default().borders(Borders::ALL).title("主菜单"))
            .highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, area, &mut self.menu);
    }

    fn draw_input(&self, f: &mut Frame, area: Rect) {
        let label = match &self.pending {
            Pending::New { step, .. } => match step {
                NewStep::Name => "新项目名称",
                NewStep::Namespace => "namespace (小写，如 mypack)",
                NewStep::Path => "创建目录路径 (相对 cwd，默认可用项目名)",
            },
            Pending::Open => "项目路径 (相对 cwd)",
            Pending::None => "输入",
        };
        let p = Paragraph::new(format!("{label}\n> {}_", self.input)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("输入 (Enter 确认 / Esc 取消)"),
        );
        f.render_widget(p, area);
    }

    fn draw_project(&mut self, f: &mut Frame, area: Rect) {
        let name = self
            .project
            .as_ref()
            .map(|p| {
                format!(
                    "{} ({}) @ {}",
                    p.build.project.name,
                    p.build.project.namespace,
                    p.root.display()
                )
            })
            .unwrap_or_default();
        let items = [
            "构建 (导出 CE resources + resource_pack.zip)",
            "查看映射表配置 (WHOLE)",
            "扫描 configuration 节",
            "查看特性开关",
            "关闭项目",
        ];
        let list = List::new(items.iter().map(|s| ListItem::new(*s)).collect::<Vec<_>>())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("项目: {name}")),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, area, &mut self.menu);
    }

    fn draw_build(&self, f: &mut Frame, area: Rect) {
        let text = if let Some(r) = &self.last_report {
            format!(
                "构建完成\n\nCE resources:\n  {}\n\nResource pack zip:\n  {}\n\nsections: {}\nfonts: {}  langs: {}  sounds: {}  copied: {}\n\n按 Enter 返回",
                r.ce_resources.display(),
                r.resource_pack_zip.display(),
                r.sections.join(", "),
                r.fonts_written,
                r.langs_written,
                r.sounds_written,
                r.files_copied
            )
        } else {
            "无报告".into()
        };
        let p = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title("构建结果"));
        f.render_widget(p, area);
    }
}
