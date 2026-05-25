use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use elevator::args::Args;
use elevator::policy::{Decision, Policy};
use elevator::stats::Stats;
use elevator::traffic::{Random, Traffic};
use elevator::{Building, policies};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, LegendPosition, Paragraph},
};
use std::time::{Duration, Instant};

const MAX_SPEED: u64 = 1048576;
const START_SPEED: u64 = 1;
const INITIAL_WINDOW_MS: u64 = 1_000;
const INITIAL_TRAFFIC_SCALE: f64 = 0.1;

const TICK_RATE: Duration = Duration::from_millis(20);

const ELEV_COLORS: [Color; 6] = [Color::Red, Color::Green, Color::Yellow, Color::Blue, Color::Magenta, Color::Cyan];

struct SimInstance {
    color: Color,
    building: Building,
    policy: Box<dyn Policy>,
    traffic: Box<dyn Traffic>,
    decision: Decision,
    stats: Stats,
}

fn sim<P: Policy + 'static>(color: Color, building: Building, traffic: Box<dyn Traffic>, stats: Stats) -> SimInstance {
    let decision = Decision::new(building.elevators.len());
    let policy = Box::new(P::new(&building));
    SimInstance {
        color,
        building,
        policy,
        traffic,
        decision,
        stats,
    }
}

fn create_sims(floors: usize, elevators: usize, window_ms: u64, traffic_scale: f64) -> Vec<SimInstance> {
    let b = Building::new(floors, elevators);
    let traffic = Box::new(Random::new(floors, vec![floors as f64], vec![floors as f64], traffic_scale));
    let stats = Stats::new(window_ms);

    vec![
        sim::<policies::Simple>(Color::Indexed(99), b.clone(), traffic.clone(), stats.clone()),
        sim::<policies::Scan>(Color::Indexed(117), b.clone(), traffic.clone(), stats.clone()),
        sim::<policies::Gemini2>(Color::Indexed(200), b.clone(), traffic.clone(), stats.clone()),
        sim::<policies::OpenAi>(Color::Indexed(82), b.clone(), traffic.clone(), stats.clone()),
        sim::<policies::Bogo>(Color::Indexed(33), b.clone(), traffic.clone(), stats.clone()),
    ]
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let floors = args.floors;
    let elevators = args.elevators;

    let quantiles = [0.5, 0.95, 0.99, 0.999];
    let mut q_idx = 0;
    let mut vis_idx = 0;
    let mut current_window_ms = INITIAL_WINDOW_MS;
    let mut traffic_scale = INITIAL_TRAFFIC_SCALE;

    let mut sims = create_sims(floors, elevators, current_window_ms, traffic_scale);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut last_tick = Instant::now();
    let mut sim_speed: u64 = START_SPEED;

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

            // 1. Controls Panel
            let speed_text = format!(
                " [Q] Quit    [+/-] Speed ({}x)    [</>] Window ({}s)    [←/→] P    [↑/↓] Traffic ({} {:.3}p/s)    [Tab] Vis",
                sim_speed,
                current_window_ms / 1000,
                sims[0].traffic.name(),
                traffic_scale,
            );
            f.render_widget(
                Paragraph::new(speed_text).block(Block::default().borders(Borders::ALL).title("Simulation Controls")),
                left_layout[0],
            );

            // 2. Elevator Visualization Panel (Full Height on Right)
            let vis_sim = &sims[vis_idx];
            let mut vis_lines = Vec::new();
            let tpf = vis_sim.building.time_per_floor();

            for f_idx in (0..floors).rev() {
                let mut line_spans = vec![Span::raw(format!("{:>2} ", f_idx))];
                for (e_idx, e) in vis_sim.building.elevators.iter().enumerate() {
                    let color = ELEV_COLORS[e_idx % ELEV_COLORS.len()];
                    let pos_f = e.pos as f64 / tpf as f64;
                    if (pos_f - f_idx as f64).abs() <= 0.25 {
                        let elevator_icon = format!("[{}] ", e.passengers.len());
                        line_spans.push(Span::styled(elevator_icon, Style::default().fg(color)));
                    } else {
                        line_spans.push(Span::styled(" ║  ", Style::default().fg(Color::White)));
                    }
                }
                line_spans.push(Span::raw(" "));
                for e_idx in 0..vis_sim.building.elevators.len() {
                    let color = ELEV_COLORS[e_idx % ELEV_COLORS.len()];
                    let count = vis_sim.building.waiting_for_elevator(f_idx, e_idx);
                    if count > 0 {
                        line_spans.push(Span::styled("☺".repeat(count), Style::default().fg(color)));
                    }
                }
                vis_lines.push(Line::from(line_spans));

                if f_idx > 0 {
                    let mut spacer_spans = vec![Span::raw("   ")];
                    for (e_idx, e) in vis_sim.building.elevators.iter().enumerate() {
                        let color = ELEV_COLORS[e_idx % ELEV_COLORS.len()];
                        let pos_f = e.pos as f64 / tpf as f64;
                        if pos_f < f_idx as f64 - 0.25 && pos_f > f_idx as f64 - 0.75 {
                            let elevator_icon = format!("[{}] ", e.passengers.len());
                            spacer_spans.push(Span::styled(elevator_icon, Style::default().fg(color)));
                        } else {
                            spacer_spans.push(Span::styled(" ║  ", Style::default().fg(Color::White)));
                        }
                    }
                    vis_lines.push(Line::from(spacer_spans));
                }
            }
            f.render_widget(
                Paragraph::new(vis_lines).block(Block::default().borders(Borders::ALL).title(Span::styled(
                    format!("Visualization ({})", vis_sim.policy.name()),
                    Style::default().add_modifier(Modifier::BOLD),
                ))),
                app_layout[1],
            );

            // Combined Throughput Chart with Panning and Axes
            let chart_area = left_layout[1];
            let max_data_points = chart_area.width.saturating_sub(10) as usize;
            for sim in &mut sims {
                sim.stats.trim(max_data_points);
            }
            let history_len = sims[0].stats.len();

            let mut global_max_y = 5.0f64;
            let mut datasets_data = Vec::with_capacity(sims.len());

            for sim in &sims {
                let history = sim.stats.throughput_history();
                let points: Vec<(f64, f64)> = history
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &v)| {
                        let x = i as f64 + max_data_points as f64 - history_len as f64;
                        if x >= 0.0 && x < max_data_points as f64 {
                            let val = v as f64;
                            if val > global_max_y {
                                global_max_y = val;
                            }
                            Some((x, val))
                        } else {
                            None
                        }
                    })
                    .collect();
                datasets_data.push(points);
            }

            let datasets: Vec<Dataset> = sims
                .iter()
                .enumerate()
                .map(|(i, sim)| {
                    Dataset::default()
                        .name(sim.policy.name())
                        .marker(symbols::Marker::Octant)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(sim.color))
                        .data(&datasets_data[i])
                })
                .collect();

            let start = sims[0].stats.start / 1000;
            let end = sims[0].stats.end() / 1000;
            let chart = Chart::new(datasets)
                .block(Block::default().borders(Borders::ALL).title("Throughput"))
                .legend_position(Some(LegendPosition::TopLeft))
                .hidden_legend_constraints((Constraint::Min(0), Constraint::Min(0)))
                .x_axis(
                    Axis::default()
                        .title("Time (Seconds)")
                        .bounds([0.0, max_data_points as f64])
                        .labels(vec![Line::from(format!("{:<6}", start)), Line::from(format!("{:>6}", end))]),
                )
                .y_axis(Axis::default().title("Served").bounds([0.0, global_max_y * 1.2]).labels(vec![
                    Line::from(format!("{:<6}", "0")),
                    Line::from(format!("{:<6.0}", global_max_y)),
                ]));

            f.render_widget(chart, chart_area);

            // 3. Combined Latency Percentile Chart (Time Series)
            let hist_area = left_layout[2];
            let q_target = quantiles[q_idx];
            let mut hist_max_y = 0.1f64;
            let mut hist_datasets_data = Vec::with_capacity(sims.len());

            let max_lat_points = hist_area.width.saturating_sub(10) as usize;
            for sim in &mut sims {
                sim.stats.trim(max_lat_points);
            }
            let lat_history_len = sims[0].stats.len();

            for sim in &sims {
                let history = sim.stats.latency_history(q_target);
                let points: Vec<(f64, f64)> = history
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &val)| {
                        let x = i as f64 + max_lat_points as f64 - lat_history_len as f64;
                        if x >= 0.0 && x < max_lat_points as f64 {
                            let val_sec = val / 1000.0;
                            if val_sec > hist_max_y {
                                hist_max_y = val_sec;
                            }
                            Some((x, val_sec))
                        } else {
                            None
                        }
                    })
                    .collect();
                hist_datasets_data.push(points);
            }

            let hist_datasets: Vec<Dataset> = sims
                .iter()
                .enumerate()
                .map(|(i, sim)| {
                    Dataset::default()
                        .name(sim.policy.name())
                        .marker(symbols::Marker::Octant)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(sim.color))
                        .data(&hist_datasets_data[i])
                })
                .collect();

            let hist_chart = Chart::new(hist_datasets)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Latency P{}", q_target * 100.0)),
                )
                .legend_position(Some(LegendPosition::TopLeft))
                .hidden_legend_constraints((Constraint::Min(0), Constraint::Min(0)))
                .x_axis(
                    Axis::default()
                        .title("Time (Seconds)")
                        .bounds([0.0, max_lat_points as f64])
                        .labels(vec![Line::from(format!("{:<6}", start)), Line::from(format!("{:>6}", end))]),
                )
                .y_axis(
                    Axis::default()
                        .title("Latency (Seconds)")
                        .bounds([0.0, hist_max_y * 1.1])
                        .labels(vec![
                            Line::from(format!("{:<8}", "0.00s")),
                            Line::from(format!("{:<8.2}s", hist_max_y)),
                        ]),
                );

            f.render_widget(hist_chart, hist_area);
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
                    KeyCode::Char('+') | KeyCode::Char('=') => sim_speed = (sim_speed * 2).min(MAX_SPEED),
                    KeyCode::Char('-') | KeyCode::Char('_') => sim_speed = (sim_speed / 2).max(1),
                    KeyCode::Right => q_idx = (q_idx + 1) % quantiles.len(),
                    KeyCode::Left => q_idx = q_idx.saturating_sub(1),
                    KeyCode::Tab => vis_idx = (vis_idx + 1) % sims.len(),
                    KeyCode::Char(',') | KeyCode::Char('<') => {
                        current_window_ms = (current_window_ms / 2).max(1000);
                        sims = create_sims(floors, elevators, current_window_ms, traffic_scale);
                    }
                    KeyCode::Char('>') | KeyCode::Char('.') => {
                        current_window_ms = (current_window_ms * 2).min(3600_000);
                        sims = create_sims(floors, elevators, current_window_ms, traffic_scale);
                    }
                    KeyCode::Down => {
                        traffic_scale = (traffic_scale - 0.1).max(0.1);
                        for sim in &mut sims {
                            sim.traffic.scale(traffic_scale);
                        }
                    }
                    KeyCode::Up => {
                        traffic_scale += 0.1;
                        for sim in &mut sims {
                            sim.traffic.scale(traffic_scale);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Tick Simulation
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        last_tick = now;

        let sim_step = elapsed.as_millis() as u64 * sim_speed;
        for sim in &mut sims {
            let target_time = sim.building.prev_time() + sim_step;
            sim.building.run(
                target_time,
                sim.policy.as_mut(),
                &mut sim.decision,
                sim.traffic.as_mut(),
                &mut sim.stats,
            );
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
