use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use elevator::{
    args::Args,
    controls::Controls,
    policies,
    simulation::Simulation,
    traffic::{self, Traffic},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, LegendPosition, Paragraph},
};
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(20);

const POLICY_COLORS: [Color; 5] = [
    Color::Indexed(99),
    Color::Indexed(117),
    Color::Indexed(33),
    Color::Indexed(200),
    Color::Indexed(82),
];

fn traffic(floors: usize, scale: f64) -> Box<dyn Traffic> {
    let lull = Box::new(traffic::Random::new(floors, vec![floors as f64], vec![floors as f64], scale));
    let spike = Box::new(traffic::Random::new(floors, vec![floors as f64], vec![floors as f64], 10.0 * scale));
    Box::new(traffic::Cycle::new(vec![lull, spike], vec![60_000, 10_000]))
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let b = args.building();
    let floors = b.num_floors();

    let quantiles = [0.5, 0.95, 0.99, 0.999];
    let mut q_idx = 0;
    let mut vis = Visualization::new(floors, b.elevators.len());
    let mut vis_idx = 0;

    let mut controls = Controls::default();

    let mut sims: Vec<Simulation> = args
        .policies
        .iter()
        .map(|name| {
            let t = traffic(floors, controls.traffic_scale());
            Simulation::new(b.clone(), policies::new(name, &b), t, &controls)
        })
        .collect();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            // Horizontal split: Left (Charts/Controls) and Right (Elevator Vis)
            let app_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(f.area());

            // Vertical split for the left side: Controls, Throughput, and Latency
            let left_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),      // Controls
                    Constraint::Percentage(50), // Throughput Comparison
                    Constraint::Percentage(50), // Latency Time Series
                ])
                .split(app_layout[0]);

            f.render_widget(
                Paragraph::new(controls.to_string()).block(Block::default().borders(Borders::ALL).title("Simulation Controls")),
                left_layout[0],
            );

            vis.update(&sims[vis_idx], &args.policies[vis_idx]);
            vis.render(f, app_layout[1], &sims[vis_idx]);

            for sim in &mut sims {
                sim.stats.trim(left_layout[1].width.into());
            }

            let start = sims[0].stats.start / 1000;
            let end = sims[0].stats.end() / 1000;

            let throughput = sims.iter().map(|s| s.stats.throughput().map(move |v| *v as f64)).collect();
            render_graph(f, left_layout[1], &args.policies, throughput, "Throughput", start, end);

            let q = quantiles[q_idx];
            let latency = sims.iter().map(|s| s.stats.latency(q).map(move |v| v / 1000.0)).collect();
            let title = format!("Latency (Seconds) P{}", q * 100.0);
            render_graph(f, left_layout[2], &args.policies, latency, &title, start, end);
        })?;

        // Input Handling
        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                // Only process the initial key press to avoid double-triggers on Windows
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Right => q_idx = (q_idx + 1) % quantiles.len(),
                    KeyCode::Left => q_idx = q_idx.saturating_sub(1),
                    KeyCode::Tab => vis_idx = (vis_idx + 1) % sims.len(),
                    _ => {}
                }

                controls.handle_input(key.code, &mut sims);
            }
        }

        // Tick Simulation
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        last_tick = now;
        let sim_step = elapsed.as_millis() as u64 * controls.speed();
        for sim in &mut sims {
            sim.tick(sim.building.prev_time() + sim_step);
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

const ELEV_COLORS: [Color; 10] = [
    Color::from_u32(0x5e4fa2),
    Color::from_u32(0x3288bd),
    Color::from_u32(0x66c2a5),
    Color::from_u32(0xabdda4),
    Color::from_u32(0xe6f598),
    Color::from_u32(0xfee08b),
    Color::from_u32(0xfdae61),
    Color::from_u32(0xf46d43),
    Color::from_u32(0xd53e4f),
    Color::from_u32(0x9e0142),
];

pub struct Visualization {
    floor_labels: Vec<String>,
    elevator_bufs: Vec<String>,
    plus_bufs: Vec<String>,
    title: String,
    elevator_rows: Vec<u64>,
}

impl Visualization {
    pub fn new(floors: usize, elevators: usize) -> Self {
        let floor_labels = (0..floors).map(|i| format!("{:>2} ", i)).collect();
        Self {
            floor_labels,
            elevator_bufs: vec![String::with_capacity(4); elevators],
            plus_bufs: vec![String::with_capacity(4); floors * elevators],
            title: Default::default(),
            elevator_rows: vec![0; elevators],
        }
    }

    fn title(&mut self, name: &str) {
        self.title.clear();
        self.title.push_str("Visualization (");
        self.title.push_str(name);
        self.title.push(')');
    }

    fn waiting_lines<'a: 'b, 'b>(&'a self, spans: &mut Vec<Span<'b>>, sim: &Simulation, floor: usize) {
        spans.push(Span::raw(" "));
        const FACES: &'static str = "☺☺☺☺☺☺";
        let elevators = self.elevator_bufs.len();
        for e_idx in 0..elevators {
            let style = Style::default().fg(ELEV_COLORS[e_idx % ELEV_COLORS.len()]);
            let count = sim.building.waiting_for_elevator(floor, e_idx);
            let show = count.min(FACES.chars().count());
            if show > 0 {
                spans.push(Span::styled(&FACES[..(show * "☺".len())], style));
            }
            if count > show {
                spans.push(Span::styled(self.plus_bufs[floor * elevators + e_idx].as_str(), style));
            }
        }
    }

    fn rows(floors: usize) -> impl Iterator<Item = (usize, usize, bool)> {
        (0..floors * 2 - 1).rev().map(|row| (row, row / 2, row % 2 == 0))
    }

    pub fn update(&mut self, sim: &Simulation, name: &str) {
        let building = &sim.building;
        let floors = building.num_floors();
        let tpf = building.time_per_floor();
        for (i, e) in building.elevators.iter().enumerate() {
            self.elevator_rows[i] = (e.pos * 2 + tpf / 2) / tpf;
        }
        self.title(name);
        for (row, floor, is_floor) in Self::rows(floors) {
            for (e_idx, e) in building.elevators.iter().enumerate() {
                if self.elevator_rows[e_idx] == row as u64 {
                    let buf = &mut self.elevator_bufs[e_idx];
                    buf.clear();
                    buf.push('[');
                    buf.push_str(itoa::Buffer::new().format(e.passengers.len()));
                    buf.push_str("] ");
                }
            }
            if is_floor {
                let elevators = building.elevators.len();
                for e_idx in 0..elevators {
                    let count = building.waiting_for_elevator(floor, e_idx);
                    let shown = count.min(6);
                    if count > shown {
                        let i = floor * elevators + e_idx;
                        self.plus_bufs[i].clear();
                        self.plus_bufs[i].push('+');
                        self.plus_bufs[i].push_str(itoa::Buffer::new().format(count - shown));
                    }
                }
            }
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, sim: &Simulation) {
        let building = &sim.building;
        let floors = building.num_floors();
        let elevators = &building.elevators;

        let mut lines = Vec::with_capacity(64);
        let content_height = floors as u16 * 2;
        let inner_height = area.height.saturating_sub(2);
        let top_padding = inner_height.saturating_sub(content_height);
        for _ in 0..top_padding {
            lines.push(Line::default());
        }

        for (row, floor, is_floor) in Self::rows(floors) {
            let mut spans = Vec::with_capacity(2 + elevators.len() * 2);

            // floor label
            spans.push(Span::raw(if is_floor { self.floor_labels[floor].as_str() } else { "   " }));

            // elevators
            for e_idx in 0..elevators.len() {
                let style = Style::default().fg(ELEV_COLORS[e_idx % ELEV_COLORS.len()]);
                if self.elevator_rows[e_idx] == row as u64 {
                    spans.push(Span::styled(self.elevator_bufs[e_idx].as_str(), style));
                } else {
                    spans.push(Span::styled(" ║  ", style));
                }
            }

            // waiting people
            if is_floor {
                self.waiting_lines(&mut spans, &sim, floor);
            }

            lines.push(Line::from(spans));
        }

        let block = Block::default().borders(Borders::ALL).title(self.title.as_str());
        f.render_widget(Paragraph::new(lines).block(block), area);
    }
}

fn render_graph<I>(f: &mut Frame, area: Rect, policies: &[String], y_data: Vec<I>, title: &str, start: u64, end: u64)
where
    I: Iterator<Item = f64>,
{
    let mut max_y = 0.0f64;
    let points: Vec<Vec<(f64, f64)>> = y_data
        .into_iter()
        .map(|y_datum| {
            y_datum
                .enumerate()
                .map(|(i, v)| {
                    max_y = max_y.max(v);
                    (i as f64, v)
                })
                .collect()
        })
        .collect();

    let datasets: Vec<Dataset> = (0..points.len())
        .map(|i| {
            Dataset::default()
                .name(policies[i].as_str())
                .marker(symbols::Marker::Octant)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(POLICY_COLORS[i % POLICY_COLORS.len()]))
                .data(&points[i])
        })
        .collect();

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(title))
        .legend_position(Some(LegendPosition::TopLeft))
        .hidden_legend_constraints((Constraint::Min(0), Constraint::Min(0)))
        .x_axis(
            Axis::default()
                .title("Time (Seconds)")
                .bounds([0.0, area.width as f64])
                .labels(vec![Line::from(format!("{:<6}", start)), Line::from(format!("{:>6}", end))]),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max_y * 1.2])
                .labels(vec![Line::from("0"), Line::from(format!("{:<8.2}", max_y))]),
        );
    f.render_widget(chart, area);
}
