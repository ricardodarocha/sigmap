//! Simulador de processadores Multijanelas em Rust
//! iced é a biblioteca de user interface ICED UI
//! Arquitetura ELM, struct App possui o Estado do Processador
//! impl App {} adiciona comportamento POO ao App
//! fn new() inicializa o app
//! fn update() atualiza o estado com base em messages 
//! fn view() renderiza o estado
use iced::time::{self, Instant, milliseconds};
use iced::widget::canvas::{self, Canvas, Geometry, Path};
use iced::border::Radius;
use iced::widget::{
    Column, button, column, container, operation, progress_bar, row, scrollable, slider, space, text, text_input,
};
use iced::{
    Background, Center, Color, Element, Fill, Point, Rectangle, Renderer, Size, Subscription, Task,
    Theme, keyboard, mouse, window,
};

use std::collections::BTreeMap;

const GRID_COLS: usize = 24;
const GRID_ROWS: usize = 16;

fn coord_to_index(col: usize, row: usize) -> usize {
    col * GRID_ROWS + row
}

// Cores disponíveis
const _GRID_CODES: u64 = 6;
const NULL: u8 = 99;

const PID_MIN: u32 = 1234;
const PID_MAX: u32 = 9999;

fn main() -> iced::Result {
    iced::daemon(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title(App::title)
        .theme(App::theme)
        .run()
}

// ───────────────────────────── janelas ─────────────────────────────

//Os diferentes tipos de janelas do projeto
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Menu,
    Tasks,
    Detail,
    Grid,
    Register,
}

impl Kind {
    const ALL: [Kind; 4] = [Kind::Menu, Kind::Tasks, Kind::Detail, Kind::Grid];

    fn title(self) -> &'static str {
        match self {
            Kind::Menu => "Menu",
            Kind::Tasks => "F4 - Lista de Processos",
            Kind::Detail => "F5 - Detalhes do PID",
            Kind::Grid => "F6 - Memória",
            Kind::Register => "Novo Processo",
        }
    }

    fn settings(self) -> window::Settings {
        let (size, position) = match self {
            Kind::Menu => (Size::new(360.0, 260.0), Point::new(20.0, 20.0)),
            Kind::Tasks => (Size::new(520.0, 480.0), Point::new(400.0, 20.0)),
            Kind::Detail => (Size::new(380.0, 520.0), Point::new(940.0, 20.0)),
            Kind::Grid => (Size::new(360.0, 340.0), Point::new(20.0, 320.0)),
            Kind::Register => (Size::new(380.0, 520.0), Point::new(440.0, 120.0)),
        };

        window::Settings {
            size,
            position: window::Position::Specific(position),
            ..window::Settings::default()
        }
    }
}

fn open_window(kind: Kind) -> Task<Message> {
    let (_, open) = window::open(kind.settings());

    open.map(move |id| Message::WindowOpened(id, kind))
}

// ───────────────────────────── dados ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcState {
    Novo,
    Running,
    Ready,
    Blocked,
    Suspend,
    Terminated,
}

impl ProcState {
    fn label(self) -> &'static str {
        match self {
            ProcState::Novo => "Novo",
            ProcState::Running => "Executando",
            ProcState::Ready => "Pronto",
            ProcState::Blocked => "Espera",
            ProcState::Suspend => "[Suspenso]",
            ProcState::Terminated => "Finalizando...",
        }
    }
    fn _translate(self) -> &'static str {
        match self {
            ProcState::Novo => "New",
            ProcState::Running => "Running",
            ProcState::Ready => "Ready",
            ProcState::Blocked => "Blocked",
            ProcState::Suspend => "[Suspended]",
            ProcState::Terminated => "Terminated...",
        }
    }
}

#[derive(Debug, Clone)]
struct Process {
    pid: u32,
    code: u8, // índice da cor (00 green, 03 blue, ...)
    priority: u8,
    name: String,
    state: ProcState,
    celulas: u8,
    memory: MemoryBlock,
}

impl Process {
    fn new(pid: u32, code: u8, priority: u8, name: &str, state: ProcState, celulas: u8, memory: MemoryBlock) -> Self {
        Self {
            pid,
            code,
            priority,
            name: name.to_string(),
            state,
            celulas,
            memory,
        }
    }

    fn color(&self) -> Color {
        palette(self.code)
    }
}

fn palette(code: u8) -> Color {
    match code % 6 {
        0 => Color::from_rgb8(0x2E, 0xCC, 0x71), // green
        1 => Color::from_rgb8(0xE7, 0x4C, 0x3C), // red
        2 => Color::from_rgb8(0xE6, 0x7E, 0x22), // orange
        3 => Color::from_rgb8(0x34, 0x98, 0xDB), // blue
        4 => Color::from_rgb8(0x9B, 0x59, 0xB6), // purple
        _ => Color::from_rgb8(0xF1, 0xC4, 0x0F), // yellow
    }
}

fn color_name(code: u8) -> &'static str {
    match code % 6 {
        0 => "green",
        1 => "red",
        2 => "orange",
        3 => "blue",
        4 => "purple",
        _ => "yellow",
    }
}

/// Cor de cada célula da grade: 00 = green, 03 = blue, os demais = vazio.
fn grid_color(code: u8) -> Option<Color> {
    if code == NULL {
        None
    } else {
        Some(palette(code))
    } 
}

fn background() -> Color {
    Color::from_rgb8(0x40, 0x44, 0x4B)
}

fn empty_cell() -> Color {
    Color::from_rgb8(0x4A, 0x4E, 0x56)
}

/// Gerador xorshift simples (evita depender da crate `rand`).
struct Rng(u64);

impl Rng {
    fn from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);

        Self(nanos | 1)
    }

    fn below(&mut self, n: u64) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;

        x.wrapping_mul(0x2545_F491_4F6C_DD1D) % n
    }
}

// ───────────────────────────── aplicação ─────────────────────────────

/// Rascunho do formulário "Novo processo".
#[derive(Default)]
struct Draft {
    name: String,
    code: u8,
    priority: u8,     // 0..=15
    pid: Option<u32>, // gerado automaticamente
    /// O usuário informa o número de células de memória o processo consome
    celulas: u8,
}

struct App {
    windows: BTreeMap<window::Id, Kind>,
    processes: Vec<Process>,
    selected: Option<u32>, // PID selecionado
    focus: Option<u32>, // PID clicado
    draft: Draft,
    // next_pid: u32,
    rng: Rng,
    /// O vetor contém as cores de cada célula de memória
    grid: Vec<u8>, 
    interval_ms: f32, // "t": tempo entre PIDs (e fim da progressbar)
    elapsed_ms: f32,
    last_tick: Instant,
    detail_cache: canvas::Cache,
    grid_cache: canvas::Cache,
}

#[derive(Debug, Clone)]
enum Message {
    Open(Kind),
    WindowOpened(window::Id, Kind),
    WindowClosed(window::Id),
    Tick(Instant),
    Focus(u32),
    DraftName(String),
    DraftColor(u8),
    DraftPriority(u8),
    DraftCelulas(u8),
    RegeneratePid,
    SaveProcess,
    CancelRegister,
    StopFocused,
    RemoveFocused,
    IntervalChanged(f32),
}

#[derive(Debug, Clone)]
struct MemoryBlock {
    x: usize,
    y: usize,
    // direction: Direction,
    length: usize,
    code: u8,
}

impl MemoryBlock {
    fn new(coord: (usize, usize,), length: usize, code: u8 ) -> Self {
        let (x, y) = coord;
        Self { x, y, length, code, }
    }

    fn offset(&self, delta: usize, length: usize, color: u8) -> Self {
        Self {
            x: delta / GRID_ROWS,
            y: delta % GRID_ROWS,
            length,
            code: color,
        }
    }
}

impl App {
    const COR0: u8 = 0;
    const COR1: u8 = 1;
    const COR2: u8 = 2;
    const COR3: u8 = 3;
    const COR4: u8 = 4;
    const COR5: u8 = 5;

    fn new() -> (Self, Task<Message>) {

        let mem_shape = MemoryBlock::new((0, 0), 2, Self::COR0);

        let mem1 = mem_shape.offset(0, 2, Self::COR0);
        let mem2 = mem_shape.offset(2, 2, Self::COR3);
        let mem3 = mem_shape.offset(4, 8, Self::COR1);
        let mem4 = mem_shape.offset(12, 8, Self::COR4);
        let mem5 = mem_shape.offset(20, 2, Self::COR2);
        let mem6 = mem_shape.offset(36, 16, Self::COR5);
        let processes = vec![
            Process::new(1204, Self::COR0, 0, "systemd", ProcState::Running, 2, mem1),
            Process::new(1310, Self::COR3, 0, "sshd", ProcState::Ready, 2, mem2),
            Process::new(1877, Self::COR1, 0, "nginx", ProcState::Ready, 8, mem3),
            Process::new(2045, Self::COR4, 0, "postgres", ProcState::Ready, 8, mem4),
            Process::new(2231, Self::COR2, 0, "cron", ProcState::Ready, 2, mem5),
            Process::new(2390, Self::COR5, 0, "rustc", ProcState::Ready, 16, mem6),
        ];

        let mem_template =  [NULL; GRID_COLS * GRID_ROWS]; 

        let mut app = Self {
            windows: BTreeMap::new(),
            selected: processes.first().map(|p| p.pid),
            focus: None,
            processes,
            draft: Draft::default(),
            // next_pid: 2400,
            rng: Rng::from_time(),
            grid: Vec::from(mem_template),
            interval_ms: 3000.0,
            elapsed_ms: 0.0,
            last_tick: Instant::now(),
            detail_cache: canvas::Cache::default(),
            grid_cache: canvas::Cache::default(),
        };  

        App::allocate_memory(&mut app.grid, Self::COR0, 2);
        App::allocate_memory(&mut app.grid, Self::COR3, 2);
        App::allocate_memory(&mut app.grid, Self::COR1, 8);
        App::allocate_memory(&mut app.grid, Self::COR4, 8);
        App::allocate_memory(&mut app.grid, Self::COR2, 2);
        App::allocate_memory(&mut app.grid, Self::COR5, 16);

        // app.randomize_grid();

        // app.render_memory();

        let open = Task::batch(Kind::ALL.into_iter().map(open_window));

        (app, open)
    }

    fn pid_exists(&self, pid: u32) -> bool {
        self.processes.iter().any(|p| p.pid == pid)
    }

    /// Sorteia um PID que ainda não existe em nenhum item da lista.
    fn generate_pid(&mut self) -> Option<u32> {
        let span = (PID_MAX - PID_MIN + 1) as u64;

        for _ in 0..64 {
            let pid = PID_MIN + self.rng.below(span) as u32;

            if !self.pid_exists(pid) {
                return Some(pid);
            }
        }

        // Faixa quase cheia: procura o primeiro PID livre, em ordem.
        (PID_MIN..=PID_MAX).find(|pid| !self.pid_exists(*pid))
    }

    fn title(&self, id: window::Id) -> String {
        self.windows
            .get(&id)
            .map_or("Multijanelas", |kind| kind.title())
            .to_string()
    }

    fn theme(&self, _id: window::Id) -> Option<Theme> {
        Some(Theme::Dark)
    }

    fn selected_index(&self) -> Option<usize> {
        let pid = self.selected?;

        self.processes.iter().position(|p| p.pid == pid)
    }

    fn focused_index(&self) -> Option<usize> {
        let pid = self.focus?;

        self.processes.iter().position(|p| p.pid == pid)
    }

    fn selected_process(&self) -> Option<&Process> {
        self.processes.get(self.selected_index()?)
    }

    // fn randomize_grid(&mut self) {
    //     self.grid = (0..GRID_COLS * GRID_ROWS)
    //         .map(|_| self.rng.below(GRID_CODES) as u8)
    //         .collect();
    // }

    /// Encontra o próximo bloco disponível contendo n células adjacentes livres
    fn find_free_memory(memory: &[u8], cells: usize) -> Option<usize> {
        memory
            .windows(cells)
            .position(|block| block.iter().all(|&cell| cell == NULL))
    }

    /// Preenche o bloco com a cor atual 99=vazio
    fn allocate_memory(memory: &mut [u8], code: u8, cells: usize) -> Option<usize> {
        let start = App::find_free_memory(memory, cells).expect("MEMÓRIA INSUFICIENTE");

        for cell in &mut memory[start..start + cells] {
            *cell = code;
        }

        Some(start)
    }

    fn free_memory(memory: &mut [u8], start: usize, cells: usize) -> Option<usize> { 

        for cell in &mut memory[start..start + cells] {
            *cell = NULL;
        }

        Some(start)
    }

    // fn render_memory(&mut self, blocks: &[MemoryBlock]) {
    //     self.grid.fill(0);

    //     for block in blocks {
    //         for i in 0..block.length {
    //             let (x, y) = (block.x, block.y + i);
    //             // let (x, y) = match block.direction {
    //             //     Direction::Row => (block.x + i, block.y),
    //             //     Direction::Column => (block.x, block.y + i),
    //             // };

    //             if x < GRID_COLS && y < GRID_ROWS {
    //                 let index = y * GRID_COLS + x;
    //                 self.grid[index] = block.code;
    //             }
    //         }
    //     }
    // }

    fn reset_draft(&mut self) {
        self.draft = Draft {
            pid: self.generate_pid(),
            ..Draft::default()
        };
    }

    fn window_of(&self, kind: Kind) -> Option<window::Id> {
        self.windows
            .iter()
            .find_map(|(id, k)| (*k == kind).then_some(*id))
    }

    fn close_register(&mut self) -> Task<Message> {
        match self.window_of(Kind::Register) {
            Some(id) => {
                self.windows.remove(&id);

                window::close(id)
            }
            None => Task::none(),
        }
    }

    /// Vai para o próximo PID da lista e sorteia uma nova grade.
    fn advance(&mut self) {
        
        if let Some(pid) = self.selected {
            if let Some(process) = self.processes.iter_mut().find(|p| p.pid == pid) {
                 if process.state != ProcState::Terminated {
                process.state = ProcState::Ready;
            }
                
            }
        } // self.processes[self.selected].state = ProcState::Ready; se IOBOUND ::Blocked 
        
        self.selected = if self.processes.is_empty() {
            None
        } else {
            let next = self.selected_index()
                .map_or(0, |i| (i + 1) % self.processes.len());

            if self.processes[next].state != ProcState::Terminated {
                self.processes[next].state = ProcState::Running;
            }

            Some(self.processes[next].pid)
        };

        // self.randomize_grid();
        self.detail_cache.clear();
        self.grid_cache.clear();
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(kind) => match self.window_of(kind) {
                Some(id) => window::gain_focus(id), // já aberta: só foca
                None => {
                    if kind == Kind::Register {
                        self.reset_draft(); // formulário novo, já com um PID livre
                    }

                    open_window(kind)
                }
            },
                        Message::WindowOpened(id, kind) => {
                self.windows.insert(id, kind);

                if kind == Kind::Register {
                    operation::focus("register-name")
                } else {
                    Task::none()
                }
            }

            Message::WindowClosed(id) => match self.windows.remove(&id) {
                Some(Kind::Menu) => iced::exit(), // fechar o Menu encerra o programa
                _ => Task::none(),
            },
            Message::Tick(now) => {
                let dt = now.saturating_duration_since(self.last_tick);

                self.last_tick = now;
                self.elapsed_ms += dt.as_secs_f32() * 1000.0;

                if self.elapsed_ms >= self.interval_ms {
                    self.elapsed_ms = 0.0;
                    let index_to_remove = self.selected_index();
                    self.advance();
                     {
                        if let Some(id) = index_to_remove {
                            let proc = self.processes[id].clone();
                            if proc.state == ProcState::Terminated {
                                let (col, row, cells) = (proc.memory.x, proc.memory.y, proc.memory.length);
                                let start = coord_to_index(col, row); 
                                App::free_memory(&mut self.grid, start, cells);
                                self.processes.remove(id);
                            }
                        }
                    }
                }                        

                Task::none()    
            }
            Message::Focus(pid) => {
                self.focus = Some(pid);
                // self.elapsed_ms = 0.0;
                // self.detail_cache.clear();

                Task::none()
            }
            Message::DraftName(name) => {
                self.draft.name = name;

                Task::none()
            }
            Message::DraftColor(code) => {
                self.draft.code = code;

                Task::none()
            }
            Message::DraftPriority(priority) => {
                self.draft.priority = priority;

                Task::none()
            }
            Message::DraftCelulas(value) => {
                self.draft.celulas = value;

                Task::none()
            }
            Message::RegeneratePid => {
                self.draft.pid = self.generate_pid();

                Task::none()
            }
            Message::SaveProcess => {
                let name = self.draft.name.trim().to_string();

                if name.is_empty() {
                    return Task::none();
                }

                // O PID do rascunho pode ter sido ocupado nesse meio tempo:
                // confere de novo na lista e, se preciso, gera outro.
                let pid = self
                    .draft
                    .pid
                    .filter(|pid| !self.pid_exists(*pid))
                    .or_else(|| self.generate_pid());

                let Some(pid) = pid else {
                    return Task::none();
                };

                let tamanho_memoria  = self.draft.celulas.clamp(2, 16) as usize;
                let nova_cor = self.draft.code;
                let free_memory = App::find_free_memory(&self.grid, tamanho_memoria);
                let memory = MemoryBlock::new((0, 0), 0, 0).offset(free_memory.unwrap_or(0), tamanho_memoria, nova_cor);
                App::allocate_memory(&mut self.grid, nova_cor, tamanho_memoria);

                self.processes.push(Process {
                    pid,
                    code: self.draft.code,
                    priority: self.draft.priority,
                    name,
                    state: ProcState::Novo,
                    celulas: self.draft.celulas,
                    memory,
                });

                // self.selected = Some(pid);
                // self.elapsed_ms = 0.0;
                // self.detail_cache.clear();

                self.close_register()
            }
            Message::CancelRegister => self.close_register(),

            Message::StopFocused => {
                if let Some(i) = self.focused_index() {
                    self.processes[i].state = ProcState::Suspend;
                    self.detail_cache.clear();
                }

                Task::none()
            }
            Message::RemoveFocused => {

                if let Some(i) = self.focused_index() {
                    self.processes[i].state = ProcState::Terminated;
                } else if let Some(process) = self.processes.last_mut() {
                    self.focus = Some(process.pid);
                    process.state = ProcState::Terminated;
                }

                Task::none()
            }
            Message::IntervalChanged(ms) => {
                self.interval_ms = ms;
                self.elapsed_ms = self.elapsed_ms.min(ms);

                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        fn hotkey(event: keyboard::Event) -> Option<Message> {
            use keyboard::key;

            let keyboard::Event::KeyPressed { modified_key, .. } = event else {
                return None;
            };

            match modified_key.as_ref() {
                keyboard::Key::Named(key::Named::F4) => Some(Message::Open(Kind::Tasks)),
                keyboard::Key::Named(key::Named::F5) => Some(Message::Open(Kind::Detail)),
                keyboard::Key::Named(key::Named::F6) => Some(Message::Open(Kind::Grid)),
                _ => None,
            }
        }

        Subscription::batch(vec![
            time::every(milliseconds(20)).map(Message::Tick), // tick automático
            window::close_events().map(Message::WindowClosed),
            keyboard::listen().filter_map(hotkey),
        ])
    }

    // ───────────────────────────── views ─────────────────────────────

    fn view(&self, id: window::Id) -> Element<'_, Message> {
        match self.windows.get(&id) {
            Some(Kind::Menu) => self.menu_view(),
            Some(Kind::Tasks) => self.tasks_view(),
            Some(Kind::Detail) => self.detail_view(),
            Some(Kind::Grid) => self.grid_view(),
            Some(Kind::Register) => self.register_view(),
            None => space().into(),
        }
    }

    fn menu_view(&self) -> Element<'_, Message> {
        let entry = |label: &'static str, kind: Kind| {
            button(text(label)).width(Fill).on_press(Message::Open(kind))
        };

        let current = self
            .selected_process()
            .map_or("—".to_string(), |p| format!("{} ({})", p.pid, p.name));

        let remaining = (self.interval_ms - self.elapsed_ms).max(0.0) / 1000.0;

        let content = column![
            text("Menu").size(28),
            entry("F4  ·  Lista de Processos", Kind::Tasks),
            entry("F5  ·  CPU - Núcleo 1", Kind::Detail),
            entry("F6  ·  Memória", Kind::Grid),
            text!("PID atual: {current}"),
            text!("Próximo PID em {remaining:.1} s"),
        ]
        .spacing(12);

        container(content).padding(20).into()
    }

    fn tasks_view(&self) -> Element<'_, Message> {
        let toolbar = row![
            button("novo").on_press(Message::Open(Kind::Register)),
            button("parar").on_press_maybe(self.selected.map(|_| Message::StopFocused)),
            button("remover").on_press_maybe(self.selected.map(|_| Message::RemoveFocused)),
        ]
        .spacing(8);

        let header = container(
            row![
                text("Cor").width(40),
                text("PID").width(70),
                text("Nome").width(Fill),
                text("Estado").width(90),
            ]
            .spacing(10),
        )
        .padding(10);

        let rows = self.processes.iter().map(|p| {
            let selected = self.selected == Some(p.pid);
            let focus = self.focus == Some(p.pid);

            let line = row![
                swatch(p.color(), 40.0),
                text(p.pid.to_string()).width(70),
                text(p.name.as_str()).width(Fill),
                text(p.state.label()).width(90),
            ]
            .spacing(10)
            .align_y(Center);

            button(line)
                .width(Fill)
                .on_press(Message::Focus(p.pid))
                .style(move |theme, status| {
                    if selected  {
                        button::secondary(theme, status)
                    } else if focus {
                        button::primary(theme, status)
                    } else {
                        button::text(theme, status)
                    }
                })
                .into()
        });

        let list = container(scrollable(column(rows).spacing(2)).height(Fill))
            .style(container::bordered_box)
            .height(Fill);

        container(column![toolbar, header, list].spacing(10))
            .padding(10)
            .into()
    }

    fn detail_view(&self) -> Element<'_, Message> {
        let process = self.selected_process();

        let preview = Canvas::new(Preview {
            process,
            cache: &self.detail_cache,
        })
        .width(Fill)
        .height(180);

        let field = |label: &'static str, value: String| {
            column![text(label).size(13), text_input("", &value)].spacing(4)
        };

        let pid = process.map_or("—".to_string(), |p| p.pid.to_string());
        let name = process.map_or("—".to_string(), |p| p.name.clone());
        let state = process.map_or("—".to_string(), |p| p.state.label().to_string());
        let color = process.map_or("—".to_string(), |p| {
            format!("{:02} · {}", p.code, color_name(p.code))
        });

        let timer = column![
            text!("Progresso: {:.0} / {:.0} ms", self.elapsed_ms, self.interval_ms).size(13),
            progress_bar(0.0..=self.interval_ms, self.elapsed_ms),
            row![
                text("t (ms)").size(13),
                slider(500.0..=10_000.0, self.interval_ms, Message::IntervalChanged).step(100.0_f32),
            ]
            .spacing(10)
            .align_y(Center),
        ]
        .spacing(6);

        container(
            column![
                preview,
                row![field("PID", pid), field("Estado", state)].spacing(10),
                row![field("Nome", name), field("Cor", color)].spacing(10),
                timer,
            ]
            .spacing(14),
        )
        .padding(16)
        .into()
    }

    fn grid_view(&self) -> Element<'_, Message> {
        let grid = Canvas::new(LifeGrid {
            cells: &self.grid,
            cache: &self.grid_cache,
        })
        .width(Fill)
        .height(Fill);

        let legend = row![
            swatch(palette(0), 16.0),
            text("").size(13),
            swatch(palette(3), 16.0),
            text("").size(13),
            text("").size(13),
        ]
        .spacing(8)
        .align_y(Center);

        container(column![grid, legend].spacing(10))
            .padding(10)
            .into()
    } 

    /// Janela "Novo processo": nome, cor, prioridade (0..15) e PID automático.
    fn register_view(&self) -> Element<'_, Message> {
        let draft = &self.draft;

        let pid_free = draft.pid.is_some_and(|pid| !self.pid_exists(pid));
        let can_save = pid_free && !draft.name.trim().is_empty();

        let pid_text = draft.pid.map_or("—".to_string(), |pid| pid.to_string());
        let pid_status = match draft.pid {
            Some(pid) if !self.pid_exists(pid) => "PID livre: não existe na lista",
            Some(_) => "Esse PID já existe na lista, gere outro",
            None => "Não há PID livre",
        };

        let name = column![
            text("Nome do processo").size(13),
            text_input("ex.: nginx", &draft.name)
                .id("register-name")
                .on_input(Message::DraftName)
                .on_submit(Message::SaveProcess),
        ]
        .spacing(4);

        // Seletor de cor: um botão por cor da paleta
        let swatches: Vec<Element<'_, Message>> = (0..6u8)
            .map(|code| {
                let selected = draft.code == code;

                button(swatch(palette(code), 28.0))
                    .on_press(Message::DraftColor(code))
                    .style(move |theme, status| {
                        if selected {
                            button::primary(theme, status)
                        } else {
                            button::secondary(theme, status)
                        }
                    })
                    .into()
            })
            .collect();

        let color = column![
            text!("Cor: {:02} · {}", draft.code, color_name(draft.code)).size(13),
            row(swatches).spacing(6),
        ]
        .spacing(4);

        // Seletor de prioridade 0..=15
        let priority = column![
            text!("Prioridade: {}", draft.priority).size(13),
            slider(0..=15, draft.priority, Message::DraftPriority),
        ]
        .spacing(4);

        let celulas: Column<'_, Message> = column![
        text!("Células: {}", draft.celulas).size(13),

        text_input("2..16", &draft.celulas.to_string())
            .on_input(|value| {
                Message::DraftCelulas(
                    value.parse::<u8>().unwrap_or(2).clamp(2, 16)
                )
            })
            .width(80),
        ]
        .spacing(4);

        // PID gerado automaticamente e conferido contra a lista
        let pid = column![
            text("PID").size(13),
            row![
                text_input("", &pid_text),
                button("↻")
                .style(button::secondary)
                .on_press(Message::RegeneratePid),
            ]
            .spacing(8)
            .align_y(Center),
            text(pid_status).size(12),
        ]
        .spacing(4);

        let actions = row![
            button("Cancelar")
                .style(button::secondary)
                .on_press(Message::CancelRegister),
            button("Salvar").on_press_maybe(can_save.then_some(Message::SaveProcess)),
        ]
        .spacing(10);

        container(
            column![text("Novo processo").size(24), name, color, priority, celulas, pid, actions]
                .spacing(16),
        )
        .padding(20)
        .into()
    }
}

/// Quadradinho colorido usado na coluna "Cor" e na legenda.
fn swatch<'a>(color: Color, width: f32) -> Element<'a, Message> {
    container(space())
        .width(width)
        .height(16)
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            ..container::Style::default()
        })
        .into()
}

// ───────────────────────────── canvas: F2 ─────────────────────────────

/// Retângulo do PID selecionado (cor = cor do processo; tamanho varia com o PID).
struct Preview<'a> {
    process: Option<&'a Process>,
    cache: &'a canvas::Cache,
}

/// Interface Gráfica do Preview Nucleo de CPU
impl canvas::Program<Message> for Preview<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let drawing = self.cache.draw(renderer, bounds.size(), |frame| {
            let size = frame.size();

            frame.fill_rectangle(Point::ORIGIN, size, background());

            if let Some(process) = self.process {
                let w = 80.0;
                let h = 60.0;
                let origin = Point::new((size.width - w) / 2.0, (size.height - h) / 2.0);

                let mut color = process.color();

                if process.state == ProcState::Blocked {
                    color.a = 0.35;
                }

                let radius = Radius::from(8.0);

                // 2. Desenha a moldura branca arredondada
                let border_origin = Point::new(origin.x - 1.5, origin.y - 1.5);
                let border_size = Size::new(w + 3.0, h + 3.0);
                let border_path = Path::rounded_rectangle(border_origin, border_size, radius);
                let cor_borda = Color::from_rgba8(0x1F, 0x75, 0xFE, 1.0);
                frame.fill(&border_path, cor_borda);

                // 3. Desenha o retângulo colorido arredondado por cima
                // Dica: Use um raio ligeiramente menor no interior para um visual proporcional, ou o mesmo (radius)
                let inner_radius = Radius::from(6.5); 
                let inner_path = Path::rounded_rectangle(origin, Size::new(w, h), inner_radius);

                let cor_inside = Color::from_rgba8(0xEC, 0xDF, 0xDF, 0.9);
                frame.fill(&inner_path, cor_inside);

                // 4. Calcula o centro do retângulo e desenha o círculo colorido
                let center = Point::new(
                    origin.x + (w / 2.0),
                    origin.y + (h / 2.0)
                );
                let circle_radius = 12.0; // Defina o tamanho do círculo aqui (raio)

                let circle_path = Path::circle(center, circle_radius);
                frame.fill(&circle_path, color); // Preenche o círculo com a cor do processo
            }
        });

        vec![drawing]
    }
}

// ───────────────────────────── canvas: F3 ─────────────────────────────

/// Grade estilo Game of Life com retângulos coloridos por código.
struct LifeGrid<'a> {
    cells: &'a [u8],
    cache: &'a canvas::Cache,
}

impl canvas::Program<Message> for LifeGrid<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let drawing = self.cache.draw(renderer, bounds.size(), |frame| {
            let size = frame.size();

            frame.fill_rectangle(Point::ORIGIN, size, background());

            let cell = (size.width / GRID_COLS as f32).min(size.height / GRID_ROWS as f32);
            let offset = Point::new(
                (size.width - cell * GRID_COLS as f32) / 2.0,
                (size.height - cell * GRID_ROWS as f32) / 2.0,
            );
            let gap = (cell * 0.1).max(1.0);

            for col in 0..GRID_COLS {
                for row in 0..GRID_ROWS {
                    let color = self
                        .cells
                        .get(coord_to_index(col, row))
                        .copied()
                        .and_then(grid_color)
                        .unwrap_or_else(empty_cell);

                    frame.fill_rectangle(
                        Point::new(
                            offset.x + col as f32 * cell + gap / 2.0,
                            offset.y + row as f32 * cell + gap / 2.0,
                        ),
                        Size::new(cell - gap, cell - gap),
                        color,
                    );
                }
            }
        });

        vec![drawing]
    }
}